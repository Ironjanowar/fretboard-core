//! Task `C11`: fixed-reference pitch editing against the frozen oracle.
//!
//! Every record of `fixtures/oracle/tunings.jsonl` is consumed here, grouped by
//! the baseline operation that produced it:
//!
//! * `change_tuning_note` (1229) — `Fretboard.Music.change_tuning_note/4`. The
//!   pinned rule is the one `02-core-contract.md` states: editing one string
//!   chooses the pitch of the named class nearest to **that string's pitch in the
//!   reference preset**, not to the previously edited pitch, and a six-semitone
//!   tie resolves downward. The records cover every preset of every fretted
//!   instrument, every string of it and all twelve chromatic notes, plus
//!   multi-step `edit-sequence/…` records whose later steps start from a state
//!   that has already drifted away from its preset.
//! * `detect_preset` (40) — exact-pitch detection in catalog order, `Custom` for
//!   every semitone-shifted preset, so matching pitch classes alone is not a
//!   detection.
//! * `tuning_notes` (20), `preset_tuning` (20) — the derived note names of a
//!   tuning state and the committed state of a named preset.
//! * `instrument_strings` (4), `standard_tuning` (1), `tuning_presets` (1),
//!   `tuning_preset_names` (1) — the legacy guitar aliases and the string
//!   counts, which task `C07` already pins in `tests/instrument_catalog.rs`;
//!   they are consumed here as well so this file's completeness claim holds,
//!   and the values are still the fixture's, never retyped.
//!
//! Nothing here retypes an expectation from the plan: every pitch, note name and
//! preset comes out of the fixture record that carries it, and every failure
//! message names the `case_id` it belongs to.
//!
//! The plan's named behaviours are pinned by name below — downward tritone tie,
//! Drop D `E→G#` on the fixed anchor, Standard versus Low G, Baritone, exact
//! preset detection and bad-state rejection — and each of them points at the
//! fixture case ids that carry it instead of restating their values.

// Test target: the same relaxations as the other contract tests. Arithmetic on
// small integers, direct indexing of fixture values and panicking assertions are
// the idiom in tests, and a panic is a failure report.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod common;

use std::collections::BTreeMap;
use std::sync::OnceLock;

use common::{
    assert_error_code, instrument_ids, open_pitch, oracle_records, pitch_class, preset_name,
    preset_pitches, string_array,
};
use fretboard_core::{
    CoreError, InstrumentId, OpenPitch, PresetName, StringIndex, TuningState, change_tuning_note,
    chromatic_scale, detect_preset, guitar_standard_tuning, guitar_tuning_preset_names,
    guitar_tuning_presets, instrument_strings, preset_tuning, tuning_notes, validate_state,
};
use serde_json::Value;

/// The frozen fixture this file consumes.
const FIXTURE: &str = "fixtures/oracle/tunings.jsonl";

/// The eight operations of the fixture and the number of records each carries.
const OPERATIONS: [(&str, usize); 8] = [
    ("change_tuning_note", 1229),
    ("detect_preset", 40),
    ("preset_tuning", 20),
    ("tuning_notes", 20),
    ("instrument_strings", 4),
    ("standard_tuning", 1),
    ("tuning_presets", 1),
    ("tuning_preset_names", 1),
];

/// Every record of the fixture, parsed once and shared by every test.
fn fixture() -> &'static Vec<Value> {
    static FIXTURE_RECORDS: OnceLock<Vec<Value>> = OnceLock::new();
    FIXTURE_RECORDS.get_or_init(|| oracle_records(FIXTURE))
}

/// The fixture's records by case id.
fn fixture_by_case() -> &'static BTreeMap<String, Value> {
    static BY_CASE: OnceLock<BTreeMap<String, Value>> = OnceLock::new();
    BY_CASE.get_or_init(|| {
        fixture()
            .iter()
            .map(|record| (case_id(record), record.clone()))
            .collect()
    })
}

/// The twelve chromatic note names the fixture edits: the domain's own
/// sharp-only scale, which task `C03` pins against the oracle.
fn chromatic_notes() -> Vec<String> {
    chromatic_scale()
        .iter()
        .map(|note| (*note).to_string())
        .collect()
}

/// Every record of one fixture operation, in file order.
fn records_of(operation: &str) -> Vec<Value> {
    fixture()
        .iter()
        .filter(|record| record["operation"].as_str() == Some(operation))
        .cloned()
        .collect()
}

/// One record by case id, or a loud failure naming the missing case.
fn fixture_case(case_id: &str) -> Value {
    fixture_by_case()
        .get(case_id)
        .cloned()
        .unwrap_or_else(|| panic!("the fixture has no case {case_id}"))
}

/// The identifier of a fixture record.
fn case_id(record: &Value) -> String {
    record["case_id"]
        .as_str()
        .expect("every record has a case id")
        .to_owned()
}

/// A fixture instrument identifier.
fn instrument_of(record: &Value, field: &str) -> InstrumentId {
    record[field]
        .as_str()
        .expect("an instrument id")
        .parse()
        .unwrap_or_else(|error| panic!("{field} of {record} is not an instrument: {error:?}"))
}

/// A fixture record's instrument: every operation that names one carries it in
/// its `input`.
fn instrument_id(record: &Value) -> InstrumentId {
    instrument_of(&record["input"], "instrument")
}

/// A fixture tuning state: exact pitches plus the preset reference.
fn tuning_state_of(value: &Value) -> TuningState {
    let pitches = value["pitches"]
        .as_array()
        .unwrap_or_else(|| panic!("pitches must be an array in {value}"))
        .iter()
        .map(|pitch| {
            let value = u8::try_from(pitch.as_u64().expect("a pitch is an integer"))
                .expect("a fixture pitch fits in u8");
            open_pitch(value)
        })
        .collect::<Vec<OpenPitch>>();
    let reference = preset_name(
        value["reference"]
            .as_str()
            .expect("a tuning state carries its reference"),
    );
    TuningState { pitches, reference }
}

/// A fixture list of absolute pitches.
fn pitches_of(value: &Value, field: &str) -> Vec<OpenPitch> {
    value[field]
        .as_array()
        .unwrap_or_else(|| panic!("{field} must be an array in {value}"))
        .iter()
        .map(|pitch| {
            let value = u8::try_from(pitch.as_u64().expect("a pitch is an integer"))
                .expect("a fixture pitch fits in u8");
            open_pitch(value)
        })
        .collect()
}

/// A fixture string index.
fn string_of(record: &Value) -> StringIndex {
    let value = u8::try_from(record["input"]["string"].as_u64().expect("a string index"))
        .expect("a fixture string index fits in u8");
    StringIndex::try_from(value).expect("any u8 is a valid string index")
}

/// The pitch of one string in a preset, from the frozen catalog: the fixed
/// anchor an edit of that string resolves against.
fn anchor(instrument: InstrumentId, preset: &str, string: StringIndex) -> OpenPitch {
    let pitches = preset_pitches(instrument.as_str(), preset);
    let index = usize::from(u8::from(string));
    pitches
        .get(index)
        .copied()
        .unwrap_or_else(|| panic!("{preset} has no string {index}"))
}

/// A tuning state's note names, as the fixture spells them.
fn note_names(state: &TuningState) -> Vec<String> {
    tuning_notes(state)
        .iter()
        .map(|note| note.name().to_string())
        .collect()
}

// ---------------------------------------------------------------------------
// The fixture itself
// ---------------------------------------------------------------------------

/// The fixture is the frozen record list, operation by operation.
#[test]
fn the_fixture_carries_the_frozen_operation_counts() {
    let records = oracle_records(FIXTURE);
    for (operation, count) in OPERATIONS {
        assert_eq!(
            records_of(operation).len(),
            count,
            "the fixture carries {count} records of {operation}"
        );
    }

    let total: usize = OPERATIONS.iter().map(|(_, count)| count).sum();
    assert_eq!(
        records.len(),
        total,
        "every record of the fixture belongs to a pinned operation"
    );
}

/// Every preset of every fretted instrument, every string and all twelve
/// chromatic notes: the 1212 single edits plus the 17 multi-step ones.
#[test]
fn the_edits_cover_every_preset_string_and_note() {
    let notes = chromatic_notes();
    assert_eq!(notes.len(), 12, "the catalog has twelve pitch classes");

    let mut combinations = 0;
    for id in instrument_ids() {
        let fretted: InstrumentId = id.parse().expect("a catalog instrument id");
        if fretted == InstrumentId::Piano {
            continue;
        }
        let names = common::preset_names(&id);
        let preset_count = names.len();
        let strings = instrument_strings(fretted).expect("a fretted instrument has strings");
        let single = names.len() * usize::from(strings) * notes.len();
        let present = records_of("change_tuning_note")
            .into_iter()
            .filter(|record| {
                record["case_id"]
                    .as_str()
                    .is_some_and(|case| !case.starts_with("edit-sequence/"))
            })
            .filter(|record| instrument_id(record) == fretted)
            .count();
        assert_eq!(
            present, single,
            "{id} pins {single} single edits, one per preset, string and note"
        );
        for preset in names {
            for string in 0..strings {
                for note in &notes {
                    let case = format!("change_tuning_note/{id}/{preset}/s{string}/{note}");
                    let record = fixture_case(&case);
                    assert_eq!(
                        record["input"]["note"].as_str(),
                        Some(note.as_str()),
                        "{case} edits the note it names"
                    );
                }
            }
        }
        combinations += strings as usize * preset_count;
    }
    assert_eq!(
        combinations * notes.len() + 17,
        1229,
        "101 preset/string combinations times twelve notes plus 17 sequence records"
    );
}

// ---------------------------------------------------------------------------
// change_tuning_note/4 — the single edit
// ---------------------------------------------------------------------------

/// Every single edit reproduces the baseline's tuning state exactly.
#[test]
fn every_edit_produces_the_frozen_state() {
    let records = records_of("change_tuning_note");
    assert!(!records.is_empty(), "the fixture carries the edits");

    for record in records {
        let case = case_id(&record);
        let instrument = instrument_id(&record);
        let state = tuning_state_of(&record["input"]["tuning_state"]);
        let note = record["input"]["note"].as_str().expect("a note name");
        let expected = tuning_state_of(&record["output"]["tuning_state"]);

        let edited = change_tuning_note(instrument, &state, string_of(&record), note)
            .unwrap_or_else(|error| panic!("{case} must edit: {error:?}"));

        assert_eq!(edited.pitches, expected.pitches, "{case}: wrong pitches");
        assert_eq!(
            edited.reference, expected.reference,
            "{case}: the reference must survive an edit"
        );
    }
}

/// An edited state is a committed state: it passes the contract's own
/// validation.
#[test]
fn every_edited_state_is_valid() {
    for record in records_of("change_tuning_note") {
        let case = case_id(&record);
        let instrument = instrument_id(&record);
        let state = tuning_state_of(&record["input"]["tuning_state"]);
        let note = record["input"]["note"].as_str().expect("a note name");
        let edited = change_tuning_note(instrument, &state, string_of(&record), note)
            .unwrap_or_else(|error| panic!("{case} must edit: {error:?}"));

        common::assert_valid(validate_state(&fretted_page_of(instrument, &edited)));
    }
}

/// An edit touches one string and nothing else.
#[test]
fn an_edit_changes_only_the_edited_string() {
    for record in records_of("change_tuning_note") {
        let case = case_id(&record);
        let instrument = instrument_id(&record);
        let state = tuning_state_of(&record["input"]["tuning_state"]);
        let string = string_of(&record);
        let note = record["input"]["note"].as_str().expect("a note name");
        let edited = change_tuning_note(instrument, &state, string, note)
            .unwrap_or_else(|error| panic!("{case} must edit: {error:?}"));

        assert_eq!(
            edited.pitches.len(),
            state.pitches.len(),
            "{case}: an edit never adds or drops a string"
        );
        let index = usize::from(u8::from(string));
        for (position, (before, after)) in
            state.pitches.iter().zip(edited.pitches.iter()).enumerate()
        {
            if position == index {
                continue;
            }
            assert_eq!(before, after, "{case}: string {position} must be untouched");
        }
    }
}

/// The draft calculation never mutates the committed state it starts from: the
/// same input edited twice gives the same answer, and the input still holds its
/// own pitches afterwards.
#[test]
fn the_input_state_is_never_mutated() {
    for record in records_of("change_tuning_note") {
        let case = case_id(&record);
        let instrument = instrument_id(&record);
        let state = tuning_state_of(&record["input"]["tuning_state"]);
        let before = state.clone();
        let note = record["input"]["note"].as_str().expect("a note name");

        let first = change_tuning_note(instrument, &state, string_of(&record), note)
            .unwrap_or_else(|error| panic!("{case} must edit: {error:?}"));
        let second = change_tuning_note(instrument, &state, string_of(&record), note)
            .unwrap_or_else(|error| panic!("{case} must edit again: {error:?}"));

        assert_eq!(state, before, "{case}: the input state must be unchanged");
        assert_eq!(first, second, "{case}: the edit must be deterministic");
    }
}

// ---------------------------------------------------------------------------
// The plan's named behaviours
// ---------------------------------------------------------------------------

/// A six-semitone tie resolves downward: the outcome of a tritone edit is the
/// pitch six semitones *below* the string's anchor, never the one above.
#[test]
fn the_tritone_tie_resolves_downward() {
    // The fixture's own tie case: guitar Standard, first string, A#.
    let record = fixture_case("edit-sequence/guitar-standard-tritone-tie-downward/s0-A#");
    let instrument = instrument_id(&record);
    let state = tuning_state_of(&record["input"]["tuning_state"]);
    let string = string_of(&record);
    let note = record["input"]["note"].as_str().expect("a note name");

    let edited = change_tuning_note(instrument, &state, string, note)
        .unwrap_or_else(|error| panic!("{}: {error:?}", case_id(&record)));
    let expected = tuning_state_of(&record["output"]["tuning_state"]);
    assert_eq!(
        edited.pitches,
        expected.pitches,
        "{}: the frozen tie answer",
        case_id(&record)
    );

    let anchor = anchor(instrument, state.reference.as_str(), string);
    let index = usize::from(u8::from(string));
    let chosen = u8::from(edited.pitches[index]);
    assert_eq!(
        u8::from(anchor) - chosen,
        6,
        "{}: the tie must take the pitch six semitones below {anchor:?}",
        case_id(&record)
    );

    // The following step of the same sequence starts from the already edited
    // state and still resolves against the reference preset's own G.
    let next = fixture_case("edit-sequence/guitar-standard-tritone-tie-downward/s3-D#");
    let next_state = tuning_state_of(&next["input"]["tuning_state"]);
    let next_edited = change_tuning_note(instrument, &next_state, string_of(&next), "D#")
        .unwrap_or_else(|error| panic!("{}: {error:?}", case_id(&next)));
    assert_eq!(
        next_edited.pitches,
        tuning_state_of(&next["output"]["tuning_state"]).pitches,
        "{}: the second step of the tie sequence",
        case_id(&next)
    );
}

/// Drop D keeps its E string as the fixed anchor: editing that string to `G#`
/// resolves a major third *above* the preset's E, not an octave below it, and
/// the multi-step Drop D sequence resolves every string against the preset.
#[test]
fn drop_d_e_to_g_sharp_uses_the_fixed_anchor() {
    let record = fixture_case("change_tuning_note/guitar/Drop D/s5/G#");
    let instrument = instrument_id(&record);
    let state = tuning_state_of(&record["input"]["tuning_state"]);
    let string = string_of(&record);

    let edited = change_tuning_note(instrument, &state, string, "G#")
        .unwrap_or_else(|error| panic!("{}: {error:?}", case_id(&record)));
    let expected = tuning_state_of(&record["output"]["tuning_state"]);
    assert_eq!(
        edited.pitches,
        expected.pitches,
        "{}: the frozen answer",
        case_id(&record)
    );

    let anchor = anchor(instrument, state.reference.as_str(), string);
    let index = usize::from(u8::from(string));
    let chosen = u8::from(edited.pitches[index]);
    assert_eq!(
        chosen - u8::from(anchor),
        4,
        "{}: G# sits a major third above the anchor {anchor:?}",
        case_id(&record)
    );
    assert_eq!(chosen % 12, 8, "{}: the result is a G#", case_id(&record));

    // The Drop D sequence: two edits, each resolved against the reference
    // preset's own pitch for the string it edits.
    for case in [
        "edit-sequence/guitar-drop-d-fixed-reference/s3-G",
        "edit-sequence/guitar-drop-d-fixed-reference/s5-D",
    ] {
        let record = fixture_case(case);
        let state = tuning_state_of(&record["input"]["tuning_state"]);
        let string = string_of(&record);
        let note = record["input"]["note"].as_str().expect("a note name");
        let edited = change_tuning_note(instrument, &state, string, note)
            .unwrap_or_else(|error| panic!("{case}: {error:?}"));
        assert_eq!(
            edited.pitches,
            tuning_state_of(&record["output"]["tuning_state"]).pitches,
            "{case}: the frozen answer"
        );
    }
}

/// Standard and Low G are different anchors for the same note name: the same
/// `G` edit of the ukulele's first string stays high under Standard and lands an
/// octave lower under Low G, and preset detection tells the two apart.
#[test]
fn standard_versus_low_g() {
    let standard = fixture_case("edit-sequence/ukelele-standard-to-low-g-string-0/s0-G");
    let instrument = instrument_id(&standard);
    let state = tuning_state_of(&standard["input"]["tuning_state"]);
    let string = string_of(&standard);
    let note = standard["input"]["note"].as_str().expect("a note name");
    let edited = change_tuning_note(instrument, &state, string, note)
        .unwrap_or_else(|error| panic!("{}: {error:?}", case_id(&standard)));
    assert_eq!(
        edited.pitches,
        tuning_state_of(&standard["output"]["tuning_state"]).pitches,
        "{}: the frozen answer",
        case_id(&standard)
    );

    let low_g = fixture_case("preset_tuning/ukelele/Low G");
    let low_g_state = tuning_state_of(&low_g["output"]["tuning_state"]);
    let second = fixture_case("edit-sequence/ukelele-low-g-second-edit/s0-A");
    let second_state = tuning_state_of(&second["input"]["tuning_state"]);
    let second_edited = change_tuning_note(instrument, &second_state, string_of(&second), "A")
        .unwrap_or_else(|error| panic!("{}: {error:?}", case_id(&second)));
    assert_eq!(
        second_edited.pitches,
        tuning_state_of(&second["output"]["tuning_state"]).pitches,
        "{}: the frozen answer",
        case_id(&second)
    );

    let index = usize::from(u8::from(string));
    let chosen = u8::from(edited.pitches[index]);
    assert_eq!(
        chosen - u8::from(low_g_state.pitches[index]),
        12,
        "the same G is an octave apart under Standard and under Low G"
    );

    // Exact-pitch detection separates the two presets.
    assert_eq!(
        detect_preset(instrument, &state.pitches),
        Some(preset_name("Standard")),
        "{}: Standard pitches detect Standard",
        case_id(&standard)
    );
    assert_eq!(
        detect_preset(instrument, &low_g_state.pitches),
        Some(preset_name("Low G")),
        "{}: Low G pitches detect Low G",
        case_id(&low_g)
    );
}

/// The anchor is the string's pitch **in the reference preset**, never the
/// string's current pitch.
///
/// The fixture cannot tell the two apart: every record it edits starts from the
/// preset's own pitch for that string. So this test builds the one state that
/// does — the frozen Standard state with one string moved to another preset's
/// pitch, its reference still Standard — and pins the plan's rule
/// (`02-core-contract.md`: the nearest pitch to that string's pitch in the
/// reference preset, a six-semitone tie resolving downward, so the answer is
/// never further than six semitones from that preset's pitch).
#[test]
fn an_edit_resolves_against_the_reference_preset_not_the_current_pitch() {
    let instrument = InstrumentId::Ukelele;
    let standard_case = fixture_case("preset_tuning/ukelele/Standard");
    let standard = tuning_state_of(&standard_case["output"]["tuning_state"]);
    let low_g_case = fixture_case("preset_tuning/ukelele/Low G");
    let low_g = tuning_state_of(&low_g_case["output"]["tuning_state"]);

    let mut drifted = standard.clone();
    drifted.pitches[0] = low_g.pitches[0];
    assert_eq!(
        drifted.reference,
        standard.reference,
        "{}: the reference stays Standard",
        case_id(&standard_case)
    );
    assert_ne!(
        drifted.pitches[0], standard.pitches[0],
        "the state's first string has moved away from its preset"
    );

    let string = StringIndex::try_from(0).expect("any u8 is a string index");
    let edited = change_tuning_note(instrument, &drifted, string, "A")
        .unwrap_or_else(|error| panic!("the edit must apply: {error:?}"));

    let chosen = i16::from(u8::from(edited.pitches[0]));
    let anchor = i16::from(u8::from(standard.pitches[0]));
    let current = i16::from(u8::from(drifted.pitches[0]));
    assert_eq!(u8::from(edited.pitches[0]) % 12, 9, "the answer is an A");
    assert!(
        (chosen - anchor).abs() <= 6,
        "the answer {chosen} is within six semitones of the reference preset's {anchor}"
    );
    assert!(
        (chosen - current).abs() > 6,
        "the answer {chosen} is not the nearest A to the current pitch {current}"
    );
}

/// Baritone is a real ukulele anchor: its committed state, its detection and
/// both of its sequence edits come from the fixture.
#[test]
fn baritone_is_a_fixed_anchor() {
    let instrument = InstrumentId::Ukelele;

    let preset = fixture_case("preset_tuning/ukelele/Baritone");
    let state = tuning_state_of(&preset["output"]["tuning_state"]);
    let committed = preset_tuning(instrument, &preset_name("Baritone"))
        .unwrap_or_else(|error| panic!("{}: {error:?}", case_id(&preset)));
    assert_eq!(committed, state, "{}: the frozen state", case_id(&preset));

    let detection = fixture_case("detect_preset/ukelele/Baritone");
    assert_eq!(
        detect_preset(instrument, &state.pitches),
        Some(preset_name("Baritone")),
        "{}: Baritone pitches detect Baritone",
        case_id(&detection)
    );

    for case in [
        "edit-sequence/ukelele-baritone-fixed-reference/s0-D",
        "edit-sequence/ukelele-baritone-fixed-reference/s3-E",
    ] {
        let record = fixture_case(case);
        let state = tuning_state_of(&record["input"]["tuning_state"]);
        let note = record["input"]["note"].as_str().expect("a note name");
        let edited = change_tuning_note(instrument, &state, string_of(&record), note)
            .unwrap_or_else(|error| panic!("{case}: {error:?}"));
        assert_eq!(
            edited.pitches,
            tuning_state_of(&record["output"]["tuning_state"]).pitches,
            "{case}: the frozen answer"
        );
    }
}

// ---------------------------------------------------------------------------
// detect_preset/2
// ---------------------------------------------------------------------------

/// Detection compares exact pitches in catalog order: a preset's own pitches
/// name it, and a semitone-shifted copy is `Custom`.
#[test]
fn detection_is_exact() {
    let records = records_of("detect_preset");
    assert_eq!(records.len(), 40, "the frozen detection records");

    let shifted = records
        .iter()
        .filter(|record| case_id(record).ends_with("-semitone-shifted"))
        .count();
    assert_eq!(shifted, 20, "every preset has a shifted counterexample");

    for record in records {
        let case = case_id(&record);
        let instrument = instrument_id(&record);
        let pitches = pitches_of(&record["input"], "pitches");
        let expected = record["output"]["preset"]
            .as_str()
            .expect("a detected preset name");

        let detected = detect_preset(instrument, &pitches);
        let actual = detected.map_or_else(|| "Custom".to_owned(), |name| name.as_str().to_owned());
        assert_eq!(actual, expected, "{case}: wrong detection");
    }
}

/// Detection needs the exact pitches of the named preset, from the catalog.
#[test]
fn a_preset_detects_itself_from_its_own_catalog_pitches() {
    for record in records_of("detect_preset") {
        let case = case_id(&record);
        if case.ends_with("-semitone-shifted") {
            continue;
        }
        let instrument = instrument_id(&record);
        let name = record["output"]["preset"]
            .as_str()
            .expect("a detected preset name");
        let catalog = preset_pitches(instrument.as_str(), name);
        assert_eq!(
            pitches_of(&record["input"], "pitches"),
            catalog,
            "{case}: the fixture's pitches are the catalog's"
        );
    }
}

// ---------------------------------------------------------------------------
// tuning_notes/1 and preset_tuning/2
// ---------------------------------------------------------------------------

/// The note names of a tuning state are derived from its pitches, never stored.
#[test]
fn tuning_notes_are_derived_from_the_pitches() {
    let records = records_of("tuning_notes");
    assert_eq!(records.len(), 20, "the frozen note-name records");

    for record in records {
        let case = case_id(&record);
        let state = tuning_state_of(&record["input"]["tuning_state"]);
        let expected = string_array(&record["output"], "notes");
        assert_eq!(
            note_names(&state),
            expected,
            "{case}: wrong note names for {state:?}"
        );
    }
}

/// A named preset produces its committed tuning state, reference included.
#[test]
fn preset_tuning_produces_the_frozen_state() {
    let records = records_of("preset_tuning");
    assert_eq!(records.len(), 20, "the frozen preset records");

    for record in records {
        let case = case_id(&record);
        let instrument = instrument_id(&record);
        let name = preset_name(record["input"]["preset"].as_str().expect("a preset name"));
        let expected = tuning_state_of(&record["output"]["tuning_state"]);

        let state = preset_tuning(instrument, &name)
            .unwrap_or_else(|error| panic!("{case} must resolve: {error:?}"));
        assert_eq!(state, expected, "{case}: wrong committed state");
    }
}

// ---------------------------------------------------------------------------
// The legacy aliases C07 already pins
// ---------------------------------------------------------------------------

/// The string counts and the guitar-only aliases: pinned by
/// `tests/instrument_catalog.rs` (C07), consumed here as well so that every
/// record of the fixture is read by this task too.
#[test]
fn the_legacy_aliases_match_the_fixture() {
    for record in records_of("instrument_strings") {
        let case = case_id(&record);
        let instrument = instrument_id(&record);
        let expected = u8::try_from(record["output"]["strings"].as_u64().expect("a count"))
            .expect("a string count fits in u8");
        assert_eq!(
            instrument_strings(instrument).unwrap_or_else(|error| panic!("{case}: {error:?}")),
            expected,
            "{case}: wrong string count"
        );
    }

    let standard = fixture_case("standard_tuning/guitar");
    assert_eq!(
        guitar_standard_tuning()
            .iter()
            .map(|note| (*note).to_string())
            .collect::<Vec<String>>(),
        string_array(&standard["output"], "notes"),
        "{}: the guitar's Standard tuning",
        case_id(&standard)
    );

    let names = fixture_case("tuning_preset_names/guitar");
    assert_eq!(
        guitar_tuning_preset_names()
            .iter()
            .map(|name| name.as_str().to_string())
            .collect::<Vec<String>>(),
        string_array(&names["output"], "names"),
        "{}: the guitar's preset names",
        case_id(&names)
    );

    let presets = fixture_case("tuning_presets/guitar");
    let expected = presets["output"]["presets"]
        .as_array()
        .expect("presets must be an array")
        .iter()
        .map(|preset| {
            (
                preset_name(preset["name"].as_str().expect("a preset name")),
                string_array(preset, "notes"),
            )
        })
        .collect::<Vec<(PresetName, Vec<String>)>>();
    let actual = guitar_tuning_presets()
        .into_iter()
        .map(|(name, notes)| {
            (
                name,
                notes.iter().map(|note| note.name().to_string()).collect(),
            )
        })
        .collect::<Vec<(PresetName, Vec<String>)>>();
    assert_eq!(
        actual,
        expected,
        "{}: the guitar's presets with their note names",
        case_id(&presets)
    );
}

// ---------------------------------------------------------------------------
// Bad-state rejection
// ---------------------------------------------------------------------------

/// A fretted page carrying a tuning, for `validate_state`.
fn fretted_page_of(instrument: InstrumentId, tuning: &TuningState) -> fretboard_core::PageState {
    common::fretted_page(instrument, tuning.clone(), Vec::new())
}

/// The command source's tuning, edited nowhere: the rejection cases start here.
fn standard_guitar() -> TuningState {
    preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
        .expect("the guitar has a Standard preset")
}

/// A string index the instrument does not have, an unknown note name and an
/// unknown preset are typed rejections, never a panic and never a silent no-op.
#[test]
fn bad_states_are_rejected_with_the_frozen_codes() {
    let standard = standard_guitar();
    let before = standard.clone();

    // A string index outside the instrument, for every fretted instrument: the
    // state is that instrument's own Standard tuning, so the only thing wrong
    // with the call is the index.
    for instrument in [
        InstrumentId::Guitar,
        InstrumentId::Bass4,
        InstrumentId::Bass5,
        InstrumentId::Ukelele,
    ] {
        let tuning = preset_tuning(instrument, &preset_name("Standard"))
            .unwrap_or_else(|error| panic!("{instrument:?} has a Standard preset: {error:?}"));
        let strings = instrument_strings(instrument).expect("a fretted instrument has strings");
        for index in [strings, strings + 1, 255] {
            let string = StringIndex::try_from(index).expect("any u8 is a string index");
            let result = change_tuning_note(instrument, &tuning, string, "C");
            assert_error_code(result, "OutOfRange");
        }
    }

    // An unknown note name: neither a sharp name nor one of the flat aliases.
    for note in ["H", "c", "", "C##", "Db "] {
        let string = StringIndex::try_from(0).expect("a string index");
        let result = change_tuning_note(InstrumentId::Guitar, &standard, string, note);
        assert_error_code(result, "UnknownIdentifier");
    }

    // A reference that is not a preset of this instrument: `Low G` is real, but
    // not a guitar one, so the state is structurally invalid (contracts.md).
    let mut foreign = standard.clone();
    foreign.reference = preset_name("Low G");
    let string = StringIndex::try_from(0).expect("a string index");
    let result = change_tuning_note(InstrumentId::Guitar, &foreign, string, "C");
    assert_error_code(result, "InvalidState");

    // The piano has no strings and no tuning at all.
    let result = change_tuning_note(
        InstrumentId::Piano,
        &standard,
        StringIndex::try_from(0).expect("a string index"),
        "C",
    );
    assert_error_code(result, "InvalidState");

    // A state whose pitch count is not the instrument's string count.
    let mut short = standard.clone();
    short.pitches.pop();
    let result = change_tuning_note(
        InstrumentId::Guitar,
        &short,
        StringIndex::try_from(0).expect("a string index"),
        "C",
    );
    assert_error_code(result, "InvalidState");

    // Every rejection left the caller's own state exactly as it was.
    assert_eq!(
        standard, before,
        "a rejected edit must not mutate its input"
    );
    assert!(
        validate_state(&fretted_page_of(InstrumentId::Guitar, &standard)).is_ok(),
        "the rejection cases start from a valid state"
    );
}

/// A rejected edit is reported, never silently applied: the same state edited
/// with a bad note and with a good one tells them apart.
#[test]
fn a_rejected_edit_is_not_a_no_op() {
    let standard = standard_guitar();
    let string = StringIndex::try_from(3).expect("a string index");

    let rejected = change_tuning_note(InstrumentId::Guitar, &standard, string, "H");
    match rejected {
        Ok(state) => panic!("an unknown note must not be applied: {state:?}"),
        Err(error) => assert_eq!(error.code(), "UnknownIdentifier"),
    }

    let applied = change_tuning_note(InstrumentId::Guitar, &standard, string, "C")
        .expect("a known note edits");
    assert_ne!(
        applied, standard,
        "the same string and state with a valid note does change the tuning"
    );
    assert_eq!(
        u8::from(applied.pitches[3]) % 12,
        0,
        "the applied note is the requested pitch class"
    );
}

/// The pure helper the edit is built on rejects nothing here: it is the shared
/// rule (`closest_pitch`) and it resolves ties downward.
#[test]
fn the_shared_closest_pitch_rule_resolves_ties_downward() {
    use fretboard_core::{PitchClass, closest_pitch};

    // Six semitones either way: the lower pitch wins, for every one of the
    // twelve pitch classes.
    for class in 0..12_u8 {
        let note = PitchClass::try_from(class).expect("a pitch class");
        let reference = open_pitch(40);
        let chosen = closest_pitch(note, reference).expect("a pitch of that class");
        let distance = i16::from(u8::from(chosen)) - i16::from(u8::from(reference));
        assert!(
            distance.unsigned_abs() <= 6,
            "{note:?} away from {reference:?} by {distance}"
        );
        assert_eq!(
            u8::from(chosen) % 12,
            class,
            "{note:?} must keep its pitch class"
        );
    }

    // 40 is an E; A# is six semitones above and six below, and the answer is
    // the lower one.
    let tie = closest_pitch(pitch_class(10), open_pitch(40)).expect("a pitch");
    assert_eq!(u8::from(tie), 34, "the tritone tie resolves downward");
}

/// The `CoreError` type is re-exported for callers of the edit surface.
#[test]
fn the_edit_surface_reports_the_domain_error_type() {
    let standard = standard_guitar();
    let result = change_tuning_note(
        InstrumentId::Guitar,
        &standard,
        StringIndex::try_from(9).expect("a string index"),
        "C",
    );
    let error: CoreError = result.expect_err("an out-of-range string is rejected");
    assert_eq!(error.code(), "OutOfRange");
}
