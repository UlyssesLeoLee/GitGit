# gm-desktop installer CI — both install scopes, one command

Date: 2026-10-05
Lane: L (`chore/lane-l-tauri-bundle-ci`)
Scope of this report: making the two Windows installer variants buildable in one
scripted shot, and proving in CI that they do not overwrite each other.

Every claim below is tagged. Nothing here is a narrative about how the repo got
here; each item is a statement about the current state, with the command that
establishes it.

---

## 1. The defect

`[FACT]` The Tauri bundler names an MSI `{productName}_{version}_{arch}_{lang}.msi`.
Observed verbatim in a real build log (`light` invocation):

```
Running light to produce E:\DevCache\cargo\target-lane-l\release\bundle\msi\gitgit Desktop_0.1.0_x64_en-US.msi
```

`[FACT]` That name carries product, version, arch and language but not the
install scope, and both scopes write into the same directory
(`<cargo-target>/release/bundle/msi/`). `[FACT]` A per-machine build and a
per-user build of the same version and language both emit
`gitgit Desktop_0.1.0_x64_en-US.msi` there — the second overwrites the first,
with exit code 0 on both runs.

`[FACT]` `productName` cannot carry the scope instead: it is the installed
application name and the Start Menu folder name.

`[INFERENCE]` Any fix that does not move the first artifact out of the shared
directory before the second build starts is not a fix.

## 2. The fix

`[FACT]` `apps/gm-desktop/scripts/build-msi.ps1` is the single entry point. Per
scope it runs `tauri build --bundles msi`, records which `.msi` files in the
bundle directory are **newer than the moment the build started**, moves each to
a deterministic output directory as
`{product}_{version}_{arch}_{scope}_{lang}.msi`, and asserts the original is
gone from the bundle directory.

`[FACT]` `apps/gm-desktop/scripts/verify-msi.ps1` is the smoke check: no
install, no elevation.

## 3. Artifacts produced

`[FACT]` One command, four variants, all coexisting in one directory:

```
pwsh -NoProfile -File apps/gm-desktop/scripts/build-msi.ps1 -Scope both
# EXIT=0  ELAPSED=00:04:44
```

| file | bytes | sha256 |
| --- | --- | --- |
| `gitgit Desktop_0.1.0_x64_perMachine_en-US.msi` | 6,742,016 | `6e8e79bc2f728b3afd16742e30bdfeb666a03e6e531222fa3c252a193784c22c` |
| `gitgit Desktop_0.1.0_x64_perMachine_zh-CN.msi` | 6,737,920 | `eed5f14bf1ec0aff5a2f5b06835c2a63946f1eb7f77cbb2effeab15b9dde99e8` |
| `gitgit Desktop_0.1.0_x64_perUser_en-US.msi` | 6,742,016 | `c8f835a4e83db56f2eb390e3e4834878018503f7ea383e52e7503299754e0364` |
| `gitgit Desktop_0.1.0_x64_perUser_zh-CN.msi` | 6,737,920 | `02697db58398fea8e6f273a02b5b85cbbb8da81eb8635f275fd31b099f405d7a` |

`[FACT]` The collision is gone, and the mechanism is observable rather than
inferred: after the per-machine pair was moved out, the bundle directory held
**0** `.msi` files before the per-user build started, and the per-user build
then wrote the *same* filenames (`gitgit Desktop_0.1.0_x64_en-US.msi`) into
that now-empty directory. Both survive.

`[FACT]` The two scopes of one language are the **same byte length but a
different hash** (6,742,016 B each, different sha256). The payload is the same
executable; only the package metadata differs. `[INFERENCE]` A size-only check
would therefore not have detected the collision — this is why the script
compares hashes.

`[FACT]` **MSI builds are not byte-reproducible.** A second run of the same
command produced different hashes for all four files at identical byte sizes
(e.g. perMachine/en-US `6e8e79bc…` then `ff04a272…`). `[INFERENCE]` Windows
Installer mints a fresh `ProductCode` GUID per build, so the hashes below
identify *that run's* artifacts and must not be treated as stable constants to
compare a future build against. What is stable across runs is the shape of the
result: four distinct filenames, four distinct hashes, the same byte sizes.

## 4. Smoke check evidence

`[FACT]`

```
pwsh -NoProfile -File apps/gm-desktop/scripts/verify-msi.ps1 -Dir <out>
# EXIT=0  ELAPSED=00:00:07
```

`[FACT]` Payload, via `msiexec /a /qn TARGETDIR=<tmp>` (administrative install:
resolves and lays out the file table, registers nothing, needs no elevation):

| variant | extracted path | exe bytes | files |
| --- | --- | --- | --- |
| perMachine | `PFiles\gitgit Desktop\gm-desktop.exe` | 19,577,344 | 3 |
| perUser | `LocalAppDataFolder\Programs\gitgit Desktop\gm-desktop.exe` | 19,577,344 | 3 |

`[FACT]` Scope, read out of the MSI tables through Windows Installer COM
automation (`Directory` and `Property`):

| variant | INSTALLDIR chain | ALLUSERS |
| --- | --- | --- |
| perMachine | `INSTALLDIR > ProgramFiles64Folder > TARGETDIR` | `1` |
| perUser | `INSTALLDIR > ProgramsFolder > LocalAppDataFolder > TARGETDIR` | *absent* |

`[FACT]` `ALLUSERS` is absent from the per-user packages. `[FACT]` This is the
load-bearing per-user invariant, and the check asserts it: `ALLUSERS=2` means
dual-purpose and overrides the scope WiX wrote, which is what produces error
1925 in a non-elevated session.

`[FACT]` `manifest.json` was re-read and all four recorded hashes and sizes
match the files on disk.

## 5. The loud-failure path

`[FACT]` The requirement is a non-zero exit when a build reports success but
the expected artifact is not there. Verified by pointing the script's
`-TargetDir` at an empty directory while the Tauri CLI wrote to the warm cargo
target — so the build genuinely succeeded, and the script looked in the wrong
place:

```
pwsh -NoProfile -File apps/gm-desktop/scripts/build-msi.ps1 -Scope perMachine \
  -Language en-US -TargetDir <empty-dir> -OutputDir <tmp>
# EXIT=1
# build-msi.ps1: tauri build (perMachine) exited 0 but wrote no new .msi.
#   Expected: gitgit Desktop_0.1.0_x64_en-US.msi.
#   Present in <empty-dir>\release\bundle\msi: <bundle directory does not exist>
```

`[FACT]` Four further defects were found by running the script rather than
reading it, and all are fixed: a scalar-string index that invoked `p` instead of
`pnpm`; a missing working directory for the Tauri call
(`ERR_PNPM_NO_IMPORTER_MANIFEST_FOUND`); `GetFullPath` on a relative
`-OutputDir` resolving against the .NET current directory rather than the repo
root; and the `Cargo.lock` warning, which failed twice — first because
`Write-Warning ("...") -f $lockFile` binds `-f` as a *parameter* of
`Write-Warning` and aborts with *a parameter cannot be found that matches
parameter name 'f'*, then because `("a {0} " + "b ") -f $x` leaves `{0}`
unsubstituted (`-f` binds tighter than `+`, so it applies only to the last
fragment). `[INFERENCE]` That warning path runs exactly when a fresh checkout
re-resolves the lock — that is, on the new CI job's first run — so shipping it
unfixed would have failed the job immediately.

## 6. CI

`[FACT]` `.github/workflows/gm-desktop.yml` gains a second job, `msi`, on
`windows-latest`. The existing `build` job is untouched at 15 steps; verified by
parsing the YAML (`build steps: 15`, `msi steps: 12`).

`[FACT]` The `msi` job builds both scopes, runs the smoke check, reads the
version out of `manifest.json` for the artifact names, and uploads
`gm-desktop-msi-<version>-perMachine` and `gm-desktop-msi-<version>-perUser` as
two distinct artifacts, plus the manifest.

`[FACT]` Exit-code discipline, against the two hazards named for this repo:
both scripts run via `pwsh -File` so their exit code lands in `$LASTEXITCODE`
and an explicit `if ($LASTEXITCODE -ne 0) { throw }` makes the step red; every
upload sets `if-no-files-found: error`, because the default `warn` leaves a
green job with no installer; there is no `continue-on-error`.

`[FACT]` The `msi` job does **not** use `needs: build`. `[INFERENCE]` Chaining
it would serialise a multi-minute Windows build behind the Linux gate and would
make a Linux failure skip the installer job entirely; running them in parallel
means the two gates report independently and neither can hide the other.

`[UNVERIFIED-FACT]` The job has never run. Everything above about the job's
*content* is verified by the same commands run locally; its wall-clock on a
GitHub-hosted runner is not. Measured locally: cold `cargo build --release` of
this crate tree is 8m 21s, and a warm-deps `-Scope both` run is 4m 44s, so
`[INFERENCE]` a cold runner lands near 15 minutes against a `timeout-minutes: 45`
ceiling. `[PROPOSAL]` If that is too slow for the runner budget, the cheaper
gate I would defend is dropping `zh-CN` from the default language list (one MSI
per scope instead of two) — the collision, the scope assertions and the payload
check are all unaffected by language count. I would not drop either scope,
because the two-scope comparison *is* the check.

## 7. Known problems found, not fixed here

`[FACT]` `apps/gm-desktop/src-tauri/Cargo.lock` does not satisfy the workspace
crate it depends on by path, so a Windows build re-resolves and rewrites it:

```
cargo build --release --locked
# error: cannot update the lock file ... because --locked was passed to prevent this
# EXIT=101

# without --locked, cargo reports:
#   Locking 3 packages to latest compatible versions
#   Updating aws-creds  v0.39.1 -> v0.40.0
#   Updating aws-region v0.28.1 -> v0.29.0
#   Updating rust-s3     v0.37.2 -> v0.38.0
```

`[FACT]` The root `Cargo.toml` requires `rust-s3 = "0.38"`. `[INFERENCE]` The
committed lock is stale, not Windows-specific.

`[FACT]` `build-msi.ps1` does not pass `--locked` (that fails outright) and
does not commit the refresh; it prints a warning naming the file when the build
changes it. `[FACT]` `Cargo.lock` is outside this lane's file ownership, so it
is left as found. **This needs an owner**: until it is refreshed, the MSIs are
not built from the committed lock, and the existing `cargo clippy --locked` step
in the `build` job is worth re-checking.

`[FACT]` `.github/CI.md` 已知限制 item 2 states that full Tauri packaging
(`.msi` / `tauri-action` / multi-OS matrix) is still not in CI, and suggests
adding a `needs: build` job later. `[FACT]` This change partially supersedes
that entry — `.msi` on `windows-latest` is now gated — and does not follow the
`needs: build` suggestion. Per instruction that file is **not** edited here;
it should be updated by whoever owns it.

## 8. Reproduce

```powershell
$env:CARGO_TARGET_DIR = 'E:\DevCache\cargo\target-lane-l'   # a private one
pwsh -NoProfile -File apps/gm-desktop/scripts/build-msi.ps1 -Scope both
pwsh -NoProfile -File apps/gm-desktop/scripts/verify-msi.ps1 -Dir "$env:CARGO_TARGET_DIR\msi-out"
```

`[FACT]` Toolchain used: cargo 1.98.1, node v22.19.0, pnpm 9.15.9, PowerShell 7.
`[FACT]` The WiX toolset is not on `PATH`; the Tauri CLI uses its own copy at
`%LOCALAPPDATA%\tauri\WixTools314`, so no WiX install step is needed.
`[FACT]` No code-signing certificate is involved: all four MSIs are unsigned.
