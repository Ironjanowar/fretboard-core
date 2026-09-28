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
//! The note, interval and chord surfaces are pinned against the frozen oracle
//! (`fixtures/oracle/catalogs.json` and `fixtures/oracle/chords.jsonl`). The
//! chord catalog is complete since `C06`: every one of the 47 catalog qualities
//! answers with its own chord, and `tests/chord_catalog.rs` owns the full
//! catalog, group and mode coverage.
//!
//! First observation for C03 is the expected failure to compile: the note,
//! interval and chord entry points below do not exist yet.

mod common;

use common::{
    SHARP_NOTE_NAMES, assert_error_code, chord, oracle_chord_interval_labels,
    oracle_chord_interval_pairs, oracle_chord_label, oracle_chord_notes,
    oracle_chord_quality_label, oracle_interval_names, oracle_records, pitch_class, quality,
    quality_formula, quality_ids, string_array,
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

/// The pitch class a root transposed by a *large* `semitones` must land on.
/// The expectation is computed in `i64` from the same modular definition, so
/// the expected value itself cannot overflow where an `i32` implementation
/// under test would.
fn wrap_large(root: u8, semitones: i32) -> u8 {
    u8::try_from((i64::from(root) + i64::from(semitones)).rem_euclid(12))
        .expect("a wrapped class fits in u8")
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
        // The expected pitch class comes from the sharp spelling's position in
        // the chromatic scale, not from `note_index`, so an implementation that
        // rejects every flat name (or mis-resolves one) cannot satisfy this.
        let position = SHARP_NOTE_NAMES
            .iter()
            .position(|name| *name == sharp)
            .unwrap_or_else(|| panic!("{sharp} must be one of the twelve sharp names"));
        let expected = pitch_class(u8::try_from(position).expect("a pitch-class index fits in u8"));
        let flat_class =
            note_index(flat).unwrap_or_else(|error| panic!("{flat} must resolve: {error:?}"));
        assert_eq!(
            flat_class, expected,
            "the flat name {flat} must resolve through note_index to {sharp}'s pitch class"
        );

        // The *real* chord of the flat root equals the *real* chord of the
        // sharp root, and both name exactly the notes the oracle pins for the
        // sharp root. This reaches through note_index and chord_details instead
        // of comparing oracle data with oracle data.
        let flat_spec = ChordSpec {
            root: flat_class,
            quality: quality("major"),
        };
        let sharp_spec = ChordSpec {
            root: expected,
            quality: quality("major"),
        };
        let flat_chord = chord_details(&flat_spec)
            .unwrap_or_else(|error| panic!("{flat}maj must be implemented: {error:?}"));
        let sharp_chord = chord_details(&sharp_spec)
            .unwrap_or_else(|error| panic!("{sharp}maj must be implemented: {error:?}"));
        let flat_notes: Vec<&str> = flat_chord.notes.iter().map(|note| note.name()).collect();
        let sharp_notes: Vec<&str> = sharp_chord.notes.iter().map(|note| note.name()).collect();
        assert_eq!(
            flat_notes, sharp_notes,
            "the real chord notes of {flat} and {sharp} must be identical"
        );
        assert_eq!(
            flat_notes,
            oracle_chord_notes(sharp, "major"),
            "the real chord notes of the {flat} root must be the oracle's {sharp} notes"
        );
    }
}

#[test]
fn unknown_note_names_are_rejected() {
    for name in [
        "H",
        "C##",
        "do",
        "",
        "c",
        "B#",
        "E#",
        "Db ",
        "\t",
        // Unicode and mixed forms: the lookup is exact ASCII, so an
        // implementation that normalises Unicode or accepts double accidentals
        // must still reject every one of these as an unknown identifier.
        "\u{FF23}",  // fullwidth C
        "D\u{266D}", // D + musical flat sign
        "C\u{266F}", // C + musical sharp sign
        "C\u{200B}", // C + zero-width space
        "Cb#",       // double accidental, flat then sharp
        "C#b",       // double accidental, sharp then flat
        "C\u{0301}", // C + combining acute accent
    ] {
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
fn transposition_wraps_offsets_at_the_i32_extremes() {
    // `note_at` takes an `i32` offset. An implementation that computes
    // `base + semitones` in `i32` overflows near the extremes — panicking in
    // debug, or wrapping to a wrong class in release. These offsets, against
    // every base, pin the real modular result, computed independently in `i64`.
    let offsets = [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 5,
        i32::MAX - 12,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 6,
        i32::MIN + 12,
        1_000_000_000,
        -1_000_000_000,
        987_654_321,
        -2_147_000_000,
    ];

    for root in 0..12u8 {
        for offset in offsets {
            assert_eq!(
                u8::from(note_at(pitch_class(root), offset)),
                wrap_large(root, offset),
                "pitch class {root} plus {offset}"
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

    // Compound distances, with their expected labels written out rather than
    // compared against the implementation. The rule comes from plan section 2
    // (and the pinned `lib/fretboard/music/pitch.ex`): a positive multiple of
    // twelve is an `Octave`; every other distance reduces modulo twelve to its
    // simple interval `0..=11` and takes that name from the frozen
    // `interval_names` table. That fixture covers only `0..=11`, so the
    // compound values are pinned explicitly here.
    let compounds = [
        (12_u32, "Octave"),      // 12 * 1
        (13, "Minor 2nd"),       // 13 % 12 == 1
        (14, "Major 2nd"),       // 14 % 12 == 2
        (19, "Perfect 5th"),     // 19 % 12 == 7
        (24, "Octave"),          // 12 * 2
        (25, "Minor 2nd"),       // 25 % 12 == 1
        (36, "Octave"),          // 12 * 3
        (47, "Major 7th"),       // 47 % 12 == 11
        (u32::MAX, "Minor 3rd"), // u32::MAX % 12 == 3
    ];
    for (semitones, expected) in compounds {
        assert_eq!(
            interval_name(semitones),
            expected,
            "the compound interval name of {semitones} semitones"
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
fn every_catalog_quality_answers_with_its_own_chord() {
    // C03 reached this file through the documented handoff, which authorized
    // exactly one quality (`major`) and reported the rest as
    // `CoreError::UnsupportedCapability` (`CORE-D05`). `C06` completes the
    // catalog, so that pending state is over: every quality of the frozen
    // catalog — read from the fixture, not retyped — must answer, and its answer
    // must be the oracle's. An implementation that leaves one quality pending,
    // or that answers one with another quality's chord, is caught here.
    let ids = quality_ids();
    assert!(
        ids.iter().any(|id| id == "major"),
        "the frozen catalog must carry the major quality"
    );

    let mut checked = 0_usize;
    for id in &ids {
        let details = chord_details(&chord(0, id))
            .unwrap_or_else(|error| panic!("{id} must be implemented since C06: {error:?}"));
        let notes: Vec<&str> = details.notes.iter().map(|note| note.name()).collect();
        assert_eq!(notes, oracle_chord_notes("C", id), "the notes of C {id}");
        assert_eq!(
            details.label,
            oracle_chord_label("C", id),
            "the label of C {id}"
        );
        assert_eq!(details.quality.as_str(), id, "the identity of C {id}");
        checked = checked.saturating_add(1);
    }
    assert_eq!(
        checked,
        ids.len(),
        "every catalog quality must be exercised"
    );

    // A quality is not a synonym for another: the same root with two different
    // formulas must not produce the same notes.
    let major = chord_details(&chord(0, "major")).expect("major");
    let minor = chord_details(&chord(0, "minor")).expect("minor");
    assert_ne!(major.notes, minor.notes, "C major and C minor differ");
    assert_ne!(major.label, minor.label, "C major and C minor differ");
}
