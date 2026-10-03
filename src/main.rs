//! `gitgit` — single-crate MVP for a local Git HTTP server.
//!
//! The binary is a thin CLI shell over the `gitgit` **library** target.
//! It deliberately does not re-declare `mod cli;` / `mod server;` and so
//! on: with a `[lib]` and a `[[bin]]` both present, re-declaring the
//! modules compiles every source file twice, and the two copies are free
//! to drift. Going through the library makes the binary and the
//! `apps/gm-desktop/src-tauri` consumer compile exactly one copy of the
//! business logic.

use anyhow::Context;
use clap::Parser;

use gitgit::ai::{diff::DiffRequest, Invocation, Task, API_KEY_ENV};
use gitgit::cli::{Cli, Command, GitaiCommand, GitaiTaskArgs, KeyCommand};
use gitgit::config::Config;
use gitgit::server::vault::{FileVault, VaultError};
use gitgit::server::vault_versioned::VersionedVault;
use gitgit::server::AppState;
use gitgit::{ai, repo, server};

/// Initialize logging once. Honors `RUST_LOG`, defaults to `info`.
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,gitgit=info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

fn build_config(cli: &Cli) -> Config {
    Config::new(
        cli.bind.clone(),
        cli.repos_dir.clone(),
        cli.vault_file_root.clone(),
    )
}

/// Build the V0 Credential Vault. Per ADR-0021 §1.2 + ADR-0022 §follow-up
/// the V0 close-out wires the default `FileVault` into `AppState`.
///
/// Return type is `Arc<dyn VersionedVault>` (not `Arc<dyn Vault>`)
/// because every CLI `vault *` subcommand in this commit already needs
/// the version-management surface (per ADR-0022); super-trait
/// auto-deref is available for any future handler that only needs the
/// 5 KV operations.
fn build_vault(config: &Config) -> std::sync::Arc<dyn VersionedVault> {
    let vault = FileVault::new(&config.vault_file_root);
    std::sync::Arc::new(vault)
}

async fn run_serve(config: Config) -> anyhow::Result<()> {
    config.ensure_repos_dir()?;
    config.ensure_vault_root()?;
    let bind = config.bind.clone();
    let vault = build_vault(&config);
    let state = AppState::new(config, vault);
    let router = server::build_router(state);
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind to {bind}"))?;
    tracing::info!(%bind, "listening on {bind}");
    axum::serve(listener, router)
        .await
        .context("axum::serve failed")?;
    Ok(())
}

async fn run_init_repo(config: &Config, name: &str) -> anyhow::Result<()> {
    let path = repo::create_bare_repo(config, name).await?;
    println!("created bare repo at {}", path.display());
    Ok(())
}

fn run_list(config: &Config) -> anyhow::Result<()> {
    config.ensure_repos_dir()?;
    let names = repo::list_repos(&config.repos_dir)?;
    if names.is_empty() {
        println!("(no repos)");
    } else {
        for n in names {
            println!("{n}");
        }
    }
    Ok(())
}

/// Dispatcher for every `vault *` subcommand. Per ADR-0022 the canonical
/// write surface is `set_with_version` (super-trait `Vault::set` bypasses
/// the timeline). Every action here is intentionally synchronous from
/// the CLI's perspective — the V0 binaries don't need streaming output.
fn run_key(config: &Config, sub: KeyCommand) -> anyhow::Result<()> {
    config.ensure_vault_root()?;
    let vault = build_vault(config);
    // The local runtime isn't async at the binary entrypoint (cmd is
    // `tokio::main`), so block on each call directly.
    let result: anyhow::Result<()> = match sub {
        KeyCommand::Set { key, value } => {
            let new_v =
                tokio::runtime::Handle::current().block_on(vault.set_with_version(&key, &value))?;
            println!("set {key} -> v{new_v}");
            Ok(())
        }
        KeyCommand::Get { key } => {
            let value = tokio::runtime::Handle::current().block_on(vault.get(&key))?;
            match value {
                Some(v) => println!("{v}"),
                None => println!("(not set)"),
            }
            Ok(())
        }
        KeyCommand::Delete { key } => {
            tokio::runtime::Handle::current().block_on(vault.delete(&key))?;
            println!("deleted {key}");
            Ok(())
        }
        KeyCommand::Rotate { key, value } => {
            // `vault.rotate(&key)` is super-trait (no payload); the
            // CLI sub-command's intent is "bump with a new payload", so
            // we exercise the versioned write path explicitly and emit
            // the resulting version number.
            let new_v =
                tokio::runtime::Handle::current().block_on(vault.set_with_version(&key, &value))?;
            println!("rotated {key} -> v{new_v}");
            Ok(())
        }
        KeyCommand::Versions { key } => {
            let entries = tokio::runtime::Handle::current().block_on(vault.list_versions(&key))?;
            if entries.is_empty() {
                println!("(no versions)");
            } else {
                for v in entries {
                    let note = v.change_note.as_deref().unwrap_or("-");
                    println!(
                        "v{}  sha256={}  bytes={}  created_at_unix_ms={}  note={note}",
                        v.version, v.bytes_sha256, v.byte_len, v.created_at_unix_ms
                    );
                }
            }
            Ok(())
        }
        KeyCommand::Diff { key, base, head } => {
            let diff = tokio::runtime::Handle::current()
                .block_on(vault.diff_versions(&key, base, head))?;
            println!(
                "{} v{} -> v{}: object_changed={}  byte_size_delta={}",
                key, base, head, diff.object_changed, diff.file_size_delta
            );
            Ok(())
        }
        KeyCommand::Restore {
            key,
            target_version,
        } => {
            let new_v = tokio::runtime::Handle::current()
                .block_on(vault.restore_to_version(&key, target_version))?;
            println!("restored {key} to v{target_version} -> v{new_v}");
            Ok(())
        }
        KeyCommand::ListKeys => {
            let keys = tokio::runtime::Handle::current().block_on(vault.list())?;
            if keys.is_empty() {
                println!("(no keys)");
            } else {
                for k in keys {
                    println!("{k}");
                }
            }
            Ok(())
        }
    };
    result.map_err(|e| match e.downcast::<VaultError>() {
        Ok(ve) => anyhow::anyhow!("vault error: {ve}"),
        Err(other) => other,
    })
}

/// Dispatcher for the `gitai *` subcommands (V0 T7).
///
/// `--dry-run` is handled here rather than inside [`Invocation`] so the
/// resolution logic (preset lookup, base-URL and model overrides) runs
/// without an API key or a network call. That makes the flag usable to
/// debug a misconfigured provider, which is when it is most needed.
fn run_gitai(sub: GitaiCommand) -> anyhow::Result<()> {
    match sub {
        GitaiCommand::Providers => {
            let reg = ai::registry::ProviderRegistry::builtin();
            println!("{} provider preset(s):", reg.len());
            for key in reg.keys() {
                let spec = reg
                    .get(key)
                    .map_err(|e| anyhow::anyhow!("registry error: {e}"))?;
                println!("  {key:<12} {}", spec.default_model());
            }
            Ok(())
        }
        GitaiCommand::Commit(args) => run_gitai_task(Task::Commit, args),
        GitaiCommand::Explain(args) => run_gitai_task(Task::Explain, args),
        GitaiCommand::Review(args) => run_gitai_task(Task::Review, args),
    }
}

fn run_gitai_task(task: Task, args: GitaiTaskArgs) -> anyhow::Result<()> {
    let handle = tokio::runtime::Handle::current();

    if args.dry_run {
        let reg = ai::registry::ProviderRegistry::builtin();
        let spec = reg.get(&args.provider)?;
        let spec = match &args.ai_base_url {
            Some(u) => spec.with_base_url(u),
            None => spec.clone(),
        };
        let model = args
            .ai_model
            .clone()
            .unwrap_or_else(|| spec.default_model().to_string());
        println!("task:     {}", task.as_str());
        println!("provider: {}", args.provider);
        println!("base_url: {}", spec.base_url());
        println!("model:    {model}");
        println!("repo:     {}", args.repo.display());
        println!(
            "diff:     {}",
            match (&args.range, args.from_diff) {
                (Some(r), _) => format!("range {r}"),
                (None, true) => "working tree".to_string(),
                (None, false) => "(none — pass --from-diff or --range)".to_string(),
            }
        );
        println!("(dry run — no API call was made)");
        return Ok(());
    }

    // The key is read here, after --dry-run returns, so a dry run never
    // requires one.
    let api_key = std::env::var(API_KEY_ENV).map_err(|_| {
        anyhow::anyhow!(
            "no API key: set the {API_KEY_ENV} environment variable \
             (e.g. `$env:{API_KEY_ENV} = \"...\"` in PowerShell)"
        )
    })?;
    if api_key.trim().is_empty() {
        return Err(anyhow::anyhow!("{API_KEY_ENV} is set but empty"));
    }

    let req = DiffRequest {
        range: args.range.clone(),
        include_worktree: args.from_diff,
        paths: args.paths.clone(),
    };
    let diff = handle.block_on(ai::diff::collect_diff(&args.repo, &req))?;

    let inv = Invocation {
        task,
        provider_key: args.provider.clone(),
        base_url_override: args.ai_base_url.clone(),
        model_override: args.ai_model.clone(),
        diff,
    };
    let out = handle.block_on(inv.run(&api_key))?;
    // stdout is the product: this is what a caller pipes into a commit
    // message file.
    println!("{out}");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    let config = build_config(&cli);

    match cli.command {
        Command::Serve => run_serve(config).await,
        Command::InitRepo { name } => run_init_repo(&config, &name).await,
        Command::List => run_list(&config),
        Command::Key(sub) => run_key(&config, sub),
        Command::Gitai(sub) => run_gitai(sub),
    }
}
