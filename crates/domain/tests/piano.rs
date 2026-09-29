//! Task `C14`: the piano's exact-pitch selection and its analysis against the
//! frozen oracle.
//!
//! The piano is the one instrument whose selection is already absolute: its keys
//! *are* the sounding pitches, so [`analyze_page`] maps them straight through to
//! [`analyze_pitches`] — the same instrument-independent analyzer `C13` wrote
//! (`Fretboard.Music.Analyzer`, reached by the pinned `piano_analysis/1` as
//! `Music.analyze_pitches(selection)`). This file pins that there is **one**
//! recognizer: for every selection the piano can hold, its page analysis is the
//! analyzer's answer, and the frozen `analyze_pitches/1` answers that fit inside
//! the keyboard are reached key for key.
//!
//! It also pins the metadata the keyboard selection rests on, which `C10` and
//! `C07` own and which `C14` closes:
//!
//! * the frozen 36-key range (`instrument_definitions.piano.pitch_range`,
//!   `48..=83`), every key of which toggles on and off;
//! * the canonical selection — ascending, unique, always valid;
//! * the catalog's ordered preset names per instrument, the enumeration the
//!   Android preset picker needs and that no client may re-type.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod common;

use std::collections::BTreeSet;
use std::str::FromStr;

use common::{open_pitch, oracle_catalogs, oracle_records, preset_names};
use fretboard_core::{
    Analysis, InstrumentId, InstrumentState, OpenPitch, PageEvent, PageState, SoundingPitch, Tab,
    analyze_page, analyze_pitches, apply_event, encode_page_params, keyboard_pitch_range,
    validate_state,
};
use serde_json::Value;

const ANALYZER: &str = "fixtures/oracle/analyzer.jsonl";

/// A piano page on the analyzer tab with this canonical key selection.
fn piano_page(keys: &[u8]) -> PageState {
    let selected: Vec<OpenPitch> = keys.iter().copied().map(open_pitch).collect();
    PageState {
        instrument: InstrumentState::Piano { selected },
        chords: Vec::new(),
        highlight: None,
        tab: Tab::Analyzer,
    }
}

/// The keys of a piano page.
fn keys_of(state: &PageState) -> Vec<u8> {
    match &state.instrument {
        InstrumentState::Piano { selected } => selected.iter().map(|key| u8::from(*key)).collect(),
        InstrumentState::Fretted { .. } => panic!("the page is not the piano"),
    }
}

/// The frozen piano range, from `fixtures/oracle/catalogs.json`.
fn frozen_keyboard_range() -> (u8, u8) {
    let catalogs = oracle_catalogs();
    let definitions = catalogs["instrument_definitions"]
        .as_array()
        .expect("instrument_definitions is an array");
    let piano = definitions
        .iter()
        .find(|definition| definition["id"].as_str() == Some("piano"))
        .expect("the oracle has a piano definition");
    let fields = piano["definition"]["pitch_range"]["fields"]
        .as_array()
        .expect("the piano definition carries its range");
    (
        u8::try_from(fields[0].as_u64().expect("the lower bound")).expect("fits"),
        u8::try_from(fields[1].as_u64().expect("the upper bound")).expect("fits"),
    )
}

/// The fixture's variant name of an analysis.
const fn variant_name(analysis: &Analysis) -> &'static str {
    match analysis {
        Analysis::Empty => "empty",
        Analysis::Single { .. } => "single",
        Analysis::Interval { .. } => "interval",
        Analysis::Chords { .. } => "chords",
    }
}

/// The sounding pitches of a key list; a piano key is already absolute.
fn sounding(keys: &[u8]) -> Vec<SoundingPitch> {
    keys.iter()
        .map(|key| SoundingPitch::try_from(u16::from(*key)).expect("a sounding pitch"))
        .collect()
}

/// The keyboard is the frozen 36-key range, and every one of its keys toggles on
/// and off as a valid lone selection.
#[test]
fn every_key_of_the_frozen_keyboard_toggles() {
    let (lowest, highest) = keyboard_pitch_range();
    let (frozen_low, frozen_high) = frozen_keyboard_range();
    assert_eq!(
        (u8::from(lowest), u8::from(highest)),
        (frozen_low, frozen_high),
        "the domain range is the frozen one"
    );
    assert_eq!(
        frozen_high - frozen_low + 1,
        36,
        "the keyboard is the frozen 36 keys (48..=83)"
    );

    let empty = piano_page(&[]);
    for value in frozen_low..=frozen_high {
        let key = open_pitch(value);
        let event = PageEvent::TogglePianoKey(key);

        let on = apply_event(&empty, &event);
        assert_eq!(keys_of(&on), vec![value], "key {value} toggles on");
        assert!(
            validate_state(&on).is_ok(),
            "key {value} is a valid selection"
        );

        let off = apply_event(&on, &event);
        assert_eq!(
            keys_of(&off),
            Vec::<u8>::new(),
            "key {value} toggles off again"
        );
    }
}

/// The selection is canonical: keys are inserted ascending and unique, so a
/// scrambled order still encodes to the frozen ascending `keys` parameter.
#[test]
fn the_key_selection_is_canonical_and_ascending() {
    let mut state = piano_page(&[]);
    for value in [64u8, 48, 60] {
        state = apply_event(&state, &PageEvent::TogglePianoKey(open_pitch(value)));
    }
    assert_eq!(keys_of(&state), vec![48, 60, 64], "the keys are ascending");
    assert!(validate_state(&state).is_ok(), "the selection is valid");
    assert_eq!(
        encode_page_params(&state).get("keys"),
        Some(&Value::from("48,60,64")),
        "the parameters carry the canonical ascending keys"
    );

    // Toggling a key that is already there removes it without disturbing the
    // others, and the order stays canonical.
    let removed = apply_event(&state, &PageEvent::TogglePianoKey(open_pitch(60)));
    assert_eq!(keys_of(&removed), vec![48, 64]);
}

/// The piano's page analysis *is* the one analyzer: for any selection the
/// keyboard can hold, `analyze_page` answers exactly `analyze_pitches` of the
/// same absolute pitches. There is no piano-specific chord algorithm.
#[test]
fn the_piano_analysis_is_the_single_analyzer() {
    let (lowest, highest) = keyboard_pitch_range();
    let (lowest, highest) = (u8::from(lowest), u8::from(highest));

    let mut selections: Vec<Vec<u8>> = (lowest..=highest).map(|key| vec![key]).collect();
    selections.push(vec![]);
    selections.push(vec![48, 60, 72]);
    selections.push(vec![60, 64, 67]);
    selections.push(vec![52, 56, 59]);
    selections.push(vec![64, 67, 72]);
    selections.push(vec![48, 60, 64, 67]);
    selections.push(vec![60, 61, 62, 63]);
    selections.push(vec![48, 49, 50]);

    for keys in &selections {
        let page = piano_page(keys);
        assert_eq!(
            analyze_page(&page),
            Some(analyze_pitches(&sounding(keys))),
            "keys {keys:?}: the piano must reach the one analyzer"
        );
    }
}

/// Every frozen `analyze_pitches/1` case whose pitches fit inside the keyboard is
/// reached key for key: the same variant, the same notes and the same bass.
#[test]
fn the_piano_reaches_the_frozen_analyzer_answers() {
    let (lowest, highest) = frozen_keyboard_range();
    let mut chord_cases = 0;

    for record in oracle_records(ANALYZER) {
        if record["operation"].as_str() != Some("analyze_pitches") {
            continue;
        }
        let case = record["case_id"].as_str().expect("a case id").to_owned();
        let pitches: Vec<u8> = record["input"]["pitches"]
            .as_array()
            .expect("pitches")
            .iter()
            .map(|pitch| u8::try_from(pitch.as_u64().expect("a pitch")).expect("fits"))
            .collect();
        let inside: Vec<u8> = pitches
            .iter()
            .copied()
            .filter(|pitch| *pitch >= lowest && *pitch <= highest)
            .collect();
        if inside.is_empty() {
            continue;
        }

        let keys: Vec<u8> = inside
            .iter()
            .copied()
            .collect::<BTreeSet<u8>>()
            .into_iter()
            .collect();
        let page = piano_page(&keys);
        let analysis = analyze_page(&page).expect("the analyzer tab answers");
        let frozen = &record["output"]["analysis"];

        assert_eq!(
            variant_name(&analysis),
            frozen["variant"].as_str().expect("a variant"),
            "{case}: the variant differs"
        );

        if let Analysis::Chords { notes, bass, .. } = &analysis {
            let fields = frozen["fields"].as_array().expect("chord fields");
            let frozen_notes: Vec<String> = fields[0]
                .as_array()
                .expect("the note list")
                .iter()
                .map(|note| note.as_str().expect("a note").to_owned())
                .collect();
            let our_notes: Vec<String> = notes.iter().map(|note| note.name().to_owned()).collect();
            assert_eq!(our_notes, frozen_notes, "{case}: the notes differ");
            assert_eq!(
                bass.name(),
                fields[1].as_str().expect("the bass"),
                "{case}: the bass differs"
            );
            chord_cases += 1;
        }
    }

    assert!(
        chord_cases >= 2,
        "the fixture carries at least two chord cases inside the keyboard"
    );
}

/// The lowest key is the bass: a selection analysed as chords is seen from its
/// lowest sounding pitch, not from its root.
#[test]
fn the_lowest_key_is_the_bass() {
    let root_position =
        analyze_page(&piano_page(&[48, 60, 64, 67])).expect("the analyzer tab answers");
    let Analysis::Chords { bass, .. } = root_position else {
        panic!("four distinct classes are chords");
    };
    assert_eq!(bass.name(), "C", "the lowest key is C");

    let inverted = analyze_page(&piano_page(&[64, 67, 72])).expect("the analyzer tab answers");
    let Analysis::Chords { bass, .. } = inverted else {
        panic!("three distinct classes are chords");
    };
    assert_eq!(
        bass.name(),
        "E",
        "the same triad seen from its lowest key is an inversion"
    );
}

/// Independent octaves are distinct keys, and the analyzer reads them as heights:
/// two C keys an octave apart are one class at two heights (the frozen `Octave`),
/// not one key.
#[test]
fn independent_octaves_are_distinct_heights() {
    let state = piano_page(&[48, 60]);
    assert_eq!(keys_of(&state), vec![48, 60], "both keys are held");

    let analysis = analyze_page(&state).expect("the analyzer tab answers");
    match analysis {
        Analysis::Interval { low, high, label } => {
            assert_eq!(low.name(), "C", "the low representative");
            assert_eq!(high.name(), "C", "the high representative");
            assert_eq!(label, "Octave", "one class two octaves apart");
        }
        other => panic!("two C keys are the frozen Octave, got {other:?}"),
    }
}

/// The analysis is gated by the tab, exactly as it is for a fretted page: the
/// visualizer tab carries none, and an empty selection on the analyzer tab is the
/// computed `empty` answer — a different thing from an absent one.
#[test]
fn the_piano_analysis_is_gated_by_the_tab() {
    let mut page = piano_page(&[60]);
    page.tab = Tab::Visualizer;
    assert_eq!(
        analyze_page(&page),
        None,
        "the visualizer tab carries no analysis"
    );

    assert_eq!(
        analyze_page(&piano_page(&[])),
        Some(Analysis::Empty),
        "an empty analyzer tab is the computed empty answer"
    );
}

/// Every instrument enumerates its frozen preset names, in catalog order, and the
/// piano enumerates none. This is the enumeration the preset picker needs: no
/// client re-types a preset list.
#[test]
fn every_instrument_lists_its_frozen_preset_names() {
    for id in common::instrument_ids() {
        let instrument = InstrumentId::from_str(&id).expect("a catalog instrument");
        let ours: Vec<String> = fretboard_core::preset_names(instrument)
            .iter()
            .map(ToString::to_string)
            .collect();
        assert_eq!(
            ours,
            preset_names(&id),
            "{id}: the preset names must be the frozen catalog's, in order"
        );
    }

    assert!(
        fretboard_core::preset_names(InstrumentId::Piano).is_empty(),
        "the piano has no tuning and no preset"
    );
}
