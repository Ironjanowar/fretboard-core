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
| `CORE-D01` | This repository is MIT, matching the web application, with the same copyright line | `LICENSE` copied from `Ironjanowar/fretboard`; `license = "MIT"` in the workspace manifest | 2026-09-28 |
| `CORE-D02` | No emulator evidence: every phase gate uses the real phone, and the missing emulator run is a disclosed limitation | User decision | 2026-09-28 |
| `CORE-D03` | No adb workflow: each phase publishes a signed APK that the user installs by hand, and the app displays its own API level and supported ABIs so device evidence needs no command line | User decision (2 answers: "no tengo conocimiento de desarrollo Android… ¿se pueden generar APKs para probar yo?" and the emulator answer) | 2026-09-28 |
| `DEC-09` (part 1) | Both new repositories are public, so release artifacts can be fetched without a token | `gh api repos/Ironjanowar/fretboard-core --jq .private` → `false`; same for `fretboard-android` | 2026-09-28 |
| `DEC-10` (parts 1 and 2) | Toolchain tuple locked (Rust 1.98.1, UniFFI 0.32.2, JDK 21.0.2, AGP 9.4.1, Gradle 9.8.0, Kotlin 2.4.20, Compose BOM 2026.09.00, NDK 28.2.13676358, compileSdk/targetSdk 37) and `minSdk 29` | `docs/toolchains.md`; official AGP 9.4 release notes and Gradle compatibility matrix; user decision on 29 | 2026-09-28 |

## Open

| ID | Question | Blocks | Recommendation |
|---|---|---|---|
| `DEC-08` | Release signing key: the user does not want to handle a keystore himself | P1 signed APK | Agent generates one dedicated release keystore in this environment, outside every repository, with a randomly generated password stored beside it and backed up as a CI secret; the user is asked once to copy the two files somewhere safe. Key material never enters the chat, the repositories or any release artifact |
| `DEC-07` | Which exact HTTPS origin and base path are authorized for share URLs? | P6 share links | The deployed Fretboard web origin; no example hostname ships |
| `DEC-06` | Which concrete URL/import resource limits apply? | URL import and adapter boundaries | Choose caps after a corpus review of real generated links |

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
