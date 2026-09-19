//! `gitgit` — single-crate MVP for a local Git HTTP server.

use anyhow::Context;
use clap::Parser;

mod cli;
mod config;
mod error;
mod repo;
mod server;

use crate::cli::{Cli, Command, KeyCommand};
use crate::config::Config;
use crate::server::AppState;
use crate::server::vault::{FileVault, VaultError};
use crate::server::vault_versioned::VersionedVault;

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
            let new_v = tokio::runtime::Handle::current()
                .block_on(vault.set_with_version(&key, &value))?;
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
            let new_v = tokio::runtime::Handle::current()
                .block_on(vault.set_with_version(&key, &value))?;
            println!("rotated {key} -> v{new_v}");
            Ok(())
        }
        KeyCommand::Versions { key } => {
            let entries = tokio::runtime::Handle::current()
                .block_on(vault.list_versions(&key))?;
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
        KeyCommand::Restore { key, target_version } => {
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
    }
}
