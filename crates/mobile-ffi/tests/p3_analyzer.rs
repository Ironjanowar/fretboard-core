//! The P3 analyzer boundary: the C13 slice driven through the **exported**
//! functions of the adapter.
//!
//! The domain tests pin the rules; these pin the crossing — that the inputs
//! arrive, become the domain's values, and come back as the DTOs the frozen
//! oracle records, including the tab gate, the draft that never touches the
//! page, and the typed failures.
//!
//! Expectations come from `fixtures/oracle/analyzer.jsonl` (all 34 cases) and
//! `fixtures/oracle/page-events.jsonl` (the 19 fretted cases of `C13`), never
//! from the adapter.
//!
//! The *order* of a chord answer's identifications is pinned by the domain's
//! `tests/analyzer.rs` against the approved `Contract.D03` tie rule; here the
//! identifications are compared as records keyed by `(root, quality)`, which is
//! what the crossing can add or lose.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use fretboard_mobile_ffi::{
    AdapterError, AnalysisDto, ChordDto, ErrorCode, InstrumentDto, InstrumentStateDto,
    InterpretationDto, PageEventDto, PageStateDto, PositionDto, TabDto, TuningDto, analyze_page,
    analyze_pitches, apply_page_event, change_tuning_string, default_state, open_tuning_draft,
    select_tuning_preset,
};
use serde_json::{Value, json};

const ANALYZER: &str = "fixtures/oracle/analyzer.jsonl";
const EVENTS: &str = "fixtures/oracle/page-events.jsonl";

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

/// The records of one operation.
fn operation(operation: &str) -> Vec<Value> {
    records(ANALYZER)
        .into_iter()
        .filter(|record| record["operation"].as_str() == Some(operation))
        .collect()
}

/// The case identifier of a record.
fn case_id(record: &Value) -> String {
    record["case_id"].as_str().expect("a case id").to_owned()
}

/// The canonical text of a JSON value; `serde_json`'s map is sorted, so the
/// comparison never depends on a key order.
fn canonical(value: &Value) -> String {
    serde_json::to_string(value).unwrap_or_else(|error| panic!("cannot encode {error:?}"))
}

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

/// One interpretation DTO as the JSON object the fixture spells.
fn interpretation_json(entry: &InterpretationDto) -> Value {
    json!({
        "root": entry.root,
        "quality": entry.quality,
        "exact": entry.exact,
        "incomplete": entry.incomplete,
        "notes": entry.notes,
        "intervals": entry.intervals,
        "missing_intervals": entry.missing_intervals,
        "bass": entry.bass,
        "inversion": entry.inversion,
        "slash_label": entry.slash_label,
    })
}

/// One analysis DTO as the JSON object the fixture spells.
fn analysis_json(analysis: &AnalysisDto) -> Value {
    match analysis {
        AnalysisDto::Empty => json!({"variant": "empty", "fields": []}),
        AnalysisDto::Single { note } => json!({"variant": "single", "fields": [note]}),
        AnalysisDto::Interval { low, high, label } => {
            json!({"variant": "interval", "fields": [low, high, label]})
        }
        AnalysisDto::Chords {
            notes,
            bass,
            interpretations,
        } => json!({
            "variant": "chords",
            "fields": [
                notes,
                bass,
                interpretations
                    .iter()
                    .map(interpretation_json)
                    .collect::<Vec<Value>>(),
            ],
        }),
    }
}

/// Compare one crossing answer with the frozen record.
///
/// A non-chord answer must equal the frozen one exactly. A chord answer must
/// name the same notes and the same bass, and carry the same identifications
/// keyed by `(root, quality)` — the ordered list is the domain test's.
fn check_answer(case: &str, ours: &AnalysisDto, frozen: &Value) {
    let our_json = analysis_json(ours);
    if canonical(&our_json) == canonical(frozen) {
        return;
    }

    assert_eq!(
        our_json["variant"], frozen["variant"],
        "{case}: the analysis variant differs"
    );
    assert_eq!(
        frozen["variant"].as_str(),
        Some("chords"),
        "{case}: only a chord answer may differ in the identification order"
    );

    let our_fields = our_json["fields"].as_array().expect("fields");
    let frozen_fields = frozen["fields"].as_array().expect("fields");
    assert_eq!(
        canonical(&our_fields[0]),
        canonical(&frozen_fields[0]),
        "{case}: the note list differs"
    );
    assert_eq!(
        canonical(&our_fields[1]),
        canonical(&frozen_fields[1]),
        "{case}: the bass differs"
    );

    let our_entries = our_fields[2].as_array().expect("interpretations");
    let frozen_entries = frozen_fields[2].as_array().expect("interpretations");
    assert_eq!(
        our_entries.len(),
        frozen_entries.len(),
        "{case}: a different number of identifications crossed"
    );
    for expected in frozen_entries {
        let key = (
            expected["root"].as_str().expect("a root"),
            expected["quality"].as_str().expect("a quality"),
        );
        let found = our_entries
            .iter()
            .find(|entry| {
                entry["root"].as_str() == Some(key.0) && entry["quality"].as_str() == Some(key.1)
            })
            .unwrap_or_else(|| panic!("{case}: the identification {key:?} did not cross"));
        assert_eq!(
            canonical(found),
            canonical(expected),
            "{case}: the identification {key:?} differs"
        );
    }
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

/// The fretted page state DTO of one `analyzer_state` record.
fn state_record_dto(record: &Value, tab: TabDto) -> PageStateDto {
    let mut strings: Vec<u8> = record["input"]["marked_notes"]
        .as_object()
        .expect("marked notes are an object")
        .keys()
        .map(|string| string.parse::<u8>().expect("a string index"))
        .collect();
    strings.sort_unstable();

    PageStateDto {
        instrument: InstrumentStateDto::Fretted {
            instrument: instrument_of(
                record["input"]["instrument"]
                    .as_str()
                    .expect("an instrument"),
            ),
            tuning: TuningDto {
                pitches: pitches_of(&record["input"]["string_pitches"]),
                reference: record["input"]["preset"]
                    .as_str()
                    .expect("a preset")
                    .to_owned(),
            },
            selected: strings
                .into_iter()
                .map(|string| PositionDto {
                    string,
                    fret: u8::try_from(
                        record["input"]["marked_notes"][string.to_string()]
                            .as_u64()
                            .expect("a fret"),
                    )
                    .expect("fits"),
                })
                .collect(),
        },
        chords: Vec::new(),
        highlight: None,
        tab,
    }
}

/// The failure of one call, with its code checked.
fn code_of<T: std::fmt::Debug>(result: Result<T, AdapterError>) -> ErrorCode {
    match result {
        Ok(value) => panic!("expected a failure, got Ok({value:?})"),
        Err(error) => error.code(),
    }
}

/// Every frozen `analyze_pitches/1` record crosses as the recorded answer.
#[test]
fn every_frozen_pitch_answer_crosses_the_boundary() {
    let records = operation("analyze_pitches");
    assert_eq!(records.len(), 13, "the frozen pitch records");

    for record in &records {
        let case = case_id(record);
        let pitches: Vec<u16> = record["input"]["pitches"]
            .as_array()
            .expect("pitches")
            .iter()
            .map(|pitch| u16::try_from(pitch.as_u64().expect("a pitch")).expect("fits"))
            .collect();
        let analysis = analyze_pitches(pitches).unwrap_or_else(|error| panic!("{case}: {error:?}"));
        check_answer(&case, &analysis, &record["output"]["analysis"]);
    }
}

/// Every frozen `analyzer_state/2` record crosses through its marked positions,
/// and the tab gate crosses with it: the same page on the visualizer tab answers
/// nothing.
#[test]
fn every_frozen_position_answer_crosses_the_boundary() {
    let records = operation("analyzer_state");
    assert_eq!(records.len(), 21, "the frozen position records");

    for record in &records {
        let case = case_id(record);
        let state = state_record_dto(record, TabDto::Analyzer);
        let analysis = analyze_page(state.clone())
            .unwrap_or_else(|error| panic!("{case}: {error:?}"))
            .unwrap_or_else(|| panic!("{case}: the analyzer tab answers an analysis"));
        check_answer(&case, &analysis, &record["output"]["analysis"]);

        assert_eq!(
            analyze_page(PageStateDto {
                tab: TabDto::Visualizer,
                ..state
            })
            .unwrap_or_else(|error| panic!("{case}: {error:?}")),
            None,
            "{case}: the visualizer tab carries no analysis"
        );
    }
}

/// The analysis of the analyzer tab depends on the selection, never on the
/// stored visualizer chords: adding chords and a highlight changes nothing.
#[test]
fn the_crossed_analysis_does_not_see_the_stored_chords() {
    let record = records(ANALYZER)
        .into_iter()
        .find(|record| case_id(record) == "analyzer_state/guitar-c-major-triad-shape")
        .expect("the fixture carries the case");
    let state = state_record_dto(&record, TabDto::Analyzer);
    let plain = analyze_page(state.clone())
        .expect("a state")
        .expect("an analysis");

    let cmaj = ChordDto {
        root: "C".to_owned(),
        quality: "major".to_owned(),
    };
    let chorded = PageStateDto {
        chords: vec![
            cmaj.clone(),
            ChordDto {
                root: "A".to_owned(),
                quality: "min7".to_owned(),
            },
        ],
        highlight: Some(cmaj),
        ..state
    };
    assert_eq!(
        analyze_page(chorded).expect("a state"),
        Some(plain),
        "the stored chords must not reach the analyzer"
    );
}

/// An empty selection on the analyzer tab is the computed `empty` answer, which
/// is not the `None` of the visualizer tab.
#[test]
fn an_empty_selection_is_an_empty_answer_not_an_absent_one() {
    let state = PageStateDto {
        tab: TabDto::Analyzer,
        ..default_state()
    };
    assert_eq!(
        analyze_page(state).expect("a state"),
        Some(AnalysisDto::Empty),
        "the analyzer tab computes an empty analysis"
    );
}

/// A pitch outside the sounding range is a typed rejection, never a clamped or
/// wrapped answer; the maximum itself is accepted.
#[test]
fn an_out_of_range_pitch_is_a_typed_failure() {
    assert_eq!(
        analyze_pitches(vec![151]).expect("the maximum sounding pitch is accepted"),
        AnalysisDto::Single {
            note: "G".to_owned()
        },
        "151 is 127 + 24, the highest sounding pitch"
    );
    assert_eq!(
        code_of(analyze_pitches(vec![152])),
        ErrorCode::OutOfRange,
        "one above the sounding range"
    );
}

/// One step of a fretted case, driven through the exported functions.
///
/// Returns the page, the draft and how many patches this step pushed — the
/// baseline's own rule: the tuning modal's Apply pushes unconditionally on a
/// fretted page, and every other event pushes exactly when the page changed.
fn apply_fretted_step(
    case: &str,
    state: PageStateDto,
    draft: Option<TuningDto>,
    step: &Value,
) -> (PageStateDto, Option<TuningDto>, u64) {
    let instrument = match &state.instrument {
        InstrumentStateDto::Fretted { instrument, .. } => *instrument,
        InstrumentStateDto::Piano { .. } => InstrumentDto::Piano,
    };
    let mut next_draft = draft.clone();
    let (next, patches) = match step["event"].as_str() {
        Some("toggle_note") => {
            let value = &step["value"];
            let event = PageEventDto::ToggleNote {
                position: PositionDto {
                    string: value["string"]
                        .as_str()
                        .expect("a string")
                        .parse()
                        .expect("a number"),
                    fret: value["fret"]
                        .as_str()
                        .expect("a fret")
                        .parse()
                        .expect("a number"),
                },
            };
            let next = apply_page_event(state.clone(), event).expect("the event applies");
            (next.clone(), u64::from(next != state))
        }
        Some("clear_notes") => {
            let next = apply_page_event(state.clone(), PageEventDto::ClearSelection)
                .expect("the event applies");
            (next.clone(), u64::from(next != state))
        }
        Some("toggle_tab") => {
            let tab = TabDto::parse(step["value"]["tab"].as_str().expect("a tab"))
                .unwrap_or(TabDto::Visualizer);
            let next = apply_page_event(state.clone(), PageEventDto::SetTab { tab })
                .expect("the event applies");
            (next.clone(), u64::from(next != state))
        }
        Some("open_tuning_modal") => {
            next_draft = open_tuning_draft(state.clone()).expect("a state");
            (state, 0)
        }
        Some("select_preset") => {
            let name = step["value"]["preset"].as_str().expect("a preset");
            next_draft = Some(
                select_tuning_preset(instrument, draft.expect("an open draft"), name.to_owned())
                    .expect("the draft selection applies"),
            );
            (state, 0)
        }
        Some("change_string") => {
            let value = &step["value"];
            next_draft = Some(
                change_tuning_string(
                    instrument,
                    draft.expect("an open draft"),
                    value["string"].as_str().expect("a string").to_owned(),
                    value["note"].as_str().expect("a note").to_owned(),
                )
                .expect("the draft edit applies"),
            );
            (state, 0)
        }
        Some("close_tuning_modal") => {
            next_draft = None;
            (state, 0)
        }
        Some("apply_tuning") => match draft {
            Some(tuning) => (
                apply_page_event(state, PageEventDto::CommitTuning { tuning })
                    .expect("the commit applies"),
                1,
            ),
            None => (state, 0),
        },
        Some("add_chord") => {
            let chord = &step["value"]["chord"];
            let next = apply_page_event(
                state.clone(),
                PageEventDto::AddChord {
                    chord: ChordDto {
                        root: chord["root"].as_str().expect("a root").to_owned(),
                        quality: chord["quality"].as_str().expect("a quality").to_owned(),
                    },
                },
            )
            .expect("the event applies");
            (next.clone(), u64::from(next != state))
        }
        None => {
            // A recorded `change` of the instrument select, which also closes
            // the tuning modal.
            let instrument =
                instrument_of(step["value"]["instrument"].as_str().expect("an instrument"));
            let next = apply_page_event(state.clone(), PageEventDto::SetInstrument { instrument })
                .expect("the event applies");
            let patches = u64::from(next != state);
            next_draft = if patches == 1 { None } else { draft };
            (next, patches)
        }
        other => panic!("{case}: unhandled step {other:?}: {step}"),
    };
    (next, next_draft, patches)
}

/// The fretted page events and the tuning draft of `C13` cross the boundary and
/// reach the frozen final pages with the frozen patch counts.
#[test]
fn every_frozen_fretted_case_crosses_the_boundary() {
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
        let mut draft: Option<TuningDto> = None;
        let mut patches = 0u64;
        for step in record["input"]["steps"].as_array().expect("steps") {
            let (next, next_draft, pushed) = apply_fretted_step(case, state, draft, step);
            state = next;
            draft = next_draft;
            patches += pushed;
        }

        assert_eq!(
            patches, frozen_patches,
            "{case}: the crossing pushed a different number of patches"
        );
        assert_eq!(state, expected, "{case}: the final page differs");
    }
}

/// The 19 fretted cases of the frozen page-event fixture.
const OWNED: [&str; 19] = [
    "page_event/applied-tuning-survives-unrelated-patch",
    "page_event/change-instrument-closes-open-tuning-draft",
    "page_event/change-instrument-fretted-to-fretted-keeps-fitting-selection",
    "page_event/change-instrument-same-is-noop",
    "page_event/change-tab-invalid-value-falls-back-to-visualizer",
    "page_event/change-tab-same-is-noop",
    "page_event/change-tab-to-analyzer-keeps-selection",
    "page_event/clear-notes-fretted",
    "page_event/toggle-note-adds-position",
    "page_event/toggle-note-different-fret-replaces-string",
    "page_event/toggle-note-same-position-removes",
    "page_event/toggle-note-second-string-keeps-first",
    "page_event/tuning-draft-close-discards-draft",
    "page_event/tuning-draft-edit-string-then-apply",
    "page_event/tuning-draft-invalid-note-ignored",
    "page_event/tuning-draft-invalid-string-index-ignored",
    "page_event/tuning-draft-select-preset-then-apply",
    "page_event/tuning-draft-unknown-preset-ignored",
    "page_event/tuning-modal-events-ignored-on-piano",
];
