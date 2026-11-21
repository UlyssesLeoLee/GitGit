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

    let key = format!("{SNAPSHOT_PREFIX}{repo}/{created}-{}.tar", &digest[..8]);
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

    // Extract beside the repository and swap it in, so a failure never
    // leaves a half-written repository.
    let staging = repos_dir.join(format!(".{target_repo}.restore-staging"));
    let retired = repos_dir.join(format!(".{target_repo}.restore-retired"));
    for scratch in [&staging, &retired] {
        remove_path(scratch)?;
    }
    std::fs::create_dir_all(&staging)?;
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
    Ok(meta)
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
    use crate::cloud::{
        access_key_env, secret_key_env, CloudDiskEntry, CloudObject, SNAPSHOT_MAGIC,
    };

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
        std::fs::write(
            repo_dir(&repos, "demo").join("HEAD"),
            "ref: refs/heads/other\n",
        )
        .unwrap();
        let restored = restore(&disk, &key, Some("demo"), &repos).await.unwrap();
        assert_eq!(restored.repo, "demo");
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

        let err = restore(&disk, &key, Some("demo"), &repos)
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
        let err = restore(&disk, "snapshots/demo/x.tar", Some("demo"), &repos)
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
        let err = restore(&disk, &key, Some("other"), &repos)
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
        };
        let mut file = Vec::new();
        file.extend_from_slice(SNAPSHOT_MAGIC);
        file.extend_from_slice(&serde_json::to_vec(&meta).unwrap());
        file.push(b'\n');
        file.extend_from_slice(&archive);
        disk.put("snapshots/demo/x.tar", &file).await.unwrap();

        let err = restore(&disk, "snapshots/demo/x.tar", Some("demo"), &repos)
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
        };
        disk.put("snapshots/demo/x.tar", &snapshot_bytes(&meta, &archive))
            .await
            .unwrap();

        let err = restore(&disk, "snapshots/demo/x.tar", None, &repos)
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
        };
        disk.put("snapshots/demo/x.tar", &snapshot_bytes(&meta, &archive))
            .await
            .unwrap();

        let err = restore(&disk, "snapshots/demo/x.tar", Some("demo"), &repos)
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
        let restored = restore(&disk, &key, Some("demo"), &repos).await.unwrap();
        assert_eq!(restored.repo, "demo");
        assert_eq!(restored.sha256, meta.sha256);
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
