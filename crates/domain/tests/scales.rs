//! Task `C15`: the fifteen scales and their inferred diatonic chords, against
//! the frozen oracle.
//!
//! Every record of `fixtures/oracle/scales.jsonl` (555 records) is consumed
//! here. The file carries three operations of the pinned baseline:
//!
//! * `scale_notes/2` (`Fretboard.Music.Scale.scale_notes/2`, 180 records) —
//!   the notes of every tonic and every scale type;
//! * `diatonic_chords/3` (`Fretboard.Music.Scale.diatonic_chords/3`, 360
//!   records) — the inferred diatonic chords of every tonic, every scale type
//!   and both chord modes;
//! * `scale_label/1` (`Fretboard.Music.Scale.scale_label/1`, 15 records) — the
//!   display label of every scale type.
//!
//! The scale types themselves are pinned by `fixtures/oracle/catalogs.json`:
//! `scale_types` carries the identifiers, their labels and the notes from the
//! tonic `C`, and `grouped_scale_types` carries the seven display groups. The
//! remaining group fields of this file — `counts.scale_types` and
//! `counts.grouped_scale_types` — are the declared totals.
//!
//! ## What is asserted, and what is not
//!
//! The formulas are checked against the fixture's own `notes_from_tonic`
//! (converted to semitone offsets with the domain's own note lookup), never
//! against a numeric table retyped from the plan, and the notes of every tonic
//! are checked against `scale_notes/2`. The inference is the interesting half:
//! the baseline classifies the intervals available above **every** scale
//! degree and does not stack thirds by scale index, so these tests pin the
//! classifier's priority order, its exclusions and its fallback on the scales
//! where they decide the answer — the dense chromatic scale (every semitone
//! present) and the non-heptatonic scales (five- and six-note formulas).
//!
//! Nothing here is derived from the implementation under test: each expected
//! value is read from the frozen record it belongs to, and every failure names
//! its case id.

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

use common::{oracle_catalogs, oracle_records};
use fretboard_core::{
    ChordMode, DiatonicChord, PitchClass, ScaleId, diatonic_chords, grouped_scale_types,
    scale_formula, scale_label, scale_notes,
};
use serde_json::Value;

/// The frozen fixture and the record count it must carry
/// (`fixtures/oracle/manifest.json`).
const FIXTURE: &str = "fixtures/oracle/scales.jsonl";
const RECORDS: usize = 555;

/// The number of records each operation of the fixture carries.
const NOTE_RECORDS: usize = 180;
const CHORD_RECORDS: usize = 360;
const LABEL_RECORDS: usize = 15;

/// The twelve tonics and the fifteen scale types of the frozen catalog.
const TONIC_COUNT: usize = 12;
const SCALE_COUNT: usize = 15;

/// The seven display groups of the frozen catalog.
const GROUP_COUNT: usize = 7;

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

/// The records of one operation.
fn records_of(operation: &str) -> Vec<Value> {
    records()
        .into_iter()
        .filter(|record| record["operation"].as_str() == Some(operation))
        .collect()
}

/// The identifier of a record.
fn case_id(record: &Value) -> String {
    record["case_id"]
        .as_str()
        .expect("every record has a case id")
        .to_owned()
}

/// The scale type of a record's input, as a validated catalog identifier.
fn scale_of(record: &Value) -> ScaleId {
    record["input"]["scale_type"]
        .as_str()
        .unwrap_or_else(|| panic!("{}: no scale_type", case_id(record)))
        .parse()
        .unwrap_or_else(|error| panic!("{}: unknown scale type: {error:?}", case_id(record)))
}

/// The tonic of a record's input.
fn tonic_of(record: &Value) -> PitchClass {
    record["input"]["tonic"]
        .as_str()
        .unwrap_or_else(|| panic!("{}: no tonic", case_id(record)))
        .parse()
        .unwrap_or_else(|error| panic!("{}: unknown tonic: {error:?}", case_id(record)))
}

/// The chord mode of a record's input.
fn mode_of(record: &Value) -> ChordMode {
    match record["input"]["mode"].as_str() {
        Some("triad") => ChordMode::Triad,
        Some("seventh") => ChordMode::Seventh,
        other => panic!("{}: unknown chord mode {other:?}", case_id(record)),
    }
}

/// The frozen notes of a `scale_notes` record.
fn frozen_notes(record: &Value) -> Vec<String> {
    record["output"]["notes"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no notes", case_id(record)))
        .iter()
        .map(|note| {
            note.as_str()
                .unwrap_or_else(|| panic!("{}: a note is not a string", case_id(record)))
                .to_owned()
        })
        .collect()
}

/// The frozen diatonic chords of a record, as `(root, quality)` pairs in order.
fn frozen_chords(record: &Value) -> Vec<(String, String)> {
    record["output"]["chords"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no chords", case_id(record)))
        .iter()
        .map(|chord| {
            let root = chord["root"]
                .as_str()
                .unwrap_or_else(|| panic!("{}: a chord has no root", case_id(record)));
            let quality = chord["quality"]
                .as_str()
                .unwrap_or_else(|| panic!("{}: a chord has no quality", case_id(record)));
            (root.to_owned(), quality.to_owned())
        })
        .collect()
}

/// The native notes of a record, in the fixture's spelling.
fn our_notes(record: &Value) -> Vec<String> {
    scale_notes(tonic_of(record), scale_of(record))
        .iter()
        .map(|note| note.name().to_owned())
        .collect()
}

/// The native diatonic chords of a record, as `(root, quality)` pairs in order.
fn our_chords(record: &Value) -> Vec<(String, String)> {
    diatonic_chords(tonic_of(record), scale_of(record), mode_of(record))
        .iter()
        .map(|chord| {
            (
                chord.root.name().to_owned(),
                chord.quality.as_str().to_owned(),
            )
        })
        .collect()
}

/// The formula of every catalog scale, as the fixture's own `notes_from_tonic`
/// converted to semitone offsets with the domain's note lookup.
///
/// The fixture exports the notes from the tonic `C` only; the offsets are the
/// distance of each exported note name from `C`, which is the definition of a
/// scale formula.
fn frozen_formulas() -> Vec<(String, Vec<u8>)> {
    oracle_catalogs()["scale_types"]
        .as_array()
        .expect("scale_types must be an array")
        .iter()
        .map(|entry| {
            let id = entry["id"]
                .as_str()
                .expect("a scale type has an id")
                .to_owned();
            let formula = entry["notes_from_tonic"]
                .as_array()
                .expect("a scale type carries notes_from_tonic")
                .iter()
                .map(|note| {
                    let name = note.as_str().expect("a note name");
                    let pitch = name
                        .parse::<PitchClass>()
                        .unwrap_or_else(|error| panic!("{id}: {name} is not a note: {error:?}"));
                    u8::from(pitch)
                })
                .collect();
            (id, formula)
        })
        .collect()
}

/// The frozen display groups, as `(group, scale types)` pairs in order.
fn frozen_groups() -> Vec<(String, Vec<String>)> {
    oracle_catalogs()["grouped_scale_types"]
        .as_array()
        .expect("grouped_scale_types must be an array")
        .iter()
        .map(|entry| {
            let group = entry["group"].as_str().expect("a group name").to_owned();
            let scales = entry["scale_types"]
                .as_array()
                .expect("a group carries its scale types")
                .iter()
                .map(|scale| scale.as_str().expect("a scale type identifier").to_owned())
                .collect();
            (group, scales)
        })
        .collect()
}

#[test]
fn the_frozen_fixture_carries_every_scale_case() {
    let records = records();
    assert_eq!(records_of("scale_notes").len(), NOTE_RECORDS);
    assert_eq!(records_of("diatonic_chords").len(), CHORD_RECORDS);
    assert_eq!(records_of("scale_label").len(), LABEL_RECORDS);

    let mut ids: BTreeSet<String> = BTreeSet::new();
    for record in &records {
        assert!(ids.insert(case_id(record)), "duplicate case id");
    }
    assert_eq!(ids.len(), RECORDS, "the case ids are unique");

    // The reference count the whole task hangs on: every tonic, every scale
    // type and both modes, exactly once.
    let mut note_triples: BTreeSet<(String, String)> = BTreeSet::new();
    for record in records_of("scale_notes") {
        note_triples.insert((tonic_of(&record).name().to_owned(), case_id(&record)));
        assert!(
            note_triples.len() <= TONIC_COUNT * SCALE_COUNT,
            "more scale_notes cases than tonics times scales"
        );
    }
    let mut chord_triples: BTreeSet<(String, String, String)> = BTreeSet::new();
    for record in records_of("diatonic_chords") {
        chord_triples.insert((
            tonic_of(&record).name().to_owned(),
            scale_of(&record).as_str().to_owned(),
            mode_of(&record).as_str().to_owned(),
        ));
    }
    assert_eq!(chord_triples.len(), CHORD_RECORDS);
    assert_eq!(chord_triples.len(), TONIC_COUNT * SCALE_COUNT * 2);

    let tonics: BTreeSet<String> = chord_triples
        .iter()
        .map(|(tonic, _, _)| tonic.clone())
        .collect();
    let scales: BTreeSet<String> = chord_triples
        .iter()
        .map(|(_, scale, _)| scale.clone())
        .collect();
    let modes: BTreeSet<String> = chord_triples
        .iter()
        .map(|(_, _, mode)| mode.clone())
        .collect();
    assert_eq!(tonics.len(), TONIC_COUNT, "every tonic is covered");
    assert_eq!(scales.len(), SCALE_COUNT, "every scale type is covered");
    assert_eq!(modes.len(), 2, "both chord modes are covered");
}

#[test]
fn the_scale_catalog_matches_the_frozen_export() {
    // Every identifier of the fixture is one the crate accepts, and the
    // crate's own exhaustive list is exactly that set.
    let frozen = frozen_formulas();
    assert_eq!(frozen.len(), SCALE_COUNT);
    let accepted: BTreeSet<String> = ScaleId::ALL
        .iter()
        .map(|scale| scale.as_str().to_owned())
        .collect();
    for (id, _) in &frozen {
        let scale: ScaleId = id
            .parse()
            .unwrap_or_else(|error| panic!("{id} is not an accepted scale type: {error:?}"));
        assert_eq!(scale.as_str(), id);
        assert!(accepted.contains(id), "{id} is missing from ScaleId::ALL");
    }
    assert_eq!(accepted.len(), SCALE_COUNT, "no extra scale identifiers");

    // The formulas are the fixture's own notes from C, as offsets.
    for (id, formula) in &frozen {
        let scale: ScaleId = id.parse().expect("an accepted scale type");
        assert_eq!(
            scale_formula(scale),
            formula.as_slice(),
            "{id}: the formula differs from the fixture's notes from C"
        );
        assert_eq!(formula[0], 0, "{id}: every formula starts at its tonic");
    }
}

#[test]
fn the_scale_groups_match_the_frozen_export() {
    let frozen = frozen_groups();
    assert_eq!(frozen.len(), GROUP_COUNT);

    let ours = grouped_scale_types();
    assert_eq!(ours.len(), GROUP_COUNT, "the crate carries every group");
    for ((group, scales), entry) in frozen.iter().zip(ours) {
        assert_eq!(entry.group, group.as_str(), "{group}: group name");
        let ours: Vec<&str> = entry.scales.iter().map(|scale| scale.as_str()).collect();
        let theirs: Vec<&str> = scales.iter().map(String::as_str).collect();
        assert_eq!(ours, theirs, "{group}: group members and order");
    }

    // The groups partition the catalog: every scale type appears exactly once.
    let mut seen: Vec<&str> = Vec::new();
    for entry in ours {
        for scale in entry.scales {
            assert!(
                !seen.contains(&scale.as_str()),
                "{} appears in two groups",
                scale.as_str()
            );
            seen.push(scale.as_str());
        }
    }
    assert_eq!(seen.len(), SCALE_COUNT, "the groups cover every scale type");
    assert_eq!(
        common::oracle_count("grouped_scale_types"),
        u64::try_from(GROUP_COUNT).expect("the group count fits in u64")
    );
}

#[test]
fn every_scale_label_answers_its_frozen_record() {
    let records = records_of("scale_label");
    assert_eq!(records.len(), LABEL_RECORDS);
    let mut covered: BTreeSet<String> = BTreeSet::new();
    for record in &records {
        let id = record["input"]["scale_type"]
            .as_str()
            .unwrap_or_else(|| panic!("{}: no scale_type", case_id(record)));
        let scale: ScaleId = id
            .parse()
            .unwrap_or_else(|error| panic!("{}: {error:?}", case_id(record)));
        let frozen = record["output"]["label"]
            .as_str()
            .unwrap_or_else(|| panic!("{}: no label", case_id(record)));
        assert_eq!(
            scale_label(scale),
            frozen,
            "{}: the label differs",
            case_id(record)
        );
        assert!(covered.insert(id.to_owned()));
    }
    assert_eq!(covered.len(), SCALE_COUNT, "every scale type is labelled");

    // The catalog's own labels agree with the label operation's records.
    for (id, _) in frozen_formulas() {
        let scale: ScaleId = id.parse().expect("an accepted scale type");
        let record = records
            .iter()
            .find(|record| record["input"]["scale_type"].as_str() == Some(id.as_str()))
            .unwrap_or_else(|| panic!("{id}: no label record"));
        assert_eq!(
            scale_label(scale),
            record["output"]["label"].as_str().unwrap()
        );
    }
}

#[test]
fn every_tonic_of_every_scale_answers_its_frozen_notes() {
    let records = records_of("scale_notes");
    assert_eq!(records.len(), NOTE_RECORDS);
    let mut covered: BTreeSet<(String, String)> = BTreeSet::new();
    for record in &records {
        let case = case_id(record);
        assert_eq!(
            our_notes(record),
            frozen_notes(record),
            "{case}: the scale notes differ"
        );
        covered.insert((
            tonic_of(record).name().to_owned(),
            scale_of(record).as_str().to_owned(),
        ));
    }
    assert_eq!(
        covered.len(),
        TONIC_COUNT * SCALE_COUNT,
        "every tonic and scale type is covered"
    );
}

#[test]
fn every_tonic_of_every_scale_answers_its_frozen_diatonic_chords() {
    let records = records_of("diatonic_chords");
    assert_eq!(records.len(), CHORD_RECORDS);
    let mut covered: BTreeSet<(String, String, String)> = BTreeSet::new();
    for record in &records {
        let case = case_id(record);
        assert_eq!(
            our_chords(record),
            frozen_chords(record),
            "{case}: the diatonic chords differ"
        );
        covered.insert((
            tonic_of(record).name().to_owned(),
            scale_of(record).as_str().to_owned(),
            mode_of(record).as_str().to_owned(),
        ));
    }
    assert_eq!(
        covered.len(),
        TONIC_COUNT * SCALE_COUNT * 2,
        "every tonic, scale type and mode is covered"
    );
}

#[test]
fn the_dense_chromatic_scale_pins_the_triad_priority() {
    // The chromatic scale carries all twelve semitones, so every triad rule
    // could match; the first one (4/7, major) is the answer on every degree and
    // the fixture says so. A classifier that consults a later rule, or that
    // stacks thirds by scale index, cannot reproduce this.
    let records = records_of("diatonic_chords");
    let record = records
        .iter()
        .find(|record| record["case_id"].as_str() == Some("diatonic_chords/C/chromatic/triad"))
        .expect("the fixture carries the chromatic triad case");
    let frozen = frozen_chords(record);
    assert_eq!(frozen.len(), TONIC_COUNT, "one chord per semitone");
    assert!(
        frozen.iter().all(|(_, quality)| quality == "major"),
        "the dense scale keeps the first triad rule on every degree"
    );
    assert_eq!(our_chords(record), frozen, "the dense triad answer differs");
}

#[test]
fn the_dense_chromatic_scale_pins_the_seventh_exclusions() {
    // dim7 (3/6/9) and m7b5 (3/6/10) both require the diminished fifth and are
    // excluded when the scale also carries the perfect fifth (7); the chromatic
    // scale carries it, so the answer on every degree is min7 and dim7 appears
    // nowhere. Without the exclusion the dense scale would answer dim7.
    let records = records_of("diatonic_chords");
    let record = records
        .iter()
        .find(|record| record["case_id"].as_str() == Some("diatonic_chords/C/chromatic/seventh"))
        .expect("the fixture carries the chromatic seventh case");
    let frozen = frozen_chords(record);
    assert_eq!(frozen.len(), TONIC_COUNT, "one chord per semitone");
    assert!(
        frozen
            .iter()
            .all(|(_, quality)| !quality.starts_with("dim") && quality != "m7b5"),
        "the excluded fifth keeps dim7 and m7b5 out of the dense answer"
    );
    assert!(
        frozen.iter().all(|(_, quality)| quality == "min7"),
        "the dense scale answers min7 on every degree"
    );
    assert_eq!(
        our_chords(record),
        frozen,
        "the dense seventh answer differs"
    );
}

#[test]
fn seventh_mode_falls_back_to_the_triad_classifier() {
    // The blues scale has six notes and no seventh rule matches on some of its
    // degrees, so the answer falls back to the triad classifier. The fixture
    // carries both kinds of answer in one record: a quality that carries a
    // seventh and one that does not.
    let records = records_of("diatonic_chords");
    let record = records
        .iter()
        .find(|record| record["case_id"].as_str() == Some("diatonic_chords/C/blues/seventh"))
        .expect("the fixture carries the blues seventh case");
    let frozen = frozen_chords(record);
    assert_eq!(frozen.len(), 6, "the blues scale has six degrees");
    let with_seventh = frozen.iter().filter(|(_, quality)| {
        let quality: fretboard_core::QualityId = quality.parse().expect("a catalog quality");
        fretboard_core::chord_formula(quality).contains(&10)
            || fretboard_core::chord_formula(quality).contains(&11)
    });
    assert!(
        with_seventh.count() > 0,
        "the record carries a seventh quality"
    );
    assert!(
        frozen.iter().any(|(_, quality)| {
            let quality: fretboard_core::QualityId = quality.parse().expect("a catalog quality");
            !fretboard_core::chord_formula(quality).contains(&10)
                && !fretboard_core::chord_formula(quality).contains(&11)
        }),
        "and at least one triad quality, which is the fallback"
    );
    assert_eq!(
        our_chords(record),
        frozen,
        "the blues seventh answer differs"
    );
}

#[test]
fn the_non_heptatonic_scales_answer_their_frozen_chords() {
    // Five- and six-note formulas, both modes, every tonic: the part of the
    // task the baseline's classifier has to get right without a third, a fifth
    // or a seventh to build on.
    let records = records_of("diatonic_chords");
    let non_heptatonic = [
        "pentatonic_major",
        "pentatonic_minor",
        "blues",
        "whole_tone",
        "chromatic",
    ];
    let mut checked = 0usize;
    for record in &records {
        let scale = scale_of(record);
        if !non_heptatonic.contains(&scale.as_str()) {
            continue;
        }
        assert_ne!(
            scale_formula(scale).len(),
            7,
            "{}: {scale} is not heptatonic",
            case_id(record)
        );
        assert_eq!(
            our_chords(record),
            frozen_chords(record),
            "{}: the answer differs",
            case_id(record)
        );
        checked += 1;
    }
    assert_eq!(
        checked,
        non_heptatonic.len() * TONIC_COUNT * 2,
        "every non-heptatonic case is covered"
    );
}

#[test]
fn a_scale_without_a_third_answers_sus2() {
    // The pentatonic major scale carries no third above two of its degrees; the
    // fixture answers sus2 there and the triad classifier is not a stack of
    // thirds. Every frozen record, and the sus2 answers among them, are read
    // from the fixture rather than typed here.
    let records = records_of("diatonic_chords");
    let record = records
        .iter()
        .find(|record| {
            record["case_id"].as_str() == Some("diatonic_chords/C/pentatonic_major/triad")
        })
        .expect("the fixture carries the pentatonic major triad case");
    let frozen = frozen_chords(record);
    assert_eq!(frozen.len(), 5, "the pentatonic major scale has five notes");
    assert!(
        frozen.iter().any(|(_, quality)| quality == "sus2"),
        "the fixture answers sus2 where the scale has no third"
    );
    assert_eq!(
        our_chords(record),
        frozen,
        "the pentatonic major triad answer differs"
    );

    // The whole fixture, so a suspended answer anywhere else is checked too.
    let mut suspended = 0usize;
    for record in &records {
        for (_, quality) in frozen_chords(record) {
            if quality == "sus2" || quality == "sus4" {
                suspended += 1;
            }
        }
        assert_eq!(
            our_chords(record),
            frozen_chords(record),
            "{}: the answer differs",
            case_id(record)
        );
    }
    assert!(suspended > 0, "the fixture carries suspended answers");
}

#[test]
fn the_exclusion_only_applies_when_the_perfect_fifth_is_present() {
    // The diminished seventh needs the excluded perfect fifth to be absent: the
    // harmonic minor and the phrygian dominant scales carry dim7 somewhere
    // (their degrees have the diminished fifth and no perfect fifth), while the
    // chromatic scale carries none because every degree also has the perfect
    // fifth. Both halves come from the fixture.
    let records = records_of("diatonic_chords");
    let mut with_dim7: BTreeSet<String> = BTreeSet::new();
    let mut without_dim7: BTreeSet<String> = BTreeSet::new();
    for record in &records {
        let qualities: BTreeSet<String> = frozen_chords(record)
            .into_iter()
            .map(|(_, quality)| quality)
            .collect();
        let scale = scale_of(record).as_str().to_owned();
        if qualities.contains("dim7") {
            with_dim7.insert(scale);
        } else {
            without_dim7.insert(scale);
        }
        assert_eq!(
            our_chords(record),
            frozen_chords(record),
            "{}: the answer differs",
            case_id(record)
        );
    }
    assert!(
        with_dim7.contains("harmonic_minor"),
        "the harmonic minor keeps dim7 where the perfect fifth is absent"
    );
    assert!(
        without_dim7.contains("chromatic"),
        "the dense scale drops dim7 where the perfect fifth is present"
    );
}

#[test]
fn every_diatonic_chord_of_a_record_is_a_catalog_quality() {
    // The classifier answers catalog identifiers, and their order is the
    // formula order of the scale: the roots of a record are the scale's own
    // notes, in order, which is what ties the inference to the notes above.
    for record in records_of("diatonic_chords") {
        let case = case_id(&record);
        let notes = scale_notes(tonic_of(&record), scale_of(&record));
        let chords: Vec<DiatonicChord> =
            diatonic_chords(tonic_of(&record), scale_of(&record), mode_of(&record));
        assert_eq!(chords.len(), notes.len(), "{case}: one chord per degree");
        assert!(
            !chords.is_empty(),
            "{case}: a scale has at least one degree"
        );
        for (chord, note) in chords.iter().zip(&notes) {
            assert_eq!(chord.root, *note, "{case}: the roots follow the notes");
            assert!(
                fretboard_core::QualityId::ALL.contains(&chord.quality),
                "{case}: {} is not a catalog quality",
                chord.quality.as_str()
            );
        }
    }
}
