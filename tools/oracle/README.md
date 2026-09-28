# Oracle exporter (task C01)

Produces the frozen baseline fixtures for the portable Rust core from the
**real, pinned Elixir implementation** of the Fretboard web application.

Nothing in this directory re-implements, translates or copies a musical rule.
Every `output` value in every fixture is the untouched return value of a call
into the pinned `Fretboard.Music` facade (or into an explicitly allowlisted
internal helper listed under *Provenance*). Expected values are never computed
here, never taken from documentation and never taken from the Rust port.

## Files

| File | Role |
|---|---|
| `export.exs` | entry point: reads the environment, runs `export` or `manifest` mode |
| `normalize.exs` | `FretboardOracle.Normalize` - canonical JSON shape, hashing, writers |
| `catalog.exs` | `FretboardOracle.Catalog` - `catalogs.json` plus metadata helpers |
| `cases.exs` | `FretboardOracle.Cases` - the nine case-based JSONL fixtures |

Python counterparts (`check_export.py`, `test_check_export.py`) and the
page-level exporter (`page_events_test.exs`) are separate files owned by other
tasks.

## Environment contract

| Variable | Mode | Meaning |
|---|---|---|
| `ORACLE_SOURCE_SHA` | `export`, `manifest` | required; 40 lowercase hex characters |
| `ORACLE_OUT` | `export`, `manifest` | required; the run directory |
| `ORACLE_MODE` | both | `export` (default) writes the nine unsharded domain fixtures, the `identify-NN.jsonl` shard set and `run-info.json`; `manifest` writes `manifest.json` listing every fixture file actually present in `ORACLE_OUT` (the 13 unsharded fixtures plus the identify files it discovers in the directory) and fails when any unsharded fixture or the identify set is missing |
| `ORACLE_ROUNDTRIP_INPUTS` | - | **not implemented until task C19**: passing it always fails with an explicit error, never a silent no-op |

`manifest` mode never writes or repairs a fixture: a missing page-level file is
a hard error naming the four owners of the page fixtures. It also refuses to
write a manifest for a broken identify set: the files must be either a single
`identify.jsonl` or numbered `identify-NN.jsonl` shards contiguous from `01`
with no gaps. `manifest.json` never lists `manifest.json` or `run-info.json`,
and `files` is no longer a fixed list of 14 — it is whatever fixture files are
actually present, ascending by name.

## Running it

```sh
cd <scratch checkout of the pinned commit>
ORACLE_SOURCE_SHA=2daa8c665efa268942dda352691f39d78db42512 \
ORACLE_OUT=/workspace/repos/fretboard-core/dist/oracle-run-a \
MIX_ENV=test mise exec -- mix run --no-start \
  /workspace/repos/fretboard-core/tools/oracle/export.exs
```

* `--no-start` is mandatory: normal `mix run` application boot hangs here.
* The exporter must run with the pinned runtime (Elixir 1.19.5 / OTP 27) and
  with its **current working directory inside the pinned checkout**: two source
  attributes are read from the checkout for the contract-D03 tie order and for
  the frozen presentation palette. The exporter aborts with a clear message if
  it cannot find them, rather than silently emitting less coverage.
* `scripts/export-oracle.sh --source /workspace/repos/fretboard --commit <sha>
  --out <dir>` performs the whole procedure (pin check, `git archive` into a
  temporary directory outside any repository, `deps.get --check-locked`,
  `compile --warnings-as-errors`, export, page-event tests, manifest) and never
  modifies the source repository.

## Fixture coverage

| Fixture | Records | Coverage |
|---|---|---|
| `catalogs.json` | 16 top-level keys | 5 instruments, all presets, 47 chord qualities (formula + interval labels), 8 quality groups, 15 scale types, 7 scale groups, 59 full progression definitions, 4 progression groups, 12 simple interval names, asserted unique counts 47/15/59 |
| `chords.jsonl` | 2543 | 12 sharp roots x 47 qualities for notes / labels / zipped note-interval pairs; 7 flat aliases x 47 for notes and zipped pairs; per-quality formula, interval labels and label; chord-mode inference for every quality and five quality sets |
| `identify-01.jsonl` … `identify-15.jsonl` | 15693 total | every 12-bit pitch-class subset with at least 3 classes (4017); every full formula with every member bass (2652); missing-1 and missing-2 subsets (7896); foreign basses (1128). Whole ordered result arrays are recorded, including ties. One stream, written as ordered 16 MiB shards (Amendment 1 below) |
| `analyzer.jsonl` | 34 | empty, single, repeated pitch, octave, two classes with extra octaves, permutations, real fretted crossings, reentrant ukulele, all five instruments, sounding pitches above 127 |
| `tunings.jsonl` | 1316 | every preset x every string x every chromatic note edit (1212), fixed-reference multi-edit sequences, exact preset detection incl. shifted `Custom`, legacy guitar aliases, string counts |
| `surfaces.jsonl` | 525 | every instrument and preset metadata, every fret 0-24 on every standard string, representative `fretboard_data` grids, all 36 piano keys, duplicate/highlight fill semantics, color slots |
| `scales.jsonl` | 555 | every tonic x all 15 scale types (notes), every scale label, every tonic x scale x both chord modes |
| `keys.jsonl` | 17 | empty, single, exact, seventh, extended, duplicate, flat root, two no-compatible inputs, lexical-order and tie inputs |
| `multi-keys.jsonl` | 15 | under 3, all-singleton, duplicate indices, overlapping full membership, unmatched tail, maximum of three groups, tie, `Dmin/Gmaj/Emaj/Fmaj`, `Dmin/Gmaj/Fmaj`, A-minor family and a seventh chain |
| `progressions.jsonl` | 1180 | every sharp tonic x all 59 ids (708), every stored example key x all 59 ids (354, includes `Bb`), full definition metadata and labels for all 59 ids |

Record counts above are the measured values of the frozen run; the authoritative
values live in `manifest.json`.

### Amendment 1 — the identify stream is sharded

`identify.jsonl` is large by construction: all 4017 subsets plus the bass,
missing-note and foreign-bass families record whole ordered interpretation
arrays. The measured single-file size is **244 365 265 bytes**, which cannot be
committed or pushed (GitHub rejects blobs over 100 MB, and a blob that size is
hostile to every future clone).

* The stream is written as `identify-01.jsonl`, `identify-02.jsonl`, … — `NN`
  zero padded from `01`, contiguous with no gaps, records in the frozen
  generation order. A new shard is started as soon as appending the next record
  would push the current shard past **16 MiB (16 × 1024 × 1024 bytes)**. A shard
  always holds at least one record, so a record larger than the limit cannot
  stall the stream.
* Coverage is **not** reduced and no record is dropped, renamed or re-shaped:
  the shard set is a byte-exact partition of the record stream this exporter
  revision produces. Measured on the frozen corpus: **15 shards, 244 365 265
  bytes, 15 693 records** in total — the same byte and line counts as the old
  single file — with an **identical `case_id` sequence**.
* Proof that the split itself changes no byte: re-running the *same* exporter
  revision with the shard limit raised to 1 GiB writes a single
  `identify-01.jsonl` whose SHA-256
  (`938046e682d240a02fe067a467edfe391be9480485f36817f6f9fa7cd04cefbb`) equals
  `cat identify-*.jsonl | sha256sum` of the 16 MiB shard set.
* The tie order *inside* the records is nevertheless not identical to the
  pre-amendment `identify.jsonl`: the amendment edits `cases.exs` and
  `export.exs`, and any exporter source edit interns new atoms and permutes
  ties. Measured against the pre-amendment file: identical `case_id` set and
  sequence, 4 983 records byte-identical, 10 710 records differing **only** in
  the order of nested lists, and 0 records with any other change. The shard set
  is therefore a new frozen revision that must be re-reviewed and re-accepted
  together with its new `exporter_sha256` (next section).
* `case_id` uniqueness is checked **across the whole shard set**, not per file,
  and a re-export removes stale identify files first so the set cannot become
  non-contiguous. Page-level fixtures (other owner) are never touched.
* A bare `identify.jsonl` remains the legal degenerate single-shard form; this
  exporter always writes the numbered form.

With the current corpus `manifest.json` therefore carries **28 entries**: the 13
unsharded fixtures plus the 15 identify shards, ascending by name.

## Determinism

* Identical inputs produce identical bytes: no timestamps, host names,
  durations, absolute paths or random ordering ever enter a fixture or
  `manifest.json`. `run-info.json` holds the run evidence and is never part of
  the compared payload.
* JSON objects are emitted with keys sorted ascending through
  `Jason.OrderedObject`; lists keep their original order, always.
* `nil`/`true`/`false` keep their JSON-native form so boolean flags such as
  `exact` stay booleans; every other atom becomes its name.
* Tagged tuples become `{"variant": ..., "fields": [...]}`.
* Catalog key/label pairs returned by the facade (`{instrument, label}`,
  `{preset_name, values}`, `{group, members}`) are converted to explicit objects
  (`{"id"/"name": ..., ...}`) so the `variant` form stays reserved for real
  tagged tuples such as `{:chords, notes, bass, interpretations}`.
* Map-set valued terms are the only sorted lists (`{"variant": "map_set",
  "fields": [...]}`), because a `MapSet` has no meaningful iteration order to
  preserve. Nothing musical is sorted.
* `case_id` values are asserted unique per file while writing; a collision
  aborts the run instead of producing an ambiguous fixture.

### Exporter-revision reproducibility — measured atom-interning dependence

`Fretboard.Music.Chord.identify/1` shows `Map.to_list/1` over the 47-entry
`@formulas` map as its candidate order and then applies a **stable** sort, so
ties in the result array follow that map's enumeration order. The exporter
captures the observed order in `catalogs.json`
(`chord_formula_enumeration_order`) instead of sorting it away, and rebuilds the
`@formulas` literal from the pinned source with an allowlisted AST reader,
verifying the rebuilt map against `Fretboard.Music.Chord.formula/1` for all 47
qualities before use.

That map is rebuilt from the pinned source at run time, so its enumeration order
is fixed by the Erlang VM's **atom-table interning state** at the moment the map
is built — and the exporter's own source text participates in that state, because
`catalog.exs`/`cases.exs` are compiled (`Code.require_file`) before
`Fretboard.Music.Chord` is loaded. Consequence: **any edit to an exporter source
file that interns new atoms can silently permute tie order in `identify.jsonl`
and `analyzer.jsonl`.**

**Measured behaviour (this matters more than the list itself):**

* Two exports of byte-identical sources in separate OS processes produce
  byte-identical fixtures. Re-verified for the sharded exporter as well:
  `dist/dev-b3/run-a` and `run-b` are identical (`diff -r --exclude=run-info.json`
  is empty).
* The pre-amendment exporter still reproduces its frozen `identify.jsonl`
  exactly in the pinned checkout: SHA-256
  `a4b2f0dc3c3beedde8ffd224509e484a19cd9de32e996ae6e161702030fcada9`
  (244 365 265 bytes), i.e. the environment is deterministic and the RUN is not
  the variable.
* The recorded order is *not* a function of the key set alone. Adding three
  harmless atom literals (`:zz_probe_one`, `:zz_probe_two`, `:zz_probe_three`)
  to `catalog.exs` changed `identify.jsonl`, `analyzer.jsonl` and
  `catalogs.json` while `chords.jsonl`, `keys.jsonl`, `multi-keys.jsonl`,
  `scales.jsonl`, `tunings.jsonl`, `surfaces.jsonl` and `progressions.jsonl`
  stayed **byte-identical**. Measured shape of that change for the identify
  fixture: identical `case_id` set and sequence, 5 107 of 15 693 records
  byte-identical, the remaining 10 586 differing *only* by nested-list (tie)
  order, and **zero** records with any other change — every interpretation array
  still holds the same elements, with identical result counts.
* The exact permutation is a function of the exact interning sequence, not of
  the atom count: placing three literals somewhere else yields yet another hash.
  So the tie order is captured provenance, not a rule that can be recomputed.
* Adding fresh atom literals *after* the quality atoms are already interned had
  no effect.

Consequences:

1. The freeze is sound only for identical `(source_commit, runtime,
   exporter_sha256)`. That triple is exactly what `manifest.json` records and
   what `check_export.py` compares, so a frozen run is reproducible — but any
   edit to any exporter source can reorder ties, which is why `exporter_sha256`
   must be compared before trusting fixture bytes, and why a fixture freeze is
   only meaningful together with the exporter revision that produced it.
2. Whenever the fixtures are regenerated, `chord_formula_enumeration_order` in
   `catalogs.json` must be re-read and re-reviewed, and any explicit legacy rank
   table derived from it (plan Contract.D03) must be re-derived and re-verified.
   Do not assume alphabetical order, `available_qualities/0` order, Rust enum
   order or `HashMap` order.
3. The order is *not* stable across environments, so it must be treated as
   captured provenance carried by the frozen fixture, not as a rule that can be
   recomputed.
4. A change to the *fixture layout* is such an edit too: Amendment 1 (sharding)
   permutes ties relative to the pre-amendment single-file fixture without
   changing coverage. Any future edit to `cases.exs`, `catalog.exs`,
   `export.exs`, `normalize.exs` or `page_events_test.exs` therefore requires
   re-reviewing the identify and analyzer fixtures and re-recording
   `exporter_sha256`.

Closing this properly (an approved explicit deterministic tie-break, per plan
Contract.D03) is a coordinator/user decision; the exporter reports the order it
observed rather than inventing one.

## Provenance

`catalogs.json` carries `internal_calls` (every non-facade module function the
exporter calls, with what uses it) and `source_literals` (the two allowlisted
source literals and why they are read). Every JSONL record carries
`baseline_source_function`, naming the function that actually produced its
`output`.

Internal (non-facade) calls:

* `Fretboard.Music.Chord.formula/1`, `Fretboard.Music.Chord.interval_labels/1`
* `Fretboard.Music.Progression.all/0`
* `Fretboard.Music.Intervals.name/1`
* `FretboardWeb.FretboardSVG.note_fill/4`, `FretboardWeb.Modals.chord_color/2`
* source literals `@formulas` and `@chord_colors`, rebuilt with an allowlisted
  literal-only AST reader (maps, lists, binaries, integers, floats and atoms
  only; anything else aborts)

## Known limitations and open decisions

* **`active_chord_colors/1` is private.** The LiveView's per-occurrence colour
  slot list cannot be called, so `surfaces.jsonl` supplies the `colors` argument
  to `note_fill/4` explicitly as an input (palette from the pinned
  `@chord_colors`, the frozen slot rule documented in the core contract). The
  function under test, `note_fill/4`, is the real one.
* **`identify` sharding** (Amendment 1, see above) replaces the single ~244 MB
  fixture with 15 ordered `identify-NN.jsonl` shards. Coverage is unchanged;
  the new tie order must still be accepted as a reviewed frozen revision
  (`exporter_sha256`) as described under *Exporter-revision reproducibility*.
* **`@formulas` and `@chord_colors` are read from the working tree**, not from
  compiled module attributes, because Elixir offers no public accessor. This is
  why the exporter refuses to run outside the pinned checkout.
* Distinct-but-overlapping behaviour is preserved as-is: flat root handling in
  `suggest_keys/1` (for example `[Bb major, G major]` and the flat-root
  scoring asymmetry), the zipped note/interval pairing of contract D01, and
  inversion mapping of contract D04 are exported raw, not "corrected".
