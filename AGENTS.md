# AGENTS.md — Fretboard Core

## What this repository is

The portable Rust music engine and its UniFFI adapter. It is consumed by the
native applications (`Ironjanowar/fretboard-android`); it is not a service and
has no network, storage or UI concern.

## Boundaries

- `crates/domain` (`fretboard-core`, library `fretboard_core`) owns music:
  catalogs, pitch and interval rules, canonical state, state transitions, URL
  and snapshot codecs, evaluation and derivation.
- `crates/mobile-ffi` (`fretboard-mobile-ffi`) owns only the foreign boundary:
  DTOs, conversions, error mapping and the exported API. It must not contain
  musical rules, and it never re-implements domain logic.
- `crates/bindgen` owns binding generation for the pinned UniFFI version.
- The domain crate depends only on `serde` and `serde_json` (`CORE-D04` in
  `docs/decisions.md`): no UniFFI, no JNI, no Android, no Phoenix, no filesystem,
  no clock, no randomness, no network. Any further dependency requires an
  approved decision recorded in `docs/decisions.md`.
- Generated bindings and native libraries are build outputs. Never hand-edit or
  commit generated Kotlin, generated Rust scaffolding, or binary artifact
  content; commit the generation command instead.

## Language

Everything in this repository is English: code, comments, documentation, test
names, error messages and user-facing strings surfaced through the adapter.
Preserve stable identifiers, wire values and URL compatibility when wording
changes (for example `ukelele` stays spelled that way).

## Test-driven development

Tests come first, always, and every task runs as independent roles:

1. **Test writer** — writes the tests for one behavior slice, runs them, and
   reports the actual failure that proves the behavior is missing.
2. **Implementer** — writes the minimum production code that makes those tests
   pass, then runs the focused tests and the regressions.
3. **Reviewer** — independently checks the diff against the contract, runs the
   tests, and reports findings before the commit.

Rules:

- Missing tooling, an absent dependency or a corrupt fixture is *blocked setup*,
  never evidence of a red test.
- Never weaken, delete or rewrite an assertion to make an implementation pass.
- Never generate expected values from the implementation under test. Baseline
  expectations come from frozen Elixir oracle fixtures.
- One behavior per task; keep functions short and modules small.

## Commands

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
python3 -m unittest discover -s scripts/tests
```

Everything needed for the Rust workspace is pinned in `rust-toolchain.toml` and
`Cargo.lock`; run with `--locked` so a build never resolves new versions.

## Commits and delivery

- Commit only the paths the current task owns. Never `git add .`.
- One logical change per commit; imperative subject line.
- Work happens on a branch and is delivered as a pull request: branch, push,
  `gh pr create`, then report the link. Merging requires the user's approval.
- `Cargo.lock` is committed and updated deliberately, never as a side effect.
- Never commit secrets, keystores, downloaded AAR/APK files or fixture dumps.

## Verification honesty

Do not report a planned result as an executed one. Reports contain the actual
command and its actual output; unrun checks, unavailable tools and blocked
gates are stated as such.
