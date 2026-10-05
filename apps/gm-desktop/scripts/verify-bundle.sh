#!/usr/bin/env bash
#
# verify-bundle.sh — prove that a bundled desktop artifact is the format it
# claims to be, then record what was actually produced.
#
# WHY THIS SCRIPT EXISTS
# ======================
# `tauri build` exiting 0 is not evidence that a usable installer was written.
# A bundler can produce a 0-byte file, or a payload-less package, and still
# exit 0. This repo has been bitten by the "green but produced nothing" shape
# more than once (see the coverage gate in `gm-desktop.yml` and the scope
# collision in `build-msi.ps1`), so the check is written to fail closed:
# every assertion below either passes or exits non-zero. Nothing is skipped
# silently, and there is no "warn and continue" path.
#
# It is the Unix counterpart of `verify-msi.ps1`, and deliberately mirrors it:
#
#   MSI (Windows)                          this script
#   ------------------------------         ---------------------------
#   msiexec /a  (extract, no install)      hdiutil attach -readonly
#     (registers nothing, no elevation)      -nobrowse (mount, no install)
#   Windows Installer COM table reads      dpkg-deb --info / --contents
#   manifest.json cross-checked            manifest.json cross-checked
#                                           against the files on disk
#
# PER TARGET
# =========
#   deb       `dpkg-deb --info` must parse the control fields, and the
#             `Version:` field must equal `tauri.conf.json > version`.
#             `dpkg-deb --contents` must list `usr/bin/<bin>` at a plausible
#             size plus a freedesktop `.desktop` entry. The size assertion is
#             what distinguishes "a real package" from "a package that links
#             nothing"; the MSI script makes the same distinction with
#             `-MinExeBytes 1MB`.
#
#   appimage  The first four bytes must be the ELF magic `\x7fELF`, `file`
#             must agree, and the file must clear a size floor. The magic
#             check is what fails an AppImage that is really the Vite web
#             bundle: `dist/` is a few hundred KB of HTML/JS and would pass a
#             naive extension check.
#
#   dmg       `hdiutil imageinfo` must parse the image, and the image is then
#             mounted read-only to prove the `.app` and its executable are
#             really inside it. If the image cannot be mounted the script
#             fails; it does not fall back to "the file exists, call it a
#             pass". An artifact we cannot open is not proof of anything.
#
# THE VERSION IS NEVER HARDCODED
# ==============================
# It is read from `tauri.conf.json > version` at run time and then asserted
# against the artifact itself (filename, `.deb` control field, and the
# mounted `.app`'s Info.plist). If the version in the manifest and the
# version in the package ever disagree, that is an error, not a cosmetic
# mismatch. The recorded `manifest.json` is what the workflow reads to name
# its artifacts, exactly as the `msi` job reads `msi-out/manifest.json`.
#
# UNVERIFIED
# ==========
# [UNVERIFIED-FACT] This script has never been executed. It was written on a
# Windows-only maintainer machine, where a `.dmg` and an `.AppImage` cannot be
# produced at all (Tauri does not cross-compile desktop bundles), so the first
# real execution will be its first execution. Everything stated about the
# bundler's output format below is read from the bundler source, not from a
# local build:
#
#   deb       {product}_{version}_{arch}.deb, arch `amd64` for x86_64;
#             `Package:` is the kebab-case product name; `Depends:` is written
#             only when the list is non-empty.
#             crates/tauri-bundler/src/bundle/linux/debian.rs
#   appimage  {product}_{version}_{arch}.AppImage
#             crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs
#   dmg       {product}_{version}_{arch}.dmg, arch `x64` / `aarch64` /
#             `universal`
#             crates/tauri-bundler/src/bundle/macos/dmg/mod.rs
#
# The arch token is deliberately NOT asserted: it depends on the runner
# (`macos-latest` is currently an Apple Silicon image, but that is a runner
# property, not a repo one, and hardcoding it here would produce a red CI the
# day GitHub moves the image). The version is asserted because it comes from
# this repo.
#
# EXIT CODES
# ==========
#   0  the artifact passed every check and manifest.json was written.
#   1  a check failed; the reason is printed as `::error::...`.
#   2  bad arguments.

set -euo pipefail

SCRIPT_NAME=$(basename "$0")

die() {
  echo "::error::${1}" >&2
  exit 1
}

step() { echo "==> ${1}"; }
ok()   { echo "    ok  ${1}"; }
note() { echo "    NOTE: ${1}"; }

usage() {
  cat <<EOF
usage: ${SCRIPT_NAME} --target <deb|appimage|dmg> --dir <bundle-dir>
                       [--out-dir <dir>] [--config <tauri.conf.json>]
                       [--cargo-toml <Cargo.toml>] [--bin <name>]
                       [--min-bytes <n>]

  --target     which bundle to verify; selects the structural check
  --dir        directory the bundler wrote into
  --out-dir    where manifest.json goes (default: --dir)
  --config     tauri.conf.json (default: ../src-tauri/tauri.conf.json)
  --cargo-toml Cargo.toml to read the binary name from
               (default: ../src-tauri/Cargo.toml)
  --bin        main executable name (default: read from Cargo.toml [[bin]])
  --min-bytes  size floor for the whole artifact (default: 5242880 = 5 MiB)
EOF
}

# ---------------------------------------------------------------------------
# Arguments
# ---------------------------------------------------------------------------
TARGET=""
DIR=""
OUT_DIR=""
CONFIG=""
CARGO_TOML=""
BIN_NAME=""
# 5 MiB. Far below any real bundle (the MSI payload alone measured 19.5 MB in
# docs/reports/2026-10-05-bundle-ci/README.md) and far above the Vite web
# bundle, so the floor fails an empty or mislabelled artifact without ever
# being the reason a real one fails.
MIN_BYTES=5242880

while [ $# -gt 0 ]; do
  case "$1" in
    --target)     TARGET="${2:-}";     shift 2 ;;
    --dir)        DIR="${2:-}";        shift 2 ;;
    --out-dir)    OUT_DIR="${2:-}";    shift 2 ;;
    --config)     CONFIG="${2:-}";     shift 2 ;;
    --cargo-toml) CARGO_TOML="${2:-}"; shift 2 ;;
    --bin)        BIN_NAME="${2:-}";   shift 2 ;;
    --min-bytes)  MIN_BYTES="${2:-}";  shift 2 ;;
    -h|--help)    usage; exit 0 ;;
    *) echo "::error::unknown argument '${1}'" >&2; usage >&2; exit 2 ;;
  esac
done

[ -n "$TARGET" ] || { echo "::error::--target is required" >&2; usage >&2; exit 2; }
[ -n "$DIR" ]    || { echo "::error::--dir is required" >&2;    usage >&2; exit 2; }

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
APP_DIR=$(dirname "$SCRIPT_DIR")
: "${CONFIG:=${APP_DIR}/src-tauri/tauri.conf.json}"
: "${CARGO_TOML:=${APP_DIR}/src-tauri/Cargo.toml}"
: "${OUT_DIR:=${DIR}}"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
sha256_of() {
  # GNU coreutils on Linux, shasum on macOS. Both are present on the hosted
  # runners; neither is guaranteed on a developer machine, so the absence is
  # an error rather than a silently skipped hash.
  #
  # [FACT] No `--` end-of-options marker is passed to either tool: shasum is a
  # perl script on macOS whose option handling differs from coreutils, and
  # none of the paths this script touches can begin with a dash (they all come
  # from `find` inside a bundle directory, named after the product).
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    die "neither sha256sum nor shasum is on PATH; cannot record a digest"
  fi
}

file_size() {
  # Byte count as a bare integer. `wc -c` is the only portable option that
  # does not depend on `stat` flag spelling differing between GNU and BSD.
  wc -c < "$1" | tr -d '[:space:]'
}

# ---------------------------------------------------------------------------
# Inputs
# ---------------------------------------------------------------------------
command -v jq >/dev/null 2>&1 \
  || die "jq is required to read the version from the manifest (the Tauri docs' own GitHub pipeline example reads the version with jq: https://v2.tauri.app/distribute/pipelines/github/)"

[ -f "$CONFIG" ]     || die "tauri config not found: ${CONFIG}"
[ -d "$DIR" ]        || die "bundle directory not found: ${DIR} (the bundler wrote nothing here)"

VERSION=$(jq -r '.version // empty' "$CONFIG")
[ -n "$VERSION" ] || die "no 'version' in ${CONFIG}; cannot assert the artifact carries the manifest version"
PRODUCT=$(jq -r '.productName // empty' "$CONFIG")

# The main executable name comes from the crate, not from a literal here, so
# renaming `[[bin]] name` in Cargo.toml cannot silently desynchronise the
# contents check from the package.
if [ -z "$BIN_NAME" ] && [ -f "$CARGO_TOML" ]; then
  BIN_NAME=$(awk '
    /^\[\[bin\]\]/            { inbin = 1; next }
    inbin && /^name[[:space:]]*=/ { gsub(/"/, "", $3); print $3; exit }
  ' "$CARGO_TOML")
fi
[ -n "$BIN_NAME" ] || BIN_NAME="gm-desktop"
if [ ! -f "$CARGO_TOML" ]; then
  note "no Cargo.toml at ${CARGO_TOML}; falling back to the expected binary name '${BIN_NAME}'"
fi

case "$TARGET" in
  deb)      EXT="deb" ;;
  appimage) EXT="AppImage" ;;
  dmg)      EXT="dmg" ;;
  *) die "unsupported --target '${TARGET}' (expected deb, appimage or dmg)" ;;
esac

step "verify ${TARGET} in ${DIR}"
echo "    product     : ${PRODUCT} ${VERSION}"
echo "    binary      : ${BIN_NAME}"
echo "    min bytes   : ${MIN_BYTES}"

# ---------------------------------------------------------------------------
# Locate the artifact — exactly one, and it must carry the manifest version
# ---------------------------------------------------------------------------
# [FACT] No `mapfile` here. macOS still ships bash 3.2, `mapfile` arrived in
# bash 4, and the .dmg job runs this script on a macOS runner where a Homebrew
# bash may or may not be first on PATH. A `bash`-4-only construct here would
# fail on the least-observable job in the file. The while-read form works on
# 3.2 and on 5.x alike.
FOUND_COUNT=0
ARTIFACT=""
while IFS= read -r CANDIDATE; do
  FOUND_COUNT=$((FOUND_COUNT + 1))
  ARTIFACT="$CANDIDATE"
done < <(find "$DIR" -maxdepth 1 -type f -name "*.${EXT}" -print | sort)

if [ "$FOUND_COUNT" -eq 0 ]; then
  # [FACT] `find -printf` is a GNU extension and BSD find rejects it, so the
  # listing is built with a portable loop instead. A diagnostic that only
  # works on one of the two platforms this script runs on is not a diagnostic.
  PRESENT=""
  while IFS= read -r F; do
    PRESENT="${PRESENT}  $(basename "$F") ($(file_size "$F") B)
"
  done < <(find "$DIR" -maxdepth 1 -type f -print | sort)
  [ -n "$PRESENT" ] || PRESENT="  <directory is empty>
"
  die "the bundler reported success but wrote no *.${EXT} in ${DIR}.
Expected one file matching *.${EXT}. Present:
${PRESENT}"
fi
if [ "$FOUND_COUNT" -gt 1 ]; then
  die "expected exactly one *.${EXT} in ${DIR}, found ${FOUND_COUNT}"
fi

ARTIFACT_NAME=$(basename "$ARTIFACT")
BYTES=$(file_size "$ARTIFACT")
echo "    artifact    : ${ARTIFACT_NAME}  ${BYTES} B"

if [ "$BYTES" -lt "$MIN_BYTES" ]; then
  die "${ARTIFACT_NAME} is ${BYTES} B, below the ${MIN_BYTES} B floor: the package carries no real payload."
fi
ok "size ${BYTES} B clears the ${MIN_BYTES} B floor"

if ! printf '%s' "$ARTIFACT_NAME" | grep -q -- "_${VERSION}_"; then
  die "${ARTIFACT_NAME} does not carry the manifest version ${VERSION} as a delimited filename component. The bundler and tauri.conf.json have diverged; do not pass this artifact off as a build of the current version."
fi
ok "filename carries the manifest version ${VERSION}"

# ---------------------------------------------------------------------------
# Structural check, per target
# ---------------------------------------------------------------------------
case "$TARGET" in

  deb)
    command -v dpkg-deb >/dev/null 2>&1 || die "dpkg-deb is required to inspect a .deb (package dpkg)"
    if ! INFO=$(dpkg-deb --info "$ARTIFACT" 2>&1); then
      die "dpkg-deb --info rejected ${ARTIFACT_NAME}; it is not a readable Debian package:
${INFO}"
    fi
    ok "dpkg-deb --info parsed the control fields"

    control_field() {
      # awk rather than `sed ... | head -1`: under `set -o pipefail` a
      # two-stage pipeline can hand back sed's SIGPIPE status, and awk's own
      # `exit` has no second process to upset. `sub` strips up to the first
      # colon only, so a value that itself contains a colon survives intact.
      #
      # The leading `[[:space:]]*` is not decoration: `dpkg-deb --info` indents
      # every control field by two spaces, so anchoring the key to column 1
      # matches nothing and every field assertion below would fail on a
      # perfectly good package.
      printf '%s\n' "$INFO" \
        | awk -v k="$1" '$0 ~ "^[[:space:]]*" k ":" { sub("^[^:]*:[[:space:]]*", ""); print; exit }'
    }

    PKG=$(control_field Package)
    CTRL_VERSION=$(control_field Version)
    ARCH_FIELD=$(control_field Architecture)
    [ -n "$PKG" ]         || die "the control file has no 'Package:' field"
    [ -n "$ARCH_FIELD" ]  || die "the control file has no 'Architecture:' field"
    [ "$CTRL_VERSION" = "$VERSION" ] \
      || die "the package declares Version '${CTRL_VERSION}' but tauri.conf.json says '${VERSION}'"
    ok "Package: ${PKG}  Version: ${CTRL_VERSION}  Architecture: ${ARCH_FIELD}"

    # `Depends:` is emitted only when the list is non-empty
    # (debian.rs: `if !dependencies.is_empty()`), and
    # `bundle.linux.deb.depends` is `[]` in tauri.conf.json. So the package
    # is expected to carry no Depends field at all, which means `dpkg -i` will
    # not pull in webkit2gtk and the app will not start on a clean machine.
    # That is a tauri.conf.json concern, not a workflow one, and it is
    # reported rather than hidden and rather than failed here — failing it
    # would make this script a gate on a config the workflow does not own.
    DEPENDS=$(control_field Depends)
    if [ -n "$DEPENDS" ]; then
      note "the package declares Depends: ${DEPENDS}"
    else
      note "the package declares NO Depends field (bundle.linux.deb.depends is [] in tauri.conf.json). A real install would not pull in webkit2gtk. See .github/CI.md."
    fi

    if ! CONTENTS=$(dpkg-deb --contents "$ARTIFACT" 2>&1); then
      die "dpkg-deb --contents could not list ${ARTIFACT_NAME}:
${CONTENTS}"
    fi
    # `./` is stripped before comparing because `dpkg-deb --contents` prints
    # every path with a leading `./` — comparing against `usr/bin/<bin>`
    # directly would never match and the check would fail on a good package.
    BIN_ENTRY=$(printf '%s\n' "$CONTENTS" \
      | awk -v b="usr/bin/${BIN_NAME}" '{ p = $NF; sub(/^\.\//, "", p); if (p == b) { print; exit } }')
    [ -n "$BIN_ENTRY" ] \
      || die "${ARTIFACT_NAME} contains no usr/bin/${BIN_NAME}. The package installs no executable, so it is not a runnable application."
    BIN_BYTES=$(printf '%s' "$BIN_ENTRY" | awk '{ print $3 }')
    case "$BIN_BYTES" in
      ''|*[!0-9]*) die "could not read a size out of the usr/bin/${BIN_NAME} entry: '${BIN_ENTRY}'" ;;
    esac
    [ "$BIN_BYTES" -ge 1048576 ] \
      || die "usr/bin/${BIN_NAME} inside ${ARTIFACT_NAME} is ${BIN_BYTES} B, below the 1 MiB floor: the package links no real binary."
    ok "payload: usr/bin/${BIN_NAME} = ${BIN_BYTES} B"

    DESKTOP_ENTRY=$(printf '%s\n' "$CONTENTS" | awk '$0 ~ /usr\/share\/applications\/[^/]+\.desktop$/ { print; exit }')
    [ -n "$DESKTOP_ENTRY" ] \
      || die "${ARTIFACT_NAME} contains no usr/share/applications/*.desktop: the freedesktop launch metadata was not generated."
    ok "freedesktop entry: $(printf '%s' "$DESKTOP_ENTRY" | awk '{ print $NF }')"
    # `|| true` because `grep -c` exits 1 on zero matches, and under `set -e`
    # that would abort the script with no message instead of printing 0.
    # Unreachable in practice — the usr/bin assertion above would have failed
    # first — but a bare unexplained exit is exactly the kind of thing this
    # script exists to avoid.
    FILE_COUNT=$(printf '%s\n' "$CONTENTS" | grep -c . || true)
    ok "${FILE_COUNT} path(s) in the package"
    ;;

  appimage)
    command -v file >/dev/null 2>&1 || die "the 'file' utility is required to inspect an AppImage (apt install file)"

    # The first four bytes must be the ELF magic. This is the check that fails
    # an "AppImage" that is really the Vite web bundle: dist/ is HTML, JS and
    # a manifest.json, none of which start with \x7fELF.
    MAGIC=$(head -c 4 "$ARTIFACT" | od -An -tx1 | tr -d ' \n')
    [ "$MAGIC" = "7f454c46" ] \
      || die "${ARTIFACT_NAME} does not start with the ELF magic (got '${MAGIC}'). It is not an executable AppImage; a renamed web bundle lands here."
    ok "ELF magic present (${MAGIC} = 7f454c46)"

    # The word 'ELF' alone is too weak: `file` reports a file whose only
    # correct property is the magic as "ELF invalid class invalid byte order
    # ... unknown class 0", and that string contains 'ELF'. Requiring a
    # 64-bit ELF rejects it.
    #
    # What this deliberately does NOT require is the trailing word. A real
    # x86-64 binary is reported as "pie executable", but an AppImage is built
    # by concatenating a downloaded runtime with a squashfs image, and
    # [UNVERIFIED-FACT] whether that composite is reported as "executable" or
    # "shared object" depends on the runtime that linuxdeploy happened to
    # fetch. Requiring "executable" would be a bet on wording nobody here can
    # check, and a wrong bet turns a good build red. The full `file` output is
    # printed either way, so the first run settles it in the log.
    FILE_OUT=$(file -b "$ARTIFACT")
    case "$FILE_OUT" in
      *"ELF 64-bit"*) ok "file(1): ${FILE_OUT}" ;;
      *) die "file(1) does not report a 64-bit ELF for ${ARTIFACT_NAME}, which is what a real AppImage is:
  ${FILE_OUT}" ;;
    esac

    # Diagnostic only, deliberately not an assertion. An AppImage type-2
    # runtime carries the ASCII bytes `AI\x02` at offset 8. The bundler zeroes
    # offset 8 of the linuxdeploy *tool* it downloads, to stop integration
    # tools from detecting it
    # (crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs, the `dd`
    # invocation), but it does not touch the produced image, so `414902` is
    # expected here. [UNVERIFIED-FACT] That expectation is read from the source
    # and has never been observed on a real build; the first run decides
    # whether it becomes an assertion. Printing it is how that gets decided
    # without a guess being enforced as a gate.
    RUNTIME_MAGIC=$(head -c 11 "$ARTIFACT" | od -An -tx1 | tr -d ' \n' | cut -c9-14)
    echo "    bytes 8-10  : ${RUNTIME_MAGIC} (expect 414902 for an AppImage type-2 runtime)"
    ;;

  dmg)
    command -v hdiutil >/dev/null 2>&1 || die "hdiutil is required to inspect a .dmg (macOS only)"

    if ! hdiutil imageinfo "$ARTIFACT" >/dev/null 2>&1; then
      die "hdiutil imageinfo rejected ${ARTIFACT_NAME}; it is not a readable Apple disk image."
    fi
    ok "hdiutil imageinfo parsed the disk image"

    # Mount read-only, exactly as the MSI job performs an administrative
    # install: open the package, change nothing, no privileges required. The
    # image is mounted because the question being asked is "does this contain
    # a working application", and only the mounted volume can answer it.
    MOUNT_DIR=$(mktemp -d)
    MOUNTED=0
    cleanup() {
      if [ "$MOUNTED" = "1" ]; then
        hdiutil detach "$MOUNT_DIR" >/dev/null 2>&1 || true
      fi
      rmdir "$MOUNT_DIR" >/dev/null 2>&1 || true
    }
    trap cleanup EXIT

    hdiutil attach -readonly -nobrowse -mountpoint "$MOUNT_DIR" "$ARTIFACT" >/dev/null \
      || die "could not mount ${ARTIFACT_NAME} read-only. Failing closed: an image we cannot open proves nothing. (If this is the runner refusing to mount rather than a bad artifact, that is a finding, not something to work around silently.)"
    MOUNTED=1
    ok "mounted read-only at ${MOUNT_DIR}"

    # [FACT] `find -quit` is a GNU extension that BSD find rejects, so the
    # first match is taken with `head -1` instead. The `|| true` is deliberate:
    # under `set -e` a failing command substitution would abort the script with
    # no explanation, whereas letting it through reaches the assertion below
    # and fails with the directory listing, which is the message that helps.
    APP_PATH=$(find "$MOUNT_DIR" -maxdepth 1 -name '*.app' -print 2>/dev/null | head -1 || true)
    [ -n "$APP_PATH" ] \
      || die "the mounted image contains no .app bundle:
$(find "$MOUNT_DIR" -maxdepth 2 -print 2>/dev/null | head -20)"

    APP_NAME=$(basename "$APP_PATH")
    MACHO_BIN="${APP_PATH}/Contents/MacOS/${BIN_NAME}"
    [ -f "$MACHO_BIN" ] \
      || die "the mounted ${APP_NAME} has no Contents/MacOS/${BIN_NAME}:
$(find "${APP_PATH}/Contents" -maxdepth 2 -print 2>/dev/null | head -20)"
    MACHO_BYTES=$(file_size "$MACHO_BIN")
    [ "$MACHO_BYTES" -ge 1048576 ] \
      || die "Contents/MacOS/${BIN_NAME} inside ${APP_NAME} is ${MACHO_BYTES} B, below the 1 MiB floor: the bundle links no real binary."
    ok "payload: ${APP_NAME}/Contents/MacOS/${BIN_NAME} = ${MACHO_BYTES} B"

    PLIST="${APP_PATH}/Contents/Info.plist"
    [ -f "$PLIST" ] || die "the mounted ${APP_NAME} has no Contents/Info.plist"
    # `mktemp` rather than a literal path: writing into a world-writable /tmp
    # by a fixed name is a symlink hazard, and $TMPDIR is the portable
    # location on both GNU and BSD.
    PLIST_OUT=$(mktemp)
    # plutil -p prints the plist without requiring a specific key name, so
    # this asserts the version is in the bundle rather than betting on which
    # of CFBundleShortVersionString / CFBundleVersion carries it.
    if ! plutil -p "$PLIST" >"$PLIST_OUT" 2>/dev/null; then
      rm -f "$PLIST_OUT"
      die "plutil could not read ${PLIST}"
    fi
    if ! grep -q -- "$VERSION" "$PLIST_OUT"; then
      echo "$(head -30 "$PLIST_OUT")" >&2
      rm -f "$PLIST_OUT"
      die "the mounted ${APP_NAME} Info.plist does not mention the manifest version ${VERSION} (dumped above)"
    fi
    rm -f "$PLIST_OUT"
    ok "Info.plist carries the manifest version ${VERSION}"
    ;;
esac

# ---------------------------------------------------------------------------
# Record, then re-read and re-verify the record
# ---------------------------------------------------------------------------
# Signing is read from the environment rather than asserted here, because the
# value differs per platform: the Linux bundlers sign nothing, and the macOS
# job sets APPLE_SIGNING_IDENTITY=- for an ad-hoc signature, which is what
# Tauri's docs prescribe when no Apple-authenticated identity exists
# (https://v2.tauri.app/distribute/sign/macos/, "Ad-Hoc Signing"). Apple
# notarization is a different thing entirely and is not performed at all: it
# requires an Apple ID or App Store Connect key, neither of which exists for
# this project, so nothing here is notarized and nothing here is claimed to
# be distributable to an end user.
case "$TARGET" in
  dmg) SIGNING="${APPLE_SIGNING_IDENTITY:+ad-hoc (${APPLE_SIGNING_IDENTITY})}"; : "${SIGNING:=none}" ;;
  *)   SIGNING="none" ;;
esac

SHA=$(sha256_of "$ARTIFACT")
MANIFEST="${OUT_DIR}/manifest.json"
mkdir -p -- "$OUT_DIR"

jq -n \
  --arg ts "$(date -u +%Y-%m-%dT%H:%M:%SZ)" \
  --arg p  "$PRODUCT" \
  --arg v  "$VERSION" \
  --arg t  "$TARGET" \
  --arg a  "$ARTIFACT_NAME" \
  --argjson b "$BYTES" \
  --arg s  "$SHA" \
  --arg sg "$SIGNING" \
  --arg r  "$(uname -s -m)" \
  '{
     generatedAtUtc: $ts,
     productName:    $p,
     version:        $v,
     target:         $t,
     runner:         $r,
     signing:        $sg,
     notarized:      false,
     artifacts: [ { file: $a, bytes: $b, sha256: $s } ]
   }' > "$MANIFEST"

# The manifest is the contract the workflow names its artifacts with, so it is
# read back off disk and re-checked against the files rather than trusted.
# This is the same cross-check verify-msi.ps1 performs on msi-out/manifest.json.
jq -e '.artifacts | length == 1' "$MANIFEST" >/dev/null \
  || die "${MANIFEST} does not record exactly one artifact"
R_FILE=$(jq -r '.artifacts[0].file'   "$MANIFEST")
R_BYTES=$(jq -r '.artifacts[0].bytes'  "$MANIFEST")
R_SHA=$(jq  -r '.artifacts[0].sha256' "$MANIFEST")
R_VERSION=$(jq -r '.version'           "$MANIFEST")

[ "$R_VERSION" = "$VERSION" ] || die "${MANIFEST} records version '${R_VERSION}', expected '${VERSION}'"
[ "$R_FILE"    = "$ARTIFACT_NAME" ] || die "${MANIFEST} records '${R_FILE}', expected '${ARTIFACT_NAME}'"
[ "$R_BYTES"   = "$BYTES" ]         || die "${MANIFEST} records ${R_BYTES} B, the file on disk is ${BYTES} B"
[ "$R_SHA"     = "$SHA" ]           || die "${MANIFEST} records sha256 ${R_SHA}, the file on disk hashes to ${SHA}"
ok "manifest ${MANIFEST}: 1 artifact, size and sha256 match the file on disk"

step "PASS: ${ARTIFACT_NAME} is a real ${TARGET} bundle (unsigned; not notarized)"
