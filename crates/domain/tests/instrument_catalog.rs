// Test target: `expect`/`unwrap`, panicking assertions and direct indexing are the
// idiom in tests, so the restriction lints that forbid them in the library are
// relaxed here only. Every other lint, including `pedantic`, still applies.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

//! The instrument and preset catalog (P2, task C07).
//!
//! Every expected value is read from the frozen oracle
//! (`fixtures/oracle/catalogs.json` and `fixtures/oracle/tunings.jsonl`), record
//! by record: the five stable identifiers and their labels, the string and fret
//! counts, the ordered named pitch presets with their exact MIDI pitches, the
//! fixed piano range with no tuning, the physical string order (the ukulele's
//! Standard tuning is reentrant) and the legacy note-name aliases the guitar
//! keeps. Nothing is retyped from the plan.
//!
//! Tuning *analysis* (`detect_preset`, per-string editing) belongs to the later
//! tuning task and is deliberately not tested here.

mod common;

use common::{
    assert_error_code, instrument_ids, open_pitch, oracle_catalogs, preset_name, preset_names,
    preset_pitches,
};
use fretboard_core::{
    InstrumentId, PitchClass, PresetName, instrument_frets, instrument_kind, instrument_label,
    instrument_strings, instruments, keyboard_pitch_range, named_preset_pitches, piano,
    preset_pitches as catalog_pitches, standard_pitches, standard_tuning_notes, tuning_presets,
};

/// The oracle's definition record of one instrument.
fn oracle_definition(id: &str) -> serde_json::Value {
    oracle_catalogs()["instrument_definitions"]
        .as_array()
        .expect("instrument_definitions must be an array")
        .iter()
        .find(|entry| entry["id"].as_str() == Some(id))
        .unwrap_or_else(|| panic!("the oracle has no instrument definition {id}"))
        .clone()
}

/// The pinned pitch of one instrument preset, as a typed pitch.
fn pitch_vec(values: &[u8]) -> Vec<fretboard_core::OpenPitch> {
    values.iter().map(|value| open_pitch(*value)).collect()
}

// ---------------------------------------------------------------------------
// Identifiers, labels and the two instrument kinds
// ---------------------------------------------------------------------------

#[test]
fn the_five_instruments_have_their_frozen_identifiers_and_labels() {
    let oracle = instrument_ids();
    assert_eq!(
        oracle,
        vec!["guitar", "bass_4", "bass_5", "ukelele", "piano"],
        "the frozen catalog ids, in display order"
    );

    let catalog: Vec<(String, String)> = instruments()
        .iter()
        .map(|entry| (entry.id.as_str().to_string(), entry.label.to_string()))
        .collect();
    let expected: Vec<(String, String)> = oracle_catalogs()["instrument_models"]["instruments"]
        .as_array()
        .expect("instruments must be an array")
        .iter()
        .map(|entry| {
            (
                entry["id"].as_str().expect("an id").to_string(),
                entry["label"].as_str().expect("a label").to_string(),
            )
        })
        .collect();
    assert_eq!(catalog, expected, "the catalog matches the oracle's models");

    assert_eq!(instrument_label(InstrumentId::Guitar), "Guitar");
    assert_eq!(instrument_label(InstrumentId::Bass4), "Bass (4-string)");
    assert_eq!(instrument_label(InstrumentId::Bass5), "Bass (5-string)");
    assert_eq!(
        instrument_label(InstrumentId::Ukelele),
        "Ukulele",
        "the label is English; the identifier keeps the frozen `ukelele`"
    );
    assert_eq!(instrument_label(InstrumentId::Piano), "Piano");
}

#[test]
fn the_four_fretted_instruments_are_ordered_before_the_piano() {
    let fretted: Vec<String> = fretboard_core::fretted_instruments()
        .iter()
        .map(|instrument| instrument.id.as_str().to_string())
        .collect();
    assert_eq!(fretted, vec!["guitar", "bass_4", "bass_5", "ukelele"]);
    assert_eq!(
        instrument_kind(InstrumentId::Piano),
        fretboard_core::InstrumentKind::Keyboard
    );
    for id in [
        InstrumentId::Guitar,
        InstrumentId::Bass4,
        InstrumentId::Bass5,
        InstrumentId::Ukelele,
    ] {
        assert_eq!(
            instrument_kind(id),
            fretboard_core::InstrumentKind::Fretted,
            "{id:?} is fretted"
        );
    }
}

#[test]
fn the_string_and_fret_counts_match_the_oracle() {
    for id in [
        InstrumentId::Guitar,
        InstrumentId::Bass4,
        InstrumentId::Bass5,
        InstrumentId::Ukelele,
    ] {
        let definition = oracle_definition(id.as_str())["definition"].clone();
        assert_eq!(
            instrument_strings(id).unwrap_or_else(|error| panic!("{id:?} has strings: {error:?}")),
            u8::try_from(
                definition["strings"]
                    .as_u64()
                    .expect("the oracle declares the string count")
            )
            .expect("fits in u8"),
            "the string count of {id:?}"
        );
        assert_eq!(
            instrument_frets(id).unwrap_or_else(|error| panic!("{id:?} has frets: {error:?}")),
            u8::try_from(definition["frets"].as_u64().expect("the fret count"))
                .expect("fits in u8"),
            "the fret count of {id:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// The piano
// ---------------------------------------------------------------------------

#[test]
fn the_piano_has_a_fixed_range_and_no_tuning() {
    let definition = oracle_definition("piano")["definition"].clone();
    // The exporter writes an Elixir range as its two bounds.
    let bounds: Vec<u8> = definition["pitch_range"]["fields"]
        .as_array()
        .expect("the oracle exports the piano range as its bounds")
        .iter()
        .map(|bound| u8::try_from(bound.as_u64().expect("a numeric bound")).expect("fits in u8"))
        .collect();
    assert_eq!(bounds.len(), 2, "the range is an inclusive pair of bounds");
    assert_eq!(
        keyboard_pitch_range(),
        (open_pitch(bounds[0]), open_pitch(bounds[1])),
        "the piano range is the oracle's"
    );
    assert!(
        definition["pitch_presets"]
            .as_array()
            .expect("a preset list")
            .is_empty(),
        "the oracle gives the piano no presets"
    );

    assert_eq!(piano().id, InstrumentId::Piano);
    assert!(piano().presets.is_empty(), "the piano has no presets");
    for name in preset_names("guitar") {
        assert_error_code(
            named_preset_pitches(InstrumentId::Piano, preset_name(&name)),
            "UnknownIdentifier",
        );
    }
    // A keyboard has no tuning at all: the baseline's `instrument_standard_*`
    // crashes on the piano, and the port reports the structural violation.
    assert_error_code(standard_pitches(InstrumentId::Piano), "InvalidState");
    assert_error_code(
        standard_tuning_notes(InstrumentId::Piano).map(|_| ()),
        "InvalidState",
    );
    assert!(
        tuning_presets(InstrumentId::Piano).is_empty(),
        "the baseline returns an empty preset list for the piano"
    );
}

// ---------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------

#[test]
fn every_preset_of_every_instrument_matches_the_oracle_in_order() {
    for id in instrument_ids() {
        let entry = instruments()
            .iter()
            .find(|entry| entry.id.as_str() == id)
            .expect("the catalog carries every id");
        let expected_names = preset_names(&id);
        let actual_names: Vec<String> = entry
            .presets
            .iter()
            .map(|preset| preset.name.as_str().to_string())
            .collect();
        assert_eq!(
            actual_names, expected_names,
            "the preset names of {id}, in catalog order"
        );

        for name in &expected_names {
            assert_eq!(
                entry
                    .presets
                    .iter()
                    .find(|preset| preset.name == preset_name(name))
                    .expect("the preset is present")
                    .pitches,
                preset_pitches(&id, name).as_slice(),
                "the pitches of {id}/{name}"
            );
        }
    }
}

#[test]
fn a_preset_lookup_by_name_is_exact_and_typed() {
    let standard = catalog_pitches(InstrumentId::Guitar, preset_name("Standard"))
        .expect("guitar Standard is a catalog preset");
    assert_eq!(standard, pitch_vec(&[40, 45, 50, 55, 59, 64]).as_slice());

    // A name outside the frozen catalog never reaches the lookup: the validated
    // identifier rejects it first (the baseline's `preset_pitches/2` returns nil
    // for the same input).
    for unknown in ["Nope", "standard", "STANDARD", "Standard ", "", "low g"] {
        assert_error_code(
            unknown.parse::<PresetName>().map(|_| ()),
            "UnknownIdentifier",
        );
    }

    // A real preset of another instrument is not a preset of this one.
    assert_error_code(
        named_preset_pitches(InstrumentId::Guitar, preset_name("Low G")),
        "UnknownIdentifier",
    );
    assert!(
        catalog_pitches(InstrumentId::Ukelele, preset_name("Low G")).is_some(),
        "Low G is a real ukulele preset"
    );

    // The canonical spelling is the catalog's, not the caller's.
    assert_eq!(
        preset_name("Standard").as_str(),
        "Standard",
        "a parsed preset name keeps its canonical catalog spelling"
    );
}

#[test]
fn the_preset_names_are_the_thirteen_distinct_catalog_names() {
    let union: Vec<String> = common::all_preset_names();
    assert_eq!(union.len(), 13, "the frozen catalog has 13 distinct names");
    assert_eq!(
        PresetName::ALL
            .iter()
            .map(|name| name.as_str().to_string())
            .collect::<Vec<_>>(),
        union,
        "the accepted names are the catalog's distinct names, in catalog order"
    );
    assert_eq!(
        preset_names("guitar").len(),
        9,
        "the guitar carries nine presets"
    );
}

// ---------------------------------------------------------------------------
// Physical order and the legacy note-name aliases
// ---------------------------------------------------------------------------

#[test]
fn the_standard_tuning_keeps_physical_string_order() {
    // The ukulele's Standard tuning is reentrant: the first string is the
    // highest, so the list must never be sorted by pitch.
    let ukelele = standard_pitches(InstrumentId::Ukelele).expect("ukulele has a Standard");
    assert_eq!(
        ukelele,
        pitch_vec(&[67, 60, 64, 69]).as_slice(),
        "the physical order is the catalog's"
    );
    assert_ne!(
        ukelele,
        {
            let mut sorted = ukelele.to_vec();
            sorted.sort();
            sorted
        },
        "sorting the strings would change the instrument"
    );

    let guitar = standard_pitches(InstrumentId::Guitar).expect("guitar has a Standard");
    assert_eq!(guitar, preset_pitches("guitar", "Standard").as_slice());
}

#[test]
fn the_legacy_note_names_are_derived_from_the_pitches() {
    for id in ["guitar", "bass_4", "bass_5", "ukelele"] {
        let instrument_id = id
            .parse::<InstrumentId>()
            .unwrap_or_else(|error| panic!("{id}: {error:?}"));
        let presets = tuning_presets(instrument_id);
        let oracle = oracle_catalogs()["instrument_tuning_presets"]
            .as_array()
            .expect("instrument_tuning_presets must be an array")
            .iter()
            .find(|group| group["instrument"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("the oracle has no legacy presets for {id}"))
            .clone();

        let expected: Vec<(String, Vec<String>)> = oracle["presets"]
            .as_array()
            .expect("presets must be an array")
            .iter()
            .map(|preset| {
                (
                    preset["name"].as_str().expect("a name").to_string(),
                    preset["notes"]
                        .as_array()
                        .expect("notes must be an array")
                        .iter()
                        .map(|note| note.as_str().expect("a note").to_string())
                        .collect(),
                )
            })
            .collect();

        let actual: Vec<(String, Vec<String>)> = presets
            .iter()
            .map(|(name, notes)| {
                (
                    name.as_str().to_string(),
                    notes
                        .iter()
                        .map(|note: &PitchClass| note.name().to_string())
                        .collect(),
                )
            })
            .collect();
        assert_eq!(actual, expected, "the legacy presets of {id}");
    }

    // The notes are the pitch classes of the exact pitches, in the same order.
    let notes = standard_tuning_notes(InstrumentId::Guitar).expect("guitar has a Standard");
    let names: Vec<String> = notes.iter().map(|note| note.name().to_string()).collect();
    assert_eq!(names, vec!["E", "A", "D", "G", "B", "E"]);
}

#[test]
fn the_guitar_legacy_aliases_are_the_guitar_catalog() {
    // The baseline keeps three guitar-only aliases for backward compatibility.
    assert_eq!(
        fretboard_core::guitar_standard_tuning(),
        vec!["E", "A", "D", "G", "B", "E"]
    );
    let alias_names: Vec<String> = fretboard_core::guitar_tuning_preset_names()
        .iter()
        .map(|name| name.as_str().to_string())
        .collect();
    assert_eq!(alias_names, preset_names("guitar"));
    assert_eq!(
        fretboard_core::guitar_tuning_presets().len(),
        preset_names("guitar").len()
    );
}
