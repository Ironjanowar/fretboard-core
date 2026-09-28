// Test target: `expect`/`unwrap`, panicking assertions and direct indexing are the
// idiom in tests, so the restriction lints that forbid them in the library are
// relaxed here only. Every other lint, including `pedantic`, still applies.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

//! The complete chord-quality catalog (P2, task C06).
//!
//! These tests pin the domain against the frozen oracle only: the 47 quality
//! identifiers, their formulas, their display suffixes, their contextual
//! interval labels, the note/label pairs of every root, the eight UI groups and
//! the triad/seventh mode inference come from `fixtures/oracle/catalogs.json`
//! and `fixtures/oracle/chords.jsonl`, never from the implementation.
//!
//! Nothing here is transcribed by hand from the plan: the quality list, the
//! formulas and the interval labels are read record by record, so a
//! mistranscribed table cannot pass. The single exception is the current
//! seventh-quality set used by `infer_chord_mode`, whose expectations are read
//! from the oracle's own `infer_chord_mode/*` records.
//!
//! `Contract.D01` (the zipped note/interval pairs) is *not* decided here: the
//! pairs are reproduced exactly as the baseline zips them, raw, and the
//! decision stays open.

mod common;

use std::collections::BTreeSet;

use common::{
    assert_error_code, chord, oracle_catalogs, oracle_chord_interval_labels,
    oracle_chord_interval_pairs, oracle_chord_label, oracle_chord_notes,
    oracle_chord_quality_label, oracle_count, oracle_records, quality, quality_formula,
    quality_ids, string_array,
};
use fretboard_core::{
    ChordMode, ChordSpec, chord_details, chord_formula, chord_interval_labels, chord_quality_label,
    grouped_qualities, infer_chord_mode, note_index,
};

/// Every `case_id` suffix of one operation in the frozen chord oracle, in file
/// order, e.g. `("chord_notes", "C:major")`.
fn oracle_cases(operation: &str) -> Vec<(String, String)> {
    oracle_records("fixtures/oracle/chords.jsonl")
        .into_iter()
        .filter_map(|record| {
            let case_id = record["case_id"].as_str().expect("a case id").to_string();
            let (op, key) = case_id.split_once('/')?;
            (op == operation).then(|| (case_id.clone(), key.to_string()))
        })
        .collect()
}

/// The specification of one chord entry of an oracle record — a record's own
/// `input.root`/`input.quality`, or one element of its `active_chords`: its root
/// resolved through the domain's own note lookup (which also accepts the seven
/// flat spellings), and its quality identifier.
fn chord_spec_of(value: &serde_json::Value) -> ChordSpec {
    let root = value["root"].as_str().expect("a chord root");
    let quality_id = value["quality"].as_str().expect("a quality id");
    let root_class = note_index(root)
        .unwrap_or_else(|error| panic!("the oracle root {root} must resolve: {error:?}"));
    ChordSpec {
        root: root_class,
        quality: quality(quality_id),
    }
}

// ---------------------------------------------------------------------------
// Identifiers, formulas, labels
// ---------------------------------------------------------------------------

#[test]
fn every_catalog_quality_identifier_is_accepted_in_oracle_order() {
    let oracle = quality_ids();
    assert_eq!(
        oracle.len(),
        usize::try_from(oracle_count("chord_qualities")).expect("a quality count"),
        "the frozen catalog declares 47 qualities"
    );

    // The catalog order is the Elixir `Map.keys/1 |> Enum.sort/1` order, which
    // is the sorted identifier order; the accepted set must match it exactly.
    let accepted: Vec<String> = fretboard_core::QualityId::ALL
        .iter()
        .map(|quality| quality.as_str().to_string())
        .collect();
    assert_eq!(
        accepted, oracle,
        "the accepted catalog identifiers, in catalog order"
    );

    let mut sorted = oracle.clone();
    sorted.sort();
    assert_eq!(sorted, oracle, "the catalog order is the sorted order");

    for id in &oracle {
        assert_eq!(
            quality(id).as_str(),
            id,
            "every catalog identifier must parse and keep its string"
        );
    }
}

#[test]
fn every_quality_formula_matches_the_oracle_catalog() {
    for id in quality_ids() {
        let expected = quality_formula(&id);
        let actual = chord_formula(quality(&id));
        assert_eq!(
            actual,
            expected.as_slice(),
            "the formula of {id} must be the frozen catalog formula"
        );
        assert!(
            !actual.is_empty(),
            "every catalog quality carries at least one interval"
        );
        assert_eq!(
            actual.first(),
            Some(&0),
            "every formula starts at the chord root"
        );
    }
}

#[test]
fn every_quality_formula_matches_its_own_oracle_record() {
    let cases = oracle_cases("chord_formula");
    assert_eq!(cases.len(), 47, "one formula record per catalog quality");

    for (case_id, key) in cases {
        let record = oracle_records("fixtures/oracle/chords.jsonl")
            .into_iter()
            .find(|record| record["case_id"].as_str() == Some(case_id.as_str()))
            .expect("the record exists");
        let expected: Vec<u8> = record["output"]["formula"]
            .as_array()
            .expect("a formula array")
            .iter()
            .map(|interval| {
                u8::try_from(interval.as_u64().expect("an integer interval"))
                    .expect("the interval fits in u8")
            })
            .collect();
        assert_eq!(
            chord_formula(quality(&key)),
            expected.as_slice(),
            "the formula of {key} must equal its oracle record"
        );
    }
}

#[test]
fn every_quality_label_matches_the_oracle() {
    for id in quality_ids() {
        assert_eq!(
            chord_quality_label(quality(&id)),
            oracle_chord_quality_label(&id),
            "the display suffix of {id} must be the frozen label"
        );
    }
}

#[test]
fn every_quality_interval_labels_match_the_oracle_catalog() {
    for id in quality_ids() {
        let actual: Vec<String> = chord_interval_labels(quality(&id))
            .iter()
            .map(|label| (*label).to_string())
            .collect();
        assert_eq!(
            actual,
            oracle_chord_interval_labels(&id),
            "the interval labels of {id}, in label order"
        );
        assert_eq!(
            actual.len(),
            chord_formula(quality(&id)).len(),
            "every formula interval carries exactly one label"
        );
    }
}

// ---------------------------------------------------------------------------
// Root output: notes, labels and pairs
// ---------------------------------------------------------------------------

#[test]
fn every_oracle_chord_notes_record_is_reproduced() {
    let cases = oracle_cases("chord_notes");
    assert_eq!(cases.len(), 893, "the frozen note records");

    let mut roots = BTreeSet::new();
    for (_, key) in cases {
        let (root, quality_id) = key
            .split_once(':')
            .unwrap_or_else(|| panic!("a chord case key is root:quality, got {key}"));
        roots.insert(root.to_string());
        let details = chord_details(&chord(
            u8::from(
                note_index(root)
                    .unwrap_or_else(|error| panic!("the oracle root {root} fails: {error:?}")),
            ),
            quality_id,
        ))
        .unwrap_or_else(|error| panic!("{key} must be implemented: {error:?}"));
        let notes: Vec<String> = details
            .notes
            .iter()
            .map(|note| note.name().to_string())
            .collect();
        assert_eq!(
            notes,
            oracle_chord_notes(root, quality_id),
            "the notes of {key}"
        );
        assert_eq!(
            details.quality.as_str(),
            quality_id,
            "the details keep the requested quality identity"
        );
        assert_eq!(
            u8::from(details.root),
            u8::from(note_index(root).unwrap_or_else(|error| panic!("{root}: {error:?}"))),
            "the details keep the resolved root"
        );
    }

    assert_eq!(roots.len(), 19, "the oracle pins 19 root spellings");
}

#[test]
fn every_oracle_chord_label_record_is_reproduced() {
    let cases = oracle_cases("chord_label");
    assert_eq!(cases.len(), 564, "the frozen label records: 12 roots");

    let mut roots = BTreeSet::new();
    for (_, key) in cases {
        let (root, quality_id) = key
            .split_once(':')
            .unwrap_or_else(|| panic!("a chord case key is root:quality, got {key}"));
        roots.insert(root.to_string());
        let details = chord_details(&chord(
            u8::from(
                note_index(root)
                    .unwrap_or_else(|error| panic!("the oracle root {root} fails: {error:?}")),
            ),
            quality_id,
        ))
        .unwrap_or_else(|error| panic!("{key} must be implemented: {error:?}"));
        assert_eq!(
            details.label,
            oracle_chord_label(root, quality_id),
            "the full label of {key}"
        );
    }

    assert_eq!(roots.len(), 12, "the oracle pins 12 sharp/natural roots");
}

#[test]
fn every_oracle_notes_with_intervals_pair_is_reproduced() {
    // `Contract.D01`: the baseline zips formula-order notes with the
    // independently ordered interval labels. That is what this test pins,
    // raw — no ordering is "fixed" here.
    let cases = oracle_cases("notes_with_intervals");
    assert_eq!(cases.len(), 893, "the frozen pair records");

    for (_, key) in cases {
        let (root, quality_id) = key
            .split_once(':')
            .unwrap_or_else(|| panic!("a chord case key is root:quality, got {key}"));
        let details = chord_details(&chord(
            u8::from(
                note_index(root)
                    .unwrap_or_else(|error| panic!("the oracle root {root} fails: {error:?}")),
            ),
            quality_id,
        ))
        .unwrap_or_else(|error| panic!("{key} must be implemented: {error:?}"));

        let actual: Vec<(String, String)> = details
            .notes
            .iter()
            .map(|note| note.name().to_string())
            .zip(details.intervals.iter().map(|label| (*label).to_string()))
            .collect();
        assert_eq!(
            actual,
            oracle_chord_interval_pairs(root, quality_id),
            "the zipped (note, label) pairs of {key}"
        );
    }
}

#[test]
fn every_catalog_quality_is_implemented() {
    // After C06 no catalog quality is pending: every one of the 47 answers with
    // its own chord. An unimplemented capability must not be reported for a
    // quality the catalog carries.
    for id in quality_ids() {
        let details = chord_details(&chord(0, &id))
            .unwrap_or_else(|error| panic!("{id} must be implemented: {error:?}"));
        assert_eq!(
            details.notes.len(),
            quality_formula(&id).len(),
            "{id} names one note per formula interval"
        );
        assert_eq!(
            details.intervals.len(),
            details.notes.len(),
            "{id} carries one interval label per note"
        );
    }
}

#[test]
fn a_known_quality_never_reports_a_missing_capability() {
    for id in ["min7", "sus4", "dim", "7", "maj7", "13", "susb9"] {
        match chord_details(&chord(0, id)) {
            Ok(details) => assert!(!details.label.is_empty(), "{id} must carry a label"),
            Err(error) => panic!("{id} is a catalog quality and must answer: {error:?}"),
        }
    }
}

#[test]
fn an_unknown_quality_string_is_still_rejected() {
    // `QualityId` is validated at parse time, so an unknown quality cannot even
    // be carried by a `ChordSpec`; the rejection belongs to the identifier.
    for unknown in ["bogus", "m11b6", "MAJOR", "", "major ", "7#11#5"] {
        assert_error_code(
            fretboard_core::QualityId::parse(unknown),
            "UnknownIdentifier",
        );
    }
}

// ---------------------------------------------------------------------------
// Interval-label ordering rule
// ---------------------------------------------------------------------------

#[test]
fn interval_labels_keep_formula_order_without_a_seventh() {
    // The source's rule: without a 7th the extension has not "earned" compound
    // placement, so labels stay in formula order.
    let cases: &[(&str, &[&str])] = &[
        ("major", &["Root", "Major 3rd", "Perfect 5th"]),
        ("minor", &["Root", "Minor 3rd", "Perfect 5th"]),
        ("sus2", &["Root", "Major 2nd", "Perfect 5th"]),
        ("add9", &["Root", "Major 2nd", "Major 3rd", "Perfect 5th"]),
        ("maj6", &["Root", "Major 3rd", "Perfect 5th", "Major 6th"]),
        ("dim", &["Root", "Minor 3rd", "Tritone"]),
        ("aug", &["Root", "Major 3rd", "Augmented 5th"]),
    ];
    for (quality_id, expected) in cases {
        assert_eq!(
            chord_interval_labels(quality(quality_id)),
            *expected,
            "the label order of {quality_id}"
        );
    }
}

#[test]
fn interval_labels_are_sorted_into_chord_member_order_with_a_seventh() {
    // With a 7th the labels are sorted by chord-member rank and then by
    // semitone: root, 3rd, 5th, 7th, 9th, 11th, 13th. Semitone 8 is ranked as
    // the 13th it is named after.
    let cases: &[(&str, &[&str])] = &[
        (
            "7#11",
            &[
                "Root",
                "Major 3rd",
                "Augmented 11th",
                "Perfect 5th",
                "Minor 7th",
            ],
        ),
        (
            "7b13",
            &[
                "Root",
                "Major 3rd",
                "Perfect 5th",
                "Minor 7th",
                "Minor 13th",
            ],
        ),
        (
            "susb9",
            &[
                "Root",
                "Perfect 5th",
                "Minor 7th",
                "Flat 9th",
                "Perfect 11th",
            ],
        ),
        (
            "min_maj7",
            &["Root", "Minor 3rd", "Perfect 5th", "Major 7th"],
        ),
        (
            "11",
            &[
                "Root",
                "Major 3rd",
                "Perfect 5th",
                "Minor 7th",
                "Major 9th",
                "Perfect 11th",
            ],
        ),
    ];
    for (quality_id, expected) in cases {
        assert_eq!(
            chord_interval_labels(quality(quality_id)),
            *expected,
            "the sorted label order of {quality_id}"
        );
    }
}

#[test]
fn a_seventh_is_never_the_diminished_seventh_for_label_purposes() {
    // `has_seventh?/1` is `10 in formula or 11 in formula`: the diminished
    // seventh (9, as in `dim7`) does not switch the labels to compound names.
    assert_eq!(
        chord_interval_labels(quality("dim7")),
        ["Root", "Minor 3rd", "Tritone", "Major 6th"],
        "dim7 carries no 7th by the source's definition"
    );
    assert_eq!(
        chord_interval_labels(quality("dim7b13")),
        ["Root", "Minor 3rd", "Tritone", "Minor 13th", "Major 6th"],
        "dim7b13 keeps formula order: its ninth is a sixth, not a seventh"
    );
}

// ---------------------------------------------------------------------------
// Groups
// ---------------------------------------------------------------------------

#[test]
fn grouped_qualities_match_the_oracle_exactly() {
    let oracle = oracle_catalogs();
    let groups = oracle["grouped_qualities"]
        .as_array()
        .expect("grouped_qualities must be an array");
    assert_eq!(
        groups.len(),
        usize::try_from(oracle_count("grouped_qualities")).expect("a group count")
    );

    let actual: Vec<(String, Vec<String>)> = grouped_qualities()
        .iter()
        .map(|group| {
            (
                group.group.to_string(),
                group
                    .qualities
                    .iter()
                    .map(|quality| quality.as_str().to_string())
                    .collect(),
            )
        })
        .collect();

    let expected: Vec<(String, Vec<String>)> = groups
        .iter()
        .map(|group| {
            (
                group["group"].as_str().expect("a group name").to_string(),
                string_array(group, "qualities"),
            )
        })
        .collect();

    assert_eq!(actual, expected, "the eight UI groups, in order");

    // The groups partition the catalog: every quality appears exactly once.
    let mut seen: Vec<&str> = grouped_qualities()
        .iter()
        .flat_map(|group| group.qualities.iter().map(|quality| quality.as_str()))
        .collect();
    let total = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), total, "no quality may appear in two groups");
    assert_eq!(
        total,
        quality_ids().len(),
        "every catalog quality appears in exactly one group"
    );
}

// ---------------------------------------------------------------------------
// Chord-mode inference
// ---------------------------------------------------------------------------

#[test]
fn infer_chord_mode_matches_every_oracle_record() {
    let records: Vec<serde_json::Value> = oracle_records("fixtures/oracle/chords.jsonl")
        .into_iter()
        .filter(|record| {
            record["case_id"]
                .as_str()
                .is_some_and(|case_id| case_id.starts_with("infer_chord_mode/"))
        })
        .collect();
    assert_eq!(records.len(), 52, "the frozen inference records");

    for record in records {
        let case_id = record["case_id"].as_str().expect("a case id");
        let chords: Vec<ChordSpec> = record["input"]["active_chords"]
            .as_array()
            .unwrap_or_else(|| panic!("{case_id} carries active chords"))
            .iter()
            .map(chord_spec_of)
            .collect();
        let expected = match record["output"]["mode"]
            .as_str()
            .unwrap_or_else(|| panic!("{case_id} carries a mode"))
        {
            "triad" => ChordMode::Triad,
            "seventh" => ChordMode::Seventh,
            other => panic!("unknown oracle mode {other}"),
        };
        assert_eq!(infer_chord_mode(&chords), expected, "the mode of {case_id}");
    }
}

#[test]
fn infer_chord_mode_is_triad_without_an_explicit_seventh_quality() {
    // Extended and suspended qualities deliberately do not opt into seventh
    // mode; only the eight classic seventh qualities do.
    let sevenths = [
        "7", "maj7", "min7", "dim7", "m7b5", "min_maj7", "aug_maj7", "aug7",
    ];
    for id in quality_ids() {
        let mode = infer_chord_mode(&[chord(0, &id)]);
        let expected = if sevenths.contains(&id.as_str()) {
            ChordMode::Seventh
        } else {
            ChordMode::Triad
        };
        assert_eq!(mode, expected, "the mode of the single chord {id}");
    }
}

#[test]
fn infer_chord_mode_ignores_order_duplicates_and_roots() {
    assert_eq!(infer_chord_mode(&[]), ChordMode::Triad);
    assert_eq!(
        infer_chord_mode(&[chord(0, "major"), chord(7, "major"), chord(2, "minor")]),
        ChordMode::Triad
    );
    assert_eq!(
        infer_chord_mode(&[chord(0, "major"), chord(0, "major")]),
        ChordMode::Triad
    );
    assert_eq!(
        infer_chord_mode(&[chord(9, "major"), chord(2, "min7"), chord(9, "major")]),
        ChordMode::Seventh,
        "one seventh quality anywhere in the list is enough"
    );
    assert_eq!(
        infer_chord_mode(&[chord(2, "min7"), chord(9, "major")]),
        infer_chord_mode(&[chord(9, "major"), chord(2, "min7")]),
        "the mode does not depend on the list order"
    );
}

#[test]
fn chord_modes_have_their_frozen_wire_strings() {
    assert_eq!(ChordMode::Triad.as_str(), "triad");
    assert_eq!(ChordMode::Seventh.as_str(), "seventh");
}

// ---------------------------------------------------------------------------
// Catalog integrity
// ---------------------------------------------------------------------------

#[test]
fn the_catalog_formula_enumeration_order_is_not_the_sorted_order() {
    // The baseline's `Map.to_list(@formulas)` order is VM-dependent (it is what
    // blocks `Contract.D03`). The accepted identifier order is the *sorted*
    // one, and the two must not be confused.
    let enumeration: Vec<String> = oracle_catalogs()["chord_formula_enumeration_order"]
        .as_array()
        .expect("the oracle exports the enumeration order")
        .iter()
        .map(|id| id.as_str().expect("a quality id").to_string())
        .collect();
    assert_eq!(enumeration.len(), 47);
    assert_ne!(
        enumeration,
        quality_ids(),
        "the enumeration order is not the sorted catalog order"
    );

    let mut recreated: Vec<String> = enumeration;
    recreated.sort();
    assert_eq!(
        recreated,
        quality_ids(),
        "both orders cover exactly the same 47 identifiers"
    );
}
