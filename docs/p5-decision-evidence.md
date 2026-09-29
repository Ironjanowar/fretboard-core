# Measured evidence for the three open P5 decisions

This file belongs to `C15`'s delivery. It **measures** `Contract.D05`, `Contract.D06`
and `Contract.D07`; it does **not** decide them. `docs/decisions.md` still records
all three as open, and `C16`/`C18` remain gated on them.

Every claim below is followed by the command that produced it and the case id or
output line it rests on. Nothing here is an estimate: the baseline is the pinned
Elixir checkout (`/workspace/oracle-src-pin`, source
`2daa8c665efa268942dda352691f39d78db42512`), the frozen expectations are
`fixtures/oracle/{keys,key-groups,progressions}.jsonl`, and the native side was
run for real.

## What was run, and where

Scratch tools (never committed; they live under `/workspace/.hermes-tmp/`):

| Tool | What it is |
|---|---|
| `p5-decisions/keys.exs` | pinned `Fretboard.Music.Scale.suggest_keys/1` over every `keys.jsonl` input, plus the same inputs with flat roots spelled sharp |
| `p5-decisions/keys-asymmetry.exs` | the two `suggest_keys` answers fed to the pinned page grouping |
| `p5-decisions/keys-multi-flat.exs` | `suggest_multi_keys/1` and `suggest_keys/1` on the same flat and sharp chords |
| `p5-decisions/keys-flat-reachable.exs` | whether a flat root can reach the app through the page codec |
| `p5-decisions/key-groups.exs` | pinned `FretboardWeb.FretboardLive.group_key_suggestions/1` over every `key-groups.jsonl` input, with and without 3000 unrelated interned atoms |
| `p5-decisions/key-group-order.exs` | the order the grouping actually enumerates its note-set groups in |
| `p5-decisions/key-group-term-order.exs` | whether that order is the term order of the note sets |
| `p5-decisions/progressions.exs` | every progression's prose next to the chords its own degrees resolve to, and all 1062 resolved cases |
| `p5-native` (scratch crate) | the native prototype: it drives the committed native surface (`C03` notes, `C06` chords, `C15` scales) through the baseline's documented rule for keys, grouping and progression resolution |

```sh
# pinned Elixir (the first run prints dependency noise)
cd /workspace/oracle-src-pin && /usr/local/bin/mise exec -- mix run --no-start <driver>.exs

# native prototype, own Cargo.toml with an empty [workspace], path dependency on
# /workspace/repos/fretboard-core/crates/domain; never committed
cd /workspace/.hermes-tmp/p5-native && cargo build --offline
./target/debug/p5-native keys <keys.jsonl> | key-groups | progressions | keys-normalized
```

The prototype exists because `keys.rs`, `key_groups.rs` and `progression.rs` do
not exist yet — `C16` and `C18` are gated on exactly these three decisions. Two
things in it are transcribed from the pinned source and marked `TRANSCRIBED` in
its source: `@quality_to_triad` (47 entries, `lib/fretboard/music/scale.ex`) and
the resolution/grouping rules. Everything musical — notes, formulas, diatonic
triads, the frozen scales of `C15` — comes from committed native code; the `C15`
live cross-check proved that surface equals the pinned one line for line
(555/555, `diff` empty).

Two results hold for all three decisions and are used by every option below:

* the pinned Elixir reproduces the frozen expectations exactly — `keys.jsonl`
  17/17, `key-groups.jsonl` 16/16, `progressions.jsonl` 1180/1180
  (`progressions-live.txt`: 59 definition lines and 1062 `progression_chords`
  lines, both `diff`-empty against the fixture);
* the native prototype reproduces them too, once the rules are stated
  explicitly: keys 17/17 with the raw-root rule, key-groups 16/16 with the key
  rule of D06, progression resolution 1062/1062.

## Contract.D05 — unusual triad-base scoring and flat-root asymmetry

### What the baseline does

`Scale.suggest_keys/1` (`build_result/3`) scores a candidate key by counting input
occurrences whose root **and** triad-base quality match the candidate's diatonic
triad map. Two measured behaviours:

1. **The triad-base map is a hand-written table, not a third derived from the
   chord.** `@quality_to_triad` has 47 entries covering the whole catalog:
   `9b5`, `7b5`, `m11b5`, `dim_maj7`, `dim7b13` → `dim`; `9#5`, `aug7`,
   `aug_maj7` → `aug`; `7#9`, `7b9`, `13b9`, `7b13` → `major`; `m_add9` → `minor`;
   `sus9`/`susb9` → `sus2`; `7sus4`/`sus13` → `sus4`. The fixture pins the
   resulting scores: `suggest_keys/sus-and-dim-mixed` (C sus4, G sus2, B dim)
   answers `A:minor:1:3` and seven more at `1:3` — nothing reaches `3:3`;
   `suggest_keys/extended-ninths` (C maj9, G 13) answers `C:major:2:2` and six
   relatives at `2:2`, because `maj9` and `13` both map to `major`.
2. **The score compares the input's raw root *string* against the candidate's
   sharp note names, while containment resolves the flats first.**
   `valid_candidate?/3` builds the chord's notes with `Chord.notes/2`, which
   resolves `Eb` through `Note.note_index/1` — so a flat-rooted chord passes
   containment — but `build_result/3` does `Map.get(dc_map, root) == expected`
   with the string `"Eb"` against a map whose keys are `"D#"`, so it can never
   score. Measured:

   ```
   suggest_keys/flat-root-eb-major|14|A:locrian:0:2,A#:major:0:2,...,G#:lydian:0:2
   ```

   Every one of the 14 suggestions is `0:2`, and the whole list is ordered by the
   lexical tie-break alone. The same chords spelled sharp score perfectly:

   ```sh
   diff <(grep 'flat-root-eb-major' live-keys-raw.txt) <(grep 'flat-root-eb-major' live-keys-sharp.txt)
   # 0:2 for all 14  vs  2:2 for all 14, same order, same 14 candidates
   ```

   `suggest_keys/flat-root-Bb-with-g` has no suggestion either way, so it is not
   affected.

3. **The sibling path normalizes.** `suggest_multi_keys/1` runs every input chord
   through `normalize_chord/1` (`@flat_to_sharp`) before covering and scoring, so
   the two key surfaces disagree about the same input
   (`keys-multi-flat.exs`): with `[Eb major, Bb major, G minor]` the multi-key
   answer is `D#:major:3[D#:major+A#:major+G:minor]`, byte-identical to the sharp
   spelling, while single-key answers `1:3` for the flat spelling and `3:3` for
   the sharp one — only the `G` chord scores when the other two roots are flats.

### The difference is observable only through the raw domain API

The pinned page codec rejects a flat chord token outright
(`keys-flat-reachable.exs`):

```
decode_page_params(%{"chords" => "Dbmaj,Ab"})  ->  active_chords: []
decode_page_params(%{"chords" => "C#maj,G#maj"}) -> active_chords: [%{root: "C#",...}, ...]
```

(`URLCodec.to_chord_if_valid/2` checks `MapSet.member?(@valid_notes, root)` against
the twelve sharp names.) The asymmetry is reachable only by calling
`Scale.suggest_keys/1` directly — which is what the oracle exporter does, and why
the fixture carries these cases. The same probe shows the raw surface difference
plainly: `suggest_keys([C# major])` best is `1:1`, `suggest_keys([Db major])` best
is `0:1`.

In the native engine the asymmetry is not merely unreachable, it is
**unrepresentable**: `ChordSpec.root` is a `PitchClass` and `ChordSpec.quality` is
a `QualityId`, so there is no raw spelling to compare. The prototype had to keep
the input string to reproduce the baseline at all.

### What our current native code answers, run for real

The prototype implements the documented rule over the committed surface
(`scale_notes`, `diatonic_chords`, `chord_details`) and keeps the raw root string
only to make the baseline comparable:

```sh
diff native-keys.txt            live-keys-raw.txt   # 17/17 lines identical
diff native-keys-normalized.txt live-keys-raw.txt   # 16/17 identical
```

With pitch-class root equality (the only thing a `PitchClass` API can express):

```
suggest_keys/flat-root-eb-major|14|A:locrian:2:2,A#:major:2:2,...,G#:lydian:2:2
```

— same 14 candidates, same order, one record differs in the score field only.
The user-visible consequence is measured through the pinned page grouping
(`keys-asymmetry.exs`), because the grouping branch depends on `score == total`:

```
flat  suggestions=14 rows=3
  S[A:locrian:0:2] S[A#:major:0:2] S[A#:mixolydian:0:2]
sharp suggestions=14 rows=2
  G[A#:major:2:2;G:minor:2:2|...] G[D#:major:2:2;C:minor:2:2|...]
```

The key that actually contains the chords (`A# major`, the sharp name of the flat
tonic) is the first prominent card only in the sharp case; for a flat input the
page shows three loose cards and a best score of `0:2`.

### Observable, and what it would change

* **User-visible?** In the shipped app: no. The page codec drops flat chord tokens
  and after `CORE-D06` every wire surface is sharp-only, so no client can deliver
  the input the asymmetry needs. Through the raw domain API: yes — a caller that
  keeps flat spellings (the exporter, or a test) sees `0:2` instead of `2:2` and a
  different page grouping.
* **Frozen records:** the native pitch-class rule changes exactly **1 of 17**
  `keys.jsonl` records (`suggest_keys/flat-root-eb-major`, the `score` field of 14
  suggestions; membership and order unchanged). Nothing else in `keys.jsonl`,
  `key-groups.jsonl` or `progressions.jsonl` moves.

### Options

| # | Option | Consequence | Frozen records |
|---|---|---|---|
| A | Keep the native pitch-class equality and record D05 as an approved deviation, the way `Contract.D03` was recorded | `C16` ports the full triad-base map and the containment/scoring rule as they are; only the flat-root score cannot be reproduced. The deviation is one record, one field, no order or membership change, and no client can reach it | 1 of 17 keys records differs; a deviation entry and a parity report like `C12`'s are needed |
| B | Keep a spelling-aware root for scoring (`spelled_root: Option<...>` or a flat-capable root type on the domain API) | The fixture matches byte for byte, but the domain grows a second root representation that contradicts `CORE-D06` (roots on wire surfaces are sharp-only) purely to reproduce an unreachable case; every consumer of `ChordSpec` would have to decide which root it means | 17/17 keys records stay identical |
| C | Assert the asymmetry is out of scope and drop the two flat-root fixture cases from `keys.jsonl` | Smallest native surface, but it removes coverage instead of recording a decision: the exporter's own call would still answer differently, and `AGENTS.md` forbids weakening a fixture | 2 records removed — needs a re-frozen fixture and a release note; not recommended |

**Recommendation: A.** The asymmetry cannot be expressed by a sharp-only typed
root, cannot be reached by any client, and costs one record in one field; the
honest handling is the `Contract.D03` precedent — state the deviation, pin the
rest, and let the parity report count it. B buys byte-parity for a case no user
can produce, at the price of a second root type. C loses evidence for nothing.

## Contract.D06 — modal grouping: dropped incomplete groups, implicit row order

### What the baseline does

`FretboardWeb.FretboardLive.group_key_suggestions/1` groups only the suggestions
at the **maximum score**, and only when that score is perfect
(`max_score == total`). Within that branch it partitions the modal modes
(major, minor, dorian, phrygian, lydian, mixolydian, locrian) by note set, keeps
**only the note sets with all seven modes**, renders the major and minor of each
such set prominently, and appends the non-modal suggestions and then the
lower-scoring ones. Two independent behaviours follow.

**1. Incomplete modal groups are dropped silently.** Measured, `key-groups-live.txt`:

| Case id | Input | Rows shown |
|---|---|---|
| `key_groups/single-modal-suggestion-incomplete-group` | 1 perfect `C major 1:1` | **0 rows** (0 items) |
| `key_groups/incomplete-modal-pair-only` | perfect `C major 2:2` + `A minor 2:2` | **0 rows** (0 items) |
| `key_groups/incomplete-modal-pair-with-non-modal-survivor` | perfect C major, A minor and C pentatonic major | 1 row, only `S[C:pentatonic_major:2:2]` |
| `key_groups/incomplete-sibling-group-dropped` | 9 perfect suggestions: the C group (7 modes) and a second group with only `G major` + `E minor` | 1 collapsed row of 7 items — `G major` and `E minor` vanish |

The first two are the sharp ones: a **perfect** key suggestion produces an empty
suggestion panel. This is the behaviour the contract calls out ("baseline drops
incomplete modal groups in this branch"); the page's own gate (at least two active
chords) does not help, because both cases have perfect suggestions.

**2. The row order is the grouping map's enumeration order — and that order is
reproducible.** `group_modal_modes/1` returns `Enum.group_by(modal, &note_set).¹
|> Enum.map(...)`, so the rows come out in the iteration order of a map keyed by
the note set. Measured in this runtime (`key-group-term-order.exs`):

```
A#set (F major) < Cset (C major) < F#set (G major)      # pairwise term order
Enum.group_by([5,3,1,4,2]) -> [{1,..},{2,..},{3,..},{4,..},{5,..}]
Map.new([{"b",1},{"a",2},{"c",3}]) -> [a, b, c]         # small maps enumerate in key term order
map keyed by the three sets   -> [A#, C, G]             # = the fixture's row order
```

and the fixture's two-group case confirms it: `key_groups/tied-complete-seven-mode-row-order`
spells the input `C major` first and the fixture answers the `F major` group
first. Stability was measured too: the whole 16-case run is byte-identical with
and without 3000 unrelated interned atoms
(`diff key-groups-live.txt key-groups-live-interned.txt` → empty), and it equals
the frozen expectations, i.e. three independent runs (exporter, plain VM, interned
VM) agree. **This is not the `Contract.D03` situation**: the tie order there came
from the atom table and had cycles; here the key is a term (a `MapSet` of note
names), and the order is its term order.

The imperfect branch is separate and intentional: when the best score is not
perfect, `single_rows(Enum.take(suggestions, 3))` shows the top three and nothing
else (`key_groups/imperfect-top3-truncates-seven-candidates`: 7 candidates at the
maximum score, 3 rows).

### What our current native code answers, run for real

Implemented in the prototype against the committed `scale_notes`:

```sh
diff native-key-groups.txt        key-groups-live.txt  # discovery order: 3 of 16 differ, row order only
diff native-key-groups-sorted.txt key-groups-live.txt  # 16/16 identical
```

* With the group order stated as the term order of the note set (a `BTreeMap`
  over the sorted note *names*, which is what a `MapSet`'s term order compares),
  the native answer is identical on all 16 records — **including the dropped
  groups** — so "port the baseline" is fully implementable and testable.
* With the group order taken from the input (discovery order), 3 records differ
  in row order and nothing else: `complete-seven-mode-shared-note-set`,
  `complete-seven-mode-with-non-modal-survivors`, `tied-complete-seven-mode-row-order`.
* Membership is identical in every variant; the drop is a rule, not an accident.

### Observable, and what it would change

* **User-visible?** Yes, the drop is: for `single-modal-suggestion-incomplete-group`
  and `incomplete-modal-pair-only` the user opens the key panel on a perfect match
  and sees **nothing**; in `incomplete-sibling-group-dropped` the user sees the
  C group and not the equally perfect `G major`. The row order is only visible as
  which group appears first (a cosmetic difference between two complete groups).
* **Frozen records:** porting the baseline verbatim changes **none** (16/16
  reproducible, two of them only with the stated term-order rule). Showing the
  dropped groups changes **4 of 16** records and would need deviation entries and
  a release note.

### Options

| # | Option | Consequence | Frozen records |
|---|---|---|---|
| A | Port the baseline verbatim: drop incomplete modal groups, keep the term-order row rule | The app can show an empty panel on a perfect key; `C16` is a faithful port and everything is pinned | 16/16 stay identical |
| B | Never drop a perfect top suggestion: render a partial group (its available modes) or, when a group has fewer than 7, render its members as single cards | The panel always shows what it found; the collapse still means "7 relative modes". Rows for the 4 cases above change; the UI gains cards it has never shown | 4 of 16 records change → deviation entries + release note |
| C | Keep the drop but append one honest row, e.g. "also fits: C major 2/2", when modal suggestions were dropped | Smallest visible change that removes the empty panel; still an addition to the frozen rows | 3-4 records change |
| D | State the row order as the term order of the note set and treat the drop as a bug to fix at the UI level (group in Android, drop nothing) | Moves sorting/grouping to Kotlin, which `C16` explicitly forbids ("Kotlin must not re-score/group/sort"); rejected on the plan's own rule | 4 records change |

**Recommendation: B**, with A's row rule. The row order half needs no real
decision — it is measured, stable and reproducible with one stated comparator, so
`C16` can implement "the term order of the note set" and keep the fixture. The
drop is a real product decision: option B is the only one that removes the empty
panel for a *perfect* match while keeping the collapse feature meaningful, and it
is the smallest set of records to re-approve (4). If the user prefers the smallest
possible change to the shipped UI, C is defensible; A means shipping a panel that
sometimes shows nothing.

## Contract.D07 — progression prose disagrees with progression data

### What the baseline does

Each progression holds prose (`name` = the label the picker shows, `description`,
`genre`, `notable_songs`, `example_key`) and data (`scale_type` + ordered
`degrees` of `degree/accidental/quality`). `progression_chords/2` resolves the
degree list against the key's **triad** chords: explicit quality wins, a zero
accidental takes the diatonic quality, and a `-1` accidental on degrees 2, 3, 6
or 7 is a major triad, otherwise the diatonic quality. Measured over all 59
definitions (`progressions-live.txt`, `progressions.exs`), checking every label's
roman-numeral list against the chords the data resolves to:

| Case id | Label | Data (degrees → resolved at the example key) |
|---|---|---|
| `rhythm_changes_b` | `Jazz: Rhythm Changes B (III7-VI7-II7-V7)` | `3/0/7;6/0/7;2/0/min7;5/0/7` → `D:7,G:7,C:min7,F:7` — the label's `II7` is a **ii7** |
| `descending_chromatic_bass` | `Chromatic: Descending Bass (I-i7-IV-iv6-I)` | `1/0/nil;1/0/min7;4/0/nil;4/0/min7;1/0/nil` → `C:major,C:min7,F:major,F:min7,C:major` — the label's `iv6` is a **iv7** (no sixth in the data) |
| `modal_jazz_vamp` | `Jazz: Modal Vamp (i-iv)` | `1/0/min7;4/0/7` → `D:min7,G:7` — the label's lowercase `iv` resolves to a **dominant** chord |
| `dorian_aeolian_i_bvii_iv` | `Modal: Dorian-Aeolian (i-bVII-IV)` | `1/0/nil;7/0/nil;4/0/nil` → `A:minor,G:major,D:minor` — the label's `IV` is a **iv** (the data is Aeolian, not Dorian) |
| `canon_rock` | `Rock: V-i-VI-IV (Canon Rock)` | `5/0/major;1/0/nil;6/0/nil;4/0/nil` → `E:major,A:minor,F:major,D:minor` — the label's `IV` is a **iv** |
| `chromatic_walkdown_i_bvii_vi_bvii_i` | `Rock: Chromatic Walkdown (I-bVII-VI-bVII-I)` | `1/0/nil;7/-1/nil;6/0/nil;7/-1/nil;1/0/nil` → `A:major,G:major,F#:minor,G:major,A:major` — the label's `VI` is a **vi** |

Beyond those six, the measurement separates two further prose classes that are
worth knowing before deciding:

* **7 labels are summaries or names, not per-chord lists** — the numeral count
  does not match the degree count: `blues_12_bar` (`I-IV-V` over 12 degrees),
  `coltrane_changes`, `coltrane_sub_ii_v_i`, `minor_ii_v_i`, `bird_blues`,
  `jazz_blues_form`, `aaba_form`. Nothing to reconcile; they are not claims about
  each degree.
* **7 labels claim a mode while the data is a major/minor scale plus explicit
  qualities**: `dorian_vamp_i_iv` ("Modal: Dorian Vamp (i-IV)", scale `minor`,
  `D:min7,G:7`), `dorian_aeolian_i_bvii_iv`, `lydian_i_ii` ("Modal: Lydian
  (I-II)", scale `major`, `C:major,D:major`), `whole_tone`, `mixolydian_bvi_i_bvii_bvi_bvii`,
  `phrygian_vamp_i_bii_i`, `spanish_phrygian_i_bii_iii`. The explicit qualities in
  the data carry the intended sound, so the *chords* are right where the mode name
  suggests otherwise — the name is the part that misleads.

Of the 59 definitions, 46 carry a numeral list that is fully consistent with
their data (59 − 6 − 7).

### What our current native code answers, run for real

`progression.rs` does not exist (`C18` is gated on this decision), so the
prototype resolves the fixture's own degrees through the committed `C15` triads
and the documented rule:

```sh
diff native-progressions.txt live-progression-resolved.txt   # 1062/1062 identical
```

The native answer is therefore the **data** — `C:min7` for `rhythm_changes_b` —
and prose is a copied field, exactly as in the baseline. Nothing about the
disagreement is forced by the music engine; it is a question of which frozen field
the app shows and which one it applies.

### Observable, and what it would change

* **User-visible?** Yes. The picker shows `name` and then applies the data: the
  user reads "II7" and gets a `ii7` chip, reads `i-iv` and gets a dominant chord,
  reads "Dorian Vamp" and gets an Aeolian vamp. Nothing is wrong with the engine;
  the label misdescribes it.
* **Frozen records:** porting verbatim (label and data as frozen) changes none of
  the 1180 `progressions.jsonl` records. Correcting a **label** changes that
  progression's `progression` and `progression_label` records (2 per fix).
  Correcting the **degrees** changes the `progression` record plus every resolved
  case of that progression — 18 per progression in the fixture (the twelve sharp
  tonics plus six `example_key` cases including the stored `Bb`) — and changes
  what the app inserts, i.e. behaviour, not display.

### Options

| # | Option | Consequence | Frozen records |
|---|---|---|---|
| A | Port the catalog verbatim: keep the label and the degrees exactly as frozen; fix nothing | `C18` is a faithful port and every record stays pinned. The app keeps showing prose that disagrees with the chords it applies — a known, documented cosmetic defect | 1180/1180 stay identical |
| B | Fix the **labels** to match the data (`II7` → `ii7`, `iv6` → `iv7`, `i-iv` → `i-IV7`, `IV` → `iv`, drop or rename the misleading mode names) while keeping the degrees | Display-only: what the user hears never changes, and the picker stops lying. Each fix is a reviewed deviation with its own records; the mode-named progressions (`dorian_vamp_i_iv`, `lydian_i_ii`, …) need a wording decision, not just a case fix | 2 records per progression fixed (≤14 for the six plus the mode names) |
| C | Fix the **degrees** to match the prose (`rhythm_changes_b` degree 2 `min7` → `7`, mirroring `rhythm_changes_a`'s `1/0/7`) | Changes the chords the app inserts; the "prose" is then right and the sound changes. This is a product change to the baseline's music, invisible in tests until the degrees are re-frozen | 1 `progression` + up to 18 `progression_chords` records per progression, plus a release note |
| D | Keep the frozen data and let the Android catalog render its own labels | Forbidden by the plan's "no duplicated mobile catalog" rule; the label would then exist twice and drift | 0 (but it splits the catalog) |

**Recommendation: A now, B later as its own approved deviation.** `C18` should
port both fields verbatim — the label and the degrees are the baseline's own
frozen text, and a silent correction would be exactly the "quiet edit in ported
data" the contract forbids. If the user wants the prose fixed, B is the right
shape: it is display-only, it cannot change what the app plays, and it is the only
option that removes the user-visible lie without re-deciding the music. C changes
the behaviour of 6+ progressions and is a product decision, not a catalog
correction; D contradicts the plan.

## Not verified

* The row order of D06 was measured in this runtime only (Elixir 1.19.5 / OTP 27).
  Its stated form (term order of a small map's keys) is an Erlang implementation
  property; a future OTP could change it for maps above 32 entries, which never
  happens here (at most 3 groups).
* `Contract.D05`'s triad-base map was transcribed into the scratch prototype from
  the pinned source; the prototype was verified by reproducing all 17 frozen keys
  records, not by a unit test of its own.
* No Android or FFI surface was touched or run: all three decisions are measured
  at the domain and pinned-Elixir level. Whether the *current* Android client
  shows the flat-root asymmetry or the raw progression labels was not exercised
  (no build, no device).
* `suggest_keys` and `group_key_suggestions` were driven with the fixture's own
  inputs; the LiveView event path around them (assigns, async results) was not.
