# Fretboard Core

Portable music engine behind the native Fretboard applications.

Three Cargo members, one dependency direction:

| Crate | Package | Library | Responsibility |
|---|---|---|---|
| `crates/domain` | `fretboard-core` | `fretboard_core` | Musical rules, catalogs, canonical state, URL and snapshot codecs. Pure: no UniFFI, Android, JNI, network, filesystem, clock or rendering dependency. |
| `crates/mobile-ffi` | `fretboard-mobile-ffi` | `fretboard_mobile_ffi` | Typed foreign boundary (`cdylib` + `rlib`). DTOs, conversions and error mapping only. |
| `crates/bindgen` | `fretboard-bindgen` | binary `uniffi-bindgen` | Binding generation with the pinned UniFFI version. |

The Elixir web application at `Ironjanowar/fretboard` stays unchanged and acts as
the migration oracle: baseline behavior is frozen from real exports of commit
`2daa8c665efa268942dda352691f39d78db42512`, never re-invented in Rust.

Documentation:

- `AGENTS.md` — development rules, boundaries and verification commands.
- `docs/toolchains.md` — verified toolchain ledger.
- `docs/development.md` — local setup and the commands CI runs.
- `docs/contracts.md` — frozen contracts, schema versions and capability flags.
- `docs/decisions.md` — decision register (open and approved).
- `docs/validation.md` — phase gates and acceptance evidence.

Status: P0 bootstrap. The workspace skeleton exists; musical behavior arrives
task by task with tests first. No APK and no Android artifact exists yet.
