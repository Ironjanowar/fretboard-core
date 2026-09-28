//! Shared helpers for the typed page-state contract tests (C02).
//!
//! Expected musical values are read from the frozen oracle
//! (`fixtures/oracle/catalogs.json`) instead of being retyped from the plan.
//! The helpers also collect, in one place, the few typed conversions the plan
//! does not name explicitly (`TryFrom<u8>` for the numeric newtypes,
//! `FromStr`/`Display` for the identifier types); the contract report lists
//! those conversions as the part a human must confirm.
//!
//! The module is compiled once per test target; `dead_code` is allowed so that
//! a helper used by a single test does not become a warning under
//! `cargo clippy -- -D warnings`.

// Test target: same relaxation as the test files. `unreachable_pub` fires on
// helpers of a test-only module that has no external users.
#![allow(
    dead_code,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    unreachable_pub
)]

use std::collections::BTreeSet;
use std::fmt::Debug;
use std::path::{Path, PathBuf};

use fretboard_core::{
    ChordSpec, CoreError, Fret, InstrumentId, InstrumentState, OpenPitch, PageState, PitchClass,
    Position, PresetName, QualityId, StringIndex, Tab, TuningState,
};

/// The twelve pitch-class names in contract order (sharp-only display).
pub const SHARP_NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

fn repo_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

/// Read a repository file. A missing or unreadable fixture fails the test
/// loudly instead of silently skipping the contract check.
pub fn read_repo_file(relative: &str) -> String {
    let path = repo_file(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// The frozen Elixir catalog export.
pub fn oracle_catalogs() -> serde_json::Value {
    serde_json::from_str(&read_repo_file("fixtures/oracle/catalogs.json"))
        .expect("fixtures/oracle/catalogs.json must be valid JSON")
}

fn pitch_preset_group(instrument_id: &str) -> serde_json::Value {
    oracle_catalogs()["instrument_pitch_presets"]
        .as_array()
        .expect("instrument_pitch_presets must be an array")
        .iter()
        .find(|group| group["instrument"].as_str() == Some(instrument_id))
        .unwrap_or_else(|| panic!("the oracle has no pitch presets for {instrument_id}"))
        .clone()
}

/// Preset names of an instrument, in catalog order.
pub fn preset_names(instrument_id: &str) -> Vec<String> {
    pitch_preset_group(instrument_id)["presets"]
        .as_array()
        .expect("presets must be an array")
        .iter()
        .map(|preset| preset["name"].as_str().expect("preset name").to_string())
        .collect()
}

/// Exact MIDI pitches of one preset, in catalog order.
pub fn preset_pitches(instrument_id: &str, preset_name: &str) -> Vec<OpenPitch> {
    let group = pitch_preset_group(instrument_id);
    let preset = group["presets"]
        .as_array()
        .expect("presets must be an array")
        .iter()
        .find(|preset| preset["name"].as_str() == Some(preset_name))
        .unwrap_or_else(|| panic!("the oracle has no preset {preset_name} for {instrument_id}"));
    preset["pitches"]
        .as_array()
        .expect("pitches must be an array")
        .iter()
        .map(|pitch| {
            let value = u8::try_from(pitch.as_u64().expect("pitch is an integer"))
                .expect("catalog pitch fits in u8");
            open_pitch(value)
        })
        .collect()
}

/// Instrument ids in catalog display order.
pub fn instrument_ids() -> Vec<String> {
    oracle_catalogs()["instrument_models"]["instruments"]
        .as_array()
        .expect("instruments must be an array")
        .iter()
        .map(|instrument| {
            instrument["id"]
                .as_str()
                .expect("instrument id")
                .to_string()
        })
        .collect()
}

/// Stable quality identifiers from the frozen chord catalog.
pub fn quality_ids() -> Vec<String> {
    oracle_catalogs()["chord_qualities"]
        .as_array()
        .expect("chord_qualities must be an array")
        .iter()
        .map(|quality| quality["id"].as_str().expect("quality id").to_string())
        .collect()
}

/// A preset name that exists for ukulele but not for guitar, taken from the
/// oracle; used to prove a foreign reference is rejected.
pub fn ukelele_only_preset_name() -> String {
    let guitar = preset_names("guitar");
    preset_names("ukelele")
        .into_iter()
        .find(|name| !guitar.contains(name))
        .expect("ukelele has a preset that guitar does not")
}

pub fn pitch_class(value: u8) -> PitchClass {
    PitchClass::try_from(value).unwrap_or_else(|error| panic!("PitchClass {value}: {error:?}"))
}

pub fn open_pitch(value: u8) -> OpenPitch {
    OpenPitch::try_from(value).unwrap_or_else(|error| panic!("OpenPitch {value}: {error:?}"))
}

pub fn fret(value: u8) -> Fret {
    Fret::try_from(value).unwrap_or_else(|error| panic!("Fret {value}: {error:?}"))
}

pub fn string_index(value: u8) -> StringIndex {
    StringIndex::try_from(value).unwrap_or_else(|error| panic!("StringIndex {value}: {error:?}"))
}

pub fn preset_name(name: &str) -> PresetName {
    name.parse()
        .unwrap_or_else(|error| panic!("PresetName {name}: {error:?}"))
}

pub fn quality(id: &str) -> QualityId {
    id.parse()
        .unwrap_or_else(|error| panic!("QualityId {id}: {error:?}"))
}

pub fn chord(root: u8, quality_id: &str) -> ChordSpec {
    ChordSpec {
        root: pitch_class(root),
        quality: quality(quality_id),
    }
}

pub fn position(string: u8, fret_value: u8) -> Position {
    Position {
        string: string_index(string),
        fret: fret(fret_value),
    }
}

pub const fn page(
    instrument: InstrumentState,
    chords: Vec<ChordSpec>,
    highlight: Option<ChordSpec>,
    tab: Tab,
) -> PageState {
    PageState {
        instrument,
        chords,
        highlight,
        tab,
    }
}

pub const fn fretted_page(
    instrument: InstrumentId,
    tuning: TuningState,
    selected: Vec<Position>,
) -> PageState {
    page(
        InstrumentState::Fretted {
            instrument,
            tuning,
            selected,
        },
        Vec::new(),
        None,
        Tab::Visualizer,
    )
}

/// The stable code of a `CoreError` is its variant name. The plan fixes the
/// code list but not the exact variant field shape, so the code is read from
/// the value's `Debug` rendering, which always starts with the variant name.
pub fn error_code(error: &CoreError) -> String {
    let rendered = format!("{error:?}");
    let code: String = rendered
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    assert!(!code.is_empty(), "no error code in {rendered}");
    code
}

pub fn assert_error_code<T: Debug>(result: Result<T, CoreError>, expected: &str) {
    match result {
        Ok(value) => panic!("expected {expected}, got Ok({value:?})"),
        Err(error) => assert_eq!(
            error_code(&error),
            expected,
            "wrong error code for {error:?}"
        ),
    }
}

pub fn assert_valid(result: Result<(), CoreError>) {
    if let Err(error) = result {
        panic!("expected a valid state, got {error:?}");
    }
}

pub fn name_set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(ToString::to_string).collect()
}

pub fn keys_of(object: &serde_json::Map<String, serde_json::Value>) -> BTreeSet<String> {
    object.keys().cloned().collect()
}

pub fn object_keys(value: &serde_json::Value) -> BTreeSet<String> {
    keys_of(
        value
            .as_object()
            .unwrap_or_else(|| panic!("expected an object, got {value}")),
    )
}
