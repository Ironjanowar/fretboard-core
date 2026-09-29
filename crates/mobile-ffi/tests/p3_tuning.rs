//! The P3 tuning boundary: the edit surface the analyzer phase needs, driven
//! through the **exported** functions.
//!
//! The domain tests pin the rules; these pin the crossing — that a DTO arrives,
//! becomes a tuning, gets edited by the frozen rule and comes back as the value
//! the baseline recorded, including the typed failures.
//!
//! Expectations come from `fixtures/oracle/tunings.jsonl`
//! (`detect_preset` 40 records, `tuning_notes` 20, `change_tuning_note` 1,229),
//! never from the adapter.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use fretboard_mobile_ffi::{
    AdapterError, ErrorCode, InstrumentDto, TuningDto, change_tuning_note, detect_tuning_preset,
    tuning_notes,
};
use serde_json::Value;

/// Every record of the frozen tuning fixture.
fn records() -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/oracle/tunings.jsonl");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every record is JSON"))
        .collect()
}

/// The records of one operation.
fn operation(operation: &str) -> Vec<Value> {
    records()
        .into_iter()
        .filter(|record| record["operation"].as_str() == Some(operation))
        .collect()
}

/// The instrument of one record.
fn instrument_of(record: &Value) -> InstrumentDto {
    InstrumentDto::parse(
        record["input"]["instrument"]
            .as_str()
            .expect("an instrument"),
    )
    .expect("the instrument is in the catalog")
}

/// The pitches of one JSON array.
fn pitches_of(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .expect("a pitch list")
        .iter()
        .map(|pitch| u8::try_from(pitch.as_u64().expect("a pitch is a number")).expect("fits"))
        .collect()
}

/// The tuning of one record's input, as the boundary receives it.
fn tuning_of(record: &Value) -> TuningDto {
    let tuning = &record["input"]["tuning_state"];
    TuningDto {
        pitches: pitches_of(&tuning["pitches"]),
        reference: tuning["reference"]
            .as_str()
            .expect("a tuning has a reference")
            .to_owned(),
    }
}

/// The failure of one call, with its code checked against the frozen one.
fn code_of<T: std::fmt::Debug>(result: Result<T, AdapterError>) -> ErrorCode {
    match result {
        Ok(value) => panic!("expected a failure, got Ok({value:?})"),
        Err(error) => error.code(),
    }
}

/// The recorded edit lands on the recorded tuning.
#[test]
fn every_frozen_edit_crosses_the_boundary() {
    let records = operation("change_tuning_note");
    assert_eq!(records.len(), 1_229, "the frozen edit records");

    for record in &records {
        let case = record["case_id"].as_str().expect("a case id");
        let string = u8::try_from(record["input"]["string"].as_u64().expect("a string index"))
            .expect("fits");
        let note = record["input"]["note"].as_str().expect("a note").to_owned();

        let edited = change_tuning_note(instrument_of(record), tuning_of(record), string, note)
            .unwrap_or_else(|error| panic!("{case}: the edit failed: {error:?}"));
        let frozen = &record["output"]["tuning_state"];

        assert_eq!(
            edited.pitches,
            pitches_of(&frozen["pitches"]),
            "{case}: the pitches differ from the baseline"
        );
        assert_eq!(
            edited.reference,
            frozen["reference"].as_str().expect("a reference"),
            "{case}: the reference differs from the baseline"
        );
    }
}

/// The recorded note names land on the recorded tuning.
#[test]
fn every_frozen_tuning_note_set_crosses_the_boundary() {
    let records = operation("tuning_notes");
    assert_eq!(records.len(), 20, "the frozen note-name records");

    for record in &records {
        let case = record["case_id"].as_str().expect("a case id");
        let names = tuning_notes(tuning_of(record))
            .unwrap_or_else(|error| panic!("{case}: the notes failed: {error:?}"));
        let frozen = record["output"]["notes"]
            .as_array()
            .expect("notes")
            .iter()
            .map(|note| note.as_str().expect("a note").to_owned())
            .collect::<Vec<String>>();

        assert_eq!(
            names, frozen,
            "{case}: the note names differ from the baseline"
        );
    }
}

/// A tuning matches the recorded preset, by exact pitches.
#[test]
fn every_frozen_preset_detection_crosses_the_boundary() {
    let records = operation("detect_preset");
    assert_eq!(records.len(), 40, "the frozen detection records");

    for record in &records {
        let case = record["case_id"].as_str().expect("a case id");
        let tuning = TuningDto {
            pitches: pitches_of(&record["input"]["pitches"]),
            reference: "Standard".to_owned(),
        };
        let detected = detect_tuning_preset(instrument_of(record), tuning)
            .unwrap_or_else(|error| panic!("{case}: detection failed: {error:?}"));

        assert_eq!(
            detected,
            record["output"]["preset"]
                .as_str()
                .expect("a detected preset"),
            "{case}: the detected preset differs from the baseline"
        );
    }
}

/// A tuning that is a semitone away from every preset is the baseline's own
/// `Custom`, never the nearest preset.
///
/// The case is the fixture's own `detect_preset/guitar/Standard-semitone-shifted`.
#[test]
fn detection_is_exact_and_not_nearest() {
    let standard = TuningDto {
        pitches: vec![40, 45, 50, 55, 59, 64],
        reference: "Standard".to_owned(),
    };
    assert_eq!(
        detect_tuning_preset(InstrumentDto::Guitar, standard).expect("the catalog answers"),
        "Standard",
        "the standard tuning is Standard"
    );

    let shifted = TuningDto {
        pitches: vec![40, 45, 50, 55, 59, 65],
        reference: "Standard".to_owned(),
    };
    assert_eq!(
        detect_tuning_preset(InstrumentDto::Guitar, shifted).expect("the catalog answers"),
        "Custom",
        "one semitone away matches no preset, and the baseline calls that Custom"
    );
}

/// The frozen failures cross as typed codes, never as a wrong answer.
#[test]
fn a_rejected_edit_crosses_as_a_typed_error() {
    let guitar_standard = TuningDto {
        pitches: vec![40, 45, 50, 55, 59, 64],
        reference: "Standard".to_owned(),
    };

    assert_eq!(
        code_of(change_tuning_note(
            InstrumentDto::Guitar,
            guitar_standard.clone(),
            6,
            "C".to_owned()
        )),
        ErrorCode::OutOfRange,
        "a string the instrument does not have"
    );
    assert_eq!(
        code_of(change_tuning_note(
            InstrumentDto::Guitar,
            guitar_standard.clone(),
            0,
            "H".to_owned()
        )),
        ErrorCode::UnknownIdentifier,
        "a note outside the chromatic scale"
    );
    assert_eq!(
        code_of(change_tuning_note(
            InstrumentDto::Piano,
            guitar_standard,
            0,
            "C".to_owned()
        )),
        ErrorCode::InvalidState,
        "the piano has no fretted tuning"
    );
    assert_eq!(
        code_of(change_tuning_note(
            InstrumentDto::Guitar,
            TuningDto {
                pitches: vec![40, 45, 50, 55, 59, 64],
                // Low G is a ukulele preset, never a guitar one.
                reference: "Low G".to_owned(),
            },
            0,
            "C".to_owned()
        )),
        ErrorCode::InvalidState,
        "a reference that is not this instrument's preset is a bad state"
    );
}
