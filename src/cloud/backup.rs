//! Taking a repository snapshot onto a cloud disk, and putting it back.
//!
//! # The shape of a snapshot
//!
//! `[FACT]` A snapshot is **one file** ✓:
//!
//! ```text
//! GITGIT-SNAPSHOT\n
//! {metadata as one line of JSON}\n
//! {a ustar archive of the repository}
//! ```
//!
//! The magic comes first ✓ so a file that someone opened by mistake is
//! obvious ✓; the metadata line says which repository the snapshot holds
//! and what the archive's SHA-256 is ✓; then the archive itself ✓.
//!
//! # What a restore does
//!
//! `[FACT]` A restore **replaces** the repository ✓ — that is what a
//! rollback is ✓. It extracts into a sibling directory first ✓ and only
//! then swaps it in ✓, so a failed restore cannot leave a half-written
//! repository behind ✓.
//!
//! # The safety snapshot
//!
//! `[FACT]` Before the swap ✓, the repository that is about to be
//! replaced is taken as a **safety snapshot** ✓ and uploaded to the same
//! disk ✓. It is an **ordinary snapshot** ✓ — the same format ✓, the
//! same reader ✓ — so `gitgit cloud restore <that key>` puts the
//! repository back ✓: the rollback is itself reversible ✓.
//!
//! `[FACT]` If the safety snapshot cannot be taken, the restore is
//! **refused** ✓ and the repository is left exactly as it was ✓ — a
//! rollback that destroys what it could not preserve is not a rollback
//! ✓, it is data loss ✓.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::cloud::tar::{self, Entry};
use crate::cloud::{
    CloudDisk, SnapshotMeta, LEGACY_SNAPSHOT_VERSION, SNAPSHOT_MAGIC, SNAPSHOT_VERSION,
};
use crate::error::{GitGitError, Result};

/// The key prefix every snapshot of a disk lives under.
pub const SNAPSHOT_PREFIX: &str = "snapshots/";

/// The marker a safety snapshot's file name carries ✓.
///
/// `[FACT]` It is part of the **file name** ✓, not of the prefix ✓ — so
/// a safety snapshot sits beside the ordinary snapshots of the same
/// repository ✓, and `gitgit cloud snapshots` shows which one it is ✓
/// without downloading anything ✓.
pub const SAFETY_SNAPSHOT_MARKER: &str = "pre-restore-";

/// The bare repository directory of `repo` under `repos_dir`.
pub fn repo_dir(repos_dir: &Path, repo: &str) -> PathBuf {
    repos_dir.join(format!("{repo}.git"))
}

/// Take a snapshot of `repo` and upload it to `disk`.
///
/// Returns the snapshot's metadata ✓ and the key it was stored under ✓.
pub async fn backup(
    disk: &dyn CloudDisk,
    repo: &str,
    repos_dir: &Path,
    disk_name: &str,
) -> Result<(SnapshotMeta, String)> {
    // The name becomes a path ✓ — `<repos_dir>/<name>.git` ✓ — so it is
    // checked before it is used ✓.
    validate_cloud_repo_name(repo)?;
    let dir = repo_dir(repos_dir, repo);
    if !dir.is_dir() {
        return Err(GitGitError::Cloud(format!(
            "no repository called {repo:?} under {}",
            repos_dir.display()
        )));
    }
    let built = build_snapshot(&dir, repo, disk_name)?;
    let key = format!(
        "{SNAPSHOT_PREFIX}{repo}/{}-{}.tar",
        built.meta.created_unix,
        &built.digest[..8]
    );
    disk.put(&key, &built.file).await?;
    Ok((built.meta, key))
}

/// The bytes of a snapshot, and the digest of its archive ✓.
struct BuiltSnapshot {
    /// The metadata the snapshot carries ✓.
    meta: SnapshotMeta,
    /// The whole snapshot file ✓ — the magic ✓, the metadata line ✓ and
    /// the archive ✓.
    file: Vec<u8>,
    /// The archive's digest ✓ — its first eight characters go into the
    /// key ✓.
    digest: String,
}

/// Build the snapshot of the repository at `dir` ✓.
///
/// `[FACT]` A snapshot is **one file** ✓ — the magic ✓, one line of JSON
/// metadata ✓, then the ustar archive ✓ — and the metadata carries the
/// archive's SHA-256 ✓, so a restore can tell a damaged download from a
/// good one ✓.
fn build_snapshot(dir: &Path, repo: &str, disk_name: &str) -> Result<BuiltSnapshot> {
    let collected = collect(dir)?;
    let archive = tar::write(&collected.entries).map_err(|e| GitGitError::Cloud(e.to_string()))?;
    let digest = hex::encode(Sha256::digest(&archive));

    let created = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let meta = SnapshotMeta {
        version: SNAPSHOT_VERSION,
        disk: disk_name.to_string(),
        repo: repo.to_string(),
        created_unix: created,
        entries: collected.entries.len() as u64,
        sha256: digest.clone(),
        // The directories that hold no file ✓ — the archive cannot carry
        // them ✓, so the metadata has to ✓.
        directories: collected.empty_dirs,
    };

    let mut file = Vec::new();
    file.extend_from_slice(SNAPSHOT_MAGIC);
    let json = serde_json::to_vec(&meta)
        .map_err(|e| GitGitError::Cloud(format!("the snapshot metadata is unwritable: {e}")))?;
    file.extend_from_slice(&json);
    file.push(b'\n');
    file.extend_from_slice(&archive);

    Ok(BuiltSnapshot { meta, file, digest })
}

/// What a restore did ✓.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreOutcome {
    /// The metadata of the snapshot that was restored ✓.
    pub meta: SnapshotMeta,
    /// The key of the safety snapshot taken before the swap ✓ — `None`
    /// when there was no repository to replace ✓.
    pub safety_snapshot: Option<String>,
}

/// Restore `repo` from the snapshot stored under `key`.
///
/// The archive's SHA-256 is checked against the metadata before anything
/// is written ✓ — a truncated or tampered download must not become the
/// repository ✓.
///
/// `[FACT]` Before the swap ✓, the repository that is about to be
/// replaced is taken as a **safety snapshot** ✓ and uploaded to the same
/// disk ✓, and its key comes back with the outcome ✓ — so the restore
/// can be undone ✓. If that snapshot cannot be taken, the restore is
/// **refused** ✓ and the repository is left exactly as it was ✓.
pub async fn restore(
    disk: &dyn CloudDisk,
    key: &str,
    repo: Option<&str>,
    repos_dir: &Path,
    disk_name: &str,
) -> Result<RestoreOutcome> {
    let file = disk.get(key).await?;
    let (meta, archive) = split_snapshot(&file)?;
    let target_repo = match repo {
        Some(requested) if requested != meta.repo => {
            return Err(GitGitError::Cloud(format!(
                "the snapshot holds the repository {:?}, not {requested:?}",
                meta.repo
            )));
        }
        Some(requested) => requested.to_string(),
        None => meta.repo.clone(),
    };
    // `[FACT]` The repository name becomes a path ✓ — `<repos_dir>/<name>.git`
    // ✓ — and when the caller does not name one, the name comes from the
    // **snapshot's metadata** ✓, which came from the far end ✓. So it is
    // checked before it is used ✓, with the same rule the rest of the
    // product applies to a repository name ✓
    // (`crate::config::validate_repo_name` ✓): no separator ✓, no `..` ✓,
    // no leading dot ✓, and — because on Windows `C:` is a drive prefix
    // and `Path::join` would discard the repositories directory ✓ — no
    // colon ✓.
    validate_cloud_repo_name(&target_repo)?;
    let digest = hex::encode(Sha256::digest(&archive));
    if digest != meta.sha256 {
        return Err(GitGitError::Cloud(format!(
            "the snapshot is damaged: its digest is {digest}, the metadata says {}",
            meta.sha256
        )));
    }

    let entries = tar::read(&archive).map_err(|e| GitGitError::Cloud(e.to_string()))?;
    let target = repo_dir(repos_dir, &target_repo);

    // Every path is checked **before** anything is written ✓: a snapshot
    // that carries an unsafe path must not leave half a repository behind
    // ✓, and must not write a single byte outside the staging directory ✓.
    //
    // `[FACT]` The rule is the product's own ✓ — the same one the conflict
    // core applies before it writes into a work tree ✓
    // (`crate::repo::status::validate_rel_path` ✓): repository-relative ✓,
    // no `..` ✓, no backslash ✓, and no colon ✓ — a colon is a Windows
    // drive prefix ✓, and `Path::join("C:/x")` would throw the staging
    // directory away ✓.
    for entry in &entries {
        if crate::repo::status::validate_rel_path(&entry.path).is_err() {
            return Err(GitGitError::Cloud(format!(
                "the snapshot contains an unsafe path: {:?}",
                entry.path
            )));
        }
    }
    // The directories that hold no file ✓ are written to the same place
    // as the files ✓, so they are checked with the same rule ✓.
    for dir in &meta.directories {
        if dir.is_empty() || crate::repo::status::validate_rel_path(dir).is_err() {
            return Err(GitGitError::Cloud(format!(
                "the snapshot contains an unsafe directory: {dir:?}"
            )));
        }
    }

    // The safety snapshot ✓ — taken **after** the path pre-flight above
    // ✓ and **before** anything is written locally ✓.
    //
    // `[FACT]` A snapshot that carries an unsafe path is refused without
    // leaving a snapshot of the repository behind ✓, and a repository
    // that cannot be preserved is never replaced ✓.
    //
    // `[FACT]` There is no repository ⇒ nothing to lose ⇒ no safety
    // snapshot ✓, and the restore goes straight on ✓.
    let safety_snapshot = if target.is_dir() {
        Some(take_safety_snapshot(disk, &target_repo, repos_dir, disk_name).await?)
    } else {
        None
    };

    // Extract beside the repository and swap it in, so a failure never
    // leaves a half-written repository.
    let staging = repos_dir.join(format!(".{target_repo}.restore-staging"));
    let retired = repos_dir.join(format!(".{target_repo}.restore-retired"));
    for scratch in [&staging, &retired] {
        remove_path(scratch)?;
    }
    std::fs::create_dir_all(&staging)?;
    // The directories that hold no file ✓ — the archive cannot carry them
    // ✓, so they are created here ✓, before the files ✓.
    for dir in &meta.directories {
        std::fs::create_dir_all(staging.join(dir))?;
    }
    for entry in &entries {
        let path = staging.join(&entry.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, &entry.data)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(entry.mode))?;
        }
    }

    // The swap ✓.
    swap_in(&staging, &target, &retired)?;
    Ok(RestoreOutcome {
        meta,
        safety_snapshot,
    })
}

/// Take the safety snapshot of the repository that is about to be replaced ✓.
///
/// `[FACT]` It goes to the **same disk** ✓, under the same prefix ✓,
/// with the marker in its file name ✓ — so `gitgit cloud snapshots` shows
/// it for what it is ✓, and `gitgit cloud restore <that key>` puts the
/// repository back ✓.
///
/// `[FACT]` A failure here is **fatal** to the restore ✓: the caller
/// must not replace a repository it could not preserve ✓, so the error
/// names the step that failed ✓.
async fn take_safety_snapshot(
    disk: &dyn CloudDisk,
    repo: &str,
    repos_dir: &Path,
    disk_name: &str,
) -> Result<String> {
    let dir = repo_dir(repos_dir, repo);
    let built = build_snapshot(&dir, repo, disk_name).map_err(|e| {
        GitGitError::Cloud(format!(
            "the safety snapshot of {repo:?} could not be built: {e}; the repository was left untouched"
        ))
    })?;
    let key = safety_snapshot_key(repo, built.meta.created_unix, &built.digest);
    disk.put(&key, &built.file).await.map_err(|e| {
        GitGitError::Cloud(format!(
            "the safety snapshot of {repo:?} could not be uploaded: {e}; the repository was left untouched"
        ))
    })?;
    Ok(key)
}

/// The key a safety snapshot of `repo` is stored under ✓.
fn safety_snapshot_key(repo: &str, created: u64, digest: &str) -> String {
    format!(
        "{SNAPSHOT_PREFIX}{repo}/{SAFETY_SNAPSHOT_MARKER}{created}-{}.tar",
        &digest[..8]
    )
}

/// Put the restored repository in place of the one that is there.
///
/// `[FACT]` Three steps ✓: the old repository steps aside ✓, the restored
/// one takes its place ✓, the old one is removed ✓. If the second step
/// fails, the old repository **goes back where it was** ✓ — a failed
/// restore must not leave the repository gone ✓.
fn swap_in(staging: &Path, target: &Path, retired: &Path) -> Result<()> {
    swap_in_with(staging, target, retired, |from, to| {
        std::fs::rename(from, to)
    })
}

/// The swap, with the rename injected ✓ — so the failure path can be
/// exercised by a test ✓ instead of being taken on faith ✓.
fn swap_in_with(
    staging: &Path,
    target: &Path,
    retired: &Path,
    mut rename: impl FnMut(&Path, &Path) -> std::io::Result<()>,
) -> Result<()> {
    if target.exists() {
        rename(target, retired)?;
    }
    if let Err(e) = rename(staging, target) {
        // The old repository goes back ✓, and the staged copy is dropped ✓.
        if retired.exists() {
            let _ = rename(retired, target);
        }
        let _ = remove_path(staging);
        return Err(GitGitError::Io(e));
    }
    if retired.exists() {
        remove_path(retired)?;
    }
    Ok(())
}

/// Remove a file or a directory, whichever it is ✓.
///
/// `[FACT]` A missing path is not an error ✓ — the caller wants the path
/// gone ✓, and it already is ✓.
fn remove_path(path: &Path) -> std::io::Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => std::fs::remove_dir_all(path),
        Ok(_) => std::fs::remove_file(path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// Check a repository name that is about to become a path ✓.
///
/// `[FACT]` The rule is the product's own ✓ — the one
/// `crate::config::validate_repo_name` applies ✓ and the one the HTTP
/// layer applies ✓ — plus the colon ✓, because on Windows `C:` is a
/// drive prefix ✓ and `Path::join` would discard the repositories
/// directory ✓.
///
/// `[FACT]` It matters on both sides ✓: the name a backup reads from ✓ is
/// the caller's ✓, and the name a restore writes to ✓ may come from the
/// snapshot's metadata ✓, which came from the far end ✓.
fn validate_cloud_repo_name(name: &str) -> Result<()> {
    crate::config::validate_repo_name(name)?;
    let stem = name.strip_suffix(".git").unwrap_or(name);
    crate::config::validate_repo_name(stem)?;
    if stem.contains(':') {
        return Err(GitGitError::Cloud(format!(
            "the snapshot names an unsafe repository: {name:?}"
        )));
    }
    Ok(())
}

/// Split a snapshot file into its metadata and its archive.
fn split_snapshot(file: &[u8]) -> Result<(SnapshotMeta, Vec<u8>)> {
    if !file.starts_with(SNAPSHOT_MAGIC) {
        return Err(GitGitError::Cloud(
            "that file is not a gitgit snapshot".to_string(),
        ));
    }
    let rest = &file[SNAPSHOT_MAGIC.len()..];
    let newline = rest
        .iter()
        .position(|b| *b == b'\n')
        .ok_or_else(|| GitGitError::Cloud("the snapshot has no metadata line".to_string()))?;
    let meta: SnapshotMeta = serde_json::from_slice(&rest[..newline])
        .map_err(|e| GitGitError::Cloud(format!("the snapshot metadata is unreadable: {e}")))?;
    // The version is checked ✓ — a snapshot this build does not know how
    // to read is **refused** ✓, not read hopefully ✓.
    if meta.version != SNAPSHOT_VERSION && meta.version != LEGACY_SNAPSHOT_VERSION {
        return Err(GitGitError::Cloud(format!(
            "the snapshot's format version is {}, and this build reads version {LEGACY_SNAPSHOT_VERSION} and {SNAPSHOT_VERSION}",
            meta.version
        )));
    }
    Ok((meta, rest[newline + 1..].to_vec()))
}

/// What a repository directory holds: its files ✓, and the directories that
/// hold none ✓.
struct Collected {
    /// Every regular file, as a tar entry ✓.
    entries: Vec<Entry>,
    /// The directories that hold **no file at all** ✓ — relative to the
    /// root ✓, with `/` separators ✓.
    ///
    /// `[FACT]` A file inside a directory recreates that directory when the
    /// snapshot is restored ✓ — so only a directory with **nothing** in it
    /// would be lost ✓, and losing one is not cosmetic ✓: a bare
    /// repository without `refs/` is not a repository git will touch ✓.
    empty_dirs: Vec<String>,
}

/// Collect every regular file under `dir` as a tar entry, and every
/// directory that holds none.
///
/// `[FACT]` The paths are relative to `dir` ✓ and use `/` separators ✓.
/// Symlinks are **skipped** ✓: a snapshot that followed one could read
/// outside the repository ✓, and a bare repository has none ✓.
fn collect(dir: &Path) -> Result<Collected> {
    let mut entries = Vec::new();
    let mut dirs: BTreeSet<String> = BTreeSet::new();
    // Every directory that has a file somewhere below it ✓.
    let mut holds_a_file: BTreeSet<String> = BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let read = std::fs::read_dir(&current)?;
        for item in read {
            let item = item?;
            let path = item.path();
            let meta = item.metadata()?;
            if meta.is_symlink() {
                continue;
            }
            let relative = path
                .strip_prefix(dir)
                .map_err(|_| {
                    GitGitError::Cloud(format!("{} is not inside the repository", path.display()))
                })?
                .to_string_lossy()
                .replace('\\', "/");
            if meta.is_dir() {
                dirs.insert(relative);
                stack.push(path);
                continue;
            }
            // Every directory above this file holds a file ✓.
            let mut parent = relative.clone();
            while let Some(index) = parent.rfind('/') {
                parent.truncate(index);
                holds_a_file.insert(parent.clone());
            }
            let data = std::fs::read(&path)?;
            let mtime = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            entries.push(Entry {
                path: relative,
                data,
                mode: mode_of(&meta),
                mtime,
            });
        }
    }
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let empty_dirs = dirs
        .into_iter()
        .filter(|d| !holds_a_file.contains(d))
        .collect();
    Ok(Collected {
        entries,
        empty_dirs,
    })
}

/// The permission bits of a file.
fn mode_of(meta: &std::fs::Metadata) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o777
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        0o644
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::cloud::{
        access_key_env, secret_key_env, CloudDiskEntry, CloudObject, SNAPSHOT_MAGIC,
    };

    /// A disk that keeps objects in memory ✓ — the tests must not need a
    /// network ✓.
    #[derive(Default)]
    struct MemoryDisk {
        objects: std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>,
        /// When it is set ✓, every upload is refused ✓ — so the
        /// fail-closed path can be **exercised** ✓ instead of being taken
        /// on faith ✓.
        refuse_writes: std::sync::atomic::AtomicBool,
    }

    impl MemoryDisk {
        /// Refuse every upload from now on ✓.
        fn refuse_writes(&self) {
            self.refuse_writes
                .store(true, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[async_trait::async_trait]
    impl CloudDisk for MemoryDisk {
        async fn put(&self, key: &str, bytes: &[u8]) -> Result<Option<String>> {
            if self.refuse_writes.load(std::sync::atomic::Ordering::SeqCst) {
                return Err(GitGitError::Cloud(
                    "the disk refused the upload".to_string(),
                ));
            }
            self.objects
                .lock()
                .unwrap()
                .insert(key.to_string(), bytes.to_vec());
            Ok(Some("v1".to_string()))
        }

        async fn get(&self, key: &str) -> Result<Vec<u8>> {
            self.objects
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| GitGitError::Cloud(format!("no object at {key}")))
        }

        async fn list(&self, prefix: &str) -> Result<Vec<CloudObject>> {
            Ok(self
                .objects
                .lock()
                .unwrap()
                .iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .map(|(k, v)| CloudObject {
                    key: k.clone(),
                    size: v.len() as u64,
                    version_id: Some("v1".to_string()),
                    modified: "2026-11-18T00:00:00.000Z".to_string(),
                })
                .collect())
        }

        async fn delete(&self, key: &str) -> Result<()> {
            self.objects.lock().unwrap().remove(key);
            Ok(())
        }
    }

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "gitgit-cloud-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A tiny bare-repository shape ✓.
    fn make_repo(repos_dir: &Path, name: &str, head: &str) {
        let dir = repo_dir(repos_dir, name);
        std::fs::create_dir_all(dir.join("objects/ab")).unwrap();
        std::fs::create_dir_all(dir.join("refs/heads")).unwrap();
        std::fs::write(dir.join("HEAD"), head).unwrap();
        std::fs::write(dir.join("objects/ab/cdef0123"), "blob").unwrap();
        std::fs::write(dir.join("refs/heads/main"), "ref: refs/heads/main").unwrap();
    }

    /// Every file under `dir`, with its bytes ✓ — so a test can say the
    /// repository did not change **at all** ✓, not merely that it still
    /// exists ✓.
    fn contents_of(dir: &Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut out = std::collections::BTreeMap::new();
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            for item in std::fs::read_dir(&current).unwrap() {
                let item = item.unwrap();
                let path = item.path();
                if item.metadata().unwrap().is_dir() {
                    stack.push(path);
                    continue;
                }
                let relative = path
                    .strip_prefix(dir)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                out.insert(relative, std::fs::read(&path).unwrap());
            }
        }
        out
    }

    #[tokio::test]
    async fn a_snapshot_round_trips_and_replaces_the_repository() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("round-trip");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");

        let (meta, key) = backup(&disk, "demo", &repos, "oss").await.unwrap();
        assert_eq!(meta.repo, "demo");
        assert_eq!(meta.entries, 3);
        assert!(key.starts_with("snapshots/demo/"));
        assert!(!meta.sha256.is_empty());

        // Change the repository, then roll it back.
        std::fs::write(
            repo_dir(&repos, "demo").join("HEAD"),
            "ref: refs/heads/other\n",
        )
        .unwrap();
        let restored = restore(&disk, &key, Some("demo"), &repos, "oss")
            .await
            .unwrap();
        assert_eq!(restored.meta.repo, "demo");
        let head = std::fs::read_to_string(repo_dir(&repos, "demo").join("HEAD")).unwrap();
        assert_eq!(head, "ref: refs/heads/main\n");
        assert!(repo_dir(&repos, "demo")
            .join("objects/ab/cdef0123")
            .exists());
        // The staging directory must be gone.
        assert!(!repos.join(".demo.restore-staging").exists());
    }

    #[tokio::test]
    async fn a_snapshot_of_a_missing_repository_is_an_error() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("missing-repo");
        let err = backup(&disk, "nope", &repos, "oss").await.unwrap_err();
        assert!(format!("{err}").contains("nope"), "got: {err}");
    }

    #[tokio::test]
    async fn a_damaged_snapshot_is_refused() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("damaged");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        let (_, key) = backup(&disk, "demo", &repos, "oss").await.unwrap();

        // Flip one byte of the archive.
        let mut bytes = disk.get(&key).await.unwrap();
        let len = bytes.len();
        bytes[len - 1] ^= 0xff;
        disk.put(&key, &bytes).await.unwrap();

        let err = restore(&disk, &key, Some("demo"), &repos, "oss")
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("damaged"), "got: {err}");
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_snapshot_is_refused() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("not-a-snapshot");
        disk.put("snapshots/demo/x.tar", b"just some bytes")
            .await
            .unwrap();
        let err = restore(&disk, "snapshots/demo/x.tar", Some("demo"), &repos, "oss")
            .await
            .unwrap_err();
        assert!(
            format!("{err}").contains("not a gitgit snapshot"),
            "got: {err}"
        );
    }

    #[tokio::test]
    async fn a_snapshot_of_one_repository_cannot_be_restored_as_another() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("wrong-repo");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        let (_, key) = backup(&disk, "demo", &repos, "oss").await.unwrap();
        let err = restore(&disk, &key, Some("other"), &repos, "oss")
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("other"), "got: {err}");
    }

    #[tokio::test]
    async fn a_snapshot_cannot_write_outside_the_repository() {
        // A snapshot is a file that came from somewhere else; an entry
        // whose path climbs out of the archive must not be written next to
        // the repository.
        let disk = MemoryDisk::default();
        let repos = temp_dir("escape-path");
        let archive = tar::write(&[Entry {
            path: "../escaped.txt".to_string(),
            data: b"owned".to_vec(),
            mode: 0o644,
            mtime: 0,
        }])
        .unwrap();
        let meta = SnapshotMeta {
            version: SNAPSHOT_VERSION,
            disk: "oss".to_string(),
            repo: "demo".to_string(),
            created_unix: 0,
            entries: 1,
            sha256: hex::encode(Sha256::digest(&archive)),
            directories: Vec::new(),
        };
        let mut file = Vec::new();
        file.extend_from_slice(SNAPSHOT_MAGIC);
        file.extend_from_slice(&serde_json::to_vec(&meta).unwrap());
        file.push(b'\n');
        file.extend_from_slice(&archive);
        disk.put("snapshots/demo/x.tar", &file).await.unwrap();

        let err = restore(&disk, "snapshots/demo/x.tar", Some("demo"), &repos, "oss")
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("unsafe path"), "got: {err}");
        assert!(
            !repos.join("escaped.txt").exists(),
            "nothing may be written outside the repository"
        );
        assert!(
            !repos.parent().unwrap().join("escaped.txt").exists(),
            "nothing may be written outside the repository"
        );
    }

    #[test]
    fn a_failed_swap_puts_the_old_repository_back() {
        // The second rename fails ⇒ the old repository must be back where
        // it was, and the staged copy must be gone. The failure is
        // injected, not waited for.
        let dir = temp_dir("failed-swap");
        let staging = dir.join("staging");
        let target = dir.join("repo.git");
        let retired = dir.join("retired");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let mut calls = 0;
        let err = swap_in_with(&staging, &target, &retired, |from, to| {
            calls += 1;
            if calls == 2 {
                return Err(std::io::Error::other("the rename was refused"));
            }
            std::fs::rename(from, to)
        })
        .unwrap_err();
        assert!(
            format!("{err}").contains("the rename was refused"),
            "got: {err}"
        );
        // The old repository is back where it was.
        assert!(
            target.join("HEAD").is_file(),
            "the old repository must be back"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("HEAD")).unwrap(),
            "ref: refs/heads/main\n"
        );
        // The staged copy is gone.
        assert!(!staging.exists(), "the staged copy must be dropped");
        assert!(!retired.exists(), "the retired copy must be gone");
    }

    /// A snapshot file: the magic, the metadata line, the archive ✓.
    fn snapshot_bytes(meta: &SnapshotMeta, archive: &[u8]) -> Vec<u8> {
        let mut file = Vec::new();
        file.extend_from_slice(SNAPSHOT_MAGIC);
        file.extend_from_slice(&serde_json::to_vec(meta).unwrap());
        file.push(b'\n');
        file.extend_from_slice(archive);
        file
    }

    #[tokio::test]
    async fn a_snapshot_that_names_another_directory_is_refused() {
        // The repository name in the snapshot's metadata becomes a path:
        // `<repos_dir>/<name>.git`. A name that climbs out of the
        // repositories directory must be refused before anything is
        // written — the metadata came from the far end, so it is not
        // trusted.
        let disk = MemoryDisk::default();
        let repos = temp_dir("escape-repo-name");
        let archive = tar::write(&[Entry {
            path: "HEAD".to_string(),
            data: b"ref: refs/heads/main\n".to_vec(),
            mode: 0o644,
            mtime: 0,
        }])
        .unwrap();
        let meta = SnapshotMeta {
            version: SNAPSHOT_VERSION,
            disk: "oss".to_string(),
            repo: "../escape".to_string(),
            created_unix: 0,
            entries: 1,
            sha256: hex::encode(Sha256::digest(&archive)),
            directories: Vec::new(),
        };
        disk.put("snapshots/demo/x.tar", &snapshot_bytes(&meta, &archive))
            .await
            .unwrap();

        let err = restore(&disk, "snapshots/demo/x.tar", None, &repos, "oss")
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("invalid repo name"), "got: {err}");
        // Nothing may be written outside the repositories directory.
        assert!(
            !repos.parent().unwrap().join("escape.git").exists(),
            "nothing may be written outside the repositories directory"
        );
    }

    #[tokio::test]
    async fn a_snapshot_with_a_drive_qualified_path_is_refused() {
        // On Windows `C:/x` is a drive-qualified path, and joining it to
        // the staging directory throws the staging directory away. The
        // path has no `..` and no backslash, so only the drive prefix
        // gives it away.
        let disk = MemoryDisk::default();
        let repos = temp_dir("drive-path");
        let archive = tar::write(&[Entry {
            path: "C:/escaped.txt".to_string(),
            data: b"owned".to_vec(),
            mode: 0o644,
            mtime: 0,
        }])
        .unwrap();
        let meta = SnapshotMeta {
            version: SNAPSHOT_VERSION,
            disk: "oss".to_string(),
            repo: "demo".to_string(),
            created_unix: 0,
            entries: 1,
            sha256: hex::encode(Sha256::digest(&archive)),
            directories: Vec::new(),
        };
        disk.put("snapshots/demo/x.tar", &snapshot_bytes(&meta, &archive))
            .await
            .unwrap();

        let err = restore(&disk, "snapshots/demo/x.tar", Some("demo"), &repos, "oss")
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("unsafe path"), "got: {err}");
    }

    #[tokio::test]
    async fn a_snapshot_survives_a_real_object_store() {
        // `[FACT]` This repository **is** an S3-compatible server ✓, so
        // the whole S3 path runs end to end here ✓: no network ✓, no
        // third-party service ✓, no skipped test ✓. It is the only test
        // that exercises `S3CloudDisk` itself ✓ — the others use an
        // in-memory disk ✓.
        let server = crate::server::vault::test_support::start_object_store("cloud").await;
        let disk_name = format!("cloud-e2e-{}", std::process::id());
        // The credentials come from the environment only ✓ — so the test
        // puts them there ✓, exactly as an operator would ✓.
        std::env::set_var(
            access_key_env(&disk_name),
            crate::server::vault::test_support::ACCESS_KEY,
        );
        std::env::set_var(
            secret_key_env(&disk_name),
            crate::server::vault::test_support::SECRET_KEY,
        );

        let entry = CloudDiskEntry {
            name: disk_name.clone(),
            endpoint: server.endpoint.clone(),
            bucket: crate::server::vault::test_support::BUCKET.to_string(),
            region: "us-east-1".to_string(),
            prefix: "gitgit/".to_string(),
        };
        let registry = crate::cloud::CloudRegistry {
            disks: std::collections::BTreeMap::from([(entry.name.clone(), entry.clone())]),
        };
        let (access_key, secret_key) = registry.credentials_for(&entry).unwrap();
        let disk = crate::cloud::S3CloudDisk::connect(&entry, &access_key, &secret_key)
            .await
            .unwrap();

        let repos = temp_dir("real-store");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        let (meta, key) = backup(&disk, "demo", &repos, &disk_name).await.unwrap();
        assert!(key.starts_with("snapshots/demo/"), "got: {key}");

        // The listing must find it under the disk's own prefix ✓, and
        // report the disk-local name ✓ — the name a restore takes ✓.
        let listed = disk.list(SNAPSHOT_PREFIX).await.unwrap();
        assert!(
            listed.iter().any(|o| o.key == key),
            "the snapshot is missing from the listing: {listed:?}"
        );

        // Change the repository, then roll it back.
        std::fs::write(
            repo_dir(&repos, "demo").join("HEAD"),
            "ref: refs/heads/other\n",
        )
        .unwrap();
        let restored = restore(&disk, &key, Some("demo"), &repos, "oss")
            .await
            .unwrap();
        assert_eq!(restored.meta.repo, "demo");
        assert_eq!(restored.meta.sha256, meta.sha256);
        let head = std::fs::read_to_string(repo_dir(&repos, "demo").join("HEAD")).unwrap();
        assert_eq!(head, "ref: refs/heads/main\n");
        // The swap leaves nothing behind ✓.
        assert!(!repos.join(".demo.restore-staging").exists());
        assert!(!repos.join(".demo.restore-retired").exists());
    }

    #[test]
    fn the_snapshot_file_starts_with_the_magic() {
        let meta = SnapshotMeta {
            version: SNAPSHOT_VERSION,
            disk: "oss".to_string(),
            repo: "demo".to_string(),
            created_unix: 1_700_000_000,
            entries: 1,
            sha256: "00".repeat(32),
            directories: Vec::new(),
        };
        let json = serde_json::to_vec(&meta).unwrap();
        let mut file = Vec::new();
        file.extend_from_slice(SNAPSHOT_MAGIC);
        file.extend_from_slice(&json);
        file.push(b'\n');
        file.extend_from_slice(b"archive");
        let (parsed, archive) = split_snapshot(&file).unwrap();
        assert_eq!(parsed, meta);
        assert_eq!(archive, b"archive");
    }

    // ---- The safety snapshot ✓ ----

    #[tokio::test]
    async fn a_restore_leaves_a_safety_snapshot_of_the_repository_it_replaced() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("safety-snapshot");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");

        let (_, key) = backup(&disk, "demo", &repos, "oss").await.unwrap();
        let before = disk.list(SNAPSHOT_PREFIX).await.unwrap();
        assert_eq!(
            before.len(),
            1,
            "only the backup is on the disk: {before:?}"
        );

        let outcome = restore(&disk, &key, Some("demo"), &repos, "oss")
            .await
            .unwrap();

        // The disk holds **one more** snapshot ✓ — the safety snapshot ✓.
        let after = disk.list(SNAPSHOT_PREFIX).await.unwrap();
        assert_eq!(
            after.len(),
            2,
            "the restore must leave a safety snapshot behind: {after:?}"
        );

        // Its key has the safety-snapshot shape ✓ — so a reader of the
        // listing can tell it apart ✓.
        let safety = outcome
            .safety_snapshot
            .clone()
            .expect("a restore must report the safety snapshot's key");
        assert!(
            safety.starts_with("snapshots/demo/pre-restore-"),
            "the safety snapshot must be recognisable: {safety}"
        );
        assert!(safety.ends_with(".tar"), "got: {safety}");
        assert!(
            after.iter().any(|object| object.key == safety),
            "the safety snapshot must be on the disk: {after:?}"
        );
    }

    #[tokio::test]
    async fn the_safety_snapshot_holds_the_state_from_before_the_restore() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("safety-snapshot-content");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        let (_, key) = backup(&disk, "demo", &repos, "oss").await.unwrap();

        // Change the repository, then roll it back.
        std::fs::write(
            repo_dir(&repos, "demo").join("HEAD"),
            "ref: refs/heads/other\n",
        )
        .unwrap();
        std::fs::write(
            repo_dir(&repos, "demo").join("MARKER"),
            "only the newer state has this\n",
        )
        .unwrap();

        // The rollback replaces the repository.
        let outcome = restore(&disk, &key, Some("demo"), &repos, "oss")
            .await
            .unwrap();
        let head = std::fs::read_to_string(repo_dir(&repos, "demo").join("HEAD")).unwrap();
        assert_eq!(
            head, "ref: refs/heads/main\n",
            "the rollback must have happened"
        );

        // Restoring the safety snapshot undoes the rollback ✓ — that is
        // what "the rollback is reversible" means ✓.
        let safety = outcome
            .safety_snapshot
            .clone()
            .expect("a restore must report the safety snapshot's key");
        restore(&disk, &safety, Some("demo"), &repos, "oss")
            .await
            .unwrap();
        let head = std::fs::read_to_string(repo_dir(&repos, "demo").join("HEAD")).unwrap();
        assert_eq!(
            head, "ref: refs/heads/other\n",
            "the safety snapshot must hold the state from before the restore"
        );
        assert!(
            repo_dir(&repos, "demo").join("MARKER").is_file(),
            "the file that only the newer state had must be back"
        );
    }

    #[tokio::test]
    async fn a_restore_that_cannot_take_the_safety_snapshot_is_refused() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("safety-snapshot-refused");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        let (_, key) = backup(&disk, "demo", &repos, "oss").await.unwrap();

        // Change the repository, so a restore would really replace it.
        std::fs::write(
            repo_dir(&repos, "demo").join("HEAD"),
            "ref: refs/heads/other\n",
        )
        .unwrap();
        let before = contents_of(&repo_dir(&repos, "demo"));

        // The disk refuses every upload ⇒ the safety snapshot cannot be
        // taken ⇒ the restore must refuse too.
        disk.refuse_writes();

        let err = restore(&disk, &key, Some("demo"), &repos, "oss")
            .await
            .unwrap_err();
        assert!(
            format!("{err}").contains("safety snapshot"),
            "the error must name the step that failed: {err}"
        );

        // The local repository did not change — not one byte ✓.
        let after = contents_of(&repo_dir(&repos, "demo"));
        assert_eq!(before, after, "the repository must not have been touched");
        assert!(
            !repos.join(".demo.restore-staging").exists(),
            "nothing may be staged when the restore is refused"
        );
    }

    #[tokio::test]
    async fn a_restore_of_a_missing_repository_takes_no_safety_snapshot() {
        let disk = MemoryDisk::default();
        // The snapshot is taken from one repositories directory ...
        let source = temp_dir("no-safety-source");
        make_repo(&source, "demo", "ref: refs/heads/main\n");
        let (_, key) = backup(&disk, "demo", &source, "oss").await.unwrap();

        // ... and restored into another one that has no repository in it.
        let repos = temp_dir("no-safety-target");
        let outcome = restore(&disk, &key, Some("demo"), &repos, "oss")
            .await
            .unwrap();

        // The restore happened ✓.
        let head = std::fs::read_to_string(repo_dir(&repos, "demo").join("HEAD")).unwrap();
        assert_eq!(head, "ref: refs/heads/main\n");

        // There was nothing to lose ⇒ no safety snapshot ✓.
        assert!(
            outcome.safety_snapshot.is_none(),
            "a restore with no repository to replace must take no safety snapshot"
        );
        let objects = disk.list(SNAPSHOT_PREFIX).await.unwrap();
        assert_eq!(
            objects.len(),
            1,
            "the disk must hold only the backup: {objects:?}"
        );
    }

    // ---- The directories that hold no file ✓ ----

    #[tokio::test]
    async fn a_snapshot_records_the_directories_that_hold_no_file() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("empty-directory-recorded");
        let dir = repo_dir(&repos, "demo");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        // `refs/tags` is the shape a bare repository really has ✓: a
        // directory with nothing in it ✓.
        std::fs::create_dir_all(dir.join("refs/tags")).unwrap();

        let (meta, _) = backup(&disk, "demo", &repos, "oss").await.unwrap();

        assert_eq!(
            meta.directories,
            vec!["refs/tags".to_string()],
            "the directory that holds no file must be recorded: {meta:?}"
        );
        // The directories that DO hold a file are not recorded ✓ — the
        // files bring them back ✓.
        assert!(
            !meta.directories.iter().any(|d| d == "refs" || d == "refs/heads"),
            "only the empty directories are recorded: {meta:?}"
        );
    }

    #[tokio::test]
    async fn a_restore_brings_back_a_directory_that_holds_no_file() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("empty-directory-restored");
        let dir = repo_dir(&repos, "demo");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        std::fs::create_dir_all(dir.join("refs/tags")).unwrap();

        let (_, key) = backup(&disk, "demo", &repos, "oss").await.unwrap();
        // Take it away ✓ — so a restore that brings it back has to have
        // carried it ✓.
        std::fs::remove_dir_all(dir.join("refs/tags")).unwrap();
        assert!(!dir.join("refs/tags").exists());

        restore(&disk, &key, Some("demo"), &repos, "oss")
            .await
            .unwrap();

        assert!(
            dir.join("refs/tags").is_dir(),
            "a restore must bring back a directory that holds no file: {}",
            dir.join("refs/tags").display()
        );
    }

    #[tokio::test]
    async fn a_snapshot_of_a_version_this_build_does_not_know_is_refused() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("unknown-version");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        let archive = tar::write(&[Entry {
            path: "HEAD".to_string(),
            data: b"ref: refs/heads/main\n".to_vec(),
            mode: 0o644,
            mtime: 0,
        }])
        .unwrap();
        let meta = SnapshotMeta {
            version: 99,
            disk: "oss".to_string(),
            repo: "demo".to_string(),
            created_unix: 0,
            entries: 1,
            sha256: hex::encode(Sha256::digest(&archive)),
            directories: Vec::new(),
        };
        disk.put("snapshots/demo/x.tar", &snapshot_bytes(&meta, &archive))
            .await
            .unwrap();

        let error = restore(&disk, "snapshots/demo/x.tar", Some("demo"), &repos, "oss")
            .await
            .expect_err("a snapshot of an unknown version must be refused");
        let message = error.to_string();
        assert!(
            message.contains("99") && message.contains("version"),
            "the refusal must name the version it cannot read: {message}"
        );
    }
}
