# Dev/test object storage for the gitgit Credential Vault

Scope, stated up front because this directory is committed to the repo:

- **This is a DEVELOPMENT AND TEST fixture.** It is not a shipped
  component and nothing in the product needs it at runtime. The V0
  credential path is `FileVault` under `.gitgit/vault`; the S3 vault is
  opt-in through `GITGIT_VAULT_BACKEND=s3` and the `GITGIT_S3_*`
  settings, and ADR-0022 §2.1 keeps sqlx / PostgreSQL out of this crate
  on purpose.
- **The server is this repository's own.** `gitgit-objectstore`
  (`src/bin/gitgit-objectstore.rs` over `src/objectstore/`) speaks the
  S3 surface the credential vault's client uses. The bucket used to live
  on a GNU AGPLv3 server, which this product cannot carry, because it
  consumes the server over HTTP.
- **Nothing has to be installed but a Rust toolchain.** There is no
  container runtime, no docker-compose file and no separate S3 client.
  The binary's `--bootstrap-bucket` creates the bucket and enables
  versioning on it — the job the old fixture's `mc mb` + `mc version
  enable` pair did, in a second container, driven by a client this
  repository did not own.
- **The listen address is loopback only** by default, so nothing off
  this machine can reach the bucket.

## Start

```powershell
$env:GITGIT_OBJECTSTORE_SECRET_KEY = 'a-dev-secret'
pwsh -NoProfile -File deploy/objectstore/start-objectstore.ps1
```

The script builds `gitgit-objectstore` if it is not there yet, starts it
with the bucket provisioned and versioning enabled, and prints the
environment variables that point the credential vault at it. It runs in
the foreground; stop it with `Ctrl-C`.

The secret key is environment-only on purpose: there is no
`--secret-key` flag, because a long-lived secret in the process
arguments is visible in `ps` output and lands in the shell history. The
server has no default secret either and refuses to start without one.

## Verify

```powershell
pwsh -NoProfile -File scripts/verify-s3-vault.ps1
```

The verification script starts the same binary, drives the real product
binary against it (`gitgit key set` / `get` / `list-keys` /
`delete`), and runs the vault's own end-to-end tests. It needs nothing
installed either.

## What changed when this fixture replaced the old one

| Old | New |
| --- | --- |
| `deploy/minio/docker-compose.yml` | `deploy/objectstore/start-objectstore.ps1` |
| a container running the S3 server | the `gitgit-objectstore` binary from this repository |
| a second container running `mc` to create the bucket and enable versioning | the server's own `--bootstrap-bucket` flag |
| `mc anonymous set none` to keep the bucket private | nothing to do: every request is SigV4-verified, and an unsigned one is answered `403` |
| `scripts/verify-minio-vault.ps1` | `scripts/verify-s3-vault.ps1` |

One external dependency fewer, and one fewer thing to keep working.
