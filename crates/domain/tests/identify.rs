//! Task `C12`: full ordered identification against the frozen oracle.
//!
//! Every record of the fifteen `fixtures/oracle/identify-*.jsonl` shards is
//! consumed here — 15,693 records, all `operation: analyze_notes`, produced by
//! `Fretboard.Music.analyze_notes/1` (11,913 records) and `analyze_notes/2`
//! (3,780 records, with a `bass` note). The output of a record is an **ordered
//! list** of interpretations: the order is part of the contract, so this file
//! compares the full ordered list and never a set or a first match.
//!
//! ## The tie exception (`Contract.D03`)
//!
//! The baseline sorts candidates by its own keys — `(0,0,0,rootIndex,0)` for an
//! exact match, `(1,missingCount,missingPrioritySum,rootIndex,formulaLen)` for
//! an incomplete one and `(2,0,-coverage,rootIndex,formulaLen)` for a partial
//! one — with an Elixir `Enum.sort_by`, which is stable. Candidates that compare
//! equal on those keys therefore inherit `Map.to_list(@formulas)`'s enumeration
//! order, and that order follows the VM's atom table rather than the source text
//! (the measurement in `docs/decisions.md`). The approved native rule
//! (`Contract.D03`) keeps the keys above unchanged and breaks a tie by the
//! frozen catalog identifier order — the position of the quality in
//! `QualityId::ALL` — and then by the frozen pitch-class order. `QUALITY_IDS` in
//! `crates/domain/src/types.rs` is that catalog order, and the sort key already
//! carries `rootIndex`, so in practice the second half of the projection never
//! decides anything; it is stated for completeness.
//!
//! This file keeps that deviation visible instead of absorbing it:
//!
//! * a record whose ordered output equals the frozen one passes outright;
//! * a record that differs is accepted **only** if the two sides have the same
//!   sequence of sort keys and agree as sets *inside every run of equal keys* —
//!   exactly a permutation inside the groups the baseline's own keys cannot
//!   distinguish. Such a record is counted as tie-affected;
//! * anything else fails, with the `case_id` in the message.
//!
//! The counts are pinned as constants below (`ORDERED_IDENTICAL` and
//! `TIE_AFFECTED`) and printed by the completeness test, so a change in the
//! deviation is a visible test failure rather than a silent absorption. The
//! affected cases are listed in the parity report; the examples in this file's
//! module are `analyze_notes/pcs:C-C#-D#` (the tied `min7b13`/`7b13` pair) and
//! `analyze_notes/pcs:C-D-E`.
//!
//! ## What is pinned where
//!
//! No expectation here is retyped from the plan or from the implementation: the
//! values come out of the fixture record that carries them, and every helper
//! reads the record by `case_id` so a failure names the case. The plan's named
//! behaviours are pinned by name against concrete case ids: exact-first and the
//! incomplete priorities (`analyze_notes/pcs:C-E-G`), the actual partial
//! predicate (`analyze_notes/pcs:C-C#-D-D#-E`), the bass/slash/inversion
//! annotation (`analyze_notes/full-formula:C:11/bass=C`) and the separated
//! note/interval orders of `Contract.D01`
//! (`analyze_notes/full-formula:C:11/bass=C`).

// Test target: the same relaxations as the other contract tests. Arithmetic on
// small integers, direct indexing into fixture values and panicking assertions
// are the idiom in tests, and a panic is a failure report.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::print_stdout,
    clippy::unwrap_used
)]

mod common;

use std::sync::OnceLock;

use common::oracle_records;
use fretboard_core::{
    Interpretation, MatchSortKey, PitchClass, QualityId, identify_notes, identify_notes_with_bass,
    match_sort_key,
};
use serde_json::{Map, Value};

/// The fifteen frozen shards of the identify fixture.
const SHARDS: usize = 15;

/// The record count the fifteen shards must carry together
/// (`fixtures/oracle/manifest.json`).
const RECORDS: usize = 15_693;

/// Frozen records whose ordered output the native order reproduces exactly.
const ORDERED_IDENTICAL: usize = 5_143;

/// Frozen records that differ from the native order only by a permutation
/// inside a group of equal baseline sort keys — the recorded `Contract.D03`
/// deviation.
const TIE_AFFECTED: usize = 10_550;

/// Every record of the fifteen shards, in shard then file order.
fn records() -> &'static Vec<Value> {
    static RECORDS_BY_SHARD: OnceLock<Vec<Value>> = OnceLock::new();
    RECORDS_BY_SHARD.get_or_init(|| {
        let mut all = Vec::new();
        for number in 1..=SHARDS {
            let name = format!("fixtures/oracle/identify-{number:02}.jsonl");
            all.extend(oracle_records(&name));
        }
        all
    })
}

/// One record by `case_id`, or a loud failure naming the missing case.
fn fixture_case(case_id: &str) -> Value {
    records()
        .iter()
        .find(|record| record["case_id"].as_str() == Some(case_id))
        .cloned()
        .unwrap_or_else(|| panic!("the identify fixture has no case {case_id}"))
}

/// The identifier of a fixture record.
fn case_id(record: &Value) -> String {
    record["case_id"]
        .as_str()
        .expect("every record has a case id")
        .to_owned()
}

/// The input note names of a record, as the domain's pitch classes. Every input
/// name in the fixture is one of the twelve sharp names, which is exactly the
/// set `PitchClass::from_str` accepts (`CORE-D06`).
fn input_notes(record: &Value) -> Vec<PitchClass> {
    record["input"]["notes"]
        .as_array()
        .unwrap_or_else(|| panic!("{} has no input notes", case_id(record)))
        .iter()
        .map(|note| {
            note.as_str()
                .unwrap_or_else(|| panic!("{} has a non-string note", case_id(record)))
                .parse()
                .unwrap_or_else(|error| panic!("{note} is not a pitch class: {error:?}"))
        })
        .collect()
}

/// The optional bass note of a record.
fn input_bass(record: &Value) -> Option<PitchClass> {
    record["input"]["bass"].as_str().map(|bass| {
        bass.parse()
            .unwrap_or_else(|error| panic!("{bass} is not a pitch class: {error:?}"))
    })
}

/// The frozen interpretations of a record, in their contract order.
fn frozen_entries(record: &Value) -> Vec<Value> {
    record["output"]["interpretations"]
        .as_array()
        .unwrap_or_else(|| panic!("{} has no interpretations", case_id(record)))
        .clone()
}

/// The native identification of a record's input.
fn our_entries(record: &Value) -> Vec<Interpretation> {
    let notes = input_notes(record);
    input_bass(record).map_or_else(
        || identify_notes(&notes),
        |bass| identify_notes_with_bass(&notes, bass),
    )
}

/// One native interpretation as the JSON object the fixture spells.
///
/// The fixture carries `bass`, `inversion` and `slash_label` only for the
/// two-argument operation, so they are added only when the record has a bass:
/// the comparison is against the record's own field set, not against a superset.
fn our_entry_json(entry: &Interpretation, with_bass: bool) -> Value {
    let mut object = Map::new();
    object.insert(
        "root".to_owned(),
        Value::String(entry.root.name().to_owned()),
    );
    object.insert(
        "quality".to_owned(),
        Value::String(entry.quality.as_str().to_owned()),
    );
    object.insert("exact".to_owned(), Value::Bool(entry.exact));
    object.insert("incomplete".to_owned(), Value::Bool(entry.incomplete));
    object.insert("notes".to_owned(), json_value(&entry.notes));
    object.insert("intervals".to_owned(), json_value(&entry.intervals));
    object.insert(
        "missing_intervals".to_owned(),
        json_value(&entry.missing_intervals),
    );
    if with_bass {
        object.insert("bass".to_owned(), json_value(&entry.bass));
        object.insert("inversion".to_owned(), json_value(&entry.inversion));
        object.insert("slash_label".to_owned(), json_value(&entry.slash_label));
    }
    Value::Object(object)
}

/// A serialisable value as JSON, or a loud failure. The values here are small
/// domain values (`PitchClass`, `&str`, `Option<u8>`) whose encoding cannot
/// fail; a failure still stops the test instead of hiding.
fn json_value<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or_else(|error| panic!("cannot encode {error:?}"))
}

/// The canonical text of one JSON value: the same encoding on both sides, so a
/// comparison never depends on a map's iteration order.
fn canonical(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|error| panic!("cannot encode {error:?}"))
}

/// The baseline sort key of one frozen entry, read from its root and quality.
fn frozen_key(entry: &Value, notes: &[PitchClass]) -> MatchSortKey {
    let root: PitchClass = entry["root"]
        .as_str()
        .expect("a frozen entry names its root")
        .parse()
        .expect("a frozen root is a pitch class");
    let quality: QualityId = entry["quality"]
        .as_str()
        .expect("a frozen entry names its quality")
        .parse()
        .expect("a frozen quality is a catalog identifier");
    match_sort_key(notes, root, quality).unwrap_or_else(|| {
        panic!(
            "the baseline sort key of {} ({} {quality}) is not a match",
            canonical(entry),
            root.name()
        )
    })
}

/// The baseline sort key of one native interpretation.
fn our_key(entry: &Interpretation, notes: &[PitchClass]) -> MatchSortKey {
    match_sort_key(notes, entry.root, entry.quality).unwrap_or_else(|| {
        panic!(
            "the baseline sort key of {} {:?} is not a match",
            entry.root.name(),
            entry.quality
        )
    })
}

/// How one frozen record compares with the native order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The ordered output equals the frozen one.
    Identical,
    /// The output differs from the frozen one only inside tie groups.
    TieOnly,
}

/// Compare one record against the frozen ordered output.
///
/// Panics, naming the case, on any difference that is not a permutation inside
/// a group of equal baseline sort keys.
fn check(record: &Value) -> Outcome {
    let case = case_id(record);
    let notes = input_notes(record);
    let with_bass = input_bass(record).is_some();
    let frozen = frozen_entries(record);
    let ours = our_entries(record);

    let frozen_text: Vec<String> = frozen.iter().map(canonical).collect();
    let our_text: Vec<String> = ours
        .iter()
        .map(|entry| canonical(&our_entry_json(entry, with_bass)))
        .collect();

    if our_text == frozen_text {
        return Outcome::Identical;
    }

    assert_eq!(
        our_text.len(),
        frozen_text.len(),
        "{case}: a different number of interpretations"
    );

    let frozen_keys: Vec<MatchSortKey> = frozen
        .iter()
        .map(|entry| frozen_key(entry, &notes))
        .collect();
    let our_keys: Vec<MatchSortKey> = ours.iter().map(|entry| our_key(entry, &notes)).collect();
    assert_eq!(
        our_keys, frozen_keys,
        "{case}: the sort-key sequence differs outside a tie group"
    );

    let mut index = 0;
    while index < frozen_keys.len() {
        let mut end = index;
        while end < frozen_keys.len() && frozen_keys[end] == frozen_keys[index] {
            end += 1;
        }
        let mut expected = frozen_text[index..end].to_vec();
        let mut actual = our_text[index..end].to_vec();
        expected.sort();
        actual.sort();
        assert_eq!(
            actual, expected,
            "{case}: the tie group {index}..{end} differs as a set"
        );
        index = end;
    }

    Outcome::TieOnly
}

/// The position of a quality in the frozen catalog identifier order.
fn quality_position(quality: QualityId) -> usize {
    QualityId::ALL
        .iter()
        .position(|known| *known == quality)
        .unwrap_or_else(|| panic!("{quality} is not in the frozen catalog"))
}

/// The completeness claim of `C12`: every frozen record, in order, with the
/// approved tie deviation counted and visible.
#[test]
fn every_frozen_record_matches_in_order_except_inside_a_tie_group() {
    let all = records();
    assert_eq!(
        all.len(),
        RECORDS,
        "the fifteen shards must carry {RECORDS} records"
    );

    let mut identical = 0usize;
    let mut tie_affected = 0usize;
    let mut examples: Vec<String> = Vec::new();
    for record in all {
        match check(record) {
            Outcome::Identical => identical += 1,
            Outcome::TieOnly => {
                tie_affected += 1;
                if examples.len() < 5 {
                    examples.push(case_id(record));
                }
            }
        }
    }

    println!(
        "identify: {RECORDS} records, {identical} ordered-identical, \
         {tie_affected} tie-affected (e.g. {examples:?})"
    );

    assert_eq!(
        identical, ORDERED_IDENTICAL,
        "the number of records reproduced in full order changed"
    );
    assert_eq!(
        tie_affected, TIE_AFFECTED,
        "the number of tie-affected records changed"
    );
    assert_eq!(identical + tie_affected, RECORDS);
}

/// Exact first, then the incomplete matches ordered by missing count, missing
/// priority and root — the case `analyze_notes/pcs:C-E-G` is a triad input, so
/// it opens with the exact `C major` and continues with incomplete qualities.
#[test]
fn exact_matches_come_first_then_the_incomplete_priorities() {
    let record = fixture_case("analyze_notes/pcs:C-E-G");
    // `check` panics on any difference outside a tie group, so reaching the
    // assertions below means the record is compliant.
    check(&record);
    let ours = our_entries(&record);
    let first = ours.first().expect("C-E-G has interpretations");
    assert!(first.exact, "the first interpretation of C-E-G is exact");
    assert!(!first.incomplete);
    assert!(
        ours.iter().filter(|entry| entry.exact).count() == 1,
        "C-E-G has exactly one exact interpretation"
    );
    assert!(
        ours.iter()
            .all(|entry| !entry.exact || entry.missing_intervals.is_empty()),
        "an exact interpretation names no missing interval"
    );
    // Every later entry is not exact, so exact really does come first.
    assert!(
        ours.iter().skip(1).all(|entry| !entry.exact),
        "every interpretation after the first is not exact"
    );

    // The incomplete ordering: non-decreasing missing count, then non-decreasing
    // priority sum, then non-decreasing root — the baseline's keys, read through
    // the same projection the fixture is compared with.
    let notes = input_notes(&record);
    let keys: Vec<MatchSortKey> = ours.iter().map(|entry| our_key(entry, &notes)).collect();
    for pair in keys.windows(2) {
        assert!(
            pair[0] <= pair[1],
            "the sort keys of analyze_notes/pcs:C-E-G are not non-decreasing"
        );
    }
    // The second entry misses exactly one note, against the two-note misses that
    // follow it: the missing count orders before the priority sum.
    let second = ours.get(1).expect("C-E-G has a second interpretation");
    assert_eq!(second.missing_intervals.len(), 1);
    assert!(
        second.incomplete,
        "the second interpretation of C-E-G is incomplete"
    );
}

/// The actual partial predicate: `coverage >= 3` and `formulaLen < inputSize`,
/// with **no** requirement that the formula be contained in the input, even
/// though the baseline's own wording calls a partial a "strict subset"
/// (`Contract.D02`, still open). The case is
/// `analyze_notes/pcs:C-C#-D-D#-E`: the `add9` interpretation rooted on `C` has
/// the formula `[0,2,4,7]` and the input lacks semitone 7, so the formula is not
/// a subset, yet the entry is neither exact nor incomplete and reports no
/// missing interval.
#[test]
fn the_actual_partial_predicate_is_broader_than_its_documented_wording() {
    let record = fixture_case("analyze_notes/pcs:C-C#-D-D#-E");
    // `check` panics on any difference outside a tie group.
    check(&record);
    let ours = our_entries(&record);
    assert!(
        ours.iter()
            .filter(|entry| !entry.exact && !entry.incomplete)
            .count()
            >= 2,
        "analyze_notes/pcs:C-C#-D-D#-E has several partial matches"
    );
    let add9 = ours
        .iter()
        .find(|entry| entry.root.name() == "C" && entry.quality.as_str() == "add9")
        .expect("the case carries an add9 match on C");
    assert!(!add9.exact && !add9.incomplete);
    assert!(
        add9.missing_intervals.is_empty(),
        "a partial match reports no missing interval"
    );
    // The formula is not contained: semitone 7 is absent from the input seen
    // from C, which is exactly what "strict subset" would have excluded.
    assert!(
        !input_notes(&record).iter().any(|note| u8::from(*note) == 7),
        "the input of the case lacks the fifth above C"
    );
}

/// The bass annotation: an inversion of `0` or a non-chord bass keeps the plain
/// label, every other chord tone in the bass appends `/{bass}`. The case
/// `analyze_notes/full-formula:C:11/bass=C` carries both halves.
#[test]
fn the_bass_annotation_is_the_frozen_inversion_and_slash_label() {
    let record = fixture_case("analyze_notes/full-formula:C:11/bass=C");
    // `check` panics on any difference outside a tie group.
    check(&record);
    let ours = our_entries(&record);
    assert!(
        ours.iter().all(|entry| entry.bass.is_some()),
        "every interpretation of a bass-aware call carries its bass"
    );
    let root_position = ours
        .iter()
        .find(|entry| entry.inversion == Some(0))
        .expect("the case holds a root-position interpretation");
    let root_bass = root_position.bass.expect("a bass");
    assert!(
        !root_position
            .slash_label
            .as_deref()
            .expect("a slash label")
            .ends_with(&format!("/{}", root_bass.name())),
        "inversion 0 never appends the bass"
    );
    let inverted = ours
        .iter()
        .filter(|entry| entry.inversion.is_some_and(|value| value > 0))
        .collect::<Vec<&Interpretation>>();
    assert!(!inverted.is_empty(), "the case holds an inverted match");
    for entry in inverted {
        let label = entry.slash_label.as_deref().expect("a slash label");
        assert!(
            label.ends_with(&format!("/{}", entry.bass.expect("a bass").name())),
            "a non-zero inversion appends the bass: {label}"
        );
    }
    // A bass that is not a chord tone the baseline maps is not appended either;
    // the case reaches those through its `inversion: null` entries. The check is
    // on the `/{bass}` suffix, not on any slash at all, because a quality label
    // may itself carry a slash (`m6/9`).
    for entry in ours.iter().filter(|entry| entry.inversion.is_none()) {
        let label = entry.slash_label.as_deref().expect("a slash label");
        let suffix = format!("/{}", entry.bass.expect("a bass").name());
        assert!(
            !label.ends_with(&suffix),
            "a non-chord bass never appends a slash: {label}"
        );
    }
}

/// `Contract.D01`: the interpretation carries the notes in formula order and the
/// interval labels in their own order, and deliberately does **not** zip them —
/// a consumer that zips reproduces the baseline's raw pairs. The case
/// `analyze_notes/full-formula:C:11/bass=C` carries a `9` match on `C`, whose
/// formula order (`C D E G A#`) is not its label order (root, 3rd, 5th, 7th,
/// 9th), so the two orders are visibly different.
#[test]
fn the_note_and_interval_orders_stay_separate() {
    let record = fixture_case("analyze_notes/full-formula:C:11/bass=C");
    // `check` panics on any difference outside a tie group.
    check(&record);
    let ours = our_entries(&record);
    let ninth = ours
        .iter()
        .find(|entry| entry.root.name() == "C" && entry.quality.as_str() == "9")
        .expect("the case carries a 9 match on C");
    assert_eq!(ninth.notes.len(), ninth.intervals.len());
    assert_eq!(
        ninth
            .notes
            .iter()
            .map(|note| note.name())
            .collect::<Vec<&str>>(),
        vec!["C", "D", "E", "G", "A#"],
        "the notes stay in formula order"
    );
    assert_eq!(
        ninth.intervals,
        vec!["Root", "Major 3rd", "Perfect 5th", "Minor 7th", "Major 9th"],
        "the labels stay in their own chord-member order"
    );
    // The two orders really differ: the second formula note is not the note the
    // second label names.
    assert_ne!(ninth.notes[1].name(), "E");
    assert_ne!(ninth.intervals[1], "Major 9th");
}

/// The tie-break projection itself, on the one-line reading of `Contract.D03`:
/// both tied entries share a baseline sort key, the native order between them is
/// the frozen catalog identifier order, and the frozen record has them the other
/// way round — which is why the record is counted tie-affected. The pair is
/// `min7b13`/`7b13` on root `F` in `analyze_notes/pcs:C-C#-D#`.
#[test]
fn the_tie_break_is_the_catalog_identifier_order() {
    let record = fixture_case("analyze_notes/pcs:C-C#-D#");
    assert_eq!(
        check(&record),
        Outcome::TieOnly,
        "the frozen order and the catalog order disagree on this pair"
    );
    let notes = input_notes(&record);
    let ours = our_entries(&record);
    let tied: Vec<&Interpretation> = ours
        .iter()
        .filter(|entry| entry.root.name() == "F")
        .collect();
    assert_eq!(tied.len(), 2, "the F rooted pair is the tie group");
    assert_eq!(
        our_key(tied[0], &notes),
        our_key(tied[1], &notes),
        "the two entries share one baseline sort key"
    );
    assert!(
        quality_position(tied[0].quality) < quality_position(tied[1].quality),
        "the native order between tied entries is the catalog identifier order"
    );

    // The frozen record orders the same two entries the other way round.
    let frozen = frozen_entries(&record);
    let frozen_pair: Vec<String> = frozen
        .iter()
        .filter(|entry| entry["root"].as_str() == Some("F"))
        .map(|entry| entry["quality"].as_str().expect("a quality").to_owned())
        .collect();
    assert_eq!(
        frozen_pair,
        vec![
            tied[1].quality.as_str().to_owned(),
            tied[0].quality.as_str().to_owned()
        ],
        "the frozen record uses the baseline's map order, not the catalog order"
    );
}

/// The two-note and one-note inputs return nothing, exactly as the baseline's
/// `length(notes) < 3` guard does; the fixture's smaller inputs carry this.
#[test]
fn fewer_than_three_pitch_classes_identify_nothing() {
    let mut checked = 0usize;
    for record in records() {
        let unique = {
            let notes = input_notes(record);
            let mut seen: Vec<PitchClass> = Vec::new();
            for note in notes {
                if !seen.contains(&note) {
                    seen.push(note);
                }
            }
            seen
        };
        if unique.len() < 3 {
            checked += 1;
            assert!(
                our_entries(record).is_empty(),
                "{} identifies something from fewer than three pitch classes",
                case_id(record)
            );
            assert_eq!(check(record), Outcome::Identical);
        }
    }
    assert!(checked > 0, "the fixture carries small inputs to guard");
}
