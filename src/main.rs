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
use std::path::{Path, PathBuf};

use gitgit::ai::{diff::DiffRequest, Invocation, Task, API_KEY_ENV};
use gitgit::cli::{Cli, Command, GitaiCommand, GitaiTaskArgs, GitremoteCommand, KeyCommand};
use gitgit::config::{
    Config, VaultBackend, DEFAULT_S3_BUCKET, DEFAULT_S3_ENDPOINT, DEFAULT_S3_REGION,
    DEFAULT_VAULT_FILE_ROOT, S3_ACCESS_KEY_ENV, S3_BUCKET_ENV, S3_ENDPOINT_ENV, S3_KEY_PREFIX_ENV,
    S3_KEY_PREFIX_UNSET, S3_REGION_ENV, S3_SECRET_KEY_ENV, VAULT_BACKEND_ENV,
};
use gitgit::error::GitGitError;
use gitgit::server::vault::{FileVault, MinioVault, VaultError};
use gitgit::server::vault_versioned::VersionedVault;
use gitgit::server::AppState;
use gitgit::{ai, remote, repo, server};

/// Initialize logging once. Honors `RUST_LOG`, defaults to `info`.
fn init_tracing() {
    use tracing_subscriber::{fmt, EnvFilter};
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,gitgit=info"));
    let _ = fmt().with_env_filter(filter).try_init();
}

/// Environment variable that relocates the vault root.
const VAULT_ROOT_ENV: &str = "GITGIT_VAULT_FILE_ROOT";

/// Resolve one setting: the CLI flag first, then the environment
/// variable, then the built-in default.
///
/// `[FACT]` This is the precedence the repository already uses for
/// `--vault-file-root` / `GITGIT_VAULT_FILE_ROOT`: an explicit flag
/// wins, the environment variable fills in when the flag was not given,
/// and the default is the last resort. An environment variable set to an
/// empty string is ignored rather than honoured, for the same reason.
/// A CLI flag set to an empty string *is* honoured — an operator who
/// typed `--s3-bucket ""` meant it.
fn resolve_setting(cli_value: Option<&str>, env_name: &str, default: &str) -> String {
    if let Some(value) = cli_value {
        return value.to_string();
    }
    match std::env::var(env_name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => default.to_string(),
    }
}

/// Read a credential from the environment.
///
/// `[FACT]` The S3 credentials have no command-line flag: a flag would
/// put a long-lived secret in the process arguments (`ps`) and in the
/// shell history. This is the same rule the AI API key is held to
/// (`src/ai/mod.rs`: "If this ever moves to a CLI flag it becomes
/// visible in `ps` and in shell history").
///
/// An empty or whitespace-only value counts as unset, the same way the
/// vault root and the admin password treat theirs: an operator who
/// exported an empty variable has configured nothing, and the error
/// that follows says exactly what is missing.
fn credential_from_env(env_name: &str) -> String {
    match std::env::var(env_name) {
        Ok(value) if !value.trim().is_empty() => value,
        _ => String::new(),
    }
}

fn build_config(cli: &Cli) -> anyhow::Result<Config> {
    // The vault root has carried an `env = GITGIT_VAULT_FILE_ROOT`
    // override in its documentation since it was introduced, but nothing
    // ever read the variable -- the argument always won with its default.
    // An explicit flag still takes precedence; the env var only fills in
    // when the user did not pass one, which is the usual precedence rule
    // and the reason an empty value is ignored rather than honoured.
    let vault_file_root = match std::env::var(VAULT_ROOT_ENV) {
        Ok(v)
            if !v.trim().is_empty()
                && cli.vault_file_root.as_path() == Path::new(DEFAULT_VAULT_FILE_ROOT) =>
        {
            PathBuf::from(v)
        }
        _ => cli.vault_file_root.clone(),
    };
    let mut config = Config::new(cli.bind.clone(), cli.repos_dir.clone(), vault_file_root);

    // Which backend the vault uses. The flag wins; the environment
    // variable fills in when the flag was not given; the default is the
    // local file vault.
    //
    // `[FACT]` An unrecognised name is an error, never a fallback to the
    // file vault: an operator who asked for object storage and silently
    // got the file vault would keep writing credentials to disk while
    // believing they were in S3.
    config.vault_backend = match cli.vault_backend {
        Some(backend) => backend,
        None => match std::env::var(VAULT_BACKEND_ENV) {
            Ok(v) if !v.trim().is_empty() => v.parse::<VaultBackend>()?,
            _ => VaultBackend::default(),
        },
    };

    // The six S3 settings, same precedence as the backend.
    config.s3_vault.endpoint = resolve_setting(
        cli.s3_endpoint.as_deref(),
        S3_ENDPOINT_ENV,
        DEFAULT_S3_ENDPOINT,
    );
    config.s3_vault.bucket =
        resolve_setting(cli.s3_bucket.as_deref(), S3_BUCKET_ENV, DEFAULT_S3_BUCKET);
    config.s3_vault.region =
        resolve_setting(cli.s3_region.as_deref(), S3_REGION_ENV, DEFAULT_S3_REGION);
    config.s3_vault.key_prefix = resolve_setting(
        cli.s3_key_prefix.as_deref(),
        S3_KEY_PREFIX_ENV,
        S3_KEY_PREFIX_UNSET,
    );
    // The credentials come from the environment only, and they have no
    // default. An unset value stays empty, and
    // `Config::s3_vault_config` turns that into an error instead of an
    // invented credential.
    config.s3_vault.access_key = credential_from_env(S3_ACCESS_KEY_ENV);
    config.s3_vault.secret_key = credential_from_env(S3_SECRET_KEY_ENV);

    Ok(config)
}

/// The concrete vault implementation the configuration selects.
///
/// `[FACT]` `Arc<dyn VersionedVault>` erases the concrete type, and
/// `dyn VersionedVault` cannot be downcast to `dyn Any` (the `Any`
/// trait is `Sized`), so a trait object on its own cannot answer "which
/// backend is this?". Keeping the choice in an enum makes it
/// answerable, and `build_vault` is the one-line mapping from it.
enum SelectedVault {
    File(FileVault),
    S3(MinioVault),
}

/// Pick the vault implementation the configuration asks for.
///
/// `[FACT]` This is where the product stops being file-only. Until the
/// backend became selectable, `build_vault()` constructed a `FileVault`
/// unconditionally and the S3 implementation (`MinioVault`) was
/// reachable only from the test suite.
fn select_vault(config: &Config) -> anyhow::Result<SelectedVault> {
    Ok(match config.vault_backend {
        VaultBackend::File => SelectedVault::File(FileVault::new(&config.vault_file_root)),
        VaultBackend::S3 => SelectedVault::S3(
            MinioVault::connect(&config.s3_vault_config()?).map_err(GitGitError::from)?,
        ),
    })
}

/// Build the V0 Credential Vault. Per ADR-0021 §1.2 + ADR-0022 §follow-up
/// the V0 close-out wires the default `FileVault` into `AppState`.
///
/// Return type is `Arc<dyn VersionedVault>` (not `Arc<dyn Vault>`)
/// because every CLI `vault *` subcommand in this commit already needs
/// the version-management surface (per ADR-0022); super-trait
/// auto-deref is available for any future handler that only needs the
/// 5 KV operations.
///
/// `[FACT]` The return type is a `Result` now: the S3 backend needs
/// credentials (`Config::s3_vault_config`) and `MinioVault::connect`
/// can fail on the credentials it is handed. `connect` performs **no**
/// network I/O -- it only parses the endpoint and constructs a
/// `rust-s3` bucket handle -- so a wrong endpoint surfaces on the first
/// `get` / `set`, not here.
fn build_vault(config: &Config) -> anyhow::Result<std::sync::Arc<dyn VersionedVault>> {
    Ok(match select_vault(config)? {
        SelectedVault::File(vault) => std::sync::Arc::new(vault),
        SelectedVault::S3(vault) => std::sync::Arc::new(vault),
    })
}

async fn run_serve(config: Config) -> anyhow::Result<()> {
    config.ensure_repos_dir()?;
    config.ensure_vault_root()?;
    let bind = config.bind.clone();
    let vault = build_vault(&config)?;

    // Resolve the admin credential before binding. Order (env →
    // vault → generated) is documented in `server::auth`; the only part
    // that needs a decision here is the exposure policy: a non-loopback
    // bind with a password nobody chose is refused rather than served.
    let resolved = gitgit::server::auth::resolve_admin_credential(vault.as_ref()).await?;
    gitgit::server::auth::enforce_exposure_policy(&bind, resolved.source)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    if resolved.source == gitgit::server::auth::CredentialSource::Generated {
        // Printed exactly once, and only when the password was generated
        // — i.e. only when the operator has no other way to learn it.
        // This is a deliberate exception to the standing "never print a
        // credential" rule: an unlearnable password is a locked server,
        // and the alternative (not printing it) is worse than a secret
        // appearing once in the terminal that started the process.
        // It is never logged, never traced, and never written to disk.
        // Plain ASCII only: a box-drawing banner mangles in some
        // terminals, and a mangled credential is a wrong credential.
        println!("--- gitgit: generated admin password (one time) ---");
        println!(
            "  no {PASS_ENV} was set and the vault had no stored password.",
            PASS_ENV = gitgit::config::ADMIN_PASS_ENV
        );
        println!("  It is NOT stored anywhere. Write it down now.");
        println!("  user: {}", resolved.credential.user);
        println!("  pass: {}", resolved.credential.password());
        println!("--- end of credential ---");
    } else {
        tracing::info!(
            source = resolved.source.as_str(),
            "admin password loaded from a configured source"
        );
    }

    let admin = std::sync::Arc::new(gitgit::server::auth::AdminCredentialStore::from_shared(
        resolved.credential,
    ));
    let state = AppState::with_admin(config, vault, admin);
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
/// the timeline).
///
/// `async` rather than synchronous: `main` is `#[tokio::main]`, so the
/// process is already inside a Tokio runtime by the time a command is
/// dispatched. Calling `Handle::current().block_on(..)` from that context
/// panics with "Cannot start a runtime from within a runtime", which is
/// what every `key *` subcommand used to do. Awaiting in place is the only
/// correct option here; building a second runtime would also work but
/// would spin up a second thread pool per process.
async fn run_key(config: &Config, sub: KeyCommand) -> anyhow::Result<()> {
    config.ensure_vault_root()?;
    let vault = build_vault(config)?;
    let result: anyhow::Result<()> = match sub {
        KeyCommand::Set { key, value } => {
            let new_v = vault.set_with_version(&key, &value).await?;
            println!("set {key} -> v{new_v}");
            Ok(())
        }
        KeyCommand::Get { key } => {
            let value = vault.get(&key).await?;
            match value {
                Some(v) => println!("{v}"),
                None => println!("(not set)"),
            }
            Ok(())
        }
        KeyCommand::Delete { key } => {
            vault.delete(&key).await?;
            println!("deleted {key}");
            Ok(())
        }
        KeyCommand::Rotate { key, value } => {
            // `vault.rotate(&key)` is super-trait (no payload); the
            // CLI sub-command's intent is "bump with a new payload", so
            // we exercise the versioned write path explicitly and emit
            // the resulting version number.
            let new_v = vault.set_with_version(&key, &value).await?;
            println!("rotated {key} -> v{new_v}");
            Ok(())
        }
        KeyCommand::Versions { key } => {
            let entries = vault.list_versions(&key).await?;
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
            let diff = vault.diff_versions(&key, base, head).await?;
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
            let new_v = vault.restore_to_version(&key, target_version).await?;
            println!("restored {key} to v{target_version} -> v{new_v}");
            Ok(())
        }
        KeyCommand::ListKeys => {
            let keys = vault.list().await?;
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
async fn run_gitai(sub: GitaiCommand) -> anyhow::Result<()> {
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
        GitaiCommand::Commit(args) => run_gitai_task(Task::Commit, args).await,
        GitaiCommand::Explain(args) => run_gitai_task(Task::Explain, args).await,
        GitaiCommand::Review(args) => run_gitai_task(Task::Review, args).await,
    }
}

/// `async` for the same reason as [`run_key`]: the process is already
/// inside the `#[tokio::main]` runtime, so `block_on` here would panic.
async fn run_gitai_task(task: Task, args: GitaiTaskArgs) -> anyhow::Result<()> {
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
    let diff = ai::diff::collect_diff(&args.repo, &req).await?;

    let inv = Invocation {
        task,
        provider_key: args.provider.clone(),
        base_url_override: args.ai_base_url.clone(),
        model_override: args.ai_model.clone(),
        diff,
    };
    let out = inv.run(&api_key).await?;
    // stdout is the product: this is what a caller pipes into a commit
    // message file.
    println!("{out}");
    Ok(())
}

/// Dispatcher for the `gitremote *` subcommands (V0 T8).
///
/// The registry lives next to the `FileVault` root rather than inside it,
/// so `--vault-file-root` relocates both together and wiping a vault does
/// not silently discard the remote list.
fn remote_store(config: &Config) -> remote::RemoteStore {
    let base = config
        .vault_file_root
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    remote::RemoteStore::under(&base)
}

/// Determine which branch a bare repo's `HEAD` points at.
///
/// `HEAD` may be a symbolic ref (`ref: refs/heads/main`) or a detached
/// SHA. Only the symbolic form names a branch, and a detached HEAD has no
/// branch to sync, so that case is an explicit error rather than a guess.
fn default_branch(repo: &Path) -> anyhow::Result<String> {
    let head = repo.join("HEAD");
    let raw = std::fs::read_to_string(&head)
        .with_context(|| format!("cannot read {}", head.display()))?;
    let trimmed = raw.trim();
    match trimmed.strip_prefix("ref: refs/heads/") {
        Some(b) => Ok(b.to_string()),
        None => Err(anyhow::anyhow!(
            "{} does not point at a branch (HEAD is `{trimmed}`); pass --branch",
            repo.display()
        )),
    }
}

async fn run_gitremote(config: &Config, sub: GitremoteCommand) -> anyhow::Result<()> {
    let store = remote_store(config);
    match sub {
        GitremoteCommand::Add { name, url } => {
            let outcome = store.add(&name, &url)?;
            println!("{} {} -> {}", outcome.as_str(), name, url);
            println!("  registry: {}", store.path().display());
            Ok(())
        }
        GitremoteCommand::Ls => {
            let remotes = store.list()?;
            if remotes.is_empty() {
                println!("(no remotes)");
            } else {
                for r in remotes {
                    println!("{:<16} {}", r.name, r.url);
                }
            }
            Ok(())
        }
        GitremoteCommand::Rm { name } => {
            if store.remove(&name)? {
                println!("removed {name}");
            } else {
                println!("(no such remote: {name})");
            }
            Ok(())
        }
        GitremoteCommand::Sync { name, repo, branch } => {
            let remote_entry = store.get(&name)?;
            // Default to the bare repo `gitgit serve` manages under the
            // same name, so the common case needs no --repo.
            let repo_path = match repo {
                Some(p) => p,
                None => config.repo_path(&name)?,
            };
            let branch = match branch {
                Some(b) => b,
                None => default_branch(&repo_path)?,
            };
            println!(
                "syncing {} ({}) <-> {} [{}]",
                repo_path.display(),
                branch,
                remote_entry.url,
                name
            );
            let outcome =
                remote::sync::sync_ref(&repo_path, &remote_entry.url, &name, &branch).await?;
            println!("  {}", outcome.as_str());
            Ok(())
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();
    let config = build_config(&cli)?;

    match cli.command {
        Command::Serve => run_serve(config).await,
        Command::InitRepo { name } => run_init_repo(&config, &name).await,
        Command::List => run_list(&config),
        Command::Key(sub) => run_key(&config, sub).await,
        Command::Gitai(sub) => run_gitai(sub).await,
        Command::Gitremote(sub) => run_gitremote(&config, sub).await,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::sync::Mutex;

    /// Serialises the tests that read and write environment variables.
    ///
    /// `[FACT]` The environment is process-global and the harness runs
    /// the tests of a binary in parallel threads, so two tests that set
    /// the same variable would read each other's values. Only the tests
    /// below take this lock, and they live in the binary's own test
    /// target, so nothing else in the process reads these variables.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Remove every environment variable this lane reads.
    ///
    /// The developer's shell may have any of them set; a test that
    /// asserts on a default has to start from a known state.
    fn clear_vault_env() {
        for name in [
            VAULT_BACKEND_ENV,
            S3_ENDPOINT_ENV,
            S3_BUCKET_ENV,
            S3_REGION_ENV,
            S3_ACCESS_KEY_ENV,
            S3_SECRET_KEY_ENV,
            S3_KEY_PREFIX_ENV,
            VAULT_ROOT_ENV,
        ] {
            std::env::remove_var(name);
        }
    }

    /// A parsed command line with the given global flags.
    fn cli_with(flags: &[&str]) -> Cli {
        let mut argv = vec!["gitgit"];
        argv.extend_from_slice(flags);
        argv.push("serve");
        Cli::try_parse_from(argv).unwrap()
    }

    /// A unique scratch directory for a vault root.
    fn unique_temp(label: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let pid = std::process::id();
        let dir = std::env::temp_dir().join(format!(
            "gitgit-test-vault-{label}-{pid}-{n}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// `[FACT]` The backend is a choice, and the default has to be the
    /// one that works with no configuration at all: the local file
    /// vault. The environment variable is the documented way to pick
    /// the other one.
    #[test]
    fn the_backend_defaults_to_the_local_file_vault_and_the_environment_selects_another() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_vault_env();

        let cli = cli_with(&[]);
        let config = build_config(&cli).unwrap();
        assert_eq!(
            config.vault_backend,
            VaultBackend::File,
            "with nothing configured the vault must be the local file vault"
        );

        std::env::set_var(VAULT_BACKEND_ENV, "s3");
        let config = build_config(&cli).unwrap();
        assert_eq!(
            config.vault_backend,
            VaultBackend::S3,
            "the environment variable must select the S3 backend"
        );

        clear_vault_env();
    }

    /// `[FACT]` The precedence is the repository's existing one: an
    /// explicit flag beats the environment variable, in both
    /// directions. A flag that loses to the environment would make the
    /// environment the only way to configure anything.
    #[test]
    fn the_flag_beats_the_environment_for_the_backend() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_vault_env();

        std::env::set_var(VAULT_BACKEND_ENV, "s3");
        let cli = cli_with(&["--vault-backend", "file"]);
        let config = build_config(&cli).unwrap();
        assert_eq!(
            config.vault_backend,
            VaultBackend::File,
            "the flag must beat the environment variable"
        );

        std::env::set_var(VAULT_BACKEND_ENV, "file");
        let cli = cli_with(&["--vault-backend", "s3"]);
        let config = build_config(&cli).unwrap();
        assert_eq!(
            config.vault_backend,
            VaultBackend::S3,
            "the flag must beat the environment variable"
        );

        clear_vault_env();
    }

    /// `[FACT]` An unknown backend name is an error in both places it
    /// can arrive. A silent fallback to the file vault would be the
    /// dangerous kind: the operator would believe they were writing to
    /// object storage while their credentials went to a local
    /// directory. The message has to name the values that do work.
    #[test]
    fn an_unknown_backend_is_an_error_in_the_environment_and_on_the_command_line() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_vault_env();

        std::env::set_var(VAULT_BACKEND_ENV, "minio");
        let cli = cli_with(&[]);
        let err = build_config(&cli)
            .expect_err("an unknown backend must not fall back to the file vault");
        let msg = err.to_string();
        assert!(
            msg.contains("file") && msg.contains("s3"),
            "the error must name the valid values, got: {msg}"
        );

        clear_vault_env();

        let err = Cli::try_parse_from(["gitgit", "--vault-backend", "minio", "serve"])
            .expect_err("an unknown backend must not fall back to the file vault");
        let msg = err.to_string();
        assert!(
            msg.contains("file") && msg.contains("s3"),
            "the error must name the valid values, got: {msg}"
        );
    }

    /// `[FACT]` The six S3 settings follow the same precedence as the
    /// backend: flag, then environment, then default. The credentials
    /// are settings too -- they are read from the environment, and they
    /// have no default.
    ///
    /// The key prefix is the exception that proves the rule: it has no
    /// default at all, so the only way to get one is to say so.
    #[test]
    fn the_s3_settings_follow_the_flag_then_the_environment_then_the_default() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        clear_vault_env();

        let cli = cli_with(&[]);
        let config = build_config(&cli).unwrap();
        assert_eq!(config.s3_vault.endpoint, DEFAULT_S3_ENDPOINT);
        assert_eq!(config.s3_vault.bucket, DEFAULT_S3_BUCKET);
        assert_eq!(config.s3_vault.region, DEFAULT_S3_REGION);
        assert_eq!(config.s3_vault.key_prefix, S3_KEY_PREFIX_UNSET);
        assert!(
            config.s3_vault.key_prefix.is_empty(),
            "the key prefix must have no default"
        );
        assert!(config.s3_vault.access_key.is_empty());
        assert!(config.s3_vault.secret_key.is_empty());

        std::env::set_var(S3_ENDPOINT_ENV, "http://env.example:9000");
        std::env::set_var(S3_BUCKET_ENV, "env-bucket");
        std::env::set_var(S3_KEY_PREFIX_ENV, "env-prefix/");
        std::env::set_var(S3_ACCESS_KEY_ENV, "env-access-key");
        std::env::set_var(S3_SECRET_KEY_ENV, "env-secret-key");
        let cli = cli_with(&[]);
        let config = build_config(&cli).unwrap();
        assert_eq!(config.s3_vault.endpoint, "http://env.example:9000");
        assert_eq!(config.s3_vault.bucket, "env-bucket");
        assert_eq!(config.s3_vault.key_prefix, "env-prefix/");
        assert_eq!(config.s3_vault.access_key, "env-access-key");
        assert_eq!(config.s3_vault.secret_key, "env-secret-key");
        // Untouched by the environment, so still the default.
        assert_eq!(config.s3_vault.region, DEFAULT_S3_REGION);

        let cli = cli_with(&["--s3-bucket", "flag-bucket"]);
        let config = build_config(&cli).unwrap();
        assert_eq!(
            config.s3_vault.bucket, "flag-bucket",
            "the flag must beat the environment variable"
        );
        assert_eq!(
            config.s3_vault.access_key, "env-access-key",
            "the credentials come from the environment, which the flag cannot override"
        );
        assert_eq!(
            config.s3_vault.secret_key, "env-secret-key",
            "the credentials come from the environment, which the flag cannot override"
        );

        clear_vault_env();
    }

    /// `[FACT]` The backend setting has to reach the implementation, not
    /// just the configuration. This is the assertion that a parser
    /// test cannot make: it checks which implementation is built, and
    /// that the configured values arrived with it.
    #[test]
    fn the_selected_backend_is_the_implementation_that_gets_built() {
        let root = unique_temp("file");
        let mut config = Config::new("127.0.0.1:0", root.clone(), root.clone());
        config.vault_backend = VaultBackend::File;
        match select_vault(&config).unwrap() {
            SelectedVault::File(vault) => {
                assert_eq!(
                    vault.root(),
                    root.as_path(),
                    "the file vault must be rooted at the configured root"
                );
            }
            SelectedVault::S3(_) => {
                panic!("the file backend must not build the S3 implementation")
            }
        }

        let mut config = Config::new("127.0.0.1:0", root.clone(), root.clone());
        config.vault_backend = VaultBackend::S3;
        config.s3_vault.access_key = "an-access-key".to_string();
        config.s3_vault.secret_key = "a-secret-key".to_string();
        config.s3_vault.bucket = "a-bucket".to_string();
        config.s3_vault.key_prefix = "a-prefix/".to_string();
        match select_vault(&config).unwrap() {
            SelectedVault::S3(vault) => {
                assert_eq!(
                    vault.bucket_name(),
                    "a-bucket",
                    "the configured bucket must reach the S3 implementation"
                );
                assert_eq!(vault.key_prefix(), "a-prefix/");
            }
            SelectedVault::File(_) => {
                panic!("the s3 backend must not build the file implementation")
            }
        }
    }

    /// `[FACT]` The S3 backend has no credentials to fall back on, so
    /// selecting it without them is an error -- while the file backend
    /// starts with the same configuration. This is the guard against an
    /// operator who believes they are talking to object storage while
    /// the process is talking to nothing at all.
    #[test]
    fn the_s3_backend_refuses_to_start_without_credentials() {
        let root = unique_temp("no-credentials");
        let mut config = Config::new("127.0.0.1:0", root.clone(), root.clone());
        config.vault_backend = VaultBackend::S3;
        config.s3_vault.key_prefix = "a-prefix/".to_string();

        // `expect_err` is not available here: it needs `Debug` on the
        // success type, and `Arc<dyn VersionedVault>` does not implement
        // it (the trait does not require it).
        let err = match build_vault(&config) {
            Ok(_) => panic!("the s3 backend must not start without credentials"),
            Err(err) => err,
        };
        let msg = err.to_string();
        assert!(
            msg.contains(S3_ACCESS_KEY_ENV),
            "the error must name the environment variable, got: {msg}"
        );

        config.vault_backend = VaultBackend::File;
        build_vault(&config).expect("the file backend needs no credentials");
    }

    /// `[FACT]` The vault that `build_vault` hands to the server is a
    /// working one, not a placeholder: the file backend writes to the
    /// root it was configured with.
    #[tokio::test]
    async fn the_file_backend_writes_to_the_configured_vault_root() {
        let root = unique_temp("write");
        let mut config = Config::new("127.0.0.1:0", root.clone(), root.clone());
        config.vault_backend = VaultBackend::File;
        let vault = build_vault(&config).unwrap();

        vault.set_with_version("probe", "a-value").await.unwrap();
        assert_eq!(
            vault.get("probe").await.unwrap().as_deref(),
            Some("a-value"),
            "the vault handed to the server must actually store values"
        );
        assert!(
            root.join("probe").exists(),
            "the value must land under the configured vault root"
        );
    }
}
