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
| `CORE-D06` | A chord root on a wire surface is one of the twelve sharp names; the seven flat aliases stay a domain lookup (`note_index`) and are never a wire value | User decision 2026-09-28 ("hacemos tal y como esté el proyecto actual… el comportamiento de las urls"), which `02-core-contract.md` section 2 already states: URL chord and tuning parsing is sharp-only and never goes through `note_index`. The adapter's DTO root followed `note_index` instead, so it accepted `Db` and rewrote it to `C#`, while the pinned baseline's `chord_label/2` would label the same input `Dbmaj` — a difference no fixture covers. The rule is now enforced by `PitchClass::from_str`, so the adapter and the snapshot decoder accept the same set, and the domain keeps `note_index` for the baseline's flat-root `chord_notes` records. Covered by `crates/domain/tests/state_contract.rs` and `crates/mobile-ffi/tests/contract.rs` | 2026-09-28 |
| `DEC-08` | Release signing: one dedicated 4096-bit RSA keystore lives outside every repository (`~/.fretboard-signing/`, mode 600, password stored beside it), and the release build fails closed without it | Implemented in `C05`: the keystore was generated in this environment, the Android build reads `FRETBOARD_SIGNING` or that path and errors when it is missing, and every APK is signed with it — certificate SHA-256 `98785d6b9bf00f1440506facec750b034f1caa411c72fdb96fca38af8693a949`, recorded in `fretboard-android/docs/build-contract.md`. The user is asked once to copy the two files somewhere safe | 2026-09-28 |
| `CORE-D07` | The adapter error variant field carrying the domain's English sentence is named `sentence`, not `message` | Found in C05: with the field named `message`, the pinned UniFFI 0.32.2 generates Kotlin where each error subclass declares both `val message` and `override val message`, and the generated converter's `value.message` is ambiguous — the binding does not compile at all, so the engine AAR could never be built. Independent of the name, the shape is unchanged: the frozen code is the variant name, the sentence is the domain's text, and `field` stays diagnostic detail. UniFFI still renders the fields into Kotlin's own `message` property | 2026-09-28 |
| `Contract.D03` | Equal recognition sort keys inherit the baseline's Elixir map enumeration order, which is not reproducible; the native tie-break is **the frozen catalog identifier order** (`QualityId::ALL`) and, within one quality, the order of appearance | User decision 2026-09-29 ("Apruebo a con a1"), on the measured evidence below. `C12` implements it, and the parity tests compare the full ordered result *except* inside tie groups, where they compare sets; the deviation and the affected cases are listed in the parity report. Rejected: replaying the accidental order (impossible in general — the induced order has cycles — and tied to the exporter revision) and leaving it undecided (results would vary between runs) | 2026-09-29 |
| `Contract.D01` | The baseline zips notes in formula order with the independently ordered interval labels, and that zip **is** the accepted behavior — it is not a discrepancy to resolve | User approved 2026-09-29, after testing the P2 APK by hand ("Las pruebas manuales son correctas"). The domain exposes the two orders separately (`ChordDetails.notes` in formula order, `intervals` in label order) and deliberately does not zip them; a consumer that zips reproduces the baseline's raw pairs. The Android chord cards do exactly that (`CardModel.noteIntervals`), so the approved behavior is what the P2 gate shipped. Recorded here because the plan required this decision *before* chip acceptance | 2026-09-29 |
| `CORE-D08` | The adapter's exported surface is a *revision*, not a silent addition: `fixtures/contract/api-v2.json` is revision 2, the first one that freezes the capability set, and the next artifact is `fretboard-engine-0.2.0` with `api_version` 2 | Taken in the P2 integration, on the plan's own instruction that the integration owner adds the endpoints "and candidate version" (`04-core-phases.md`, C10). Revision 1 stays as the released P1 record. The capabilities are declared, not implied: `true` for what the adapter exports today (catalogs, page events, both surfaces), `false` for what the frozen order of phases has not built yet (page params, snapshot, analyzer, keys, progressions), so a client branches on the file instead of guessing. `scripts/check_aar.py` and its tests now expect revision 2 | 2026-09-28 |
| `DEC-10` (parts 1 and 2) | Toolchain tuple locked (Rust 1.98.1, UniFFI 0.32.2, JDK 21.0.2, AGP 9.4.1, Gradle 9.8.0, Kotlin 2.4.20, Compose BOM 2026.09.00, NDK 28.2.13676358, compileSdk/targetSdk 37) and `minSdk 29` | `docs/toolchains.md`; official AGP 9.4 release notes and Gradle compatibility matrix; user decision on 29 | 2026-09-28 |
| `Contract.D05` | **Decided (option A): record it as an approved deviation.** The baseline scores a key candidate against the input's *raw root string*, so a flat-root input scores 0:2 where the sharp spelling scores 2:2 | User decision 2026-09-29, on the measured evidence in `docs/p5-decision-evidence.md`. `C16` ports the full triad-base map and the containment/scoring rule unchanged; only `suggest_keys/flat-root-eb-major` differs, and only in the `score` field of its 14 suggestions, with membership and order identical. No client can reach the case: the URL codec rejects non-sharp roots and after `CORE-D06` every wire surface is sharp-only. Rejected: keeping a spelling-aware root (a second root representation contradicting `CORE-D06` for an unreachable case) and dropping the two fixture cases (a fixture is never weakened). Recorded in `fixtures/contract/approved-deviations.json` | 2026-09-29 |
| `Contract.D06` | **Decided (option B): a perfect suggestion is never dropped.** The row order is ported verbatim — it is deterministic, the note-set key term order, stable across runs — but a suggestion that fits perfectly is always shown | User decision 2026-09-29 on `docs/p5-decision-evidence.md`: the baseline groups only perfect seven-mode sets, so `single-modal-suggestion-incomplete-group` and `incomplete-modal-pair-only` leave the panel empty on a perfect match, and `incomplete-sibling-group-dropped` hides G major and E minor. 4 of the 16 `key_groups` records change, the deviation is recorded in `fixtures/contract/approved-deviations.json` with its case ids, and the four records are asserted as the only ones that differ. Rejected: porting the drop verbatim (the user declined to inherit a panel that shows nothing while a perfect match exists), the extra "also fits" row (a third behaviour nobody asked for) and grouping in Kotlin (forbidden by `C16`) | 2026-09-29 |
| `Contract.D07` | **Decided (option A): port label and data verbatim.** The prose contradictions belong to the baseline and are inherited; correcting them, if ever, is a later approved display-only change | User decision 2026-09-29 on `docs/p5-decision-evidence.md`: 6 of the 59 labels contradict their own degrees (the label reads `II7` where the data holds a ii7, reads `iv6` where the data is an iv7 with no sixth, reads "Dorian" where the data is Aeolian), 7 are names rather than chord lists and 7 claim a mode the data has not, while 46 are fully consistent. Porting both fields verbatim changes 0 of the 1180 records; fixing a label would change 2 records per progression (display only) and fixing the degrees would change what the app plays. Rejected: silently correcting the degrees to match the prose | 2026-09-29 |

## Open

| ID | Question | Blocks | Recommendation |
|---|---|---|---|
| `DEC-07` | Which exact HTTPS origin and base path are authorized for share URLs? | P6 share links | The deployed Fretboard web origin; no example hostname ships |
| `DEC-06` | Which concrete URL/import resource limits apply? | URL import and adapter boundaries | Choose caps after a corpus review of real generated links |

## Contract discrepancy gates (inherited from the plan)

Not decisions yet: each needs captured baseline evidence before it can be
approved or rejected. Implementing them silently is a defect.

| ID | Subject | Affects |
|---|---|---|
| `Contract.D02` | Partial-match predicate is broader than its documented wording | Recognition (P3) |
| `Contract.D04` | Diminished/altered inversion behavior | Recognition (P3) |
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
   `m_add9 → aug_maj7 → aug7 → m_add9`). This is the measurement the
   decision above was taken on: no total order explains the frozen fixture, so
   the native engine states its own and records the difference.

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
