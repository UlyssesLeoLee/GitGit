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
    ///
    /// Env override: `GITGIT_VAULT_FILE_ROOT`, applied in `build_config`
    /// rather than by a clap `env` key -- the `env` feature is not enabled
    /// on the clap dependency, and the doc comment previously promised an
    /// override that was never read.
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

    /// AI-assisted Git workflows (commit message, explain, review).
    ///
    /// The API key is read from the `GITGIT_AI_API_KEY` environment
    /// variable, never from a flag: an argument is visible in `ps` output
    /// and lands in shell history.
    ///
    /// Repository content is sent to a third-party API after a
    /// heuristic redaction pass. That pass catches known credential
    /// shapes and is NOT a guarantee — the authoritative control is not
    /// committing secrets in the first place.
    #[command(subcommand)]
    Gitai(GitaiCommand),

    /// Named upstream remotes (add / list / fast-forward sync).
    ///
    /// Storage is a local JSON file under `.gitgit/`, not a database.
    /// Per ADR-0022 §2.1 this crate does not take a sqlx / PG dependency,
    /// and ADR-0022 already rejected wiring an external schema in.
    #[command(subcommand)]
    Gitremote(GitremoteCommand),
}

/// Sub-actions under `gitgit gitremote`.
#[derive(Debug, Subcommand)]
pub enum GitremoteCommand {
    /// Register a named upstream, or point an existing name at a new URL.
    Add {
        /// Short name, e.g. `gitee`. Re-adding with a different URL
        /// updates in place rather than failing.
        name: String,
        /// Upstream URL. Passed verbatim to `git fetch` / `git push`.
        url: String,
    },

    /// List every registered remote.
    Ls,

    /// Remove a registered remote.
    ///
    /// Removes the registry entry only. It never touches the remote.
    Rm {
        /// Remote name to forget.
        name: String,
    },

    /// Fast-forward sync a managed bare repo with a named remote.
    Sync {
        /// Remote name as registered by `gitremote add`.
        name: String,
        /// Bare repository to sync. Default: resolved from
        /// `--repos-dir` + name, i.e. the repo `gitgit serve` manages.
        #[arg(long)]
        repo: Option<PathBuf>,
        /// Branch to sync. Default: the remote's default branch.
        #[arg(long)]
        branch: Option<String>,
    },
}

/// Sub-actions under `gitgit gitai`.
#[derive(Debug, Subcommand)]
pub enum GitaiCommand {
    /// Write a commit message for a diff.
    Commit(GitaiTaskArgs),

    /// Explain what a change does.
    Explain(GitaiTaskArgs),

    /// Review a diff for correctness and security problems.
    Review(GitaiTaskArgs),

    /// List the registered provider presets and their endpoints.
    ///
    /// Performs no network call.
    Providers,
}

/// Arguments shared by `gitai commit` / `explain` / `review`.
#[derive(Debug, Clone, clap::Args)]
pub struct GitaiTaskArgs {
    /// Repository to read the diff from. Default: current directory.
    #[arg(long, default_value = ".")]
    pub repo: PathBuf,

    /// Provider preset key. Run `gitgit gitai providers` for the list.
    #[arg(long, default_value = "openai")]
    pub provider: String,

    /// Override the provider's base URL. Use for a proxy, a private
    /// gateway, or an OpenAI-compatible host with no preset.
    #[arg(long)]
    pub ai_base_url: Option<String>,

    /// Override the provider's default model.
    #[arg(long)]
    pub ai_model: Option<String>,

    /// Include uncommitted working-tree changes in the diff.
    #[arg(long)]
    pub from_diff: bool,

    /// Explicit revision range, e.g. `HEAD~1..HEAD`. Takes precedence over
    /// `--from-diff` when both are given.
    #[arg(long)]
    pub range: Option<String>,

    /// Limit the diff to these paths.
    #[arg(long = "path")]
    pub paths: Vec<String>,

    /// Print the provider and model that would be used, then exit without
    /// contacting the API.
    #[arg(long)]
    pub dry_run: bool,
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
