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
| `CORE-D04` | The domain crate may depend on `serde` (derive) and `serde_json`, and on nothing else without a new decision | Coordinator decision 2026-09-28, taken because the contract requires JSON codecs in the domain (snapshot, page params) while forbidding platform dependencies; `AGENTS.md` and `README.md` were updated to state this precisely instead of the earlier blanket "no third-party dependency" wording. The user was told in the phase report. Reversible: the alternative is a hand-written JSON codec in the domain, which is more code and more risk for no boundary gain | 2026-09-28 |
| `CORE-D05` | `CoreError::UnsupportedCapability` is added to the frozen error list | Coordinator decision 2026-09-28 in C03: the plan demands a documented capability/unavailable error for not-yet-implemented features but its error list had no variant for it, so an unimplemented chord quality could only have lied or mislabelled itself. Pure addition, recorded in `docs/contracts.md`; the Android client renders an explicit pending state for it | 2026-09-28 |
| `CORE-D07` | The adapter error variant field carrying the domain's English sentence is named `sentence`, not `message` | Found in C05: with the field named `message`, the pinned UniFFI 0.32.2 generates Kotlin where each error subclass declares both `val message` and `override val message`, and the generated converter's `value.message` is ambiguous — the binding does not compile at all, so the engine AAR could never be built. Independent of the name, the shape is unchanged: the frozen code is the variant name, the sentence is the domain's text, and `field` stays diagnostic detail. UniFFI still renders the fields into Kotlin's own `message` property | 2026-09-28 |
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
| `Contract.D03` | Equal recognition sort keys inherit Elixir map enumeration order | Recognition (P3) — measured, see below |
| `Contract.D04` | Diminished/altered inversion behavior | Recognition (P3) |
| `Contract.D05` | Unusual triad-base scoring and flat-root asymmetry | Keys (P5) |
| `Contract.D06` | Modal grouping: dropped incomplete groups, implicit row order | Keys (P5) |
| `Contract.D07` | Progression prose disagrees with progression data | Progressions (P5) |
| `Contract.D08` | Exact query byte ordering (canonical native order is approved) | URL codec (P6) |
| `Contract.D09` | Plug transport behavior for duplicate/nested keys | URL import (P6) |
| `Contract.D10` | Typed rejection of malformed actions instead of crashes | Adapter (P1+) |

## Measured evidence for `Contract.D03` (2026-09-28, coordinator)

The frozen identify fixtures are deterministic: repeated exports of the same
exporter revision are byte-identical. But the *order of tied interpretations* is
not a function of the pinned source alone. Three measurements, all reproduced
first-hand in this environment:

1. **The pinned code's own output order depends on the VM's atom-table state.**
   Two fresh VMs, same pinned checkout, same input, same code path
   (`Fretboard.Music.analyze_notes/1`):

   ```sh
   MIX_ENV=test mix run --no-start -e 'IO.puts(Jason.encode!(Fretboard.Music.analyze_notes(["C","D","E"])))'
   MIX_ENV=test mix run --no-start -e 'Enum.each(1..500, fn i -> String.to_atom("zz_intern_#{i}") end); IO.puts(Jason.encode!(Fretboard.Music.analyze_notes(["C","D","E"])))'
   ```

   The two outputs differ (sha256 `21f6561c…` vs `076df423…`). Interning 500
   unrelated atoms before the call changes the order of the returned
   interpretations. The candidate order comes from `Map.to_list(@formulas)`
   followed by a stable sort, so ties inherit that map's enumeration order, and
   that order follows the VM's atom state rather than the source text.
2. **The same instability reaches the fixtures.** Adding three harmless atom
   literals to `tools/oracle/catalog.exs` changed `identify.jsonl`,
   `analyzer.jsonl` and `catalogs.json` while `chords.jsonl`, `keys.jsonl`,
   `multi-keys.jsonl` and `scales.jsonl` stayed byte-identical.
3. **A single global rank table is not demonstrably equivalent.** Grouping the
   frozen results by the documented sort keys gives 176,388 tie groups, and the
   induced order over the 47 qualities contains cycles (for example
   `m_add9 → aug_maj7 → aug7 → m_add9`). `Contract.D03` therefore stays
   **blocked**.

Consequences, recorded rather than papered over:

- Fixtures are reproducible only with the exact exporter revision, which is why
  `manifest.json` records `exporter_sha256`. Any later edit to an exporter source
  file must re-review `identify.jsonl` and `analyzer.jsonl` (the sharding
  amendment already did exactly that).
- The same instability applies to the web application itself: its interpretation
  order can differ between deployments or runs depending on the VM's atom state.
  That is baseline behavior to be decided at P3, not a native defect.
- P3 (`C12`) must either derive an order provably equivalent to the frozen
  fixture or obtain approval for an explicit deterministic tie-break as a
  documented deviation. It must not assume the reconstructed `@formulas`
  enumeration order is the answer.
