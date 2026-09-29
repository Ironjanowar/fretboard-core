//! Task `C13`: the absolute analyzer against the frozen oracle.
//!
//! Every record of `fixtures/oracle/analyzer.jsonl` (34 records) is consumed
//! here. The file carries two operations of the pinned baseline:
//!
//! * `analyze_pitches/1` (`Fretboard.Music.analyze_pitches/1`, 13 records) —
//!   the instrument-independent analyzer over absolute sounding pitches;
//! * `analyzer_state/2` (`Fretboard.Music.analyzer_state/2`, 21 records) — the
//!   same analyzer reached from marked positions, each sounding pitch being its
//!   string's open pitch plus the fret.
//!
//! The analyzer itself lives in `crates/domain/src/analyzer.rs` and calls
//! [`fretboard_core::identify_notes_with_bass`] for the chord recognition: the
//! note-set rules have exactly one home (`C12`), and this file exists to prove
//! the analyzer reproduces the baseline's own answer, in the baseline's order.
//!
//! ## The tab gate
//!
//! `Evaluation.analysis` is absent on the visualizer tab and computed on the
//! analyzer tab, from the *selection* alone — never from the stored visualizer
//! chords (`02-core-contract.md` section 8). The fretted analyzer maps each
//! marked position through the committed tuning, so a reentrant tuning and a
//! real pitch crossing both work; the piano uses the same analyzer with the
//! selected absolute pitches.
//!
//! ## The tie exception (`Contract.D03`)
//!
//! The chord list of a `chords` answer is the baseline's ordered identification.
//! Its order inside a group of equal baseline sort keys follows the Elixir
//! map's enumeration order, which is not reproducible; the approved native rule
//! breaks the tie by the frozen catalog identifier order. As in `tests/identify.rs`,
//! a record that differs is accepted **only** when the two sides agree as sets
//! inside every run of equal sort keys, and the affected records are counted and
//! printed so the deviation stays visible.

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

use common::oracle_records;
use fretboard_core::{
    Analysis, InstrumentId, InstrumentState, Interpretation, MatchSortKey, OpenPitch, PageState,
    PitchClass, Position, PresetName, SoundingPitch, Tab, TuningState, analyze_page,
    analyze_pitches, identify_notes_with_bass, match_sort_key,
};
use serde_json::{Map, Value, json};

/// The frozen fixture and the record count it must carry
/// (`fixtures/oracle/manifest.json`).
const FIXTURE: &str = "fixtures/oracle/analyzer.jsonl";
const RECORDS: usize = 34;

/// The number of records each operation of the fixture carries.
const PITCH_RECORDS: usize = 13;
const STATE_RECORDS: usize = 21;

/// Frozen records whose answer the native order reproduces exactly.
const ANSWER_IDENTICAL: usize = 24;

/// Frozen records that differ only by a permutation inside a group of equal
/// baseline sort keys — the recorded `Contract.D03` deviation.
const TIE_AFFECTED: usize = 10;

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

/// The records of one operation.
fn operation(operation: &str) -> Vec<Value> {
    records()
        .into_iter()
        .filter(|record| record["operation"].as_str() == Some(operation))
        .collect()
}

/// The absolute pitches of an integer array, as sounding pitches.
fn sounding_pitches(value: &Value) -> Vec<SoundingPitch> {
    value
        .as_array()
        .expect("a pitch list")
        .iter()
        .map(|pitch| {
            let value = u16::try_from(pitch.as_u64().expect("a pitch is a number"))
                .expect("a fixture pitch fits in u16");
            SoundingPitch::try_from(value)
                .unwrap_or_else(|error| panic!("{value} is not a sounding pitch: {error:?}"))
        })
        .collect()
}

/// A pitch list as the analyzer's input, from the fixture's open `pitches`.
fn input_pitches(record: &Value) -> Vec<SoundingPitch> {
    sounding_pitches(&record["input"]["pitches"])
}

/// The open-string pitches of a record, in physical string order.
fn string_pitches(record: &Value) -> Vec<OpenPitch> {
    record["input"]["string_pitches"]
        .as_array()
        .expect("a string pitch list")
        .iter()
        .map(|pitch| {
            let value = u8::try_from(pitch.as_u64().expect("a pitch is a number"))
                .expect("a fixture open pitch fits in u8");
            OpenPitch::try_from(value)
                .unwrap_or_else(|error| panic!("{value} is not an open pitch: {error:?}"))
        })
        .collect()
}

/// The marked positions of a record, ascending by physical string.
///
/// The fixture spells `marked_notes` as an object of string index to fret;
/// a JSON object has no order, so the positions are sorted here exactly as the
/// page state's own invariant requires.
fn marked_positions(record: &Value) -> Vec<Position> {
    let marked = record["input"]["marked_notes"]
        .as_object()
        .expect("a marked-notes object");
    let mut strings: Vec<u8> = marked
        .keys()
        .map(|string| string.parse::<u8>().expect("a string index"))
        .collect();
    strings.sort_unstable();
    strings
        .into_iter()
        .map(|string| {
            let fret = u8::try_from(
                marked[&string.to_string()]
                    .as_u64()
                    .expect("a fret is a number"),
            )
            .expect("a fixture fret fits in u8");
            Position {
                string: fretboard_core::StringIndex::try_from(string).expect("a string index"),
                fret: fretboard_core::Fret::try_from(fret).expect("a fret"),
            }
        })
        .collect()
}

/// The sounding pitches of a marked-position record: open pitch plus fret.
///
/// The fixture also carries the open pitches explicitly, so the arithmetic is
/// checked against the record's own numbers and not against a preset lookup.
fn marked_pitches(record: &Value) -> Vec<SoundingPitch> {
    let open = string_pitches(record);
    marked_positions(record)
        .into_iter()
        .map(|position| {
            let index = usize::from(u8::from(position.string));
            let base = u16::from(u8::from(open[index]));
            let sounding = base + u16::from(u8::from(position.fret));
            SoundingPitch::try_from(sounding).unwrap_or_else(|error| {
                panic!(
                    "{}: string {index} fret {} sounds at {sounding}: {error:?}",
                    case_id(record),
                    u8::from(position.fret)
                )
            })
        })
        .collect()
}

/// The page state of a marked-position record: its instrument, the committed
/// tuning the record names and the marked positions.
fn page_state(record: &Value) -> PageState {
    let instrument: InstrumentId = record["input"]["instrument"]
        .as_str()
        .expect("an instrument")
        .parse()
        .expect("the instrument is in the catalog");
    let reference: PresetName = record["input"]["preset"]
        .as_str()
        .expect("a preset")
        .parse()
        .expect("the preset is a catalog name");
    PageState {
        instrument: InstrumentState::Fretted {
            instrument,
            tuning: TuningState {
                pitches: string_pitches(record),
                reference,
            },
            selected: marked_positions(record),
        },
        chords: Vec::new(),
        highlight: None,
        tab: Tab::Analyzer,
    }
}

/// One interpretation as the JSON object the fixture spells.
///
/// The key set is the record's own: `bass`, `inversion` and `slash_label` are
/// carried by every identification of a `chords` answer, which is always a
/// bass-aware call (`Chord.identify/2` with the lowest sounding pitch).
fn interpretation_json(entry: &Interpretation) -> Value {
    let mut object = Map::new();
    object.insert("root".to_owned(), json!(entry.root.name()));
    object.insert("quality".to_owned(), json!(entry.quality.as_str()));
    object.insert("exact".to_owned(), json!(entry.exact));
    object.insert("incomplete".to_owned(), json!(entry.incomplete));
    object.insert(
        "notes".to_owned(),
        json!(
            entry
                .notes
                .iter()
                .map(|note| note.name())
                .collect::<Vec<&str>>()
        ),
    );
    object.insert("intervals".to_owned(), json!(entry.intervals));
    object.insert(
        "missing_intervals".to_owned(),
        json!(entry.missing_intervals),
    );
    object.insert("bass".to_owned(), json!(entry.bass.map(PitchClass::name)));
    object.insert("inversion".to_owned(), json!(entry.inversion));
    object.insert("slash_label".to_owned(), json!(entry.slash_label));
    Value::Object(object)
}

/// The analysis as the JSON object the fixture spells.
fn analysis_json(analysis: &Analysis) -> Value {
    match analysis {
        Analysis::Empty => json!({"variant": "empty", "fields": []}),
        Analysis::Single { note } => json!({"variant": "single", "fields": [note.name()]}),
        Analysis::Interval { low, high, label } => {
            json!({"variant": "interval", "fields": [low.name(), high.name(), label]})
        }
        Analysis::Chords {
            notes,
            bass,
            interpretations,
        } => json!({
            "variant": "chords",
            "fields": [
                notes.iter().map(|note| note.name()).collect::<Vec<&str>>(),
                bass.name(),
                interpretations
                    .iter()
                    .map(interpretation_json)
                    .collect::<Vec<Value>>(),
            ],
        }),
    }
}

/// The canonical text of one JSON value: the same encoding on both sides, so a
/// comparison never depends on a map's iteration order (`serde_json`'s map is a
/// `BTreeMap`, so object keys are sorted).
fn canonical(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|error| panic!("cannot encode {error:?}"))
}

/// How one frozen record compares with the native answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    /// The answer equals the frozen one.
    Identical,
    /// The answer differs from the frozen one only inside tie groups of its
    /// chord list.
    TieOnly,
}

/// The baseline sort key of one frozen interpretation.
fn frozen_key(entry: &Value, notes: &[PitchClass]) -> MatchSortKey {
    let root: PitchClass = entry["root"]
        .as_str()
        .expect("a frozen entry names its root")
        .parse()
        .expect("a frozen root is a pitch class");
    let quality = entry["quality"]
        .as_str()
        .expect("a frozen entry names its quality")
        .parse()
        .expect("a frozen quality is a catalog identifier");
    match_sort_key(notes, root, quality)
        .unwrap_or_else(|| panic!("the baseline sort key of {entry} is not a match"))
}

/// Compare one record's frozen answer with the native one.
///
/// Panics, naming the case, on any difference that is not a permutation inside
/// a group of equal baseline sort keys.
fn check(record: &Value) -> Outcome {
    let case = case_id(record);
    let ours = analyze(record);
    let frozen = &record["output"]["analysis"];

    let our_json = analysis_json(&ours);
    if canonical(&our_json) == canonical(frozen) {
        return Outcome::Identical;
    }

    let variant = frozen["variant"].as_str().expect("a variant");
    assert_eq!(
        our_json["variant"], frozen["variant"],
        "{case}: the analysis variant differs"
    );
    assert_eq!(variant, "chords", "{case}: only a chord list may differ");

    let our_fields = our_json["fields"].as_array().expect("fields").clone();
    let frozen_fields = frozen["fields"].as_array().expect("fields");

    // The note list and the bass must agree exactly; only the ordered
    // identifications may differ, and only inside a tie group.
    assert_eq!(
        canonical(&our_fields[0]),
        canonical(&frozen_fields[0]),
        "{case}: the note list differs"
    );
    assert_eq!(
        canonical(&our_fields[1]),
        canonical(&frozen_fields[1]),
        "{case}: the bass differs"
    );

    let Analysis::Chords { notes, .. } = &ours else {
        panic!("{case}: the native answer is not a chord answer");
    };

    let our_text: Vec<String> = our_fields[2]
        .as_array()
        .expect("interpretations")
        .iter()
        .map(canonical)
        .collect();
    let frozen_text: Vec<String> = frozen_fields[2]
        .as_array()
        .expect("interpretations")
        .iter()
        .map(canonical)
        .collect();
    assert_eq!(
        our_text.len(),
        frozen_text.len(),
        "{case}: a different number of interpretations"
    );

    let frozen_keys: Vec<MatchSortKey> = frozen_fields[2]
        .as_array()
        .expect("interpretations")
        .iter()
        .map(|entry| frozen_key(entry, notes))
        .collect();
    let our_keys: Vec<MatchSortKey> = our_fields[2]
        .as_array()
        .expect("interpretations")
        .iter()
        .map(|entry| {
            let root: PitchClass = entry["root"]
                .as_str()
                .expect("a root")
                .parse()
                .expect("root");
            let quality = entry["quality"]
                .as_str()
                .expect("a quality")
                .parse()
                .expect("quality");
            match_sort_key(notes, root, quality).expect("a match")
        })
        .collect();
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

/// The native analysis of one record, by the operation the record names.
fn analyze(record: &Value) -> Analysis {
    match record["operation"].as_str().expect("an operation") {
        "analyze_pitches" => analyze_pitches(&input_pitches(record)),
        "analyzer_state" => analyze_pitches(&marked_pitches(record)),
        other => panic!("{}: unknown operation {other}", case_id(record)),
    }
}

/// Every frozen record, in order, with the approved tie deviation counted and
/// visible.
#[test]
fn every_frozen_analyzer_record_matches() {
    let pitch_records = operation("analyze_pitches");
    let state_records = operation("analyzer_state");
    assert_eq!(pitch_records.len(), PITCH_RECORDS);
    assert_eq!(state_records.len(), STATE_RECORDS);

    let mut identical = 0usize;
    let mut tie_affected = 0usize;
    let mut examples: Vec<String> = Vec::new();
    for record in records() {
        match check(&record) {
            Outcome::Identical => identical += 1,
            Outcome::TieOnly => {
                tie_affected += 1;
                examples.push(case_id(&record));
            }
        }
    }

    println!(
        "analyzer: {RECORDS} records, {identical} identical, {tie_affected} tie-affected \
         (e.g. {examples:?})"
    );
    assert_eq!(
        identical, ANSWER_IDENTICAL,
        "the number of records reproduced in full order changed"
    );
    assert_eq!(
        tie_affected, TIE_AFFECTED,
        "the number of tie-affected records changed"
    );
    assert_eq!(identical + tie_affected, RECORDS);
}

/// The fretted analyzer reaches the same answer as the recorded marked
/// positions, and the tab gate is part of the contract: the analysis is absent
/// on the visualizer tab and independent of the stored visualizer chords.
///
/// The case is `analyzer_state/guitar-c-major-triad-shape`, whose three marked
/// positions are the C/E/G# sets the record names.
#[test]
fn the_analysis_is_absent_on_the_visualizer_and_independent_of_the_chords() {
    let case = "analyzer_state/guitar-c-major-triad-shape";
    let record = records()
        .into_iter()
        .find(|record| case_id(record) == case)
        .unwrap_or_else(|| panic!("the fixture has no case {case}"));

    let analyzer = page_state(&record);
    let frozen = &record["output"]["analysis"];
    let expected = analysis_json(&analyze_pitches(&marked_pitches(&record)));
    assert_eq!(canonical(&expected), canonical(frozen), "{case}");

    assert_eq!(
        analysis_json(&analyze_page(&analyzer).expect("the analyzer tab computes an analysis")),
        expected,
        "{case}: the page analysis must be the recorded one"
    );

    // The stored visualizer chords do not reach the analyzer: the same
    // selection with chords and a highlight analyzes exactly the same way.
    let chorded = PageState {
        chords: vec![
            fretboard_core::ChordSpec {
                root: "C".parse().expect("C"),
                quality: fretboard_core::QualityId::parse("major").expect("major"),
            },
            fretboard_core::ChordSpec {
                root: "A".parse().expect("A"),
                quality: fretboard_core::QualityId::parse("min7").expect("min7"),
            },
        ],
        highlight: Some(fretboard_core::ChordSpec {
            root: "C".parse().expect("C"),
            quality: fretboard_core::QualityId::parse("major").expect("major"),
        }),
        ..analyzer.clone()
    };
    assert_eq!(
        analysis_json(&analyze_page(&chorded).expect("the analyzer tab computes an analysis")),
        expected,
        "{case}: the analysis must not depend on the stored chords"
    );

    // Absent on the visualizer tab, whether or not the selection is marked.
    let visualizer = PageState {
        tab: Tab::Visualizer,
        ..chorded
    };
    assert_eq!(
        analyze_page(&visualizer),
        None,
        "{case}: the visualizer tab carries no analysis"
    );
    let visualizer_without_chords = PageState {
        tab: Tab::Visualizer,
        chords: Vec::new(),
        highlight: None,
        ..analyzer
    };
    assert_eq!(
        analyze_page(&visualizer_without_chords),
        None,
        "{case}: the visualizer tab carries no analysis even without chords"
    );
}

/// An empty selection on the analyzer tab is the `empty` analysis, which is
/// distinct from `None`: `Evaluation.analysis` is absent only on the
/// visualizer. The case is `analyzer_state/guitar-empty`.
#[test]
fn an_empty_selection_is_an_empty_analysis_not_an_absent_one() {
    let case = "analyzer_state/guitar-empty";
    let record = records()
        .into_iter()
        .find(|record| case_id(record) == case)
        .unwrap_or_else(|| panic!("the fixture has no case {case}"));
    let state = page_state(&record);

    assert_eq!(analyze_page(&state), Some(Analysis::Empty));
    assert_eq!(
        canonical(&analysis_json(&analyze_page(&state).expect("an analysis"))),
        canonical(&record["output"]["analysis"]),
        "{case}"
    );
}

/// One pitch class at distinct heights is an `Octave`, and two classes are the
/// interval between their **lowest** occurrences, so a repeated or permuted
/// input cannot change the answer. The cases are the fixture's own.
#[test]
fn repeats_permutations_and_octaves_reduce_to_the_recorded_interval() {
    for (case, expected_label) in [
        ("analyze_pitches/single-pitch-repeated", None),
        ("analyze_pitches/one-class-one-octave", Some("Octave")),
        ("analyze_pitches/one-class-two-octaves", Some("Octave")),
        (
            "analyze_pitches/two-classes-permuted-high-first",
            Some("Major 3rd"),
        ),
        (
            "analyze_pitches/one-class-many-octaves-with-second-class",
            Some("Perfect 4th"),
        ),
    ] {
        let record = records()
            .into_iter()
            .find(|record| case_id(record) == case)
            .unwrap_or_else(|| panic!("the fixture has no case {case}"));
        let analysis = analyze(&record);
        let fields = analysis_json(&analysis)["fields"].clone();
        assert_eq!(
            canonical(&analysis_json(&analysis)),
            canonical(&record["output"]["analysis"]),
            "{case}"
        );
        if let Some(label) = expected_label {
            assert_eq!(
                fields[2].as_str(),
                Some(label),
                "{case}: the recorded interval label"
            );
        } else {
            assert_eq!(
                fields.as_array().expect("fields").len(),
                1,
                "{case}: a repeated single pitch stays a single note"
            );
        }
    }
}

/// The bass of a shape is its lowest **sounding** pitch, so a reentrant tuning
/// and a real pitch crossing both work. The cases are
/// `analyzer_state/ukelele-ukelele-standard-all-open-reentrant` (whose first
/// string sounds a fourth *above* the second) and
/// `analyzer_state/guitar-crossing-marks-on-adjacent-strings`.
#[test]
fn the_bass_is_the_lowest_sounding_pitch_even_when_it_crosses() {
    for case in [
        "analyzer_state/ukelele-ukelele-standard-all-open-reentrant",
        "analyzer_state/guitar-crossing-marks-on-adjacent-strings",
    ] {
        let record = records()
            .into_iter()
            .find(|record| case_id(record) == case)
            .unwrap_or_else(|| panic!("the fixture has no case {case}"));
        let pitches = marked_pitches(&record);
        let lowest = pitches.iter().copied().min().expect("marks");
        let expected_bass = analyze_pitches(&[lowest]);

        let analysis = analyze(&record);
        let Analysis::Chords { bass, .. } = analysis else {
            panic!("{case}: the record is a chord answer");
        };
        let Analysis::Single { note } = expected_bass else {
            panic!("{case}: one pitch analyzes as a single note");
        };
        assert_eq!(
            bass, note,
            "{case}: the bass is the lowest sounding pitch, not the first string"
        );

        // The first string of the reentrant ukulele sounds above the second:
        // the naive "first marked string" reading would name a different bass.
        if case.ends_with("reentrant") {
            let open = string_pitches(&record);
            assert!(
                u8::from(open[0]) > u8::from(open[1]),
                "{case}: the case must be reentrant"
            );
            let first_marked = page_state(&record);
            let InstrumentState::Fretted { selected, .. } = &first_marked.instrument else {
                panic!("{case}: the record is fretted");
            };
            assert_eq!(
                u8::from(selected[0].string),
                0,
                "{case}: the first position is on the first string"
            );
            assert!(
                pitches[0] > lowest,
                "{case}: the first string is not the lowest sounding pitch"
            );
        }
    }
}

/// Three or more pitch classes answer with the identified chords, in the
/// baseline's order, and each identification carries the bass annotation of a
/// bass-aware call. The case is `analyzer_state/guitar-three-classes`.
#[test]
fn a_chord_answer_carries_the_bass_aware_identifications() {
    let case = "analyzer_state/guitar-three-classes";
    let record = records()
        .into_iter()
        .find(|record| case_id(record) == case)
        .unwrap_or_else(|| panic!("the fixture has no case {case}"));
    check(&record);

    let Analysis::Chords {
        notes,
        bass,
        interpretations,
    } = analyze(&record)
    else {
        panic!("{case}: the record is a chord answer");
    };
    assert!(
        !interpretations.is_empty(),
        "{case}: the record has matches"
    );
    assert_eq!(
        interpretations,
        identify_notes_with_bass(&notes, bass),
        "{case}: the analyzer must call the only identification rule"
    );
    assert!(
        interpretations.iter().all(|entry| entry.bass == Some(bass)),
        "{case}: every identification carries the bass"
    );
    assert!(
        interpretations
            .iter()
            .any(|entry| entry.inversion.is_some()),
        "{case}: the case holds an annotated inversion"
    );
}
