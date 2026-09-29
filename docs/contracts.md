# Frozen Contracts

This file records what downstream clients may depend on, and what is still open.
The full proposed typed surface, the musical rules and the wire semantics live in
the approved plan (`Ironjanowar/fretboard`, `.hermes/plans/native-core-android/`,
documents `02-core-contract.md` and `03-android-design.md`); this repository
implements them, and this file names the versioned boundary.

## Authority

1. The frozen Elixir baseline at commit `2daa8c665efa268942dda352691f39d78db42512`
   and its exported fixtures define observed behavior.
2. `docs/decisions.md` records approved deviations from that behavior. There are
   no deviations yet.
3. Nothing may be "fixed" while porting. A suspected baseline defect becomes a
   decision entry with an approval reference, a deviation fixture and release
   notes.

## Versions

| Contract | Version | Frozen in | Status |
|---|---|---|---|
| Oracle fixture schema | `1` | task `C01` (`fixtures/oracle/manifest.json`, `fixture_schema_version`) | open until fixtures exist |
| Snapshot schema (`{"schema_version": 1, "page": …}`) | `1` | task `C02`/`C20` (`fixtures/contract/snapshot-v1.json`) | open |
| Adapter API (revision 1: `fixtures/contract/api-v1.json`; revision 2: `api-v2.json`) | `2` | revision 1 frozen by `C02`/`C04`; revision 2 (`CORE-D08`) adds the P2 endpoints and the first declared capability set | open |
| Generated binding package | `dev.ironjanowar.fretboard.core` | task `C04` (`crates/mobile-ffi/uniffi.toml`) | open |
| Engine artifact | `fretboard-engine-<version>.aar` + SHA-256 | task `C05` | open |

An unpublished contract has no consumers; a published one is never silently
changed. Breaking changes bump the version, and the Android lock moves in its own
reviewed change.

## Interface decisions frozen in C02

The plan names the types but not every conversion; these are the coordinator's
binding resolutions, implemented with the C02 tests:

- Numeric newtypes are constructed with `TryFrom<u8>` (`PitchClass` 0..=11,
  `OpenPitch` 0..=127, `SoundingPitch` wide enough for 127+24 = 151,
  `StringIndex`, `Fret` 0..=24); id types use `FromStr` + `Display` with the
  stable oracle strings.
- `CoreError`'s variant name is the stable code, with an optional field name, and
  `CoreError::code()` returns that same string for later FFI use.
- Structural violations (a fretted state carrying `Piano`, wrong pitch count,
  duplicate string indices, a highlight absent from the chords) are
  `InvalidState`; numeric range violations (piano key outside 48..=83, fret above
  24, open pitch above 127) are `OutOfRange`; unknown id strings are
  `UnknownIdentifier`.
- State types derive at least `Debug + Clone + PartialEq`, and the state, chord,
  tuning, position, instrument, tab and newtype types are serde-serialisable.
- Snapshot shape: `{"schema_version": 1, "page": {…}}`; the instrument is a
  tagged object with `kind` (`"fretted"`/`"piano"`) and `id` for both kinds,
  `tuning {pitches, reference}` only for fretted, `selection` for both. The
  adapter API file carries `api_version`, `snapshot_schema_version`,
  `binding_package` and a `capabilities` object mapping capability id to boolean,
  empty until C04 freezes the flag set.
- Deserialisation is strict and validating: unknown fields anywhere in the page
  are an error, a `tuning` key on a piano instrument is an error, a required key
  that is absent — `highlight`, for example — is a missing-field error rather
  than a silent `null`, and the state types reject by themselves the same
  violations `validate_state` rejects. So no *deserialisation* path can build a
  value the public API could not have built. The state types keep public fields,
  so a caller can still hand-build such a value; `validate_state` rejects it
  before it is stored or sent.
- Error codes for one user mistake are deliberately different by entry point: a
  caller asking for a preset that does not exist for an instrument gets
  `UnknownIdentifier` (a lookup failure), while a committed state whose reference
  does not belong to its instrument is `InvalidState` (a structural violation).
  The adapter maps both to the same user-facing English message.
- A chord root on a wire surface is one of the twelve sharp names
  (`C C# D D# E F F# G G# A A# B`). The seven flat aliases (`Db Eb Fb Gb Ab Bb
  Cb`) widen the domain's note lookup (`note_index`) only — which is how the
  baseline's `chord_notes/2` resolves them, and what the oracle's flat-root
  records pin — and are rejected on every wire surface (the adapter DTOs and the
  snapshot decoder), never rewritten to the sharp equivalent (`CORE-D06`,
  decided 2026-09-28).

## Deviations

`fixtures/contract/approved-deviations.json` is the approval ledger for
deliberate departures from the frozen baseline. It carries a schema version and
an `approved_deviations` list; an empty list is valid and is the current state.
Each entry must cite the decision ID, the baseline case ID, the old and the new
output, the reason and the phase that approved it. A discrepancy is never
resolved by regenerating a fixture from the implementation under test.

## Capability flags

`catalogs()` reports which capabilities this engine build supports, so a client
renders an explicit English pending state instead of passing an unimplemented
feature off as an empty musical result. The flag set is frozen together with the
adapter API in task `C04`; no flag may be invented by a client.

## Errors

Every fallible entry point returns a stable error code, never a panic and never a
raw stack trace: `InvalidState`, `UnknownIdentifier`, `OutOfRange`,
`InvalidAction`, `InvalidUrl`, `UnsupportedOrigin`, `InputTooLarge`,
`InvalidSnapshot`, `UnsupportedSchemaVersion`, and `UnsupportedCapability`.

`UnsupportedCapability` is an addition to the plan's list, made in C03: the plan
requires a "documented capability/unavailable error" for features a phase has not
implemented yet, while its error list contained no variant for it. Without one, an
unimplemented quality would have to masquerade as an invalid action or, worse,
answer with a wrong chord. It is a pure addition — no existing code changed
meaning — and it is what keeps an incomplete engine honest: the Android client
renders an explicit English pending state instead of showing a plausible result.
Concrete resource limits are an open decision (DEC-06); until it is approved,
limits are not invented in code.

## Web URL compatibility

Existing web page URLs stay readable and writable: the native client imports
pasted/shared links and produces links the unchanged web application can open.
Query ordering is not part of the contract (decoded semantics and percent
escaping are), and no production change is made to the web application to
accommodate the engine.
