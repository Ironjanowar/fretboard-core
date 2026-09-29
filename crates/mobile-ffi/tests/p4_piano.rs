//! The C14 adapter boundary: the piano selection, the instrument crossing and
//! the preset-name enumeration, driven through the **exported** functions.
//!
//! The domain tests pin the rules; these pin the crossing — that the inputs
//! arrive, become the domain's values, and come back as the DTOs the frozen
//! oracle records. Expectations come from `fixtures/oracle/page-events.jsonl`
//! (the nine piano and crossing cases of `C14`) and
//! `fixtures/oracle/catalogs.json` (the per-instrument preset names), never from
//! the adapter.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use fretboard_mobile_ffi::{
    AdapterError, ChordDto, InstrumentDto, InstrumentStateDto, PageEventDto, PageStateDto,
    PositionDto, TabDto, TuningDto, apply_page_event, default_state, presets, validate_state,
};
use serde_json::Value;

const EVENTS: &str = "fixtures/oracle/page-events.jsonl";
const CATALOGS: &str = "fixtures/oracle/catalogs.json";

/// Every record of a frozen fixture.
fn records(path: &str) -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every record is JSON"))
        .collect()
}

/// The nine C14 cases of the frozen page-event fixture.
const OWNED: [&str; 9] = [
    "page_event/change-instrument-fretted-to-piano-clears-selection",
    "page_event/change-instrument-piano-to-fretted-clears-keys",
    "page_event/change-instrument-piano-to-ukelele",
    "page_event/clear-notes-piano",
    "page_event/toggle-piano-key-adds-pitch",
    "page_event/toggle-piano-key-ignored-in-visualizer",
    "page_event/toggle-piano-key-outside-range-ignored",
    "page_event/toggle-piano-key-removes-pitch",
    "page_event/toggle-piano-key-without-activation-key-ignored",
];

/// The instrument DTO of a fixture identifier.
fn instrument_of(value: &str) -> InstrumentDto {
    InstrumentDto::parse(value).expect("the instrument is in the catalog")
}

/// The pitch list of a JSON array.
fn pitches_of(value: &Value) -> Vec<u8> {
    value
        .as_array()
        .expect("a pitch list")
        .iter()
        .map(|pitch| u8::try_from(pitch.as_u64().expect("a pitch")).expect("fits"))
        .collect()
}

/// The page state DTO of one fixture state, in the shape
/// `fixtures/oracle/page-events.jsonl` records it.
fn page_dto_of(state: &Value) -> PageStateDto {
    let instrument = instrument_of(state["instrument"].as_str().expect("an instrument"));
    let instrument_state = if instrument == InstrumentDto::Piano {
        InstrumentStateDto::Piano {
            selected: state["selection"]
                .as_array()
                .expect("piano keys are a list")
                .iter()
                .map(|pitch| u8::try_from(pitch.as_u64().expect("a key")).expect("fits"))
                .collect(),
        }
    } else {
        let tuning = &state["tuning_state"];
        let mut strings: Vec<u8> = state["selection"]
            .as_object()
            .expect("fretted marks are an object")
            .keys()
            .map(|string| string.parse::<u8>().expect("a string index"))
            .collect();
        strings.sort_unstable();
        InstrumentStateDto::Fretted {
            instrument,
            tuning: TuningDto {
                pitches: pitches_of(&tuning["pitches"]),
                reference: tuning["reference"]
                    .as_str()
                    .expect("a reference")
                    .to_owned(),
            },
            selected: strings
                .into_iter()
                .map(|string| PositionDto {
                    string,
                    fret: u8::try_from(
                        state["selection"][string.to_string()]
                            .as_u64()
                            .expect("a fret"),
                    )
                    .expect("fits"),
                })
                .collect(),
        }
    };

    let chords: Vec<ChordDto> = state["active_chords"]
        .as_array()
        .expect("a chord list")
        .iter()
        .map(|chord| ChordDto {
            root: chord["root"].as_str().expect("a root").to_owned(),
            quality: chord["quality"].as_str().expect("a quality").to_owned(),
        })
        .collect();
    let highlight = state["highlighted_chord"]
        .as_str()
        .and_then(|root| chords.iter().find(|chord| chord.root == root).cloned());

    PageStateDto {
        instrument: instrument_state,
        chords,
        highlight,
        tab: TabDto::parse(state["tab"].as_str().expect("a tab")).expect("a tab"),
    }
}

/// The failure of one call, with its code checked.
fn code_of<T: std::fmt::Debug>(result: Result<T, AdapterError>) -> String {
    match result {
        Ok(value) => panic!("expected a failure, got Ok({value:?})"),
        Err(error) => error.code().as_str().to_owned(),
    }
}

/// One step of a C14 case, driven through the exported functions.
///
/// A recorded step the domain's reader refuses (the pinned handler's activation
/// guard, which the adapter never sees) is one the DTO layer cannot express as a
/// toggle, so this driver checks the step and treats it as the recorded no-op.
/// The domain test `reducer_piano.rs` pins that the reader refuses exactly that
/// step; here the adapter's job is the crossing.
fn apply_piano_step(case: &str, state: PageStateDto, step: &Value) -> (PageStateDto, u64) {
    let event_name = step["event"].as_str();
    match event_name {
        Some("toggle_piano_key") => {
            let value = &step["value"];
            let key = value.get("key").and_then(Value::as_str);
            if !matches!(key, None | Some("Enter" | " ")) {
                return (state, 0);
            }
            let pitch: u8 = value["pitch"]
                .as_str()
                .expect("a pitch")
                .parse()
                .expect("a number");
            let next = apply_page_event(state.clone(), PageEventDto::TogglePianoKey { pitch })
                .expect("the event applies");
            (next.clone(), u64::from(next != state))
        }
        Some("clear_notes") => {
            let next = apply_page_event(state.clone(), PageEventDto::ClearSelection)
                .expect("the event applies");
            (next.clone(), u64::from(next != state))
        }
        None => {
            // A recorded `change` of the instrument select.
            let instrument =
                instrument_of(step["value"]["instrument"].as_str().expect("an instrument"));
            let next = apply_page_event(state.clone(), PageEventDto::SetInstrument { instrument })
                .expect("the event applies");
            (next.clone(), u64::from(next != state))
        }
        other => panic!("{case}: unhandled step {other:?}: {step}"),
    }
}

/// The nine frozen C14 cases cross the boundary and reach the frozen final pages
/// with the frozen patch counts.
#[test]
fn every_frozen_piano_case_crosses_the_boundary() {
    let records = records(EVENTS);
    for case in OWNED {
        let record = records
            .iter()
            .find(|record| record["case_id"].as_str() == Some(case))
            .unwrap_or_else(|| panic!("the fixture has no case {case}"));
        let output = &record["output"];
        let expected = page_dto_of(&output["final_state"]);
        let frozen_patches = output["patch_count"].as_u64().expect("a patch count");

        let mut state = page_dto_of(&output["initial_state"]);
        let mut patches = 0u64;
        for step in record["input"]["steps"].as_array().expect("steps") {
            let (next, pushed) = apply_piano_step(case, state, step);
            state = next;
            patches += pushed;
        }

        assert_eq!(
            patches, frozen_patches,
            "{case}: the crossing pushed a different number of patches"
        );
        assert_eq!(state, expected, "{case}: the final page differs");
    }
}

/// A piano key crosses as its DTO event and reaches the frozen page; a key
/// outside the keyboard crosses as the same event and changes nothing.
#[test]
fn the_piano_key_event_crosses_as_a_typed_toggle() {
    let records = records(EVENTS);
    let case = |id: &str| {
        records
            .iter()
            .find(|record| record["case_id"].as_str() == Some(id))
            .unwrap_or_else(|| panic!("the fixture has no case {id}"))
            .clone()
    };

    let added = case("page_event/toggle-piano-key-adds-pitch");
    let state = page_dto_of(&added["output"]["initial_state"]);
    let next = apply_page_event(state, PageEventDto::TogglePianoKey { pitch: 60 })
        .expect("the event applies");
    assert_eq!(next, page_dto_of(&added["output"]["final_state"]));

    let ignored = case("page_event/toggle-piano-key-outside-range-ignored");
    let state = page_dto_of(&ignored["output"]["initial_state"]);
    let next = apply_page_event(state.clone(), PageEventDto::TogglePianoKey { pitch: 47 })
        .expect("the event applies");
    assert_eq!(next, state, "47 is not a key of the keyboard");
}

/// The preset-name enumeration crosses per instrument and equals the frozen
/// catalog's ordered names; the piano enumerates none. This closes the gap
/// `A09` hit: the Android preset picker reads the names from the engine instead
/// of carrying a hand-written list.
#[test]
fn every_instrument_enumerates_its_frozen_preset_names() {
    let catalogs: Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../..")
                .join(CATALOGS),
        )
        .expect("the frozen catalog is readable"),
    )
    .expect("the frozen catalog is JSON");

    let groups = catalogs["instrument_pitch_presets"]
        .as_array()
        .expect("instrument_pitch_presets is an array");
    assert_eq!(groups.len(), 5, "the catalog carries five instruments");

    for group in groups {
        let id = group["instrument"].as_str().expect("an instrument id");
        let expected: Vec<String> = group["presets"]
            .as_array()
            .expect("presets is an array")
            .iter()
            .map(|preset| preset["name"].as_str().expect("a preset name").to_owned())
            .collect();
        assert_eq!(
            presets(instrument_of(id)),
            expected,
            "{id}: the enumerated preset names differ"
        );
    }

    assert!(
        presets(InstrumentDto::Piano).is_empty(),
        "the piano has no preset"
    );
}

/// The adapter keeps the two guards apart: a key outside the keyboard is a
/// *state* violation `validate_state` reports as `OutOfRange`, while a pitch the
/// wire type cannot even carry (`u8` past 127) is refused at the event
/// conversion, never clamped.
#[test]
fn a_rejected_piano_pitch_crosses_as_a_typed_failure() {
    let invalid = PageStateDto {
        instrument: InstrumentStateDto::Piano { selected: vec![47] },
        chords: Vec::new(),
        highlight: None,
        tab: TabDto::Analyzer,
    };
    assert_eq!(
        code_of(validate_state(invalid)),
        "OutOfRange",
        "47 is not a key of the keyboard"
    );

    assert_eq!(
        code_of(apply_page_event(
            default_state(),
            PageEventDto::TogglePianoKey { pitch: 200 }
        )),
        "OutOfRange",
        "a pitch above the sounding range is refused, never clamped"
    );
}
