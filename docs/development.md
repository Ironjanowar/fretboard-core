# Development

## Requirements

Everything needed for the Rust workspace is downloaded by `rustup` and pinned in
`rust-toolchain.toml`; the wrapper for future Android work is Gradle 9.8 with
JDK 21, and the Android SDK/NDK live outside the repository. Exact versions:
`docs/toolchains.md`.

Elixir/OTP is only needed to regenerate oracle fixtures; it is not needed to
build or test this repository, which reads the frozen fixtures committed under
`fixtures/`.

## Commands

```bash
# Workspace checks (what CI runs)
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked

# Python checkers and their unit tests
python3 -m unittest discover -s scripts/tests

# Dependency graph sanity: the domain crate may only pull serde and serde_json
cargo tree -p fretboard-core
```

Run cargo with `--locked`: the committed `Cargo.lock` is the resolution of
record. If a change really needs a new dependency or version, regenerate the lock
deliberately in its own commit.

## Static analysis policy

The lint policy lives in the root `Cargo.toml` under `[workspace.lints.*]`, and
every crate opts in with `[lints] workspace = true`, so a new crate inherits the
rules and an existing one cannot quietly relax them. On top of the default
groups the policy turns on:

- **rustc**: `unsafe_code = "forbid"`, `missing_docs`,
  `missing_debug_implementations`, `unreachable_pub`, `unused_qualifications`,
  `elided_lifetimes_in_paths`, `trivial_casts`, `trivial_numeric_casts`,
  `unused_lifetimes`, `single_use_lifetimes`, `variant_size_differences`,
  `non_ascii_idents`, `let_underscore_drop`, `macro_use_extern_crate`,
  `unused_macro_rules`, `unexpected_cfgs`, `rust_2018_idioms`.
- **Clippy**: the `all`, `cargo`, `pedantic` and `nursery` groups, plus the
  restriction lints this codebase wants: `unwrap_used`, `expect_used`, `panic`,
  `indexing_slicing`, `arithmetic_side_effects`, `missing_errors_doc`,
  `missing_panics_doc`, `dbg_macro`, `todo`, `unimplemented`, `unreachable`,
  `print_stdout`, `print_stderr`, `exit`, `mem_forget`,
  `undocumented_unsafe_blocks`, `same_name_method`.

Deliberate exceptions, each with its reason written next to it in the manifest:
`module_name_repetitions`, `must_use_candidate`, `doc_markdown`,
`missing_docs_in_private_items`, `multiple_crate_versions` (upstream `uniffi`
pulls two `syn` versions) and `redundant_pub_crate` (it contradicts rustc's
`unreachable_pub`, and crate-internal items in private modules stay
`pub(crate)`).

Tests relax only the panicking-by-idiom restriction lints (`expect_used`,
`unwrap_used`, `panic`, `indexing_slicing`) plus `unreachable_pub` for test-only
helper modules, in a header comment inside the test files themselves; every other
lint still applies to test code.

`overflow-checks` is on in the release profile as well as dev: silent
wrap-around in pitch arithmetic would be a musical defect, not a performance win.

A warning is a failure: CI runs the commands above with `-D warnings`, and a
local run that produces warnings is not "green".

## Repository layout

```text
Cargo.toml                 workspace (resolver 3, three members)
rust-toolchain.toml        pinned Rust channel, components and targets
crates/domain              fretboard-core: pure music domain
crates/mobile-ffi          fretboard-mobile-ffi: UniFFI adapter (cdylib + rlib)
crates/bindgen             fretboard-bindgen: uniffi-bindgen binary
tools/oracle               Elixir exporter that produces frozen fixtures
fixtures/oracle            frozen Elixir baseline exports (reviewed, immutable)
fixtures/contract          hand-authored contract examples (schema, API)
scripts                    build/package/check scripts and their tests
android                    Gradle project that packages the engine AAR (P1)
```

## Roles per task

Test writer, implementer, reviewer, run in separate contexts; the sequence and
the rules are in `AGENTS.md`. A test that was never observed failing is not a
test-first change, and a fixture regenerated from the implementation under test
is not an oracle.

## Oracle fixtures

`fixtures/oracle/` is produced by the Elixir exporter against the pinned web
commit and committed with its manifest (source commit, file hashes, tool
versions, record counts, SHA-256 per file). Ordinary development and CI read
those files; nobody needs Elixir or the web repository to run the test suite.
Regeneration is an explicit, source-pinned review task.
