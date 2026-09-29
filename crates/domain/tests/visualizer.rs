//! Task `C10`: the visualizer surface against the frozen oracle.
//!
//! `fixtures/oracle/surfaces.jsonl` records what the pinned application shows:
//! every note of every position of every fretted instrument (`note_at`, 475
//! cases), the surface rows with their chord memberships (`fretboard_data`, 9),
//! the keyboard keys (`keyboard_data`, 4) and the fill of a note
//! (`note_fill`, 12).
//!
//! Two kinds of value in that fixture are *not* the domain's to answer, and the
//! tests say so instead of pretending otherwise:
//!
//! * the hex colours come from the platform's palette, so the domain answers with
//!   a slot and the tests index the fixture's own palette with it;
//! * the palette wrap of `chord_color/2` (`index` rem `length`) is that lookup,
//!   not music: `identity_slots` pins the slot, and a client may wrap its palette
//!   however long it is.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod common;

use std::collections::BTreeMap;

use common::oracle_records;
use fretboard_core::{
    ChordSpec, InstrumentId, KeyboardKey, NoteFill, PitchClass, QualityId, SurfaceCell,
    chord_details, fretted_instruments, fretted_rows, identity_slots, keyboard_keys, note_fill,
    quality_from_label, standard_pitches, standard_tuning_notes,
};
use serde_json::Value;

/// The frozen fixture and the operations this task owns.
const FIXTURE: &str = "fixtures/oracle/surfaces.jsonl";
const NOTE_AT_CASES: usize = 475;
const ROWS_CASES: usize = 9;
const KEYS_CASES: usize = 4;
const FILL_CASES: usize = 12;

/// The overlap colour the fixture records for the platform's palette.
const OVERLAP: &str = "#9E9E9E";

/// Every record of the fixture.
fn records() -> Vec<Value> {
    oracle_records(FIXTURE)
}

/// The records of one operation, with their identifier.
fn cases(operation: &str) -> Vec<(String, Value)> {
    records()
        .into_iter()
        .filter(|record| record["operation"].as_str() == Some(operation))
        .map(|record| {
            (
                record["case_id"]
                    .as_str()
                    .expect("a case has an identifier")
                    .to_owned(),
                record,
            )
        })
        .collect()
}

/// A chord identity from its wire label, e.g. `Cmaj` or `F#7`.
fn chord_of_label(label: &str) -> ChordSpec {
    let (root, rest) = if label.as_bytes().get(1) == Some(&b'#') {
        (&label[..2], &label[2..])
    } else {
        (&label[..1], &label[1..])
    };

    ChordSpec {
        root: root.parse().expect("the label starts with a note"),
        quality: quality_from_label(rest)
            .unwrap_or_else(|| panic!("{label} carries a catalog quality")),
    }
}

/// The active chords of one record.
fn active_chords(record: &Value) -> Vec<ChordSpec> {
    record["input"]["active_chords"]
        .as_array()
        .expect("a case has active chords")
        .iter()
        .map(|chord| ChordSpec {
            root: chord["root"]
                .as_str()
                .expect("a chord has a root")
                .parse()
                .expect("the root is a note"),
            quality: QualityId::parse(chord["quality"].as_str().expect("a chord has a quality"))
                .expect("the quality is a catalog quality"),
        })
        .collect()
}

/// The labels of a membership list, in order and with repeats.
fn labels(memberships: &[ChordSpec]) -> Vec<String> {
    memberships
        .iter()
        .map(|spec| {
            chord_details(spec)
                .expect("the catalog knows every identity")
                .label
        })
        .collect()
}

/// The note names of every fretted instrument's standard tuning, computed once.
fn standard_rows() -> BTreeMap<InstrumentId, Vec<Vec<SurfaceCell>>> {
    fretted_instruments()
        .iter()
        .map(|instrument| {
            let notes =
                standard_tuning_notes(instrument.id).expect("every fretted instrument has one");
            let rows = fretted_rows(instrument.id, &notes, &[]).expect("the standard tuning fits");
            (instrument.id, rows)
        })
        .collect()
}

/// Every position of every fretted instrument carries the recorded note.
///
/// The case list names the instrument, the string and its open note; the notes are
/// computed from the instrument's own standard tuning, not from the fixture, and
/// the open note the fixture states is checked against it.
#[test]
fn every_position_of_every_fretted_instrument_carries_the_frozen_note() {
    let cases = cases("note_at");
    assert_eq!(
        cases.len(),
        NOTE_AT_CASES,
        "the fixture records {NOTE_AT_CASES} positions"
    );

    let rows = standard_rows();
    for (case, record) in cases {
        let instrument: InstrumentId = record["input"]["instrument"]
            .as_str()
            .expect("a case names an instrument")
            .parse()
            .expect("the instrument is in the catalog");
        let string = record["input"]["string"]
            .as_u64()
            .expect("a case names a string");
        let string = usize::try_from(string).expect("a string index fits");
        let fret = usize::try_from(
            record["input"]["fret"]
                .as_u64()
                .expect("a case names a fret"),
        )
        .expect("a fret fits");
        let open = record["input"]["open_note"]
            .as_str()
            .expect("a case states the open note");
        let expected = record["output"]["note"]
            .as_str()
            .expect("a case gives a note");

        let instrument_rows = rows
            .get(&instrument)
            .unwrap_or_else(|| panic!("{case}: the instrument has rows"));
        let row = instrument_rows
            .get(string)
            .unwrap_or_else(|| panic!("{case}: the string exists"));
        assert_eq!(
            row.first().expect("a row has an open position").note.name(),
            open,
            "{case}: the standard tuning must carry the recorded open note"
        );

        assert_eq!(
            row.get(fret)
                .unwrap_or_else(|| panic!("{case}: the fret exists"))
                .note
                .name(),
            expected,
            "{case}: the note at the position differs from the baseline"
        );
    }
}

/// The surface rows carry the recorded notes and memberships.
#[test]
fn every_frozen_fretboard_surface_is_reproduced() {
    let cases = cases("fretboard_data");
    assert_eq!(
        cases.len(),
        ROWS_CASES,
        "the fixture records {ROWS_CASES} surfaces"
    );

    for (case, record) in cases {
        let instrument: InstrumentId = record["input"]["instrument"]
            .as_str()
            .expect("a case names an instrument")
            .parse()
            .expect("the instrument is in the catalog");
        let tuning = record["input"]["tuning"]
            .as_array()
            .expect("a case names its tuning")
            .iter()
            .map(|note| {
                note.as_str()
                    .expect("a tuning carries notes")
                    .parse::<PitchClass>()
                    .expect("the note is in the chromatic scale")
            })
            .collect::<Vec<PitchClass>>();
        let chords = active_chords(&record);

        let rows = fretted_rows(instrument, &tuning, &chords).expect("the fixture tuning fits");
        let expected = record["output"]["rows"]
            .as_array()
            .expect("a case has rows");
        assert_eq!(
            rows.len(),
            expected.len(),
            "{case}: the number of strings differs from the baseline"
        );

        for (string, expected_row) in expected.iter().enumerate() {
            let row = rows
                .get(string)
                .unwrap_or_else(|| panic!("{case}: the row exists"));
            let expected_cells = expected_row.as_array().expect("a row has cells");
            assert_eq!(
                row.len(),
                expected_cells.len(),
                "{case}: the number of positions differs from the baseline"
            );

            for (position, expected_cell) in expected_cells.iter().enumerate() {
                let cell = row
                    .get(position)
                    .unwrap_or_else(|| panic!("{case}: the cell exists"));
                let fret = expected_cell["fret"].as_u64().expect("a cell has a fret");
                assert_eq!(
                    u8::from(cell.fret),
                    u8::try_from(fret).expect("a fret fits"),
                    "{case}: string {string} has the wrong fret at position {position}"
                );
                assert_eq!(
                    cell.note.name(),
                    expected_cell["note"].as_str().expect("a cell has a note"),
                    "{case}: string {string} fret {fret} carries the wrong note"
                );
                assert_eq!(
                    labels(&cell.memberships),
                    expected_cell["chords"]
                        .as_array()
                        .expect("a cell has memberships")
                        .iter()
                        .map(|label| label.as_str().expect("a membership is a label").to_owned())
                        .collect::<Vec<String>>(),
                    "{case}: string {string} fret {fret} has the wrong memberships"
                );
            }
        }
    }
}

/// The keyboard carries the recorded keys, notes and memberships.
#[test]
fn every_frozen_keyboard_surface_is_reproduced() {
    let cases = cases("keyboard_data");
    assert_eq!(
        cases.len(),
        KEYS_CASES,
        "the fixture records {KEYS_CASES} keyboards"
    );

    for (case, record) in cases {
        let chords = active_chords(&record);
        let keys: Vec<KeyboardKey> = keyboard_keys(&chords);
        let expected = record["output"]["keys"]
            .as_array()
            .expect("a case has keys");
        assert_eq!(
            keys.len(),
            expected.len(),
            "{case}: the number of keys differs"
        );

        for (position, expected_key) in expected.iter().enumerate() {
            let key = keys
                .get(position)
                .unwrap_or_else(|| panic!("{case}: the key exists"));
            assert_eq!(
                u8::from(key.pitch),
                u8::try_from(expected_key["pitch"].as_u64().expect("a key has a pitch"))
                    .expect("a pitch fits"),
                "{case}: key {position} has the wrong pitch"
            );
            assert_eq!(
                key.note.name(),
                expected_key["note"].as_str().expect("a key has a note"),
                "{case}: key {position} carries the wrong note"
            );
            assert_eq!(
                labels(&key.memberships),
                expected_key["chords"]
                    .as_array()
                    .expect("a key has memberships")
                    .iter()
                    .map(|label| label.as_str().expect("a membership is a label").to_owned())
                    .collect::<Vec<String>>(),
                "{case}: key {position} has the wrong memberships"
            );
        }
    }
}

/// The fill of a note is the recorded colour, once the fixture's own palette is
/// indexed with the slot the domain answers.
#[test]
fn every_frozen_note_fill_is_reproduced() {
    let cases = cases("note_fill");
    assert_eq!(
        cases.len(),
        FILL_CASES,
        "the fixture records {FILL_CASES} fills"
    );

    for (case, record) in cases {
        let chords = active_chords(&record);
        let memberships = record["input"]["chords"]
            .as_array()
            .expect("a case names its memberships")
            .iter()
            .map(|label| chord_of_label(label.as_str().expect("a membership is a label")))
            .collect::<Vec<ChordSpec>>();
        let palette = record["input"]["colors"]
            .as_array()
            .expect("a case has a palette")
            .iter()
            .map(|colour| colour.as_str().expect("a colour is a string").to_owned())
            .collect::<Vec<String>>();
        let highlight = record["input"]["highlighted_chord"]
            .as_u64()
            .map(|index| usize::try_from(index).expect("an index fits"));
        let expected = record["output"]["fill"]
            .as_str()
            .expect("a case gives a fill");

        let fill = match note_fill(&chords, &memberships, highlight) {
            NoteFill::Slot(slot) => palette
                .get(slot % palette.len())
                .unwrap_or_else(|| panic!("{case}: the palette has a colour for slot {slot}"))
                .clone(),
            NoteFill::Overlap => OVERLAP.to_owned(),
        };

        assert_eq!(fill, expected, "{case}: the fill differs from the baseline");
    }
}

/// A repeated identity shares one slot, so two chips of one chord are drawn in one
/// colour, and every other identity takes its own.
#[test]
fn a_repeated_identity_shares_one_colour_slot() {
    let chords = ["Cmaj", "Amin", "Cmaj", "G7"]
        .into_iter()
        .map(chord_of_label)
        .collect::<Vec<ChordSpec>>();

    assert_eq!(identity_slots(&chords), vec![0, 1, 0, 3]);
}

/// The keyboard runs over the instrument's own range, one key per pitch.
#[test]
fn the_keyboard_is_the_instrument_range() {
    let keys = keyboard_keys(&[]);

    let (low, high) = fretboard_core::keyboard_pitch_range();
    assert_eq!(
        keys.len(),
        usize::from(u8::from(high) - u8::from(low)) + 1,
        "one key per pitch of the range"
    );
    assert_eq!(
        keys.first().map(|key| u8::from(key.pitch)),
        Some(u8::from(low))
    );
    assert_eq!(
        keys.last().map(|key| u8::from(key.pitch)),
        Some(u8::from(high))
    );

    let standard = standard_pitches(InstrumentId::Guitar).expect("the guitar has one");
    assert!(
        u8::from(low) == 48 && u8::from(high) == 83,
        "the frozen keyboard range is 48..83"
    );
    assert_eq!(standard.len(), 6, "the guitar keeps its six strings");
}

/// A tuning that does not fit the instrument is a typed error, never a shorter
/// surface.
#[test]
fn a_tuning_of_the_wrong_length_is_rejected() {
    let notes = standard_tuning_notes(InstrumentId::Guitar).expect("the guitar has one");
    let short = &notes[..notes.len() - 1];

    assert!(
        fretted_rows(InstrumentId::Guitar, short, &[]).is_err(),
        "a five-note guitar tuning must be rejected"
    );
}
