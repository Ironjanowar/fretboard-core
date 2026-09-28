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

# Dependency graph sanity: the domain crate must have no dependencies
cargo tree -p fretboard-core
```

Run cargo with `--locked`: the committed `Cargo.lock` is the resolution of
record. If a change really needs a new dependency or version, regenerate the lock
deliberately in its own commit.

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
