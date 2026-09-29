//! Task `C18`: the frozen progression catalog and `progression_chords/2`.
//!
//! Every expectation here is read from the frozen oracle, never retyped from the
//! plan or from the ported source:
//!
//! * `fixtures/oracle/progressions.jsonl` carries the whole operation surface the
//!   pinned catalog answers — 59 `progression/1` records, 59 `progression_label/1`
//!   records and 1062 `progression_chords/2` records (59 progressions × 18
//!   tonics: the twelve chromatic names plus the six keys the catalog stores as
//!   `example_key`);
//! * `fixtures/oracle/catalogs.json` carries the same definitions as
//!   `progression_definitions` and the catalog's own grouping as
//!   `grouped_progressions`.
//!
//! `Contract.D07` is the approved decision behind this task: the label and the
//! data are ported **verbatim**, so the contradictions the decision measured
//! (6 of the 59 labels contradict their own degrees, 7 are names rather than
//! chord lists, 7 claim a mode the data has not) are inherited rather than
//! corrected, and a test that "fixed" any of them would be the defect. The
//! decision changed 0 of the 1180 records, which is why every record below is
//! compared exactly, without an exception list.
//!
//! The fixture's chord roots are sharp-only, `example-Bb` included: the pinned
//! `Note.note_at/2` answers from the sharp chromatic scale, so the port answers
//! the same names even for the one flat-rooted `example_key` the catalog stores.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    variant_size_differences
)]

mod common;

use std::collections::BTreeSet;

use common::{oracle_catalogs, oracle_records};
use fretboard_core::{
    ChordMode, ChordSpec, Progression, ProgressionId, QualityId, ScaleId, all_progressions,
    diatonic_chords, grouped_progressions, note_index, progression, progression_by_str,
    progression_categories, progression_chords, progression_label,
};
use serde_json::Value;

/// The frozen operation surface and its record counts.
const FIXTURE: &str = "fixtures/oracle/progressions.jsonl";
const DEFINITION_RECORDS: usize = 59;
const LABEL_RECORDS: usize = 59;
const CHORD_RECORDS: usize = 1062;

/// The catalog the two frozen exports agree on.
const CATALOG_DEFINITIONS: usize = 59;
const CATALOG_CATEGORIES: usize = 4;

/// The tonics the chord records sweep: the twelve chromatic names plus the six
/// keys the catalog stores as `example_key`.
const TONICS: usize = 18;

/// The explicit qualities the degrees use. The plan calls this list out ("apply
/// key uses chosen mode; suggestion uses only the explicit eight-quality mode
/// list"), and it is the set the pinned `@progressions` actually carries.
const EXPLICIT_DEGREE_QUALITIES: [&str; 8] =
    ["7", "aug", "dim", "m7b5", "maj7", "major", "min7", "minor"];

/// The three scale types the 59 progressions are written in.
const SCALE_TYPES: [&str; 3] = ["major", "minor", "phrygian_dominant"];

fn records() -> Vec<Value> {
    oracle_records(FIXTURE)
}

fn operation(operation: &str) -> Vec<Value> {
    records()
        .into_iter()
        .filter(|record| record["operation"].as_str() == Some(operation))
        .collect()
}

fn text(value: &Value, field: &str) -> String {
    value[field]
        .as_str()
        .unwrap_or_else(|| panic!("{field} must be a string in {value}"))
        .to_string()
}

fn case_id(record: &Value) -> String {
    text(record, "case_id")
}

/// A catalog definition as the fixture writes it: the portable text form the
/// three readers below agree on.
fn frozen_definition(definition: &Value) -> Vec<String> {
    let degrees: Vec<String> = definition["degrees"]
        .as_array()
        .expect("degrees must be an array")
        .iter()
        .map(|degree| {
            let quality = degree["quality"]
                .as_str()
                .map_or_else(|| "nil".to_string(), ToString::to_string);
            format!(
                "{}:{}:{}",
                degree["degree"].as_u64().expect("a degree number"),
                degree["accidental"].as_i64().expect("an accidental"),
                quality
            )
        })
        .collect();
    let songs: Vec<String> = definition["notable_songs"]
        .as_array()
        .expect("notable_songs must be an array")
        .iter()
        .map(|song| song.as_str().expect("a song title").to_string())
        .collect();

    vec![
        text(definition, "id"),
        text(definition, "name"),
        text(definition, "category"),
        text(definition, "genre"),
        text(definition, "description"),
        text(definition, "example_key"),
        text(definition, "scale_type"),
        degrees.join(","),
        songs.join("|"),
    ]
}

/// The same shape, rendered from the ported catalog.
fn our_definition(progression: &Progression) -> Vec<String> {
    let degrees: Vec<String> = progression
        .degrees
        .iter()
        .map(|degree| {
            let quality = degree
                .quality
                .map_or_else(|| "nil".to_string(), |quality| quality.as_str().to_string());
            format!("{}:{}:{}", degree.degree, degree.accidental, quality)
        })
        .collect();
    let songs: Vec<String> = progression
        .notable_songs
        .iter()
        .map(|song| (*song).to_string())
        .collect();

    vec![
        progression.id.as_str().to_string(),
        progression.name.to_string(),
        progression.category.to_string(),
        progression.genre.to_string(),
        progression.description.to_string(),
        progression.example_key.to_string(),
        progression.scale_type.as_str().to_string(),
        degrees.join(","),
        songs.join("|"),
    ]
}

/// The tonic of a chord record, as the exporter spells it in the case id.
fn record_tonic(record: &Value) -> String {
    let case = case_id(record);
    let mut parts = case.split('/');
    parts.next();
    let tonic = parts.next().expect("a case id carries its tonic");
    tonic.strip_prefix("example-").unwrap_or(tonic).to_string()
}

/// One chord record's expected chords, as `root:quality` text.
fn frozen_chords(record: &Value) -> Vec<String> {
    record["output"]["chords"]
        .as_array()
        .expect("chords must be an array")
        .iter()
        .map(|chord| format!("{}:{}", text(chord, "root"), text(chord, "quality")))
        .collect()
}

fn our_chords(chords: &[ChordSpec]) -> Vec<String> {
    chords
        .iter()
        .map(|chord| format!("{}:{}", chord.root.name(), chord.quality.as_str()))
        .collect()
}

fn tonic_pitch_class(name: &str) -> fretboard_core::PitchClass {
    note_index(name).unwrap_or_else(|error| panic!("{name} is a note name: {error:?}"))
}

#[test]
fn the_frozen_fixture_carries_every_progression_record() {
    let all = records();
    assert_eq!(
        all.len(),
        DEFINITION_RECORDS + LABEL_RECORDS + CHORD_RECORDS,
        "the frozen progression surface is 59 definitions, 59 labels and 1062 chord records"
    );
    assert_eq!(operation("progression").len(), DEFINITION_RECORDS);
    assert_eq!(operation("progression_label").len(), LABEL_RECORDS);
    assert_eq!(operation("progression_chords").len(), CHORD_RECORDS);

    let definitions = operation("progression");
    let ids: BTreeSet<String> = definitions
        .iter()
        .map(|record| text(&record["input"], "id"))
        .collect();
    assert_eq!(
        ids.len(),
        CATALOG_DEFINITIONS,
        "the fixture names every catalog progression exactly once"
    );
    assert_eq!(all_progressions().len(), CATALOG_DEFINITIONS);

    // The catalog's own order is the frozen export's order, so an identifier
    // list for a picker comes back in the baseline's order without sorting.
    let frozen_order: Vec<String> = definitions
        .iter()
        .map(|record| text(&record["input"], "id"))
        .collect();
    let our_order: Vec<String> = all_progressions()
        .iter()
        .map(|progression| progression.id.as_str().to_string())
        .collect();
    assert_eq!(
        our_order, frozen_order,
        "the catalog order is the pinned order"
    );

    // The sweep names its tonic in the case id: the twelve chromatic spellings
    // plus the six keys the catalog stores as `example_key`. Two of those
    // spellings are the same pitch class (`example-Bb` and the chromatic `A#`),
    // so the eighteen spellings resolve to the twelve pitch classes.
    let sweeps: BTreeSet<String> = operation("progression_chords")
        .iter()
        .map(|record| {
            case_id(record)
                .split('/')
                .nth(1)
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(
        sweeps.len(),
        TONICS,
        "the chord sweep covers the twelve chromatic tonics and the six example keys"
    );
    let tonics: BTreeSet<String> = operation("progression_chords")
        .iter()
        .map(record_tonic)
        .collect();
    let pitch_classes: BTreeSet<String> = tonics
        .iter()
        .map(|name| tonic_pitch_class(name).name().to_string())
        .collect();
    assert_eq!(
        pitch_classes.len(),
        12,
        "every swept tonic resolves to one of the twelve pitch classes: {tonics:?}"
    );

    let scale_types: BTreeSet<String> = definitions
        .iter()
        .map(|record| text(&record["output"]["definition"], "scale_type"))
        .collect();
    assert_eq!(
        scale_types
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<&str>>(),
        SCALE_TYPES.iter().copied().collect::<BTreeSet<&str>>(),
        "the progressions are written in exactly the three pinned scale types"
    );
}

#[test]
fn every_frozen_definition_is_the_catalog_definition() {
    let mut checked = 0;
    for record in operation("progression") {
        let case = case_id(&record);
        let expected = frozen_definition(&record["output"]["definition"]);
        let id: ProgressionId = text(&record["input"], "id")
            .parse()
            .unwrap_or_else(|error| panic!("{case}: the fixture id must parse: {error:?}"));
        assert_eq!(
            our_definition(progression(id)),
            expected,
            "{case}: the definition differs from the frozen record"
        );
        checked += 1;
    }
    assert_eq!(checked, DEFINITION_RECORDS);
}

#[test]
fn the_catalog_matches_the_frozen_catalog_export() {
    let catalogs = oracle_catalogs();
    let exported = catalogs["progression_definitions"]
        .as_array()
        .expect("progression_definitions must be an array");
    assert_eq!(exported.len(), CATALOG_DEFINITIONS);

    for (index, definition) in exported.iter().enumerate() {
        let id: ProgressionId = text(definition, "id")
            .parse()
            .unwrap_or_else(|error| panic!("the export id must parse: {error:?}"));
        assert_eq!(
            our_definition(progression(id)),
            frozen_definition(definition),
            "catalogs.json entry {index}: the catalog differs from the frozen export"
        );
    }

    // Every degree quality is one the frozen catalog carries, and the explicit
    // ones are the eight the plan names.
    let mut explicit: BTreeSet<String> = BTreeSet::new();
    for definition in exported {
        for degree in definition["degrees"].as_array().expect("degrees") {
            if let Some(quality) = degree["quality"].as_str() {
                assert!(
                    quality.parse::<QualityId>().is_ok(),
                    "{quality}: a degree quality the catalog does not carry"
                );
                explicit.insert(quality.to_string());
            }
        }
    }
    assert_eq!(
        explicit,
        EXPLICIT_DEGREE_QUALITIES
            .iter()
            .map(ToString::to_string)
            .collect::<BTreeSet<String>>(),
        "the degrees use exactly the pinned explicit qualities"
    );
}

#[test]
fn the_grouped_catalog_is_the_frozen_grouping() {
    let catalogs = oracle_catalogs();
    let grouped = catalogs["grouped_progressions"]
        .as_array()
        .expect("grouped_progressions must be an array");
    assert_eq!(grouped.len(), CATALOG_CATEGORIES);

    let frozen: Vec<(String, Vec<String>)> = grouped
        .iter()
        .map(|group| {
            let category = text(group, "category");
            let ids: Vec<String> = group["progressions"]
                .as_array()
                .expect("a group carries its progressions")
                .iter()
                .map(|entry| text(entry, "id"))
                .collect();
            (category, ids)
        })
        .collect();

    let ours: Vec<(String, Vec<String>)> = grouped_progressions()
        .iter()
        .map(|group| {
            (
                group.category.to_string(),
                group.ids.iter().map(|id| id.as_str().to_string()).collect(),
            )
        })
        .collect();
    assert_eq!(ours, frozen, "the grouping differs from the frozen catalog");

    let categories: Vec<String> = progression_categories()
        .iter()
        .map(|category| (*category).to_string())
        .collect();
    assert_eq!(
        categories,
        frozen
            .iter()
            .map(|(category, _)| category.clone())
            .collect::<Vec<String>>(),
        "the category order is the frozen order"
    );
}

#[test]
fn every_frozen_label_is_the_catalog_label() {
    let mut checked = 0;
    for record in operation("progression_label") {
        let case = case_id(&record);
        let id: ProgressionId = text(&record["input"], "id")
            .parse()
            .unwrap_or_else(|error| panic!("{case}: the fixture id must parse: {error:?}"));
        assert_eq!(
            progression_label(id),
            text(&record["output"], "label"),
            "{case}: the label differs from the frozen record"
        );
        checked += 1;
    }
    assert_eq!(checked, LABEL_RECORDS);

    // The label is the definition's own name, verbatim: `Contract.D07` ports the
    // prose as it is, so the two frozen fields must agree with each other.
    for progression in all_progressions() {
        assert_eq!(
            progression_label(progression.id),
            progression.name,
            "{}: the label is not the catalog's own name",
            progression.id.as_str()
        );
    }
}

#[test]
fn every_frozen_chord_record_is_the_ported_progression_chords() {
    let mut checked = 0;
    for record in operation("progression_chords") {
        let case = case_id(&record);
        let id: ProgressionId = text(&record["input"], "id")
            .parse()
            .unwrap_or_else(|error| panic!("{case}: the fixture id must parse: {error:?}"));
        let tonic = tonic_pitch_class(&record_tonic(&record));
        assert_eq!(
            text(&record["input"], "tonic"),
            record_tonic(&record),
            "{case}: the case id and the recorded tonic must agree"
        );
        assert_eq!(
            our_chords(&progression_chords(tonic, id)),
            frozen_chords(&record),
            "{case}: the chords differ from the frozen record"
        );
        checked += 1;
    }
    assert_eq!(checked, CHORD_RECORDS);
}

#[test]
fn the_degrees_resolve_by_the_pinned_rules() {
    // `pop_i_v_vi_iv` in C is the worked example of the plain case: every degree
    // has `accidental: 0` and no explicit quality, so the diatonic chord is kept.
    let pop: ProgressionId = "pop_i_v_vi_iv".parse().expect("a catalog id");
    assert_eq!(
        our_chords(&progression_chords(tonic_pitch_class("C"), pop)),
        vec!["C:major", "G:major", "A:minor", "F:major"],
        "the plain degrees keep the diatonic chords of C major"
    );

    // A flattened degree with no explicit quality in a major-flavoured context
    // is a major triad by the pinned `infer_altered_quality/3` rule (♭II, ♭III,
    // ♭VI, ♭VII), not the diatonic quality of the unflattened degree.
    let major: ScaleId = "major".parse().expect("a catalog scale");
    let flattened = all_progressions()
        .iter()
        .find(|progression| {
            progression.scale_type == major
                && progression.degrees.iter().any(|degree| {
                    degree.accidental == -1
                        && degree.quality.is_none()
                        && matches!(degree.degree, 2 | 3 | 6 | 7)
                })
        })
        .expect("the catalog carries a flattened degree in a major context");
    let chords = progression_chords(tonic_pitch_class("C"), flattened.id);
    let linked = flattened
        .degrees
        .iter()
        .zip(chords.iter())
        .find(|(degree, _chord)| degree.accidental == -1 && degree.quality.is_none())
        .expect("the flattened degree resolves to a chord")
        .1;
    assert_eq!(
        linked.quality.as_str(),
        "major",
        "{}: a flattened degree in a major context is a major triad",
        flattened.id.as_str()
    );

    // An explicit quality wins over the diatonic one, and a sharpened degree
    // moves the root up one semitone without touching the quality.
    let sharpened = all_progressions()
        .iter()
        .find(|progression| {
            progression
                .degrees
                .iter()
                .any(|degree| degree.accidental == 1)
        })
        .expect("the catalog carries a sharpened degree");
    let chords = progression_chords(tonic_pitch_class("C"), sharpened.id);
    let index = sharpened
        .degrees
        .iter()
        .position(|degree| degree.accidental == 1)
        .expect("the sharpened degree has a position");
    let diatonic = diatonic_chords(
        tonic_pitch_class("C"),
        sharpened.scale_type,
        ChordMode::Triad,
    );
    let degree = sharpened.degrees[index];
    let expected_root =
        common::pitch_class((u8::from(diatonic[usize::from(degree.degree - 1)].root) + 1) % 12);
    assert_eq!(
        chords[index].root,
        expected_root,
        "{}: a sharpened degree moves one semitone up",
        sharpened.id.as_str()
    );

    // An explicit quality is the one the catalog wrote, whatever the scale says.
    let explicit = all_progressions()
        .iter()
        .find(|progression| {
            progression
                .degrees
                .iter()
                .any(|degree| degree.quality.is_some())
        })
        .expect("the catalog carries explicit qualities");
    let chords = progression_chords(tonic_pitch_class("C"), explicit.id);
    for (degree, chord) in explicit.degrees.iter().zip(chords.iter()) {
        if let Some(quality) = degree.quality {
            assert_eq!(
                chord.quality,
                quality,
                "{}: an explicit quality is kept",
                explicit.id.as_str()
            );
        }
    }
}

#[test]
fn a_progression_keeps_its_repeated_chords() {
    // A progression that plays the same chord twice must keep both occurrences:
    // the catalog is data, and the apply action replaces the chord list with it.
    let repeated = all_progressions().iter().find(|progression| {
        let chords = progression_chords(tonic_pitch_class("C"), progression.id);
        let mut seen: Vec<&ChordSpec> = Vec::new();
        chords.iter().any(|chord| {
            let repeat = seen.contains(&chord);
            seen.push(chord);
            repeat
        })
    });
    let Some(repeated) = repeated else {
        panic!("the catalog carries a progression that repeats a chord");
    };
    let chords = progression_chords(tonic_pitch_class("C"), repeated.id);
    let mut distinct: Vec<&ChordSpec> = Vec::new();
    for chord in &chords {
        if !distinct.contains(&chord) {
            distinct.push(chord);
        }
    }
    assert_eq!(
        chords.len(),
        repeated.degrees.len(),
        "{}: the chord list has one entry per degree",
        repeated.id.as_str()
    );
    assert!(
        distinct.len() < chords.len(),
        "{}: the repeated chord is kept as two occurrences",
        repeated.id.as_str()
    );
}

#[test]
fn an_unknown_progression_identifier_answers_nothing() {
    assert!(progression_by_str("no_such_progression").is_none());
    assert!(
        progression_by_str("pop_i_v_vi_iv").is_some(),
        "the catalog answers its own identifiers"
    );
    // The identifiers are the frozen ones, so a parsed id round-trips to its text
    // form: nothing normalises or rewrites an id on the way through.
    for progression in all_progressions() {
        let parsed: ProgressionId = progression
            .id
            .as_str()
            .parse()
            .expect("a catalog id parses");
        assert_eq!(parsed, progression.id);
    }
}
