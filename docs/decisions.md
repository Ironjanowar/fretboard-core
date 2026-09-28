# Decision Register

Canonical register for this repository. Plan-level IDs (`DEC-xx`,
`Contract.Dxx`, `Delivery.Dxx`) keep their meaning from the approved plan; new
repository-local decisions use `CORE-Dnn`.

A dependent feature is not "complete" while its blocking decision is open.
Decisions are resolved by the user, recorded here with the approval reference,
and covered by a fixture and a test.

## Resolved

| ID | Decision | Evidence | Date |
|---|---|---|---|
| `DEC-09` (part 1) | Both new repositories are public, so release artifacts can be fetched without a token | `gh api repos/Ironjanowar/fretboard-core --jq .private` → `false`; same for `fretboard-android` | 2026-09-28 |
| `DEC-10` (part 1) | Toolchain tuple locked (Rust 1.98.1, UniFFI 0.32.2, JDK 21, AGP 9.4.1, Gradle 9.8, Kotlin 2.4.20, Compose BOM 2026.09.00, NDK 28.2.13676358, compileSdk/targetSdk 37) | `docs/toolchains.md`; official AGP 9.4 release notes and Gradle compatibility matrix | 2026-09-28 |

## Open

| ID | Question | Blocks | Recommendation |
|---|---|---|---|
| `DEC-10` (part 2) | Is `minSdk 26` acceptable? | P1 APK device coverage | 26: Android 8.0, covers the OnePlus 13R with room to spare |
| `DEC-08` | Who owns the persistent release signing key, and where is it backed up? | P1 signed APK | User-generated keystore, stored outside the repositories, injected in CI as a secret; the key is never created by an agent and never committed |
| `DEC-07` | Which exact HTTPS origin and base path are authorized for share URLs? | P6 share links | The deployed Fretboard web origin; no example hostname ships |
| `DEC-06` | Which concrete URL/import resource limits apply? | URL import and adapter boundaries | Choose caps after a corpus review of real generated links |
| `CORE-D01` | Which license applies to this repository? | `cargo deny` policy in `C22`, artifact license notices | User decision; the Rust ecosystem default is `MIT OR Apache-2.0`, but the license is not claimed in `Cargo.toml` until approved |
| `CORE-D02` | Emulator evidence is impossible here (`/dev/kvm` absent); real-device-only evidence, or KVM enabled on this host? | The emulator half of the P1 gate | Real-device acceptance plus documented emulator limitation, unless KVM can be exposed |
| `CORE-D03` | How is phone evidence collected (adb attached to this environment, or the user runs the `getprop` commands)? | Device API/ABI/fingerprint record in `D00` | Either works; adb access additionally enables install/update testing in P1 |

## Contract discrepancy gates (inherited from the plan)

Not decisions yet: each needs captured baseline evidence before it can be
approved or rejected. Implementing them silently is a defect.

| ID | Subject | Affects |
|---|---|---|
| `Contract.D01` | Analyzer zips formula-order notes with independently ordered interval labels | Chord/analyzer chip acceptance (P2/P3) |
| `Contract.D02` | Partial-match predicate is broader than its documented wording | Recognition (P3) |
| `Contract.D03` | Equal recognition sort keys inherit Elixir map enumeration order | Recognition (P3) |
| `Contract.D04` | Diminished/altered inversion behavior | Recognition (P3) |
| `Contract.D05` | Unusual triad-base scoring and flat-root asymmetry | Keys (P5) |
| `Contract.D06` | Modal grouping: dropped incomplete groups, implicit row order | Keys (P5) |
| `Contract.D07` | Progression prose disagrees with progression data | Progressions (P5) |
| `Contract.D08` | Exact query byte ordering (canonical native order is approved) | URL codec (P6) |
| `Contract.D09` | Plug transport behavior for duplicate/nested keys | URL import (P6) |
| `Contract.D10` | Typed rejection of malformed actions instead of crashes | Adapter (P1+) |
