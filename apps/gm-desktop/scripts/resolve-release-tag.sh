#!/usr/bin/env bash
#
# Resolve the ONE tag a release run builds, from the event context alone.
#
# WHY THIS IS A SCRIPT AND NOT A `run:` BLOCK
# ===========================================
# A `run:` block is text that only a workflow ever reads. It is never executed
# by anything else, so nothing can ever prove that it does what its comment
# says it does — the block can only be checked by reading it, and reading it is
# exactly what the `release.yml` history shows going wrong.
#
# The resolution is therefore a file, and the file is the single definition
# that both the workflow and the test use:
#
#   * `release.yml`'s `verify-tag` job calls it, and its output is what every
#     later job checks out.
#   * `apps/gm-desktop/tests/unit/release-tag-resolution.test.ts` executes it
#     with the four event shapes that matter and asserts what comes out.
#
# A copy of this logic inside the workflow would be a second definition that
# the test does not execute, which is the shape of a gate that reads green.
#
# WHAT IT READS
# =============
#   EVENT_NAME  the runner's event name: `push` or `workflow_dispatch`
#   INPUT_TAG   the operator-supplied tag, on `workflow_dispatch` only
#   GITHUB_REF  the runner's ref, on `push` only
#
# All three arrive through the environment, never through the command line:
# `INPUT_TAG` is operator-supplied text, and interpolated into a command it
# would be a word to be split on rather than a tag to be checked.
#
# WHAT IT WRITES
# ==============
#   `tag=<the tag>` into `$GITHUB_OUTPUT`, which is how the workflow reads it
#   back as `steps.tag.outputs.tag` and how every later job names its checkout.
#   A line into `$GITHUB_STEP_SUMMARY`, so the run's summary states which tag
#   was built.
#   The tag on stdout, so the step's log says it too.
#
# WHAT IT REFUSES
# ===============
# Every failure below exits non-zero AND prints a `::error::` line. A step that
# exits non-zero having said nothing is the shape of the 2026-10-06 failure of
# this workflow — `ref: unbound variable`, and a log that never said what was
# wrong — so a failure that cannot explain itself is not a failure mode this
# script is allowed to have.
#
# The order of the checks is load-bearing: the tag is validated BEFORE it is
# ever used as a checkout ref, because a checkout pinned to a value that has
# not been validated is how a build ends up compiling one tree and calling it
# another.

set -euo pipefail

event_name="${EVENT_NAME:-}"
input_tag="${INPUT_TAG:-}"
github_ref="${GITHUB_REF:-}"

fail() {
  # Every failure names the value it refused and why. The message goes to
  # stderr so it cannot be mistaken for the tag the script resolves.
  echo "::error::$1" >&2
  exit 1
}

case "$event_name" in
  workflow_dispatch)
    # A manual run names the tag in the `tag` input. There is no tag in the
    # ref of a dispatched run — the ref is the branch the operator picked —
    # so the input is the only source of the answer, and an empty input has
    # no answer at all. Falling back to the branch here would build the
    # branch and label it with the branch's name, which is the defect this
    # whole script exists to remove.
    if [ -z "$input_tag" ]; then
      fail "the manual run supplied no tag. A dispatch must name the tag to build; there is nothing else it could mean."
    fi
    tag="$input_tag"
    ;;
  push)
    # A tag push carries the tag in the ref. A push of anything else is not a
    # release, and treating the ref's short name as a tag would build a
    # branch and call it a version.
    case "$github_ref" in
      refs/tags/*) tag="${github_ref#refs/tags/}" ;;
      *) fail "the push ref '$github_ref' is not a tag ref. Only a 'v*' tag push is a release." ;;
    esac
    ;;
  *)
    fail "unknown event '$event_name'. This script resolves a tag for 'push' and 'workflow_dispatch' only."
    ;;
esac

# The prefix guard, and it is the same guard the workflow used to carry
# inline: a tag that does not start with `v` is not a version, and a release
# named after one would advertise a version nothing was built from.
case "$tag" in
  v*) ;;
  *) fail "'$tag' is not a v-prefixed tag" ;;
esac

# The tag's EXISTENCE is not proved here. It is proved by the checkout that
# follows, which fails with the runner's own message when the tag is not in
# the repository — a more specific answer than this script could give, and one
# that cannot be satisfied by a tag name alone.

if [ -n "${GITHUB_OUTPUT:-}" ]; then
  echo "tag=$tag" >> "$GITHUB_OUTPUT"
fi
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  echo "building $tag" >> "$GITHUB_STEP_SUMMARY"
fi

echo "$tag"
