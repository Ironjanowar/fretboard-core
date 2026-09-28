# Implementation Ledger

Coordinator-owned. Plans remain authoritative; this file records what actually
happened: completed tasks, commits, the next dependency, approved deviations and
the current core/Android pairing. It is not a substitute for the plan documents.

## Core repository

| Task | Scope | Commit | Evidence |
|---|---|---|---|
| C00 | Rust workspace skeleton, toolchain pins, project documentation | `1048992`, `47be466`, `ce33619` | `cargo check/fmt/clippy/test --locked` green; `uniffi-bindgen` runs; toolchain ledger in `docs/toolchains.md` |
| C01 | Elixir oracle exporter, checker and frozen fixtures | `14e0fe0` | Two full exports byte-identical; `check_export.py` exit 0; 20 checker tests; 28 fixture files, 22,099 records; baseline web repo untouched |
| C02 | Typed page-state contract and frozen schema examples | `e7792bf`, `9db4419`, `41fd179` | 50 contract tests green; strict, validating deserialisation; `ALL` identifier lists oracle-pinned; two independent reviews, all findings closed |
| Lint policy | Strict workspace lints (rustc + clippy `all`/`cargo`/`pedantic`/`nursery` + restriction set) | `12507f2`, `dfa76f3` | 239 warnings → 0 with `-D warnings`; exceptions justified inline |
| C03 | Note, interval and major-chord primitives | in progress | — |

Merged to `main`: `48424b8` (the approved P0 stack).

## Phase gates

| Gate | Status | Evidence |
|---|---|---|
| P0 | **Approved by the user 2026-09-28** | Toolchain ledger, frozen fixtures with manifest and hashes, typed contract with 50 tests, provenance recorded, no APK claimed |
| P1 | In progress | First installable APK is the gate: real Compose → generated UniFFI → Rust calculation, installed by hand on the phone, offline |

## Open decisions carried into P1

- `DEC-08` signing key custody: deferred by the user to just before the first APK.
- `DEC-06` import/URL resource limits: the coordinator proposes concrete caps in C19 after reviewing real generated URLs.
- `Contract.D03` recognition tie order: measured and blocked; decided in P3 (`C12`), with the reproduction commands in `docs/decisions.md`.
- `CORE-D04` domain dependencies limited to `serde`/`serde_json`: coordinator decision, reversible.

## Android repository

Not started. `Ironjanowar/fretboard-android` exists and is empty; its P0/P1 file
manifest is `05-android-phases.md` section 2, and it depends on a published core
release from C05 (`core-release.lock.json` is filled only from a real release).

## Current pairing

| Core | Android |
|---|---|
| `main` @ `48424b8` (P0 approved) | none |
