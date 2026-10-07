#!/usr/bin/env bash
#
# Assert that `src-tauri/Cargo.lock` is current and self-consistent, so a
# bundle build cannot silently resolve something other than the committed lock.
#
# WHY THIS IS A SCRIPT AND NOT A FLAG
# ==================================
# `tauri build` gives no way to forward `--locked` to the cargo it invokes, so
# the property has to be asserted one step earlier. Doing it in a script rather
# than as six copies of a `run:` block means there is one definition to keep
# correct, and it can be executed locally — a gate nobody can run is a gate
# nobody has run.
#
# WHY IT EXISTS AT ALL
# ====================
# On 2026-10-05 `src-tauri/Cargo.lock` was genuinely stale against the root
# crate it takes by path, so `cargo build --release --locked` exited 101 and the
# bundle builds re-resolved and rewrote the lock on the runner. The lock was
# refreshed in `ab632ae` the same day and the report recorded the resolution.
#
# What did not follow is the change that makes the resolution durable. Two
# `run:` blocks in `gm-desktop-bundle.yml` and three in `release.yml` still
# invoked `tauri build` with nothing asserting the lock, and one of them still
# carried a comment explaining the omission by citing the stale-lock reason that
# had already been resolved. Reproducibility was therefore *true* — the lock
# happened to be current — rather than *guaranteed*.
#
# Runnable from any working directory: the crate root is derived from this
# script's own location, not from the caller's cwd.
#
# `[FACT]` Measured 2026-10-07, this is a passing condition, not a hoped-for
# one: `cargo metadata --locked` resolves the full graph and leaves the lock
# byte-identical. The gate exists to keep it that way without anyone having to
# remember to check.

set -euo pipefail

# .../apps/gm-desktop/scripts/assert-lock-current.sh -> .../apps/gm-desktop/src-tauri
script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
crate_dir="$(cd "${script_dir}/../src-tauri" && pwd)"

lock="${crate_dir}/Cargo.lock"
if [ ! -f "$lock" ]; then
  echo "::error::no Cargo.lock at ${lock}" >&2
  exit 1
fi

before=$(sha256sum "$lock" | cut -d' ' -f1)

# `--locked` means "assert this lock will not change". A non-zero exit here is
# the lock disagreeing with a manifest — the exact condition that used to make
# the bundle builds re-resolve without a word. `--format-version 1` and the
# redirect keep the full resolution (which is the expensive part and the whole
# point) while keeping megabytes of JSON out of the log.
if ! (cd "$crate_dir" && cargo metadata --locked --format-version 1 >/dev/null); then
  echo "::error::${lock} is not consistent with its manifests. A bundle build would re-resolve and produce an artifact that is not the one this commit describes. Refresh the lock and commit it." >&2
  exit 1
fi

after=$(sha256sum "$lock" | cut -d' ' -f1)
if [ "$before" != "$after" ]; then
  echo "::error::resolving rewrote ${lock} (${before} -> ${after}); refusing to build from a lock that changed under us" >&2
  exit 1
fi

echo "    ok  lock is current and self-consistent (${before})"
