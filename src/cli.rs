//! Clap subcommands for the `gitgit` binary.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::{DEFAULT_BIND, DEFAULT_REPOS_DIR, DEFAULT_VAULT_FILE_ROOT};

/// `gitgit` — a tiny local Git HTTP server (MVP).
#[derive(Debug, Parser)]
#[command(
    name = "gitgit",
    version,
    about = "A tiny local Git HTTP server (MVP)",
    long_about = None
)]
pub struct Cli {
    /// Bind address for `serve`. Default: 0.0.0.0:8080
    #[arg(long, global = true, default_value = DEFAULT_BIND)]
    pub bind: String,

    /// Directory under which bare repos are stored. Default: ./repos
    #[arg(long, global = true, default_value = DEFAULT_REPOS_DIR)]
    pub repos_dir: PathBuf,

    /// Root directory for the V0 Credential Vault (file backend).
    /// Per ADR-0021 §1.2 + ADR-0022, default FileVault lives here.
    /// Env override: `GITGIT_VAULT_FILE_ROOT`.
    #[arg(long, global = true, default_value = DEFAULT_VAULT_FILE_ROOT)]
    pub vault_file_root: PathBuf,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Start the smart-HTTP server.
    Serve,

    /// Create a new bare repository under `<repos_dir>/<name>.git/`.
    InitRepo {
        /// Repository name (used as the `<name>.git` directory name).
        name: String,
    },

    /// List repositories that currently exist under `--repos-dir`.
    List,

    /// V0 Credential Vault operations (per ADR-0021 + ADR-0022).
    #[command(subcommand)]
    Key(KeyCommand),
}

/// Sub-actions against the V0 Credential Vault.
///
/// All actions delegate to the `VersionedVault` impl of the configured
/// backend (FileVault by default; per ADR-0022 §1.2, `set` always
/// records a sidecar entry — use `vault set` for "version-aware" writes
/// and `vault raw-set` to write through the super-trait `Vault::set`
/// without metadata).
#[derive(Debug, Subcommand)]
pub enum KeyCommand {
    /// Write `value` under `key` and record a sidecar entry. The
    /// returned number is the new 1-indexed version.
    Set {
        /// Logical key (e.g. `openai`).
        key: String,
        /// Value to store.
        value: String,
    },

    /// Read the current (latest) value for `key`. Uses the super-trait
    /// `Vault::get` directly.
    Get {
        /// Logical key.
        key: String,
    },

    /// Drop the current value (does **not** rewrite the version timeline;
    /// historical entries remain visible in `versions`).
    Delete {
        /// Logical key.
        key: String,
    },

    /// Rotate the value to `value`, recording a sidecar entry.
    /// Equivalent to `vault set`; the dedicated subcommand makes the
    /// intent explicit for ops scripts.
    Rotate {
        /// Logical key.
        key: String,
        /// New value.
        value: String,
    },

    /// List every recorded version for `key` (oldest-first).
    Versions {
        /// Logical key.
        key: String,
    },

    /// Compare two versions of `key`. Emits base / head sha256 + the
    /// delta.
    Diff {
        /// Logical key.
        key: String,
        /// Base version (1-indexed).
        #[arg(long)]
        base: i32,
        /// Head version (1-indexed).
        #[arg(long)]
        head: i32,
    },

    /// Restore `key`'s bytes to `target_version`. Returns the new
    /// current version number (timeline is append-only, so this is
    /// always > current). Per ADR-0022 V0 limitations, `FileVault`
    /// marks the restored entry with `[restored-to-vN-…]` because
    /// historical bytes are not recoverable from a flat file.
    Restore {
        /// Logical key.
        key: String,
        /// Version to restore.
        #[arg(long)]
        target_version: i32,
    },

    /// List known keys (per super-trait `Vault::list`).
    #[command(name = "list-keys")]
    ListKeys,
}
