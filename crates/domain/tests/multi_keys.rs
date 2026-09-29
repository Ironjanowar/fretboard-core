//! Task `C17`: the greedy multi-key suggestion against the frozen oracle.
//!
//! Every record of `fixtures/oracle/multi-keys.jsonl` (15 records) is consumed
//! here, each failure naming its `case_id`. The records come from the pinned
//! `Fretboard.Music.Scale.suggest_multi_keys/1`.
//!
//! ## The ported rule
//!
//! Each input chord is normalized to a sharp root, all 168 candidate keys are
//! built, and the greedy selection runs at most three times: at each step it
//! takes the candidate covering the most **still-uncovered** chords, breaking a
//! tie by the candidate's diatonic score over *all* the chords it contains and
//! then by the frozen scale priority. It stops on the third pick, when nothing
//! remains uncovered, or when no candidate covers a remaining chord. The
//! selection's own assignment (`newly_covered`) is exclusive and decides two
//! things only: the all-singleton rule and the unmatched chords. What a group
//! **displays** is the key's *full* membership — every input occurrence whose
//! notes fit that key — so one chord can appear in more than one group.
//!
//! Two behaviours of that rule have no record in the frozen fixture and are
//! therefore pinned from the pinned Elixir itself:
//!
//! * the all-singleton answer (every selected key covers exactly one chord, so
//!   the operation answers nothing at all);
//! * the unmatched group (a chord no candidate key contains is returned as a
//!   final group with no key).
//!
//! Both were measured live against the pinned source
//! (`/workspace/oracle-src-pin`, driver `multi-keys.exs` of the task's live
//! cross-check, extras file `extra-multi-keys.jsonl`), which reproduces all 15
//! frozen records byte for byte; the expectations below are that run's output,
//! not the implementation's.
//!
//! ## What the rest pins
//!
//! The plan's named case (`Dmin,Gmaj,Emaj,Fmaj` must keep `Dmin/Gmaj/Fmaj` in C
//! Major and `Dmin/Emaj/Fmaj` in A Harmonic Minor, in the oracle's order), full
//! overlap, duplicate occurrences as distinct indices, the three-group cap, the
//! raw under-three rule and the page gate (no single key *and* three active
//! chords), and the two tie-breaks — the tonic enumeration order and the frozen
//! scale priority.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::print_stdout,
    clippy::unwrap_used,
    variant_size_differences
)]

mod common;

use std::collections::BTreeSet;

use common::oracle_records;
use fretboard_core::{
    ChordMode, ChordSpec, MultiKeyGroup, PitchClass, ScaleId, chord_details, diatonic_chords,
    note_index, page_multi_key_suggestions, scale_notes, suggest_keys, suggest_multi_keys,
    triad_base,
};
use serde_json::Value;

/// The frozen fixture and the record count it must carry
/// (`fixtures/oracle/manifest.json`).
const FIXTURE: &str = "fixtures/oracle/multi-keys.jsonl";
const RECORDS: usize = 15;

/// The raw operation answers nothing below three chords; those are the first
/// three records of the fixture.
const UNDER_THREE_RECORDS: usize = 3;

/// The selection is capped at three groups (`max-three-groups`, seven chords).
const MAX_GROUPS: usize = 3;

/// The plan's named case, and the two keys its answer must carry.
const OVERLAP_CASE: &str = "suggest_multi_keys/dmin-gmaj-emaj-fmaj";

/// The all-singleton input: three major triads no single key covers twice, so
/// every selected key covers exactly one of them.
const ALL_SINGLETON_INPUT: [(&str, &str); 3] = [("C", "major"), ("E", "major"), ("G#", "major")];

/// The unmatched input: a perfect `C major` and a perfect `G major` share a key,
/// and `C7#9` is contained by no candidate key at all.
const UNMATCHED_INPUT: [(&str, &str); 3] = [("C", "major"), ("G", "major"), ("C", "7#9")];

/// Every record of the fixture, in file order.
fn records() -> Vec<Value> {
    let records = oracle_records(FIXTURE);
    assert_eq!(
        records.len(),
        RECORDS,
        "the frozen fixture carries {RECORDS} cases"
    );
    records
}

/// The identifier of a record.
fn case_id(record: &Value) -> String {
    record["case_id"]
        .as_str()
        .expect("every record has a case id")
        .to_owned()
}

/// One record by case id, or a loud failure naming the missing case.
fn fixture_case(case_id_wanted: &str) -> Value {
    records()
        .into_iter()
        .find(|record| record["case_id"].as_str() == Some(case_id_wanted))
        .unwrap_or_else(|| panic!("the multi-keys fixture has no case {case_id_wanted}"))
}

/// One chord of a frozen list, as a native chord.
fn native_chord(chord: &Value) -> ChordSpec {
    ChordSpec {
        root: note_index(chord["root"].as_str().expect("a chord root")).expect("a note name"),
        quality: chord["quality"]
            .as_str()
            .expect("a chord quality")
            .parse()
            .expect("a catalog quality"),
    }
}

/// A chord from a `(root, quality)` pair.
fn chord(root: &str, quality: &str) -> ChordSpec {
    ChordSpec {
        root: note_index(root).expect("a note name"),
        quality: quality.parse().expect("a catalog quality"),
    }
}

/// The chords of a record's input.
fn input_chords(record: &Value) -> Vec<ChordSpec> {
    record["input"]["chords"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no chords", case_id(record)))
        .iter()
        .map(native_chord)
        .collect()
}

/// One chord as `root:quality`.
fn render_chord(chord: &ChordSpec) -> String {
    format!("{}:{}", chord.root.name(), chord.quality.as_str())
}

/// One native group in the canonical text form.
fn render_group(group: &MultiKeyGroup) -> String {
    let key = group.key.as_ref().map_or_else(
        || "nil".to_owned(),
        |key| {
            format!(
                "{}:{}:{}:{}",
                key.tonic.name(),
                key.scale_type.as_str(),
                key.score,
                key.total
            )
        },
    );
    format!(
        "[{key}|{}]",
        group
            .chords
            .iter()
            .map(render_chord)
            .collect::<Vec<String>>()
            .join(",")
    )
}

/// One frozen group in the same form.
fn render_frozen_group(group: &Value) -> String {
    let key = if group["key"].is_null() {
        "nil".to_owned()
    } else {
        let key = &group["key"];
        format!(
            "{}:{}:{}:{}",
            key["tonic"].as_str().expect("a tonic"),
            key["scale_type"].as_str().expect("a scale type"),
            key["score"].as_u64().expect("a score"),
            key["total"].as_u64().expect("a total")
        )
    };
    format!(
        "[{key}|{}]",
        group["chords"]
            .as_array()
            .expect("a group carries its chords")
            .iter()
            .map(|chord| format!(
                "{}:{}",
                chord["root"].as_str().expect("a root"),
                chord["quality"].as_str().expect("a quality")
            ))
            .collect::<Vec<String>>()
            .join(",")
    )
}

/// The native groups of a record, in the canonical text form.
fn our_groups(record: &Value) -> Vec<String> {
    suggest_multi_keys(&input_chords(record))
        .iter()
        .map(render_group)
        .collect()
}

/// The frozen groups of a record, in the same form.
fn frozen_groups(record: &Value) -> Vec<String> {
    record["output"]["groups"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no groups", case_id(record)))
        .iter()
        .map(render_frozen_group)
        .collect()
}

/// The pitch classes one chord names.
fn chord_notes(chord: &ChordSpec) -> Vec<PitchClass> {
    chord_details(chord).expect("a catalog chord").notes
}

/// The note set of a key.
fn key_notes(tonic: PitchClass, scale: ScaleId) -> BTreeSet<PitchClass> {
    scale_notes(tonic, scale).into_iter().collect()
}

/// Whether a key contains every note of a chord.
fn contains(tonic: PitchClass, scale: ScaleId, chord: &ChordSpec) -> bool {
    let notes = key_notes(tonic, scale);
    chord_notes(chord).iter().all(|note| notes.contains(note))
}

/// Whether a key's diatonic triads explain a chord's root and triad base.
fn explains(tonic: PitchClass, scale: ScaleId, chord: &ChordSpec) -> bool {
    diatonic_chords(tonic, scale, ChordMode::Triad)
        .iter()
        .any(|degree| degree.root == chord.root && degree.quality == triad_base(chord.quality))
}

#[test]
fn the_frozen_fixture_carries_every_multi_key_case() {
    let records = records();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for record in &records {
        assert_eq!(
            record["operation"].as_str(),
            Some("suggest_multi_keys"),
            "an unexpected operation"
        );
        assert!(
            ids.insert(case_id(record)),
            "duplicate case id in the fixture"
        );
    }
    assert_eq!(ids.len(), RECORDS, "the case ids are unique");

    // The first three records are the under-three rule; the frozen records never
    // carry an unmatched group (no candidate key is missing for their chords),
    // which is why that half is pinned from the pinned source below.
    let under_three = records
        .iter()
        .filter(|record| input_chords(record).len() < 3)
        .collect::<Vec<&Value>>();
    assert_eq!(under_three.len(), UNDER_THREE_RECORDS);
    for (index, record) in under_three.iter().enumerate() {
        assert_eq!(
            input_chords(record).len(),
            index,
            "{}: the under-three records carry 0, 1 and 2 chords",
            case_id(record)
        );
        assert!(
            frozen_groups(record).is_empty(),
            "{}: the raw gate answers nothing",
            case_id(record)
        );
    }
    let unmatched = records
        .iter()
        .filter(|record| {
            record["output"]["groups"]
                .as_array()
                .expect("groups")
                .iter()
                .any(|group| group["key"].is_null())
        })
        .count();
    assert_eq!(unmatched, 0, "no frozen record carries an unmatched group");
}

#[test]
fn every_frozen_multi_key_case_answers_its_record() {
    // No deviation is recorded for `C17`: every frozen record is reproduced,
    // group for group, key for key and chord for chord, in order.
    let mut identical = 0usize;
    for record in records() {
        assert_eq!(
            our_groups(&record),
            frozen_groups(&record),
            "{}: the groups differ",
            case_id(&record)
        );
        identical += 1;
    }
    assert_eq!(identical, RECORDS);
}

#[test]
fn the_overlapping_case_keeps_the_full_membership_in_the_oracle_order() {
    // The plan's named case: `Dmin, Gmaj, Emaj, Fmaj`. The first group is C
    // Major with `D min`, `G` and `F`; the second is A Harmonic Minor with
    // `D min`, `E` and `F`. Two groups, in that order, and `D min` appears in
    // both — the membership is every chord the key contains, not the chords the
    // greedy step assigned to it.
    let record = fixture_case(OVERLAP_CASE);
    assert_eq!(our_groups(&record), frozen_groups(&record));
    let groups = suggest_multi_keys(&input_chords(&record));
    assert_eq!(groups.len(), 2, "two groups");

    let first = groups.first().expect("two groups");
    let first_key = first.key.as_ref().expect("the first group has a key");
    assert_eq!(first_key.tonic.name(), "C");
    assert_eq!(first_key.scale_type.as_str(), "major");
    assert_eq!(
        first
            .chords
            .iter()
            .map(render_chord)
            .collect::<Vec<String>>(),
        ["D:minor", "G:major", "F:major"],
        "C Major keeps D min, G and F"
    );

    let second = groups.get(1).expect("two groups");
    let second_key = second.key.as_ref().expect("the second group has a key");
    assert_eq!(second_key.tonic.name(), "A");
    assert_eq!(second_key.scale_type.as_str(), "harmonic_minor");
    assert_eq!(
        second
            .chords
            .iter()
            .map(render_chord)
            .collect::<Vec<String>>(),
        ["D:minor", "E:major", "F:major"],
        "A Harmonic Minor keeps D min, E and F"
    );

    // The overlap itself: D min and F are in both groups.
    for shared in ["D:minor", "F:major"] {
        assert!(
            first.chords.iter().any(|c| render_chord(c) == shared)
                && second.chords.iter().any(|c| render_chord(c) == shared),
            "{shared} is listed by both keys"
        );
    }
    // The overlap itself: D min and F are in both groups, and C major's own
    // exclusive chord is only G major — the rest of its membership is shared.
    // That is what full membership means, and an exclusive display would hide it.
    let exclusive: Vec<String> = first
        .chords
        .iter()
        .filter(|chord| !second.chords.contains(*chord))
        .map(render_chord)
        .collect();
    assert_eq!(
        exclusive,
        ["G:major"],
        "C major's exclusive share is one chord, but its display is three"
    );
    assert_eq!(first.chords.len(), 3);
    assert_eq!(second.chords.len(), 3);
}

#[test]
fn a_group_is_every_input_chord_its_key_contains() {
    // The full-membership rule, checked on every frozen group: the chords a
    // group lists are exactly the input chords whose notes fit the group's key,
    // in input order.
    let mut groups_checked = 0usize;
    for record in records() {
        let case = case_id(&record);
        let chords = input_chords(&record);
        for group in suggest_multi_keys(&chords) {
            let Some(key) = group.key else {
                continue;
            };
            let expected: Vec<String> = chords
                .iter()
                .filter(|chord| contains(key.tonic, key.scale_type, chord))
                .map(render_chord)
                .collect();
            assert_eq!(
                group
                    .chords
                    .iter()
                    .map(render_chord)
                    .collect::<Vec<String>>(),
                expected,
                "{case}: the membership of {} {} is not every chord it contains",
                key.tonic.name(),
                key.scale_type.as_str()
            );
            assert_eq!(
                key.score,
                chords
                    .iter()
                    .filter(|chord| {
                        contains(key.tonic, key.scale_type, chord)
                            && explains(key.tonic, key.scale_type, chord)
                    })
                    .count(),
                "{case}: the score is not the key's diatonic matches"
            );
            assert_eq!(
                key.total,
                chords.len(),
                "{case}: the total is the whole input"
            );
            groups_checked += 1;
        }
    }
    assert!(
        groups_checked >= RECORDS,
        "every record contributes a group"
    );
}

#[test]
fn the_greedy_assignment_is_not_the_displayed_membership() {
    // The exclusive assignment is what the greedy step covered; the displayed
    // membership is the full one. Derived from the output itself: a later
    // group's *exclusive* share is its membership minus the memberships of the
    // groups before it, and its score counts all of its covered occurrences —
    // which is why the score is larger than that share.
    let mut checked = 0usize;
    for case in [OVERLAP_CASE, "suggest_multi_keys/seventh-chain"] {
        let record = fixture_case(case);
        let groups = suggest_multi_keys(&input_chords(&record));
        assert!(groups.len() > 1, "{case}: more than one group");
        let mut earlier: BTreeSet<String> = BTreeSet::new();
        if let Some(first) = groups.first() {
            earlier.extend(first.chords.iter().map(render_chord));
        }
        for group in groups.iter().skip(1) {
            let key = group.key.as_ref().expect("every fixture group has a key");
            let exclusive = group
                .chords
                .iter()
                .filter(|chord| !earlier.contains(&render_chord(chord)))
                .count();
            assert!(
                exclusive > 0,
                "{case}: {} {} adds no chord the earlier groups miss",
                key.tonic.name(),
                key.scale_type.as_str()
            );
            assert!(
                key.score > exclusive,
                "{case}: {} {} scores {exclusive} exclusive chords as {} — the score \
                 must count every covered occurrence",
                key.tonic.name(),
                key.scale_type.as_str(),
                key.score
            );
            earlier.extend(group.chords.iter().map(render_chord));
            checked += 1;
        }
    }
    assert_eq!(checked, 3, "the two records carry three later groups");
}

#[test]
fn duplicate_occurrences_are_distinct_indices() {
    // Repeats are occurrences, not a set: they are covered, listed and scored
    // once each, and a group lists every repetition it contains in input order.
    let record = fixture_case("suggest_multi_keys/duplicate-indices");
    assert_eq!(our_groups(&record), frozen_groups(&record));
    let groups = suggest_multi_keys(&input_chords(&record));
    assert_eq!(groups.len(), 1);
    let key = groups[0].key.as_ref().expect("a key");
    assert_eq!((key.score, key.total), (3, 3), "both C's count");
    assert_eq!(
        groups[0]
            .chords
            .iter()
            .map(render_chord)
            .collect::<Vec<String>>(),
        ["C:major", "C:major", "G:major"],
        "the repetition is listed twice"
    );

    for case in [
        "suggest_multi_keys/duplicate-heavy",
        "suggest_multi_keys/duplicate-indices-with-third",
    ] {
        let record = fixture_case(case);
        assert_eq!(our_groups(&record), frozen_groups(&record), "{case}");
        let groups = suggest_multi_keys(&input_chords(&record));
        let key = groups[0].key.as_ref().expect("a key");
        assert_eq!(
            key.score,
            input_chords(&record).len(),
            "{case}: every occurrence scores"
        );
        assert_eq!(
            groups[0].chords.len(),
            input_chords(&record).len(),
            "{case}: every occurrence is listed"
        );
    }
}

#[test]
fn the_selection_stops_after_three_groups() {
    // Seven chords, and the greedy selection stops at the cap even though more
    // keys could be named: the fixture's three groups, in its order.
    let record = fixture_case("suggest_multi_keys/max-three-groups");
    let chords = input_chords(&record);
    assert_eq!(chords.len(), 7, "seven input chords");
    let groups = suggest_multi_keys(&chords);
    assert_eq!(groups.len(), MAX_GROUPS, "the cap");
    assert_eq!(our_groups(&record), frozen_groups(&record));
    assert!(
        groups.iter().all(|group| group.key.is_some()),
        "every group carries a key"
    );

    // The three keys together do account for every chord here (the fixture's
    // own answer), so the cap is not hiding an unmatched tail in this record.
    let covered: BTreeSet<String> = groups
        .iter()
        .flat_map(|group| group.chords.iter().map(render_chord))
        .collect();
    let input: BTreeSet<String> = chords.iter().map(render_chord).collect();
    assert_eq!(covered, input, "the fixture's three groups cover the input");
}

#[test]
fn the_raw_gate_needs_three_chords_and_the_page_gate_also_needs_no_single_key() {
    // The raw operation's own under-three rule (the first three frozen records).
    for record in records()
        .iter()
        .filter(|record| input_chords(record).len() < 3)
    {
        let chords = input_chords(record);
        assert!(
            suggest_multi_keys(&chords).is_empty(),
            "{}: below three chords the raw operation answers nothing",
            case_id(record)
        );
        assert!(
            page_multi_key_suggestions(&[], &chords).is_empty(),
            "{}: and the page gate agrees",
            case_id(record)
        );
    }

    // The page's own gate is the pinned LiveView's `suggestions == [] and
    // length(chords) >= 3`: a single-key answer suppresses the multi-key panel
    // entirely, even when the multi-key operation would answer something.
    let overlapping = input_chords(&fixture_case(
        "suggest_multi_keys/overlapping-full-membership",
    ));
    assert!(
        !suggest_keys(&overlapping).is_empty(),
        "single keys exist here"
    );
    assert!(
        !suggest_multi_keys(&overlapping).is_empty(),
        "and so would a multi-key answer"
    );
    assert!(
        page_multi_key_suggestions(&suggest_keys(&overlapping), &overlapping).is_empty(),
        "but the single-key answer suppresses it"
    );

    // With no single key and three active chords, the page gate is the raw
    // operation.
    let singleton = input_chords(&fixture_case(
        "suggest_multi_keys/all-singleton-distinct-keys",
    ));
    assert_eq!(singleton.len(), 3);
    assert!(
        suggest_keys(&singleton).is_empty(),
        "no single key contains all three triads"
    );
    assert_eq!(
        page_multi_key_suggestions(&[], &singleton),
        suggest_multi_keys(&singleton),
        "the gate then answers the raw operation"
    );
    assert_eq!(
        page_multi_key_suggestions(&[], &singleton)[0].chords.len(),
        2,
        "which is a real answer"
    );
}

#[test]
fn an_all_singleton_selection_answers_no_groups() {
    // Measured against the pinned source (no frozen record carries this rule):
    // three major triads no key covers twice, so every selected key covers
    // exactly one chord and the operation answers nothing at all.
    let chords: Vec<ChordSpec> = ALL_SINGLETON_INPUT
        .iter()
        .map(|(root, quality)| chord(root, quality))
        .collect();
    assert_eq!(chords.len(), 3);
    assert!(
        suggest_multi_keys(&chords).is_empty(),
        "the all-singleton rule answers nothing"
    );

    // Not because the chords are uncoverable: a fourth chord that shares a key
    // with the first (the repeated C major) makes the selection non-singleton
    // and the answer appears, with the two singletons beside it. The pinned
    // run's answer for that input is `C major [C, C]`, `C# major [G#]` and
    // `E major [E]`.
    let mut with_repeat = chords;
    with_repeat.push(chord("C", "major"));
    let groups = suggest_multi_keys(&with_repeat);
    let rendered: Vec<String> = groups.iter().map(render_group).collect();
    assert_eq!(
        rendered,
        [
            "[C:major:2:4|C:major,C:major]",
            "[C#:major:1:4|G#:major]",
            "[E:major:1:4|E:major]",
        ],
        "the pinned answer for the repeated input"
    );
}

#[test]
fn a_chord_no_key_contains_becomes_its_own_group() {
    // Measured against the pinned source: `C` and `G` share a key, `C7#9` has a
    // diminished fifth and a raised ninth that no candidate key carries, and the
    // unmatched chord closes the answer as a group with no key. The frozen
    // records never reach this branch (no frozen input carries such a chord).
    let chords: Vec<ChordSpec> = UNMATCHED_INPUT
        .iter()
        .map(|(root, quality)| chord(root, quality))
        .collect();
    let groups = suggest_multi_keys(&chords);
    assert_eq!(
        groups.iter().map(render_group).collect::<Vec<String>>(),
        ["[C:major:2:3|C:major,G:major]", "[nil|C:7#9]"],
        "the pinned answer"
    );
    assert_eq!(groups.len(), 2);
    assert!(groups[0].key.is_some(), "the first group carries a key");
    assert_eq!(
        groups[1]
            .chords
            .iter()
            .map(render_chord)
            .collect::<Vec<String>>(),
        ["C:7#9"],
        "the unmatched chord is listed alone"
    );

    // Why it is unmatched: no candidate key contains it on its own, while the
    // other two chords are contained by many.
    let ninth = chord("C", "7#9");
    assert!(
        suggest_keys(&[ninth]).is_empty(),
        "no candidate key contains C7#9"
    );
    assert!(
        !suggest_keys(&[chords[0], chords[1]]).is_empty(),
        "the first two chords do share keys"
    );
}

#[test]
fn a_tie_prefers_the_first_candidate_in_the_frozen_order() {
    // Two tie-breaks of the ported rule, both read from the fixture.
    //
    // (1) Tonic order: `C`, `G` and `A minor` are contained by both C major and
    // G major with the same diatonic score, and the fixture answers C major.
    let record = fixture_case("suggest_multi_keys/tie-between-two-keys");
    let chords = input_chords(&record);
    let groups = suggest_multi_keys(&chords);
    assert_eq!(groups.len(), 1, "a single group");
    let key = groups[0].key.as_ref().expect("a key");
    assert_eq!(
        (key.tonic.name(), key.scale_type.as_str()),
        ("C", "major"),
        "the earlier tonic wins the tie"
    );
    let (c_tonic, g_tonic) = (
        note_index("C").expect("a note"),
        note_index("G").expect("a note"),
    );
    let major: ScaleId = "major".parse().expect("a catalog scale");
    assert_eq!(
        chords
            .iter()
            .filter(|chord| contains(c_tonic, major, chord) && explains(c_tonic, major, chord))
            .count(),
        chords
            .iter()
            .filter(|chord| contains(g_tonic, major, chord) && explains(g_tonic, major, chord))
            .count(),
        "the two candidates really do tie"
    );

    // (2) Scale priority: `D min`, `G` and `F` are covered and explained by C
    // major and C melodic minor alike, so the frozen scale order (major first)
    // keeps C major and the fixture answers it. C dorian and C mixolydian cover
    // one chord fewer, so they lose on coverage before any tie is looked at.
    let record = fixture_case(OVERLAP_CASE);
    let chords = input_chords(&record);
    let coverage_of = |scale: ScaleId| {
        chords
            .iter()
            .filter(|chord| contains(c_tonic, scale, chord))
            .count()
    };
    let score_of = |scale: ScaleId| {
        chords
            .iter()
            .filter(|chord| contains(c_tonic, scale, chord) && explains(c_tonic, scale, chord))
            .count()
    };
    let melodic: ScaleId = "melodic_minor".parse().expect("a catalog scale");
    assert_eq!(coverage_of(major), 3, "C major covers three of the four");
    assert_eq!(score_of(major), 3, "and explains all three");
    assert_eq!(
        (coverage_of(melodic), score_of(melodic)),
        (coverage_of(major), score_of(major)),
        "C melodic minor ties with C major on coverage and score"
    );
    for fewer in ["dorian", "mixolydian"] {
        let scale: ScaleId = fewer.parse().expect("a catalog scale");
        assert!(
            coverage_of(scale) < coverage_of(major),
            "C {fewer} covers fewer chords"
        );
    }
    let first = suggest_multi_keys(&chords)
        .first()
        .and_then(|group| group.key.clone())
        .expect("a first group with a key");
    assert_eq!(
        (first.tonic.name(), first.scale_type.as_str()),
        ("C", "major"),
        "the frozen scale priority decides the tie"
    );
}
