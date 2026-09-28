// Test target: `expect`/`unwrap`, panicking assertions and direct indexing are the
// idiom in tests, so the restriction lints that forbid them in the library are
// relaxed here only. Every other lint, including `pedantic`, still applies.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

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

use std::collections::BTreeSet;

use common::{
    SHARP_NOTE_NAMES, all_preset_names, assert_deserialisation_rejected, assert_error_code,
    assert_valid, chord, error_code, fretted_page, instrument_ids, keys_of, name_set, object_keys,
    open_pitch, oracle_count, page, pitch_class_set, position, preset_name, preset_names,
    preset_pitches, quality_ids, read_repo_file, scale_ids, ukelele_only_preset_name,
};
use fretboard_core::{
    CoreError, Fret, InstrumentId, InstrumentState, OpenPitch, PageState, PresetName, QualityId,
    ScaleId, SoundingPitch, Tab, TuningState, default_state, preset_tuning, validate_state,
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

    assert_valid(validate_state(&piano));
    assert_valid(validate_state(&default_state()));

    // The kinds are not interchangeable. The fretted shape carrying the piano is
    // the only representation that could attach a tuning to a keyboard, so it is
    // neither reachable through the snapshot wire nor accepted by
    // `validate_state`; a `matches!` on a variant this test just built proves
    // nothing.
    let json = r#"{"kind":"fretted","id":"piano","tuning":{"pitches":[40,45,50,55,59,64],"reference":"Standard"},"selection":[]}"#;
    let error = assert_deserialisation_rejected::<InstrumentState>(json);
    assert!(
        error.to_string().contains("instrument"),
        "the rejection must name the instrument field, got: {error}"
    );

    let built = page(
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
    assert_error_code(validate_state(&built), "InvalidState");
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
    // C6 and Amin7 contain the same four pitch classes, checked against the
    // oracle formulas: maj6 = [0,4,7,9] on C, min7 = [0,3,7,10] on A.
    let c_six = chord(0, "maj6");
    let a_min7 = chord(9, "min7");
    assert_eq!(
        pitch_class_set(0, "maj6"),
        pitch_class_set(9, "min7"),
        "C6 and Amin7 must name the same pitch classes"
    );

    // They stay distinct because identity is root plus quality, not the set.
    assert_ne!(c_six.root, a_min7.root, "same pitch set, different root");
    assert_ne!(
        c_six.quality, a_min7.quality,
        "same pitch set, different quality"
    );
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

    // The chord list keeps both identities as separate occurrences.
    let state = page(
        default_state().instrument,
        vec![c_six, a_min7],
        Some(a_min7),
        Tab::Visualizer,
    );
    assert_valid(validate_state(&state));
    assert_eq!(state.chords.len(), 2, "both identities stay in the list");
    assert_eq!(state.chords[0], c_six);
    assert_eq!(state.chords[1], a_min7);
    assert_ne!(state.chords[0], state.chords[1]);

    // A repeated occurrence is a further entry, not a deduplicated set.
    let repeated = page(
        default_state().instrument,
        vec![c_six, a_min7, c_six],
        None,
        Tab::Visualizer,
    );
    assert_valid(validate_state(&repeated));
    assert_eq!(
        repeated.chords.len(),
        3,
        "a repeated chord is a separate occurrence"
    );
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
fn validation_failures_report_the_exact_stable_code() {
    // `validate_state` is the single rejection path for a page whose invariants
    // the field types cannot express. Its signature borrows the state, so
    // "not mutated" is a type guarantee, not a behaviour; what a caller can
    // depend on is which frozen code each violation reports.
    let standard = preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
        .expect("the guitar Standard preset resolves");

    // A fretted state carrying the piano.
    let fretted_piano = page(
        InstrumentState::Fretted {
            instrument: InstrumentId::Piano,
            tuning: standard.clone(),
            selected: Vec::new(),
        },
        Vec::new(),
        None,
        Tab::Visualizer,
    );
    assert_error_code(validate_state(&fretted_piano), "InvalidState");

    // A pitch count the instrument does not have.
    let mut too_short = standard.clone();
    too_short.pitches.pop();
    assert_error_code(
        validate_state(&fretted_page(InstrumentId::Guitar, too_short, Vec::new())),
        "InvalidState",
    );

    // A preset reference from another instrument.
    let mut foreign = standard.clone();
    foreign.reference = preset_name(&ukelele_only_preset_name());
    assert_error_code(
        validate_state(&fretted_page(InstrumentId::Guitar, foreign, Vec::new())),
        "InvalidState",
    );

    // A string index the instrument does not have is a numeric range violation.
    assert_error_code(
        validate_state(&fretted_page(
            InstrumentId::Guitar,
            standard.clone(),
            vec![position(6, 0)],
        )),
        "OutOfRange",
    );

    // Duplicate and descending string indices are structural violations.
    assert_error_code(
        validate_state(&fretted_page(
            InstrumentId::Guitar,
            standard.clone(),
            vec![position(2, 0), position(2, 5)],
        )),
        "InvalidState",
    );
    assert_error_code(
        validate_state(&fretted_page(
            InstrumentId::Guitar,
            standard,
            vec![position(3, 0), position(1, 0)],
        )),
        "InvalidState",
    );

    // A piano key outside 48..=83 is a numeric range violation.
    assert_error_code(
        validate_state(&page(
            InstrumentState::Piano {
                selected: vec![open_pitch(47)],
            },
            Vec::new(),
            None,
            Tab::Analyzer,
        )),
        "OutOfRange",
    );

    // A duplicate piano key is structural.
    assert_error_code(
        validate_state(&page(
            InstrumentState::Piano {
                selected: vec![open_pitch(60), open_pitch(60)],
            },
            Vec::new(),
            None,
            Tab::Analyzer,
        )),
        "InvalidState",
    );

    // A highlight absent from the chords is structural.
    let c_major = chord(0, "major");
    assert_error_code(
        validate_state(&page(
            default_state().instrument,
            vec![c_major],
            Some(chord(7, "major")),
            Tab::Visualizer,
        )),
        "InvalidState",
    );

    // An unknown catalog identifier is its own code.
    assert_error_code(
        "violin".parse::<InstrumentId>().map(|_| ()),
        "UnknownIdentifier",
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
    assert!(
        value["capabilities"].is_object(),
        "capabilities must be a JSON object, not {:?}",
        value["capabilities"]
    );
    assert_eq!(
        value["capabilities"],
        serde_json::json!({}),
        "capabilities is the empty object until C04 freezes the flag set"
    );
}

// ---------------------------------------------------------------------------
// Strict snapshot deserialisation (review follow-up)
//
// The frozen schema is strict: an unknown field anywhere in the snapshot page
// is an error, and deserialising a state type directly must reject the same
// violations `validate_state` rejects. A misspelled key silently becoming
// `None`, or a public `Deserialize` accepting a value the public API could not
// build, would let a corrupt snapshot alter state without a diagnostic. Where a
// violation has a typed equivalent the test also pins the exact `CoreError`
// code through `validate_state`, because a serde error does not carry the code.
// ---------------------------------------------------------------------------

const FRETTED_PAGE_JSON: &str = r#"{"instrument":{"kind":"fretted","id":"guitar","tuning":{"pitches":[40,45,50,55,59,64],"reference":"Standard"},"selection":[]},"chords":[],"highlight":null,"tab":"visualizer"}"#;
const PIANO_PAGE_JSON: &str = r#"{"instrument":{"kind":"piano","id":"piano","selection":[60]},"chords":[],"highlight":null,"tab":"analyzer"}"#;

#[test]
fn page_deserialisation_rejects_an_unknown_top_level_field() {
    let json = r#"{"instrument":{"kind":"piano","id":"piano","selection":[]},"chords":[],"highlight":null,"tab":"analyzer","zzz":5}"#;
    let error = assert_deserialisation_rejected::<PageState>(json);
    assert!(
        error.to_string().contains("zzz"),
        "the rejection must name the unknown field, got: {error}"
    );
}

#[test]
fn page_deserialisation_rejects_an_unknown_chord_field() {
    let json = r#"{"instrument":{"kind":"piano","id":"piano","selection":[]},"chords":[{"root":"C","quality":"major","zzz":4}],"highlight":null,"tab":"analyzer"}"#;
    let error = assert_deserialisation_rejected::<PageState>(json);
    assert!(
        error.to_string().contains("zzz"),
        "the rejection must name the unknown field, got: {error}"
    );
}

#[test]
fn page_deserialisation_rejects_an_unknown_instrument_field() {
    let json = r#"{"instrument":{"kind":"piano","id":"piano","selection":[],"zzz":1},"chords":[],"highlight":null,"tab":"analyzer"}"#;
    let error = assert_deserialisation_rejected::<PageState>(json);
    assert!(
        error.to_string().contains("zzz"),
        "the rejection must name the unknown field, got: {error}"
    );
}

#[test]
fn page_deserialisation_rejects_an_unknown_tuning_field() {
    let json = r#"{"instrument":{"kind":"fretted","id":"guitar","tuning":{"pitches":[40,45,50,55,59,64],"reference":"Standard","zzz":2},"selection":[]},"chords":[],"highlight":null,"tab":"visualizer"}"#;
    let error = assert_deserialisation_rejected::<PageState>(json);
    assert!(
        error.to_string().contains("zzz"),
        "the rejection must name the unknown field, got: {error}"
    );
}

#[test]
fn page_deserialisation_rejects_an_unknown_position_field() {
    let json = r#"{"instrument":{"kind":"fretted","id":"guitar","tuning":{"pitches":[40,45,50,55,59,64],"reference":"Standard"},"selection":[{"string":0,"fret":3,"zzz":0}]},"chords":[],"highlight":null,"tab":"visualizer"}"#;
    let error = assert_deserialisation_rejected::<PageState>(json);
    assert!(
        error.to_string().contains("zzz"),
        "the rejection must name the unknown field, got: {error}"
    );
}

#[test]
fn page_deserialisation_rejects_a_misspelled_key_instead_of_defaulting_it() {
    // A corrupt snapshot must not silently become `highlight: None` when the
    // key is misspelled: `hilight` is not the frozen field.
    let json = r#"{"instrument":{"kind":"piano","id":"piano","selection":[]},"chords":[],"hilight":{"root":"C","quality":"major"},"tab":"analyzer"}"#;
    let error = assert_deserialisation_rejected::<PageState>(json);
    assert!(
        error.to_string().contains("hilight"),
        "the rejection must name the misspelled key, got: {error}"
    );
}

#[test]
fn page_deserialisation_still_accepts_the_unmodified_pages() {
    // Guard against over-strictness: the strict readers must keep accepting the
    // frozen shapes, so rejection means the unknown field, not the whole reader.
    let fretted: PageState =
        serde_json::from_str(FRETTED_PAGE_JSON).expect("the frozen fretted page deserialises");
    assert_valid(validate_state(&fretted));
    let piano: PageState =
        serde_json::from_str(PIANO_PAGE_JSON).expect("the frozen piano page deserialises");
    assert_valid(validate_state(&piano));
}

#[test]
fn instrument_state_deserialisation_rejects_a_fretted_piano() {
    let json = r#"{"kind":"fretted","id":"piano","tuning":{"pitches":[40,45,50,55,59,64],"reference":"Standard"},"selection":[]}"#;
    assert_deserialisation_rejected::<InstrumentState>(json);

    // The same value, built through the public fields, is rejected with
    // `InvalidState`.
    let built = page(
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
    assert_error_code(validate_state(&built), "InvalidState");
}

#[test]
fn instrument_state_deserialisation_rejects_a_wrong_pitch_count() {
    let json = r#"{"kind":"fretted","id":"guitar","tuning":{"pitches":[40],"reference":"Standard"},"selection":[]}"#;
    let error = assert_deserialisation_rejected::<InstrumentState>(json);
    assert!(
        error.to_string().contains("tuning") || error.to_string().contains("pitch"),
        "the rejection must name the tuning, got: {error}"
    );

    let mut too_short = preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
        .expect("the guitar Standard preset resolves");
    too_short.pitches.pop();
    assert_error_code(
        validate_state(&fretted_page(InstrumentId::Guitar, too_short, Vec::new())),
        "InvalidState",
    );
}

#[test]
fn instrument_state_deserialisation_rejects_a_foreign_reference() {
    let json = r#"{"kind":"fretted","id":"guitar","tuning":{"pitches":[40,45,50,55,59,64],"reference":"Low G"},"selection":[]}"#;
    let error = assert_deserialisation_rejected::<InstrumentState>(json);
    assert!(
        error.to_string().contains("reference"),
        "the rejection must name the reference, got: {error}"
    );

    let mut foreign = preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
        .expect("the guitar Standard preset resolves");
    foreign.reference = preset_name("Low G");
    assert_error_code(
        validate_state(&fretted_page(InstrumentId::Guitar, foreign, Vec::new())),
        "InvalidState",
    );
}

#[test]
fn instrument_state_deserialisation_rejects_a_bad_fretted_selection() {
    let duplicate = r#"{"kind":"fretted","id":"guitar","tuning":{"pitches":[40,45,50,55,59,64],"reference":"Standard"},"selection":[{"string":2,"fret":0},{"string":2,"fret":5}]}"#;
    let error = assert_deserialisation_rejected::<InstrumentState>(duplicate);
    assert!(
        error.to_string().contains("selection"),
        "the rejection must name the selection, got: {error}"
    );

    let out_of_range = r#"{"kind":"fretted","id":"guitar","tuning":{"pitches":[40,45,50,55,59,64],"reference":"Standard"},"selection":[{"string":6,"fret":0}]}"#;
    assert_deserialisation_rejected::<InstrumentState>(out_of_range);

    let standard = preset_tuning(InstrumentId::Guitar, &preset_name("Standard"))
        .expect("the guitar Standard preset resolves");
    assert_error_code(
        validate_state(&fretted_page(
            InstrumentId::Guitar,
            standard.clone(),
            vec![position(2, 0), position(2, 5)],
        )),
        "InvalidState",
    );
    assert_error_code(
        validate_state(&fretted_page(
            InstrumentId::Guitar,
            standard,
            vec![position(6, 0)],
        )),
        "OutOfRange",
    );
}

#[test]
fn instrument_state_deserialisation_rejects_a_piano_key_out_of_range() {
    let json = r#"{"kind":"piano","id":"piano","selection":[47]}"#;
    assert_deserialisation_rejected::<InstrumentState>(json);

    assert_error_code(
        validate_state(&page(
            InstrumentState::Piano {
                selected: vec![open_pitch(47)],
            },
            Vec::new(),
            None,
            Tab::Analyzer,
        )),
        "OutOfRange",
    );
}

#[test]
fn instrument_state_deserialisation_rejects_a_duplicate_piano_key() {
    let json = r#"{"kind":"piano","id":"piano","selection":[60,60]}"#;
    assert_deserialisation_rejected::<InstrumentState>(json);

    assert_error_code(
        validate_state(&page(
            InstrumentState::Piano {
                selected: vec![open_pitch(60), open_pitch(60)],
            },
            Vec::new(),
            None,
            Tab::Analyzer,
        )),
        "InvalidState",
    );
}

#[test]
fn piano_instrument_deserialisation_rejects_any_tuning_key() {
    // Tuning is fretted-only: the key must be an error whether it is null or a
    // well-formed tuning, so a stray key cannot be ignored.
    let null_tuning = r#"{"kind":"piano","id":"piano","selection":[60],"tuning":null}"#;
    let error = assert_deserialisation_rejected::<InstrumentState>(null_tuning);
    assert!(
        error.to_string().contains("tuning"),
        "the rejection must name the tuning, got: {error}"
    );

    let object_tuning = r#"{"kind":"piano","id":"piano","selection":[60],"tuning":{"pitches":[40,45,50,55,59,64],"reference":"Standard"}}"#;
    assert_deserialisation_rejected::<InstrumentState>(object_tuning);
}

#[test]
fn tuning_state_deserialisation_rejects_a_pitch_count_no_instrument_has() {
    // No fretted instrument has zero or one string, so `preset_tuning` could
    // never produce these tunings.
    assert_deserialisation_rejected::<TuningState>(r#"{"pitches":[],"reference":"Standard"}"#);
    let error = assert_deserialisation_rejected::<TuningState>(
        r#"{"pitches":[40],"reference":"Standard"}"#,
    );
    assert!(
        error.to_string().contains("tuning") || error.to_string().contains("pitch"),
        "the rejection must name the tuning, got: {error}"
    );
}

#[test]
fn tuning_state_deserialisation_rejects_a_reference_from_another_string_count() {
    // Six pitches identify the six-string guitar; "Low G" is the ukulele's
    // four-string preset, so this is not a tuning the public API could build.
    let json = r#"{"pitches":[40,45,50,55,59,64],"reference":"Low G"}"#;
    let error = assert_deserialisation_rejected::<TuningState>(json);
    assert!(
        error.to_string().contains("reference"),
        "the rejection must name the reference, got: {error}"
    );
}

#[test]
fn tuning_state_deserialisation_round_trips_every_frozen_preset() {
    // The strict reader must not reject the tunings the public API does build.
    for instrument in instrument_ids() {
        let id: InstrumentId = instrument
            .parse()
            .unwrap_or_else(|error| panic!("{instrument}: {error:?}"));
        for name in preset_names(&instrument) {
            let tuning = preset_tuning(id, &preset_name(&name))
                .unwrap_or_else(|error| panic!("{instrument} {name}: {error:?}"));
            let json = serde_json::to_string(&tuning).expect("a tuning serialises");
            let parsed: TuningState =
                serde_json::from_str(&json).unwrap_or_else(|error| panic!("{json}: {error:?}"));
            assert_eq!(parsed, tuning, "{instrument} {name} must round-trip");
        }
    }
}

// ---------------------------------------------------------------------------
// Frozen identifier catalogs (review follow-up)
//
// The catalog tables in `types.rs` and `state.rs` are the wire identifiers, but
// nothing pinned them: changing `"m11b5"` to `"m11b6"` kept the suite green.
// These tests read the ids from the frozen oracle and check the accepted set in
// both directions: every oracle id parses and round-trips, near misses are
// rejected, and the count matches the oracle. There is no public iterator over
// the accepted ids, so the backward direction is checked with near misses of the
// oracle ids and the oracle-declared counts.
// ---------------------------------------------------------------------------

#[test]
fn quality_ids_are_pinned_to_the_frozen_oracle() {
    let oracle = quality_ids();
    assert_eq!(
        oracle.len() as u64,
        oracle_count("chord_qualities"),
        "the oracle's own declared quality count"
    );
    assert_eq!(
        oracle.len(),
        47,
        "the frozen catalog has 47 chord qualities"
    );

    let mut unique = BTreeSet::new();
    for id in &oracle {
        assert!(
            unique.insert(id.clone()),
            "duplicate oracle quality id {id}"
        );
    }

    // Forward: every oracle id parses and round-trips through Display/FromStr.
    for id in &oracle {
        let parsed = QualityId::parse(id).unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(parsed.as_str(), id, "a quality id keeps its catalog text");
        assert_eq!(parsed.to_string(), *id, "Display must round-trip {id}");
        assert_eq!(
            id.parse::<QualityId>()
                .unwrap_or_else(|error| panic!("{id}: {error:?}")),
            parsed,
            "FromStr must round-trip {id}"
        );
    }

    // Backward: a near miss of a real id (the reviewer's `m11b5` -> `m11b6`)
    // and other non-catalog strings must not be accepted.
    for near_miss in ["m11b6", "m11b5 ", " M11B5", "MAJOR", "majorr", ""] {
        assert_error_code(QualityId::parse(near_miss).map(|_| ()), "UnknownIdentifier");
    }
}

#[test]
fn scale_ids_are_pinned_to_the_frozen_oracle() {
    let oracle = scale_ids();
    assert_eq!(
        oracle.len() as u64,
        oracle_count("scale_types"),
        "the oracle's own declared scale count"
    );
    assert_eq!(oracle.len(), 15, "the frozen catalog has 15 scale types");

    let mut unique = BTreeSet::new();
    for id in &oracle {
        assert!(unique.insert(id.clone()), "duplicate oracle scale id {id}");
        let parsed = ScaleId::parse(id).unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(parsed.as_str(), id, "a scale id keeps its catalog text");
        assert_eq!(parsed.to_string(), *id, "Display must round-trip {id}");
        assert_eq!(
            id.parse::<ScaleId>()
                .unwrap_or_else(|error| panic!("{id}: {error:?}")),
            parsed,
            "FromStr must round-trip {id}"
        );
    }

    for near_miss in ["majorr", "Major", "whole-tone", "chromatic_", ""] {
        assert_error_code(ScaleId::parse(near_miss).map(|_| ()), "UnknownIdentifier");
    }
}

#[test]
fn preset_names_are_pinned_to_the_frozen_oracle() {
    let oracle = all_preset_names();
    assert_eq!(
        oracle.len(),
        13,
        "the frozen catalog has 13 distinct preset names"
    );

    let mut unique = BTreeSet::new();
    for name in &oracle {
        assert!(unique.insert(name.clone()), "duplicate preset name {name}");
        let parsed = PresetName::parse(name).unwrap_or_else(|error| panic!("{name}: {error:?}"));
        assert_eq!(
            parsed.as_str(),
            name,
            "a preset name keeps its catalog text"
        );
        assert_eq!(parsed.to_string(), *name, "Display must round-trip {name}");
        assert_eq!(
            name.parse::<PresetName>()
                .unwrap_or_else(|error| panic!("{name}: {error:?}")),
            parsed,
            "FromStr must round-trip {name}"
        );
    }

    // Every (instrument, preset) pair of the oracle resolves through the public
    // API, which ties the accepted names to the catalog tables.
    for instrument in instrument_ids() {
        let id: InstrumentId = instrument
            .parse()
            .unwrap_or_else(|error| panic!("{instrument}: {error:?}"));
        for name in preset_names(&instrument) {
            assert!(
                preset_tuning(id, &preset_name(&name)).is_ok(),
                "{instrument} {name} must resolve to a tuning"
            );
        }
    }

    for near_miss in ["low g", "Low G ", "Standard ", "lowG", ""] {
        assert_error_code(
            PresetName::parse(near_miss).map(|_| ()),
            "UnknownIdentifier",
        );
    }
}

// ---------------------------------------------------------------------------
// Sounding-pitch cap and the stable error code (review follow-up)
// ---------------------------------------------------------------------------

#[test]
fn sounding_pitch_cap_is_exactly_open_127_plus_fret_24() {
    // The cap must be wide enough for open 127 plus fret 24 ...
    let open = OpenPitch::try_from(127u8).expect("open pitch 127 is valid");
    let fret = Fret::try_from(24u8).expect("fret 24 is valid");
    let highest_legal = u16::from(u8::from(open)) + u16::from(u8::from(fret));
    assert_eq!(highest_legal, 151, "open 127 plus fret 24 is 151");

    let cap = SoundingPitch::try_from(151u16).expect("151 must be representable");
    assert_eq!(u16::from(cap), 151, "151 must round-trip");

    // ... and no wider.
    assert_error_code(SoundingPitch::try_from(152u16).map(|_| ()), "OutOfRange");
    assert!(
        serde_json::from_str::<SoundingPitch>("151").is_ok(),
        "the serde boundary must accept 151"
    );
    assert!(
        serde_json::from_str::<SoundingPitch>("152").is_err(),
        "the serde boundary must reject 152"
    );
}

#[test]
fn core_error_code_is_the_stable_variant_name() {
    // The FFI adapter reads `code()`, not the `Debug` rendering.
    assert_eq!(CoreError::invalid_state("tuning").code(), "InvalidState");
    assert_eq!(CoreError::out_of_range("selection").code(), "OutOfRange");
    assert_eq!(
        CoreError::unknown_identifier("quality").code(),
        "UnknownIdentifier"
    );

    // The real failure paths of the contract report those same codes.
    let err = validate_state(&page(
        InstrumentState::Piano {
            selected: vec![open_pitch(60), open_pitch(60)],
        },
        Vec::new(),
        None,
        Tab::Analyzer,
    ))
    .expect_err("a duplicate piano key is an invalid state");
    assert_eq!(err.code(), "InvalidState");

    let err = Fret::try_from(25u8).expect_err("fret 25 is out of range");
    assert_eq!(err.code(), "OutOfRange");

    let err = "nope".parse::<QualityId>().expect_err("no such quality");
    assert_eq!(err.code(), "UnknownIdentifier");

    // `code()` and the derived `Debug` rendering agree, as the contract claims.
    for error in [
        CoreError::invalid_state("x"),
        CoreError::out_of_range("x"),
        CoreError::unknown_identifier("x"),
    ] {
        let code = error.code();
        assert!(
            format!("{error:?}").starts_with(code),
            "the Debug rendering of {error:?} must start with its code {code}"
        );
        assert_eq!(
            error_code(&error),
            code,
            "the shared helper must read the same code"
        );
    }
}
