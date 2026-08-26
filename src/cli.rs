//! Clap subcommands for the `gitgit` binary.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

use crate::config::{DEFAULT_BIND, DEFAULT_REPOS_DIR};

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
}
