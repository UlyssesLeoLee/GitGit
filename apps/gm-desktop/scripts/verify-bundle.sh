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
# Is there a copy of THE LICENCE in there?
# ---------------------------------------------------------------------------
#
# A file whose NAME looks like a licence is not a licence.
#
# [FACT] Measured on the AppImage this repository builds: it contains
# `usr/share/doc/libwayland-egl1/copyright`, which is the copyright file of a
# BUNDLED LIBRARY. A name-only check reported that as the product's licence:
#
#     ok  licence: usr/share/doc/libwayland-egl1/copyright
#
# The name is only a way to find candidates. What decides whether an artefact
# complies is whether one of them carries the AGPL-3.0 text.
#
# TWO phrases, not one. The title alone is not enough either: a document that
# merely mentions the licence would satisfy it, and this repository's own
# README.md contains the title twice. "TERMS AND CONDITIONS" is the AGPL's own
# section heading, and no dependency's copyright file carries it.
AGPL_TITLE='GNU AFFERO GENERAL PUBLIC LICENSE'
AGPL_SECTION='TERMS AND CONDITIONS'

# Every licence-shaped file under a root, one per line. `find` is given
# `-iname` so the pattern is case-insensitive on every platform.
list_licence_candidates() {
  find "$1" -type f \
    \( -iname 'licen[cs]e*' -o -iname 'copying*' -o -iname 'copyright*' -o -iname 'notice*' \) \
    2>/dev/null || true
}

# Does this file carry the AGPL-3.0 text? `grep -F` so the phrases are taken
# literally; a regular expression would read the parentheses in them.
carries_agpl() {
  grep -q -F "$AGPL_TITLE" "$1" 2>/dev/null && grep -q -F "$AGPL_SECTION" "$1" 2>/dev/null
}

# The path, relative to the root, of a licence-bearing file that carries the
# AGPL-3.0 text. Non-zero exit if there is none.
find_product_licence() {
  local root="$1" candidate
  while IFS= read -r candidate; do
    [ -n "$candidate" ] || continue
    if carries_agpl "$candidate"; then
      printf '%s\n' "${candidate#"$root"/}"
      return 0
    fi
  done < <(list_licence_candidates "$root")
  return 1
}

# The candidates that were found but did NOT carry the licence, so a reader
# can see what was there and why it did not count.
report_rejected_licences() {
  local root="$1" candidate
  while IFS= read -r candidate; do
    [ -n "$candidate" ] || continue
    if ! carries_agpl "$candidate"; then
      echo "    not the product's licence: ${candidate#"$root"/}"
    fi
  done < <(list_licence_candidates "$root")
}

# ---------------------------------------------------------------------------
# Is there a Corresponding Source NOTICE next to the object code?
# ---------------------------------------------------------------------------
#
# [FACT] The licence copy answers AGPL-3.0 section 4. This is a different
# obligation, and it is about the DISTRIBUTION rather than about the
# package's contents.
#
# Section 6(d) (LICENSE:263-275) says, of object code conveyed from a
# designated place: "provided you maintain clear directions next to the
# object code saying where to find the Corresponding Source." So the thing
# being looked for is a NOTICE - a set of directions - and not a second copy
# of the licence.
#
# [FACT] The notice is a COMMITTED file (CORRESPONDING-SOURCE.md at the root
# of the repository), mapped into the packages through the same
# `bundle.linux.*.files` maps that carry the licence. It is not generated at
# build time: a generated notice would be a claim about the source that
# nothing in the repository holds, and it would drift from the source it
# names.
#
# TWO things have to be true of a file for it to be the notice, and BOTH are
# checked:
#
#   1. it says what the notice is about - "Corresponding Source for this
#      build". The AGPL text itself uses the phrase "Corresponding Source" 24
#      times, so the bare phrase cannot be used to tell the notice apart from
#      the licence, and this repository's README names the licence too.
#   2. it names the licence and the place the source is offered at, so a
#      reader is not left with a heading and no directions.
#
# The place is passed in rather than read from a global, so that this helper
# can be executed on its own - see the gate in
# `tests/unit/corresponding-source-notice.test.ts`.
CS_NOTICE_MARKER='Corresponding Source for this build'
CS_LICENCE_MARKER='AGPL-3.0'

# Does this file carry the Corresponding Source notice? `grep -F` so the
# phrases are taken literally.
carries_corresponding_source() {
  grep -q -F "$CS_NOTICE_MARKER" "$1" 2>/dev/null \
    && grep -q -F "$CS_LICENCE_MARKER" "$1" 2>/dev/null \
    && grep -q -F "$2" "$1" 2>/dev/null
}

# The path, relative to the root, of a file that carries the notice.
# Non-zero exit if there is none.
#
# EVERY file under the root is a candidate, not a name pattern. The name of
# the notice is the bundler's business: a check that looks for a name can be
# satisfied by a file that merely LOOKS like a notice and carries nothing,
# which is the same false green the licence check used to produce when it
# decided by name.
find_corresponding_source_notice() {
  local root="$1" place="$2" candidate
  while IFS= read -r candidate; do
    [ -n "$candidate" ] || continue
    if carries_corresponding_source "$candidate" "$place"; then
      printf '%s\n' "${candidate#"$root"/}"
      return 0
    fi
  done < <(find "$root" -type f 2>/dev/null || true)
  return 1
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

# The place the Corresponding Source is offered at, which the notice has to
# name. It is derived from the project's own homepage rather than written out
# here, so the notice and the configuration cannot drift apart silently: the
# notice is a committed file, and a committed file that names a place the
# configuration no longer names is a notice pointing nowhere.
HOMEPAGE=$(jq -r '.bundle.homepage // empty' "$CONFIG")
[ -n "$HOMEPAGE" ] \
  || die "no 'bundle.homepage' in ${CONFIG}; the Corresponding Source notice has to name the place the source is offered at, and that place is derived from it."
CS_SOURCE_PLACE="${HOMEPAGE}/releases"

# Where the notice was found inside the artifact, relative to the artifact's
# root. Empty until a branch finds one; the dmg branch has no file slot, so
# for that target it stays empty and the manifest records that.
CS_NOTICE_PATH=""

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
    # (debian.rs: `if !dependencies.is_empty()`), and `DebianSettings`
    # derives `Default`, so Tauri supplies no fallback list at all: an
    # empty or absent `bundle.linux.deb.depends` means the package
    # declares no runtime dependencies.
    #
    # `[FACT]` This was a `note` until 2026-10-05, on the reasoning that
    # "failing it would make this script a gate on a config the workflow
    # does not own". The config is in this repository, and what was being
    # noted was that the shipped .deb installs on a machine with no
    # webkit2gtk and then fails to launch. A note is the right weight for
    # "this might matter" and the wrong weight for "a real install pulls
    # nothing in".
    #
    # It asserts that the list reached the control file rather than
    # comparing against a hardcoded expected string. A fixed string would
    # only ever re-assert what the sibling config already says; reading
    # the declared list back out of `tauri.conf.json` catches the failure
    # that actually happens -- someone empties the list again, or the
    # bundler stops writing the field, and the package silently ships
    # under-declared with the note nobody reads.
    DEPENDS=$(control_field Depends)
    [ -n "$DEPENDS" ] \
      || die "${ARTIFACT_NAME} declares no Depends field, so 'dpkg -i' pulls in nothing and the app will not start on a machine without webkit2gtk. Set bundle.linux.deb.depends in tauri.conf.json."

    # Read the declared list with jq rather than a regex over the file.
    # `$CONFIG` is already required to exist and jq is already required
    # for `.version` a few lines above, so this adds no new dependency --
    # and a hand-rolled pattern over JSON is exactly the kind of thing
    # that silently matches nothing, which is the failure mode the
    # emptiness check below exists to catch.
    DECLARED=$(jq -r '.bundle.linux.deb.depends // [] | .[]' "$CONFIG")
    [ -n "$DECLARED" ] \
      || die "bundle.linux.deb.depends in ${CONFIG} is empty or missing, so there is nothing to assert the control file against."
    for dep in $DECLARED; do
      printf '%s' "$DEPENDS" | grep -q -- "${dep}" \
        || die "${ARTIFACT_NAME} Depends does not mention '${dep}', which ${CONFIG} declares. The control file and the config have diverged."
    done
    ok "every declared dependency reached the control file: $(printf '%s' "$DECLARED" | tr '\n' ' ')"

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

    # ── AGPL-3.0 §4: convey a copy of the Licence with every copy of the
    # Program. ────────────────────────────────────────────────────────────────────
    # Measured on release 404746754 before this check existed: the published
    # .deb contained exactly five files — the binary, one .desktop entry and
    # three icons — and no licence text of any kind.
    #
    # The path is deliberately NOT pinned, and the key that carries the file is
    # `bundle.linux.deb.files`, NOT `bundle.licenseFile`.
    #
    # That is a measured correction, not a preference. `bundle.licenseFile` is
    # what an earlier version of this script told a reader to go fix, on the
    # theory that the key named in the Tauri reference does the job. It does
    # not: with `licenseFile` set, run 37642251095 still produced a .deb holding
    # exactly the same five files. `crates/tauri-bundler/src/bundle/linux/
    # debian.rs` fills its data directory from `copy_resource_files`, binaries,
    # icons + desktop entry, an optional `deb.changelog`, and
    # `fs_utils::copy_custom_files(&settings.deb().files, &data_dir)` —
    # `licenseFile` is in none of them.
    #
    # Where a .deb "appropriate" copy of the licence lives is then the
    # bundler's business, and this assertion is loose on purpose.
    # `/usr/share/doc/<pkg>/copyright` by Debian convention,
    # `/usr/share/licenses/...` by some tools — pinning one exact path would
    # encode a guess about someone else's layout and turn red on a compliant
    # package; requiring that SOME licence-bearing file is present is the actual
    # requirement and cannot be satisfied by a wrong guess.
    #
    # The package is EXTRACTED rather than read from a listing, because the
    # decision is about content: a .deb can carry a bundled library's
    # copyright file, and a name-only check would call that the product's
    # licence. See `find_product_licence` above.
    DEB_EXTRACT_DIR=$(mktemp -d)
    trap 'rm -rf "$DEB_EXTRACT_DIR"' EXIT
    dpkg-deb --extract "$ARTIFACT" "$DEB_EXTRACT_DIR" \
      || die "could not extract ${ARTIFACT_NAME}, so the licence content cannot be checked."
    LICENSE_ENTRY=$(find_product_licence "$DEB_EXTRACT_DIR" || true)
    if [ -z "$LICENSE_ENTRY" ]; then
      echo "${CONTENTS}" | sed 's/^/    /' >&2
      report_rejected_licences "$DEB_EXTRACT_DIR" >&2
      die "${ARTIFACT_NAME} ships no copy of the Licence.
AGPL-3.0 section 4 requires a copy of the Licence with every copy of the
Program. A file whose NAME looks like a licence does not count: the package may
carry a dependency's copyright file and no copy of the Licence at all. Check
bundle.linux.deb.files in ${CONFIG} still maps a usr/share/doc/... destination
at a source that exists. It is destination -> source, not source -> destination
(run 37644440552 measured the build failing on the reversed spelling), and it
is that map - not bundle.licenseFile - that the deb bundler actually reads."
    fi
    ok "licence: ${LICENSE_ENTRY}"
    # The candidates that are NOT the product's licence are reported rather
    # than ignored, because their presence is exactly what made a name-only
    # check report the wrong file.
    report_rejected_licences "$DEB_EXTRACT_DIR"

    # ── AGPL-3.0 §6(d): clear directions next to the object code ──────────────
    # [FACT] The package was EXTRACTED above, so this decision is made on the
    # bytes that were really packaged, not on the configuration that was
    # meant to produce them.
    DEB_NOTICE=$(find_corresponding_source_notice "$DEB_EXTRACT_DIR" "$CS_SOURCE_PLACE" || true)
    if [ -z "$DEB_NOTICE" ]; then
      die "${ARTIFACT_NAME} ships no Corresponding Source notice next to the object code.
AGPL-3.0 section 6(d) requires the distributor of object code to maintain clear
directions next to the object code saying where to find the Corresponding Source.
A file whose NAME looks like a notice does not count: the package may carry a
dependency's file and no directions at all. Check that bundle.linux.deb.files in
${CONFIG} still maps a usr/share/... destination at a source that exists, and
that the source names the licence and the place the source is offered at
(${CS_SOURCE_PLACE})."
    fi
    CS_NOTICE_PATH="$DEB_NOTICE"
    ok "corresponding source notice: ${DEB_NOTICE}"

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
    # `od` prints two hex digits per byte and `tr -d ' \n'` leaves them in
    # order, so byte N occupies characters 2N+1 and 2N+2. The bytes at OFFSET
    # 8 are therefore characters 17-22. The previous `cut -c9-14` read bytes
    # 4-6 - EI_CLASS, EI_DATA and EI_VERSION - and printed them under a label
    # that said 8-10, which is how `020101` came to be recorded as the
    # type-2 magic.
    RUNTIME_MAGIC=$(head -c 11 "$ARTIFACT" | od -An -tx1 | tr -d ' \n' | cut -c17-22)
    echo "    bytes 8-10  : ${RUNTIME_MAGIC} (expect 414902 for an AppImage type-2 runtime)"

    # ── AGPL-3.0 §4: convey a copy of the Licence with every copy of the
    # Program. ─────────────────────────────────────
    # [FACT] The AppImage bundler DOES read a licence-carrying key:
    # `bundle.linux.appimage.files`. `tauri-utils`' `AppImageConfig` declares
    # `files: HashMap<PathBuf, PathBuf>` - "The files to include in the
    # Appimage Binary" - and
    # `crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs` calls
    # `fs_utils::copy_custom_files(&settings.appimage().files, &data_dir)`
    # before it copies `data_dir/usr/` into the AppDir. The map key is the
    # DESTINATION relative to the data dir, so it has to start with `usr/` to
    # end up inside the image.
    #
    # [FACT] The earlier claim in this file - that neither `bundle.licenseFile`
    # nor `bundle.resources` reaches this bundler, so no configuration could
    # put a copy of the Licence there - was true of those two keys and was
    # read as being true of the mechanism in general. It was not:
    # `appimage.files` was overlooked, and the conclusion "an open AGPL-3.0
    # section 4 gap" was wrong. The gap is closed by configuration, not by
    # repacking the image, so this branch now ASSERTS instead of printing.
    #
    # The runtime can unpack its own squashfs, so the contents can be listed
    # without FUSE and without root. `ARTIFACT` is relative to this script's
    # working directory, so it has to be made absolute before the extraction
    # runs somewhere else.
    APPIMAGE_PATH=$(cd "$(dirname "$ARTIFACT")" && pwd)/$(basename "$ARTIFACT")
    APPIMAGE_EXTRACT_DIR=$(mktemp -d)
    # The trap, not a trailing `rm -rf`: the failure path below exits from
    # inside this branch, and a directory that only a later line would have
    # removed would be left behind on every failure.
    trap 'rm -rf "$APPIMAGE_EXTRACT_DIR"' EXIT
    if ( cd "$APPIMAGE_EXTRACT_DIR" && "$APPIMAGE_PATH" --appimage-extract >/dev/null 2>&1 ); then
        APPIMAGE_ROOT="$APPIMAGE_EXTRACT_DIR/squashfs-root"
        APPIMAGE_FILE_COUNT=$(find "$APPIMAGE_ROOT" -type f 2>/dev/null | wc -l | tr -d ' ')
        # The decision is about CONTENT, not about the name of a file. The
        # AppImage this repository builds carries
        # `usr/share/doc/libwayland-egl1/copyright`, which is the copyright
        # file of a BUNDLED LIBRARY, and the name-only check that used to
        # stand here reported it as the product's licence:
        #
        #     ok  licence: usr/share/doc/libwayland-egl1/copyright
        #
        # The image carried no copy of the Licence at all. See
        # `find_product_licence` above.
        APPIMAGE_LICENCE=$(find_product_licence "$APPIMAGE_ROOT" || true)
        if [ -n "$APPIMAGE_LICENCE" ]; then
            ok "licence: ${APPIMAGE_LICENCE}"
            # The candidates that are NOT the product's licence are reported
            # as well, because their presence is exactly what made the
            # name-only check report the wrong file.
            report_rejected_licences "$APPIMAGE_ROOT"
        else
            # Two absences, and they mean different things. A licence-shaped
            # file that does not carry the Licence is a dependency's file
            # being mistaken for one; no licence-shaped file at all is an
            # image that never had one. The count is what tells them apart.
            APPIMAGE_CANDIDATE_COUNT=$(list_licence_candidates "$APPIMAGE_ROOT" | grep -c . || true)
            if [ "$APPIMAGE_CANDIDATE_COUNT" -gt 0 ]; then
                echo "    ${APPIMAGE_CANDIDATE_COUNT} licence-shaped file(s) were found, and none of them carries" >&2
                echo "    the AGPL-3.0 text:" >&2
                report_rejected_licences "$APPIMAGE_ROOT" >&2
            else
                echo "    the image carries no licence-shaped file at all (${APPIMAGE_FILE_COUNT} file(s) in the image)." >&2
            fi
            die "${ARTIFACT_NAME} ships no copy of the Licence.
AGPL-3.0 section 4 requires a copy of the Licence with every copy of the
Program. A file whose NAME looks like a licence does not count: the image may
carry a dependency's copyright file and no copy of the Licence at all. Check
bundle.linux.appimage.files in ${CONFIG} still maps a usr/share/... destination
at a source that exists. It is destination -> source, not source -> destination,
and the destination has to start with usr/ - only data_dir/usr/ is copied into
the AppDir (crates/tauri-bundler/src/bundle/linux/appimage/linuxdeploy.rs)."
        fi

        # ── AGPL-3.0 §6(d): clear directions next to the object code ─────────
        # [FACT] The image was EXTRACTED above, so this decision is made on the
        # bytes that were really packaged, not on the configuration that was
        # meant to produce them.
        APPIMAGE_NOTICE=$(find_corresponding_source_notice "$APPIMAGE_ROOT" "$CS_SOURCE_PLACE" || true)
        if [ -z "$APPIMAGE_NOTICE" ]; then
            die "${ARTIFACT_NAME} ships no Corresponding Source notice next to the object code.
AGPL-3.0 section 6(d) requires the distributor of object code to maintain clear
directions next to the object code saying where to find the Corresponding Source.
A file whose NAME looks like a notice does not count: the image may carry a
dependency's file and no directions at all. Check that bundle.linux.appimage.files
in ${CONFIG} still maps a usr/share/... destination at a source that exists, and
that the source names the licence and the place the source is offered at
(${CS_SOURCE_PLACE})."
        fi
        CS_NOTICE_PATH="$APPIMAGE_NOTICE"
        ok "corresponding source notice: ${APPIMAGE_NOTICE}"
    else
        die "could not extract ${ARTIFACT_NAME} to check its contents (the runtime refused --appimage-extract).
Failing closed: an image we cannot open proves nothing."
    fi
    rm -rf "$APPIMAGE_EXTRACT_DIR"
    trap - EXIT
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

    # `bundle.licenseFile` puts the AGPL into the image as a Software License
    # Agreement: create-dmg embeds it as UDIF EULA resources (TEXT/styl/STR# in
    # the image's resource fork). A disk image carrying an SLA cannot be
    # mounted until the agreement is accepted, and a non-interactive runner has
    # nobody to accept it, so hdiutil cancels the attach with
    # "hdiutil: attach canceled".
    #
    # Measured rather than guessed: the attach succeeded on every run before
    # `bundle.licenseFile` was set, and was cancelled 0.05 s after starting on
    # every run after it - 0.05 s, not a timeout, which is what a refusal to
    # prompt looks like.
    #
    # Answering the agreement is not a workaround. It accepts the licence this
    # repository ships, on our own artifact, and every check below still reads
    # the mounted image.
    #
    # `{ yes || true; }` rather than a bare `yes |`: `yes` dies of SIGPIPE when
    # hdiutil stops reading it, and under `set -o pipefail` that 141 would
    # become the pipeline's status and be mistaken for the attach having
    # failed. The `head` bounds the input so that a hdiutil which never reads
    # stdin at all cannot leave `yes` blocked on a full pipe forever.
    { yes || true; } | head -n 64 | hdiutil attach -readonly -nobrowse -mountpoint "$MOUNT_DIR" "$ARTIFACT" >/dev/null \
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

    # ── AGPL-3.0 §4: what this artifact actually carries ──────────────────
    # [FACT] The dmg bundler DOES read `bundle.licenseFile`:
    # `crates/tauri-bundler/src/bundle/macos/dmg/mod.rs` hands it to create-dmg
    # as `--eula`, so the licence text is presented when the image is mounted.
    # `bundle.resources` is not read by the macOS bundler at all.
    #
    # Where create-dmg leaves that EULA is not something this script can assert
    # without guessing, so it reports what the mounted image actually holds.
    # Until a run prints it, whether a copy is left INSIDE the image is
    # [UNVERIFIED-FACT].
    #
    # The decision is about CONTENT, for the same reason as the deb branch: a
    # mounted image can carry a dependency's copyright file, and a name-only
    # check would call that the product's licence. See `find_product_licence`
    # above.
    DMG_LICENCE=$(find_product_licence "$MOUNT_DIR" || true)
    if [ -n "$DMG_LICENCE" ]; then
        ok "licence: ${DMG_LICENCE}"
        # The candidates that are NOT the product's licence are reported as
        # well, so a reader can see what was there and why it did not count.
        report_rejected_licences "$MOUNT_DIR"
    else
        # Two absences, and they mean different things: a licence-shaped file
        # that does not carry the Licence, or no licence-shaped file at all.
        DMG_CANDIDATE_COUNT=$(list_licence_candidates "$MOUNT_DIR" | grep -c . || true)
        if [ "$DMG_CANDIDATE_COUNT" -gt 0 ]; then
            echo "    NO COPY OF THE LICENCE inside the mounted image."
            echo "    ${DMG_CANDIDATE_COUNT} licence-shaped file(s) were found, and none of them carries"
            echo "    the AGPL-3.0 text:"
            report_rejected_licences "$MOUNT_DIR"
        else
            echo "    NO LICENCE-SHAPED FILE inside the mounted image."
        fi
        echo "    [FACT] the dmg bundler passes bundle.licenseFile to create-dmg as --eula,"
        echo "    so the licence is presented when the image is mounted. Whether create-dmg"
        echo "    also leaves a copy inside the image is what this line measures."
    fi

    # ── AGPL-3.0 §6(d): what this artifact actually carries ──────────────────
    # [FACT] The licence reaches a .dmg as a UDIF EULA RESOURCE - the text
    # create-dmg embeds in the image's resource fork and presents when the
    # image is mounted - and not as a file. There is nothing inside the image
    # for a notice to be attached to, and no configuration key that puts one
    # there: the dmg bundler reads neither bundle.resources nor a files map.
    #
    # So NOTHING is asserted here. An assertion that could never pass is a
    # gate that gets switched off, and this one would be off before it was
    # ever red. What is recorded instead is the fact, and the fact is what
    # `docs/plan/v0-tasks.md` carries.
    DMG_NOTICE=$(find_corresponding_source_notice "$MOUNT_DIR" "$CS_SOURCE_PLACE" || true)
    if [ -n "$DMG_NOTICE" ]; then
        ok "corresponding source notice: ${DMG_NOTICE}"
    else
        echo "    NO CORRESPONDING SOURCE NOTICE inside the mounted image." >&2
        echo "    [FACT] the licence reaches a .dmg as a UDIF EULA resource (create-dmg --eula)," >&2
        echo "    not as a file, so there is nothing in the image to carry a notice." >&2
    fi
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
  --arg cn "$CS_NOTICE_PATH" \
  '{
     generatedAtUtc: $ts,
     productName:    $p,
     version:        $v,
     target:         $t,
     runner:         $r,
     signing:        $sg,
     notarized:      false,
     correspondingSourceNotice: $cn,
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
R_NOTICE=$(jq -r '.correspondingSourceNotice // empty' "$MANIFEST")

[ "$R_VERSION" = "$VERSION" ] || die "${MANIFEST} records version '${R_VERSION}', expected '${VERSION}'"
[ "$R_FILE"    = "$ARTIFACT_NAME" ] || die "${MANIFEST} records '${R_FILE}', expected '${ARTIFACT_NAME}'"
[ "$R_BYTES"   = "$BYTES" ]         || die "${MANIFEST} records ${R_BYTES} B, the file on disk is ${BYTES} B"
[ "$R_SHA"     = "$SHA" ]           || die "${MANIFEST} records sha256 ${R_SHA}, the file on disk hashes to ${SHA}"
# The notice the artifact carries is part of what the artifact IS, so the
# manifest records it and the record is re-read rather than trusted. An empty
# value is a real answer for the dmg, which has no file slot at all.
[ "$R_NOTICE" = "$CS_NOTICE_PATH" ] \
  || die "${MANIFEST} records corresponding source notice '${R_NOTICE}', the artifact carries '${CS_NOTICE_PATH}'"
ok "manifest ${MANIFEST}: 1 artifact, size and sha256 match the file on disk"

step "PASS: ${ARTIFACT_NAME} is a real ${TARGET} bundle (unsigned; not notarized)"
