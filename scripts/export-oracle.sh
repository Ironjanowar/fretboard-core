#!/usr/bin/env bash
#
# C01 source-oracle procedure: export the frozen fixtures from the real,
# pinned Elixir implementation without touching the web repository.
#
#   ./scripts/export-oracle.sh --source /workspace/repos/fretboard \
#     --commit 2daa8c665efa268942dda352691f39d78db42512 --out dist/oracle-run-a
#
# Steps: check the commit against the approved pin, verify the git object,
# extract `git archive` into a temporary directory outside any repository, fetch
# and compile dependencies there, run the domain export, run the page-event
# exporter, then write manifest.json.
#
# The source repository is only ever read. Existing uncommitted changes in the
# guarded paths are recorded, never reverted; a change introduced during the run
# aborts the script.

set -euo pipefail

APPROVED_PIN="2daa8c665efa268942dda352691f39d78db42512"
PINNED_ELIXIR="1.19.5"
PINNED_OTP="27"

CORE="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXPORT_SCRIPT="$CORE/tools/oracle/export.exs"
PAGE_TEST="$CORE/tools/oracle/page_events_test.exs"
GUARDED_PATHS=(lib test mix.exs mix.lock assets)

SOURCE=""
COMMIT=""
OUT=""
ROUNDTRIP=""

die() {
  printf 'ERROR: %s\n' "$*" >&2
  exit 1
}

note() {
  printf '%s\n' "$*"
}

usage() {
  cat <<'USAGE'
usage: export-oracle.sh --source <repo> --commit <sha> --out <dir>

  --source <repo>   web repository holding the pinned commit (read-only)
  --commit <sha>    pinned commit; must equal the approved baseline pin
  --out <dir>       run directory, relative to this repository or absolute
  --roundtrip-inputs <file>
                    not implemented until task C19 (always fails)
USAGE
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --source|--commit|--out|--roundtrip-inputs)
      [ "$#" -ge 2 ] || die "$1 requires a value"
      case "$1" in
        --source) SOURCE="$2" ;;
        --commit) COMMIT="$2" ;;
        --out) OUT="$2" ;;
        --roundtrip-inputs) ROUNDTRIP="$2" ;;
      esac
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

[ -n "$SOURCE" ] || die "--source is required"
[ -n "$COMMIT" ] || die "--commit is required"
[ -n "$OUT" ] || die "--out is required"

if [ -n "$ROUNDTRIP" ]; then
  die "--roundtrip-inputs is not implemented until task C19 (received $ROUNDTRIP)"
fi

[ "$COMMIT" = "$APPROVED_PIN" ] ||
  die "commit $COMMIT is not the approved pin $APPROVED_PIN; a later commit or a dirty worktree is not equivalent provenance"

command -v git >/dev/null 2>&1 || die "git not found"
command -v tar >/dev/null 2>&1 || die "tar not found"
[ -f "$EXPORT_SCRIPT" ] || die "missing $EXPORT_SCRIPT"

git -C "$SOURCE" rev-parse --git-dir >/dev/null 2>&1 || die "$SOURCE is not a git repository"
git -C "$SOURCE" cat-file -e "$COMMIT^{commit}" 2>/dev/null ||
  die "git object $COMMIT^{commit} not found in $SOURCE"
RESOLVED="$(git -C "$SOURCE" rev-parse "$COMMIT^{commit}")"
[ "$RESOLVED" = "$COMMIT" ] || die "resolved commit $RESOLVED does not match the requested $COMMIT"

case "$OUT" in
  /*) OUT_DIR="$OUT" ;;
  *) OUT_DIR="$CORE/$OUT" ;;
esac
mkdir -p "$OUT_DIR"
OUT_DIR="$(cd "$OUT_DIR" && pwd)"

# Record -- never revert -- pre-existing changes in the guarded paths, then
# fingerprint that state so a change introduced during the run is detectable.
guard_fingerprint() {
  git -C "$SOURCE" diff -- "${GUARDED_PATHS[@]}" | sha256sum | cut -d' ' -f1
}

GUARD_BEFORE="$(guard_fingerprint)"

if ! git -C "$SOURCE" diff --quiet -- "${GUARDED_PATHS[@]}"; then
  note "WARNING: $SOURCE already has uncommitted changes in: ${GUARDED_PATHS[*]}"
  note "         they are recorded below and were NOT reverted"
  git -C "$SOURCE" diff --stat -- "${GUARDED_PATHS[@]}" >&2 || true
fi

SCRATCH_ROOT="${ORACLE_SCRATCH_ROOT:-/workspace}"
[ -d "$SCRATCH_ROOT" ] || die "scratch root $SCRATCH_ROOT does not exist"
SCRATCH="$(mktemp -d "$SCRATCH_ROOT/fretboard-oracle.XXXXXX")" ||
  die "could not create a temporary directory under $SCRATCH_ROOT"

cleanup() {
  rm -rf "$SCRATCH"
}
trap cleanup EXIT
trap 'cleanup; exit 130' INT
trap 'cleanup; exit 143' TERM

note "scratch checkout: $SCRATCH"
git -C "$SOURCE" archive "$COMMIT" | tar -x -C "$SCRATCH"

cd "$SCRATCH"

EXEC_PREFIX=()
if command -v mise >/dev/null 2>&1; then
  EXEC_PREFIX=(mise exec --)
fi

run() {
  if [ "${#EXEC_PREFIX[@]}" -gt 0 ]; then
    mise exec -- "$@"
  else
    "$@"
  fi
}

run elixir --version >/dev/null 2>&1 ||
  die "Elixir/OTP unavailable through ${EXEC_PREFIX[*]:-PATH}. This is blocked setup, not a red test."

ACTUAL_ELIXIR="$(run elixir -e 'IO.write(System.version())' 2>/dev/null || true)"
ACTUAL_OTP="$(run elixir -e 'IO.write(:erlang.system_info(:otp_release))' 2>/dev/null || true)"

if [ "$ACTUAL_ELIXIR" != "$PINNED_ELIXIR" ] || [ "$ACTUAL_OTP" != "$PINNED_OTP" ]; then
  die "pinned runtime required: Elixir $PINNED_ELIXIR / OTP $PINNED_OTP, found Elixir ${ACTUAL_ELIXIR:-?} / OTP ${ACTUAL_OTP:-?}"
fi
note "runtime: Elixir $ACTUAL_ELIXIR / OTP $ACTUAL_OTP"

export MIX_ENV=test

if run mix help deps.get 2>/dev/null | grep -q -- '--check-locked'; then
  run mix deps.get --check-locked
else
  LOCK_BEFORE="$(sha256sum mix.lock)"
  run mix deps.get
  LOCK_AFTER="$(sha256sum mix.lock)"
  [ "$LOCK_BEFORE" = "$LOCK_AFTER" ] ||
    die "mix deps.get changed mix.lock (--check-locked unavailable); an unreviewed lock update is not acceptable"
fi

run mix compile --warnings-as-errors

# Domain export.
ORACLE_SOURCE_SHA="$COMMIT" ORACLE_OUT="$OUT_DIR" \
  run mix run --no-start "$EXPORT_SCRIPT"

# Page-level export (owned by the page-events task); writes the four page
# fixtures into ORACLE_OUT from the environment.
if [ ! -f "$PAGE_TEST" ]; then
  die "$PAGE_TEST is missing (owned by the page-level exporter task); the four page fixtures cannot be produced yet"
fi
ORACLE_OUT="$OUT_DIR" run mix test "$PAGE_TEST" --seed 0

# Frozen manifest over all fourteen files.
ORACLE_SOURCE_SHA="$COMMIT" ORACLE_OUT="$OUT_DIR" ORACLE_MODE=manifest \
  run mix run --no-start "$EXPORT_SCRIPT"

GUARD_AFTER="$(guard_fingerprint)"
if [ "$GUARD_AFTER" != "$GUARD_BEFORE" ]; then
  git -C "$SOURCE" diff --stat -- "${GUARDED_PATHS[@]}" >&2 || true
  die "the source repository changed in ${GUARDED_PATHS[*]} during this run"
fi

note "export-oracle.sh completed: $OUT_DIR"
