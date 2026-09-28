//! Typed page-state contract for task C02 (`crates/domain`).
//!
//! These tests pin the typed state contract from `02-core-contract.md`
//! (sections 2, 3, 6, 8 and 9) before the types exist: defaults come from the
//! frozen oracle, the invariants are asserted through `validate_state`, and the
//! frozen schema examples under `fixtures/contract/` must load and deserialise
//! into the typed `PageState`.
//!
//! Scope: this file is the state/type contract. The reducer's atomicity belongs
//! to task C09 and is deliberately not tested here.
//!
//! First observation for C02 is the expected failure to compile: the domain
//! types below do not exist yet.

mod common;

use common::{
    SHARP_NOTE_NAMES, assert_error_code, assert_valid, chord, fretted_page, instrument_ids,
    keys_of, name_set, object_keys, open_pitch, page, position, preset_name, preset_names,
    preset_pitches, quality_ids, read_repo_file, ukelele_only_preset_name,
};
use fretboard_core::{
    Fret, InstrumentId, InstrumentState, OpenPitch, PageState, PresetName, QualityId, ScaleId,
    SoundingPitch, Tab, default_state, preset_tuning, validate_state,
};

#[test]
fn default_state_is_guitar_standard_from_the_oracle() {
    let expected_pitches = preset_pitches("guitar", "Standard");
    let state = default_state();

    let InstrumentState::Fretted {
        instrument,
        tuning,
        selected,
    } = &state.instrument
    else {
        panic!(
            "default instrument must be fretted, got {instrument:?}",
            instrument = state.instrument
        );
    };
    assert_eq!(*instrument, InstrumentId::Guitar);
    assert_eq!(
        tuning.pitches, expected_pitches,
        "default pitches must equal the oracle guitar Standard preset"
    );
    assert_eq!(
        tuning.reference,
        preset_name("Standard"),
        "default reference must be the Standard preset name"
    );
    assert!(
        selected.is_empty(),
        "default fretted selection must be empty"
    );
    assert!(state.chords.is_empty(), "default chords must be empty");
    assert!(state.highlight.is_none(), "default highlight must be none");
    assert_eq!(state.tab, Tab::Visualizer);
    assert_valid(validate_state(&state));
}

#[test]
fn fretted_and_piano_are_separate_instrument_kinds() {
    let piano = page(
        InstrumentState::Piano {
            selected: vec![open_pitch(60), open_pitch(64)],
        },
        Vec::new(),
        None,
        Tab::Analyzer,
    );

    assert!(matches!(piano.instrument, InstrumentState::Piano { .. }));
    assert!(matches!(
        default_state().instrument,
        InstrumentState::Fretted { .. }
    ));
    assert_valid(validate_state(&piano));
    assert_valid(validate_state(&default_state()));
}

#[test]
fn fretted_with_a_piano_instrument_is_an_invalid_state() {
    // `Fretted { instrument: Piano }` is the only representation that could
    // carry a tuning on a piano instrument, and it must be rejected.
    let state = page(
        InstrumentState::Fretted {
            instrument: InstrumentId::Piano,
            tuning: preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
                .expect("the guitar Standard preset resolves"),
            selected: Vec::new(),
        },
        Vec::new(),
        None,
        Tab::Visualizer,
    );

    assert_error_code(validate_state(&state), "InvalidState");
}

#[test]
fn piano_selection_is_restricted_to_48_through_83() {
    let at_bounds = page(
        InstrumentState::Piano {
            selected: vec![open_pitch(48), open_pitch(83)],
        },
        Vec::new(),
        None,
        Tab::Analyzer,
    );
    assert_valid(validate_state(&at_bounds));

    let below = page(
        InstrumentState::Piano {
            selected: vec![open_pitch(47)],
        },
        Vec::new(),
        None,
        Tab::Analyzer,
    );
    assert_error_code(validate_state(&below), "OutOfRange");

    let above = page(
        InstrumentState::Piano {
            selected: vec![open_pitch(84)],
        },
        Vec::new(),
        None,
        Tab::Analyzer,
    );
    assert_error_code(validate_state(&above), "OutOfRange");
}

#[test]
fn piano_selection_must_be_unique_and_ascending() {
    let ascending = page(
        InstrumentState::Piano {
            selected: vec![open_pitch(48), open_pitch(60), open_pitch(83)],
        },
        Vec::new(),
        None,
        Tab::Analyzer,
    );
    assert_valid(validate_state(&ascending));

    let duplicate = page(
        InstrumentState::Piano {
            selected: vec![open_pitch(60), open_pitch(60)],
        },
        Vec::new(),
        None,
        Tab::Analyzer,
    );
    assert_error_code(validate_state(&duplicate), "InvalidState");

    let descending = page(
        InstrumentState::Piano {
            selected: vec![open_pitch(64), open_pitch(60)],
        },
        Vec::new(),
        None,
        Tab::Analyzer,
    );
    assert_error_code(validate_state(&descending), "InvalidState");
}

#[test]
fn fretted_positions_are_unique_and_ascending_by_physical_string() {
    let standard = preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
        .expect("the guitar Standard preset resolves");

    let ascending = fretted_page(
        InstrumentId::Guitar,
        standard.clone(),
        vec![position(0, 0), position(3, 7), position(5, 24)],
    );
    assert_valid(validate_state(&ascending));

    let duplicate_string = fretted_page(
        InstrumentId::Guitar,
        standard.clone(),
        vec![position(2, 0), position(2, 5)],
    );
    assert_error_code(validate_state(&duplicate_string), "InvalidState");

    let descending_string = fretted_page(
        InstrumentId::Guitar,
        standard,
        vec![position(3, 0), position(1, 0)],
    );
    assert_error_code(validate_state(&descending_string), "InvalidState");
}

#[test]
fn fretted_positions_follow_physical_string_order_not_pitch_order() {
    // Ukulele Standard is reentrant (string 0 sounds above string 1), so a
    // selection in physical string order is valid even though it is not
    // ascending by pitch.
    let ukelele = preset_tuning(InstrumentId::Ukelele, &preset_name("Standard"))
        .expect("the ukulele Standard preset resolves");

    let state = fretted_page(
        InstrumentId::Ukelele,
        ukelele,
        vec![
            position(0, 0),
            position(1, 0),
            position(2, 0),
            position(3, 0),
        ],
    );
    assert_valid(validate_state(&state));
}

#[test]
fn fret_is_restricted_to_zero_through_24() {
    assert!(Fret::try_from(0u8).is_ok(), "fret 0 is valid");
    assert!(Fret::try_from(24u8).is_ok(), "fret 24 is valid");
    assert_error_code(Fret::try_from(25u8).map(|_| ()), "OutOfRange");
}

#[test]
fn open_pitch_is_restricted_to_zero_through_127() {
    assert!(OpenPitch::try_from(0u8).is_ok(), "open pitch 0 is valid");
    assert!(
        OpenPitch::try_from(127u8).is_ok(),
        "open pitch 127 is valid"
    );
    assert_error_code(OpenPitch::try_from(128u8).map(|_| ()), "OutOfRange");
}

#[test]
fn sounding_pitch_represents_open_127_plus_fret_24() {
    assert!(
        OpenPitch::try_from(127u8).is_ok(),
        "open pitch 127 is valid"
    );
    assert!(Fret::try_from(24u8).is_ok(), "fret 24 is valid");

    let spanning = SoundingPitch::try_from(127u16 + 24u16)
        .expect("open pitch 127 plus fret 24 must be representable");
    let top_of_open = SoundingPitch::try_from(127u16).expect("127 as a sounding pitch");
    assert_ne!(
        spanning, top_of_open,
        "sounding pitch must widen past 127 instead of clamping"
    );
}

#[test]
fn chords_are_ordered_occurrences_not_a_set() {
    let c_major = chord(0, "major");
    let a_min7 = chord(9, "min7");
    let occurrences = vec![c_major, a_min7, c_major];

    let state = page(
        default_state().instrument,
        occurrences.clone(),
        None,
        Tab::Visualizer,
    );
    assert_valid(validate_state(&state));
    assert_eq!(
        state.chords.len(),
        3,
        "duplicate chords must be kept as separate occurrences"
    );
    assert_eq!(state.chords, occurrences, "chord order must be preserved");
    assert_eq!(
        state.chords[0], state.chords[2],
        "a repeated chord keeps its identity"
    );
    assert_ne!(state.chords[0], state.chords[1]);
}

#[test]
fn identity_is_root_and_quality_not_the_pitch_set() {
    // C6 and Amin7 contain the same four pitch classes; root plus quality keeps
    // them distinct chords.
    let c_six = chord(0, "maj6");
    let a_min7 = chord(9, "min7");
    assert_ne!(c_six, a_min7, "C6 and Amin7 are different chords");

    assert_ne!(
        chord(0, "major"),
        chord(0, "minor"),
        "same root, other quality"
    );
    assert_ne!(
        chord(0, "major"),
        chord(7, "major"),
        "other root, same quality"
    );
    assert_eq!(chord(0, "major"), chord(0, "major"));

    let state = page(
        default_state().instrument,
        vec![c_six, a_min7],
        Some(a_min7),
        Tab::Visualizer,
    );
    assert_valid(validate_state(&state));
}

#[test]
fn highlight_must_be_an_identity_that_occurs_in_the_chords() {
    let c_major = chord(0, "major");
    let a_minor = chord(9, "minor");

    let present = page(
        default_state().instrument,
        vec![c_major, a_minor, c_major],
        Some(c_major),
        Tab::Visualizer,
    );
    assert_valid(validate_state(&present));

    let absent = page(
        default_state().instrument,
        vec![c_major, a_minor],
        Some(chord(7, "major")),
        Tab::Visualizer,
    );
    assert_error_code(validate_state(&absent), "InvalidState");

    let none = page(
        default_state().instrument,
        vec![chord(9, "minor")],
        None,
        Tab::Visualizer,
    );
    assert_valid(validate_state(&none));
}

#[test]
fn tuning_state_carries_exact_pitches_and_a_preset_reference() {
    let expected_pitches = preset_pitches("guitar", "Standard");

    let tuning = preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
        .expect("the guitar Standard preset resolves");

    assert_eq!(tuning.pitches, expected_pitches);
    assert_eq!(tuning.reference, preset_name("Standard"));
    assert!(
        preset_names("guitar").contains(&tuning.reference.to_string()),
        "the reference must be a guitar preset from the oracle"
    );
}

#[test]
fn tuning_rejects_a_wrong_pitch_count_and_a_foreign_reference() {
    let standard = preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
        .expect("the guitar Standard preset resolves");

    let mut too_few = standard.clone();
    too_few.pitches.pop();
    assert_error_code(
        validate_state(&fretted_page(InstrumentId::Guitar, too_few, Vec::new())),
        "InvalidState",
    );

    let mut foreign = standard;
    foreign.reference = preset_name(&ukelele_only_preset_name());
    assert_error_code(
        validate_state(&fretted_page(InstrumentId::Guitar, foreign, Vec::new())),
        "InvalidState",
    );
}

#[test]
fn unknown_identifier_strings_are_rejected() {
    assert_error_code(
        "violin".parse::<InstrumentId>().map(|_| ()),
        "UnknownIdentifier",
    );
    assert_error_code("nope".parse::<QualityId>().map(|_| ()), "UnknownIdentifier");
    assert_error_code("nope".parse::<ScaleId>().map(|_| ()), "UnknownIdentifier");
    assert_error_code(
        "Nope".parse::<PresetName>().map(|_| ()),
        "UnknownIdentifier",
    );
}

#[test]
fn catalog_identifier_strings_round_trip() {
    for id in instrument_ids() {
        let parsed: InstrumentId = id.parse().unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(parsed.to_string(), id, "instrument id must round-trip");
    }
}

#[test]
fn invalid_construction_returns_err_without_panicking() {
    for value in [25u8, 200u8, 255u8] {
        let outcome = std::panic::catch_unwind(|| Fret::try_from(value));
        assert!(outcome.is_ok(), "constructing fret {value} must not panic");
        assert!(
            outcome.expect("checked above").is_err(),
            "fret {value} must be rejected"
        );
    }
}

#[test]
fn a_rejected_state_is_an_err_and_is_never_mutated() {
    let invalid = page(
        InstrumentState::Piano {
            selected: vec![open_pitch(60), open_pitch(60)],
        },
        vec![chord(0, "major")],
        Some(chord(7, "major")),
        Tab::Analyzer,
    );
    let before = invalid.clone();

    assert_error_code(validate_state(&invalid), "InvalidState");
    assert_eq!(
        invalid, before,
        "a failed validation must not mutate its input"
    );
}

#[test]
fn snapshot_example_matches_the_frozen_schema() {
    let text = read_repo_file("fixtures/contract/snapshot-v1.json");
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("snapshot-v1.json is valid JSON");

    assert_eq!(
        object_keys(&value),
        name_set(&["schema_version", "page"]),
        "snapshot top-level keys"
    );
    assert_eq!(value["schema_version"], serde_json::json!(1));

    let page = value["page"].as_object().expect("page is an object");
    assert_eq!(
        keys_of(page),
        name_set(&["instrument", "chords", "highlight", "tab"]),
        "page keys"
    );

    let instrument = page["instrument"]
        .as_object()
        .expect("instrument is an object");
    let kind = instrument["kind"]
        .as_str()
        .expect("instrument kind is a string");
    assert!(matches!(kind, "fretted" | "piano"), "unknown kind {kind}");
    assert!(instrument.contains_key("id"), "instrument carries an id");
    assert!(
        instrument.contains_key("selection"),
        "instrument carries a selection"
    );
    match kind {
        "fretted" => {
            let tuning = instrument["tuning"].as_object().expect("fretted tuning");
            assert_eq!(keys_of(tuning), name_set(&["pitches", "reference"]));
        }
        _ => {
            assert!(
                !instrument.contains_key("tuning"),
                "piano must not carry a tuning"
            );
        }
    }

    let quality_ids = quality_ids();
    for entry in page["chords"].as_array().expect("chords is an array") {
        let chord = entry.as_object().expect("chord is an object");
        assert_eq!(keys_of(chord), name_set(&["root", "quality"]));
        let root = chord["root"].as_str().expect("root is a string");
        assert!(
            SHARP_NOTE_NAMES.contains(&root),
            "root {root} is not a sharp pitch-class name"
        );
        let quality = chord["quality"].as_str().expect("quality is a string");
        assert!(
            quality_ids.iter().any(|id| id == quality),
            "quality {quality} is not a catalog identifier"
        );
    }

    match &page["highlight"] {
        serde_json::Value::Null => {}
        serde_json::Value::Object(highlight) => {
            assert_eq!(keys_of(highlight), name_set(&["root", "quality"]));
        }
        other => panic!("highlight must be null or a chord object, got {other}"),
    }

    let tab = page["tab"].as_str().expect("tab is a string");
    assert!(
        matches!(tab, "visualizer" | "analyzer"),
        "unknown tab {tab}"
    );
}

#[test]
fn snapshot_example_deserialises_into_the_typed_page_state() {
    let text = read_repo_file("fixtures/contract/snapshot-v1.json");
    let value: serde_json::Value =
        serde_json::from_str(&text).expect("snapshot-v1.json is valid JSON");
    let page_json = serde_json::to_string(&value["page"]).expect("page re-serialises");

    let state: PageState = serde_json::from_str(&page_json)
        .expect("the frozen page object must deserialise into PageState");
    assert_valid(validate_state(&state));
}

#[test]
fn api_example_matches_the_frozen_adapter_api() {
    let text = read_repo_file("fixtures/contract/api-v1.json");
    let value: serde_json::Value = serde_json::from_str(&text).expect("api-v1.json is valid JSON");

    assert_eq!(
        object_keys(&value),
        name_set(&[
            "api_version",
            "snapshot_schema_version",
            "binding_package",
            "capabilities",
        ]),
        "api top-level keys"
    );
    assert_eq!(value["api_version"], serde_json::json!(1));
    assert_eq!(value["snapshot_schema_version"], serde_json::json!(1));
    assert_eq!(
        value["binding_package"],
        serde_json::json!("dev.ironjanowar.fretboard.core")
    );
    assert!(!value["capabilities"].is_null(), "capabilities is present");
}
