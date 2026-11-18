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

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::cloud::tar::{self, Entry};
use crate::cloud::{CloudDisk, SnapshotMeta, SNAPSHOT_MAGIC, SNAPSHOT_VERSION};
use crate::error::{GitGitError, Result};

/// The key prefix every snapshot of a disk lives under.
pub const SNAPSHOT_PREFIX: &str = "snapshots/";

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
    let dir = repo_dir(repos_dir, repo);
    if !dir.is_dir() {
        return Err(GitGitError::Cloud(format!(
            "no repository called {repo:?} under {}",
            repos_dir.display()
        )));
    }
    let entries = collect(&dir)?;
    let archive = tar::write(&entries).map_err(|e| GitGitError::Cloud(e.to_string()))?;
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
        entries: entries.len() as u64,
        sha256: digest.clone(),
    };

    let mut file = Vec::new();
    file.extend_from_slice(SNAPSHOT_MAGIC);
    let json = serde_json::to_vec(&meta)
        .map_err(|e| GitGitError::Cloud(format!("the snapshot metadata is unwritable: {e}")))?;
    file.extend_from_slice(&json);
    file.push(b'\n');
    file.extend_from_slice(&archive);

    let key = format!(
        "{SNAPSHOT_PREFIX}{repo}/{created}-{}.tar",
        &digest[..8]
    );
    disk.put(&key, &file).await?;
    Ok((meta, key))
}

/// Restore `repo` from the snapshot stored under `key`.
///
/// The archive's SHA-256 is checked against the metadata before anything
/// is written ✓ — a truncated or tampered download must not become the
/// repository ✓.
pub async fn restore(
    disk: &dyn CloudDisk,
    key: &str,
    repo: Option<&str>,
    repos_dir: &Path,
) -> Result<SnapshotMeta> {
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
    let digest = hex::encode(Sha256::digest(&archive));
    if digest != meta.sha256 {
        return Err(GitGitError::Cloud(format!(
            "the snapshot is damaged: its digest is {digest}, the metadata says {}",
            meta.sha256
        )));
    }

    let entries = tar::read(&archive).map_err(|e| GitGitError::Cloud(e.to_string()))?;
    let target = repo_dir(repos_dir, &target_repo);
    // Extract beside the repository and swap it in, so a failure never
    // leaves a half-written repository.
    let staging = repos_dir.join(format!(".{target_repo}.restore-staging"));
    if staging.exists() {
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;
    for entry in &entries {
        // A path that climbs out of the archive must not escape it.
        if entry.path.starts_with('/')
            || entry.path.split('/').any(|part| part == "..")
            || entry.path.contains('\\')
        {
            let _ = std::fs::remove_dir_all(&staging);
            return Err(GitGitError::Cloud(format!(
                "the snapshot contains an unsafe path: {:?}",
                entry.path
            )));
        }
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

    if target.exists() {
        std::fs::remove_dir_all(&target)?;
    }
    std::fs::rename(&staging, &target)?;
    Ok(meta)
}

/// Split a snapshot file into its metadata and its archive.
fn split_snapshot(file: &[u8]) -> Result<(SnapshotMeta, Vec<u8>)> {
    if !file.starts_with(SNAPSHOT_MAGIC) {
        return Err(GitGitError::Cloud(
            "that file is not a gitgit snapshot".to_string(),
        ));
    }
    let rest = &file[SNAPSHOT_MAGIC.len()..];
    let newline = rest.iter().position(|b| *b == b'\n').ok_or_else(|| {
        GitGitError::Cloud("the snapshot has no metadata line".to_string())
    })?;
    let meta: SnapshotMeta = serde_json::from_slice(&rest[..newline]).map_err(|e| {
        GitGitError::Cloud(format!("the snapshot metadata is unreadable: {e}"))
    })?;
    Ok((meta, rest[newline + 1..].to_vec()))
}

/// Collect every regular file under `dir` as a tar entry.
///
/// `[FACT]` The paths are relative to `dir` ✓ and use `/` separators ✓.
/// Symlinks are **skipped** ✓: a snapshot that followed one could read
/// outside the repository ✓, and a bare repository has none ✓.
fn collect(dir: &Path) -> Result<Vec<Entry>> {
    let mut entries = Vec::new();
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
                stack.push(path);
                continue;
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
    Ok(entries)
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
    use crate::cloud::{CloudObject, SNAPSHOT_MAGIC};

    /// A disk that keeps objects in memory ✓ — the tests must not need a
    /// network ✓.
    #[derive(Default)]
    struct MemoryDisk {
        objects: std::sync::Mutex<std::collections::BTreeMap<String, Vec<u8>>>,
    }

    #[async_trait::async_trait]
    impl CloudDisk for MemoryDisk {
        async fn put(&self, key: &str, bytes: &[u8]) -> Result<Option<String>> {
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
        std::fs::write(repo_dir(&repos, "demo").join("HEAD"), "ref: refs/heads/other\n").unwrap();
        let restored = restore(&disk, &key, Some("demo"), &repos).await.unwrap();
        assert_eq!(restored.repo, "demo");
        let head = std::fs::read_to_string(repo_dir(&repos, "demo").join("HEAD")).unwrap();
        assert_eq!(head, "ref: refs/heads/main\n");
        assert!(repo_dir(&repos, "demo").join("objects/ab/cdef0123").exists());
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

        let err = restore(&disk, &key, Some("demo"), &repos).await.unwrap_err();
        assert!(format!("{err}").contains("damaged"), "got: {err}");
    }

    #[tokio::test]
    async fn a_file_that_is_not_a_snapshot_is_refused() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("not-a-snapshot");
        disk.put("snapshots/demo/x.tar", b"just some bytes").await.unwrap();
        let err = restore(&disk, "snapshots/demo/x.tar", Some("demo"), &repos)
            .await
            .unwrap_err();
        assert!(format!("{err}").contains("not a gitgit snapshot"), "got: {err}");
    }

    #[tokio::test]
    async fn a_snapshot_of_one_repository_cannot_be_restored_as_another() {
        let disk = MemoryDisk::default();
        let repos = temp_dir("wrong-repo");
        make_repo(&repos, "demo", "ref: refs/heads/main\n");
        let (_, key) = backup(&disk, "demo", &repos, "oss").await.unwrap();
        let err = restore(&disk, &key, Some("other"), &repos).await.unwrap_err();
        assert!(format!("{err}").contains("other"), "got: {err}");
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
}
