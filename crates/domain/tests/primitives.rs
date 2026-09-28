// Test target: `expect`/`unwrap`, panicking assertions and direct indexing are the
// idiom in tests, so the restriction lints that forbid them in the library are
// relaxed here only. `arithmetic_side_effects` is relaxed because the tests
// deliberately exercise the wrap-around arithmetic of note transposition with
// small, bounded values. Every other lint, including `pedantic`, still applies.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

//! Musical primitives for task C03 (`crates/domain`): note lookup with flat
//! aliases, modulo-twelve transposition, simple and compound interval names,
//! and the major chord — all pinned against the frozen oracle
//! (`fixtures/oracle/catalogs.json` and `fixtures/oracle/chords.jsonl`).
//!
//! This file is the test-writer's definition of the C03 surface, so the
//! implementer has an exact API to satisfy. The pinned signatures are:
//!
//! - `note.rs`
//!   - `pub fn chromatic_scale() -> [&'static str; 12]` — the twelve sharp
//!     names in pitch-class order, the display spelling.
//!   - `pub fn note_index(name: &str) -> Result<PitchClass, CoreError>` —
//!     accepts the twelve sharp names and the seven flat aliases
//!     (`Db Eb Fb Gb Ab Bb Cb`); anything else is
//!     `CoreError::UnknownIdentifier`.
//!   - `pub fn note_at(base: PitchClass, semitones: i32) -> PitchClass` —
//!     transposition modulo twelve with the same circular indexing as Elixir's
//!     `rem` plus `Enum.at` (`note_at("C", -1)` is B), not Rust's truncated
//!     `%` on a negative dividend.
//! - `interval.rs`
//!   - `pub fn interval_name(semitones: u32) -> &'static str` — `0` is the
//!     Perfect Unison, a positive multiple of twelve is `Octave`, every other
//!     distance reduces to its simple interval `0..=11`.
//! - `chord.rs` (C06's file, reached here through the documented C03 handoff)
//!   - `pub fn chord_details(spec: &ChordSpec) -> Result<ChordDetails, CoreError>`
//!     with a `ChordDetails` carrying `root: PitchClass`, `quality: QualityId`,
//!     `label: String` (root name plus wire suffix, e.g. `Cmaj`),
//!     `notes: Vec<PitchClass>` in formula order and
//!     `intervals: Vec<&'static str>` in label order.
//!
//! P1 implements only the `major` quality; every other catalog quality must
//! report a documented unimplemented-capability error rather than an empty or
//! wrong chord.
//!
//! First observation for C03 is the expected failure to compile: the note,
//! interval and chord entry points below do not exist yet.

mod common;

use common::{
    SHARP_NOTE_NAMES, assert_error_code, chord, oracle_chord_interval_labels,
    oracle_chord_interval_pairs, oracle_chord_label, oracle_chord_notes,
    oracle_chord_quality_label, oracle_interval_names, oracle_records, pitch_class, quality,
    quality_formula, string_array,
};
use fretboard_core::{
    ChordSpec, chord_details, chromatic_scale, interval_name, note_at, note_index,
};

/// The seven flat spellings and their sharp equivalents (contract section 2).
const FLAT_ALIASES: [(&str, &str); 7] = [
    ("Db", "C#"),
    ("Eb", "D#"),
    ("Fb", "E"),
    ("Gb", "F#"),
    ("Ab", "G#"),
    ("Bb", "A#"),
    ("Cb", "B"),
];

/// The pitch class a root transposed by `semitones` must land on, computed
/// from the definition (`rem` plus circular indexing) rather than from the
/// implementation under test.
fn wrap(root: u8, semitones: i32) -> u8 {
    u8::try_from((i32::from(root) + semitones).rem_euclid(12)).expect("a wrapped class fits in u8")
}

#[test]
fn sharp_names_map_to_pitch_classes_in_order() {
    assert_eq!(
        chromatic_scale(),
        SHARP_NOTE_NAMES,
        "the chromatic scale is the twelve sharp names in pitch-class order"
    );

    for (index, name) in SHARP_NOTE_NAMES.iter().enumerate() {
        let expected = pitch_class(u8::try_from(index).expect("a pitch-class index fits in u8"));
        let actual = note_index(name).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(actual, expected, "{name} must be pitch class {index}");
        assert_eq!(
            actual.name(),
            *name,
            "display is sharp-only, so {name} displays unchanged"
        );
    }
}

#[test]
fn flat_names_are_aliases_of_their_sharp_equivalents() {
    for (flat, sharp) in FLAT_ALIASES {
        let flat_class = note_index(flat).unwrap_or_else(|error| panic!("{flat}: {error:?}"));
        let sharp_class = note_index(sharp).unwrap_or_else(|error| panic!("{sharp}: {error:?}"));
        assert_eq!(flat_class, sharp_class, "{flat} must alias {sharp}");
        assert_eq!(
            flat_class.name(),
            sharp,
            "{flat} displays as its sharp equivalent {sharp}"
        );
    }
}

#[test]
fn flat_roots_name_the_same_notes_as_their_sharp_equivalents_in_the_oracle() {
    for (flat, sharp) in FLAT_ALIASES {
        assert_eq!(
            oracle_chord_notes(flat, "major"),
            oracle_chord_notes(sharp, "major"),
            "the oracle spells the {flat} root exactly as the {sharp} root"
        );
    }
}

#[test]
fn unknown_note_names_are_rejected() {
    for name in ["H", "C##", "do", "", "c", "B#", "E#", "Db ", "\t"] {
        assert_error_code(note_index(name), "UnknownIdentifier");
    }
}

#[test]
fn transposition_wraps_forward_around_the_chromatic_circle() {
    // The named examples of contract section 2.
    assert_eq!(u8::from(note_at(pitch_class(0), 1)), 1, "C plus 1 is C#");
    assert_eq!(
        u8::from(note_at(pitch_class(11), 2)),
        1,
        "B plus 2 wraps to C#"
    );
    assert_eq!(
        u8::from(note_at(pitch_class(5), 12)),
        5,
        "a full octave is the identity"
    );

    for root in 0..12u8 {
        for offset in [1_i32, 2, 12, 13, 25] {
            assert_eq!(
                u8::from(note_at(pitch_class(root), offset)),
                wrap(root, offset),
                "pitch class {root} plus {offset}"
            );
        }
    }
}

#[test]
fn transposition_wraps_downward_with_elixir_rem_semantics() {
    // The named examples of contract section 2.
    assert_eq!(u8::from(note_at(pitch_class(0), -1)), 11, "C minus 1 is B");
    assert_eq!(
        u8::from(note_at(pitch_class(0), -13)),
        11,
        "C minus 13 wraps to B, like Elixir rem plus Enum.at"
    );

    for root in 0..12u8 {
        for offset in [-1_i32, -2, -12, -13, -25] {
            assert_eq!(
                u8::from(note_at(pitch_class(root), offset)),
                wrap(root, offset),
                "pitch class {root} minus {}",
                -offset
            );
        }
    }
}

#[test]
fn transposition_agrees_with_the_frozen_chord_notes() {
    let records = oracle_records("fixtures/oracle/chords.jsonl");
    let mut checked = 0_usize;

    for record in &records {
        if record["operation"].as_str() != Some("chord_notes") {
            continue;
        }
        let root_name = record["input"]["root"].as_str().expect("a chord root");
        let quality_id = record["input"]["quality"]
            .as_str()
            .expect("a chord quality");
        let notes = string_array(&record["output"], "notes");
        let formula = quality_formula(quality_id);
        let base = note_index(root_name).unwrap_or_else(|error| panic!("{root_name}: {error:?}"));

        assert_eq!(
            formula.len(),
            notes.len(),
            "the oracle formula and notes of {root_name}:{quality_id} must line up"
        );
        for (offset, note) in formula.iter().zip(notes.iter()) {
            assert_eq!(
                note_at(base, i32::from(*offset)).name(),
                note.as_str(),
                "note_at({root_name}, {offset}) for {root_name}:{quality_id}"
            );
            checked = checked.saturating_add(1);
        }
    }

    assert!(
        checked >= 12 * 3,
        "expected the oracle to cover every root at least through the major triad offsets, checked {checked}"
    );
}

#[test]
fn simple_interval_names_match_the_frozen_interval_names() {
    let names = oracle_interval_names();

    let semitones: Vec<u8> = names.iter().map(|(semitones, _)| *semitones).collect();
    let expected_semitones: Vec<u8> = (0..12u8).collect();
    assert_eq!(
        semitones, expected_semitones,
        "the frozen interval table must cover 0..=11 exactly once"
    );

    for (semitones, name) in names {
        assert_eq!(
            interval_name(u32::from(semitones)),
            name,
            "the simple interval name of {semitones} semitones"
        );
    }
}

#[test]
fn compound_intervals_reduce_to_their_simple_interval_or_report_an_octave() {
    assert_eq!(interval_name(0), "Perfect Unison");
    assert_eq!(interval_name(14), "Major 2nd");
    assert_eq!(interval_name(19), "Perfect 5th");
    assert_eq!(interval_name(25), "Minor 2nd");

    for multiple in 1_u32..=4 {
        assert_eq!(
            interval_name(multiple * 12),
            "Octave",
            "{multiple} octaves apart is an Octave"
        );
    }

    for reduced in 1_u32..=11 {
        assert_eq!(
            interval_name(reduced + 12),
            interval_name(reduced),
            "an octave above the {reduced}-semitone interval reduces to it"
        );
    }
}

#[test]
fn major_chord_details_match_the_frozen_oracle_for_every_root() {
    let major = quality("major");
    let suffix = oracle_chord_quality_label("major");
    let expected_intervals = oracle_chord_interval_labels("major");

    for root_name in SHARP_NOTE_NAMES {
        let spec = ChordSpec {
            root: note_index(root_name).unwrap_or_else(|error| panic!("{root_name}: {error:?}")),
            quality: major,
        };
        let details = chord_details(&spec)
            .unwrap_or_else(|error| panic!("{root_name}maj must be implemented: {error:?}"));

        let notes: Vec<&str> = details.notes.iter().map(|note| note.name()).collect();
        assert_eq!(
            notes,
            oracle_chord_notes(root_name, "major"),
            "the notes of {root_name}maj, in formula order"
        );

        assert_eq!(
            details.label,
            oracle_chord_label(root_name, "major"),
            "the full label of {root_name}maj"
        );
        assert_eq!(
            details.label,
            format!("{root_name}{suffix}"),
            "the label is the root concatenated with the wire suffix"
        );

        assert_eq!(
            details.intervals, expected_intervals,
            "the interval labels of {root_name}maj"
        );

        let pairs: Vec<(String, String)> = details
            .notes
            .iter()
            .zip(details.intervals.iter())
            .map(|(note, interval)| (note.name().to_string(), (*interval).to_string()))
            .collect();
        assert_eq!(
            pairs,
            oracle_chord_interval_pairs(root_name, "major"),
            "the actual zipped note and interval pairs of {root_name}maj"
        );

        assert_eq!(details.root, spec.root);
        assert_eq!(details.quality, major);
    }
}

#[test]
fn an_unimplemented_quality_reports_a_documented_error() {
    // Coordinator resolution of the ambiguity the C03 report raised: the
    // contract now carries `CoreError::UnsupportedCapability` (`CORE-D05`,
    // documented in `docs/contracts.md`), so an unimplemented quality reports
    // that code instead of masquerading as an invalid action. The behaviour
    // pinned here is unchanged: an error, never an empty chord presented as
    // success.
    let spec = chord(0, "minor");

    match chord_details(&spec) {
        Ok(details) => {
            panic!("minor is not implemented in P1 and must not answer, got a chord: {details:?}")
        }
        Err(error) => assert_eq!(
            error.code(),
            "UnsupportedCapability",
            "the unimplemented quality must report a documented error, got {error:?}"
        ),
    }
}
