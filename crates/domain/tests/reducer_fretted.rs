//! Task `C13`: the fretted reducer families against the frozen oracle.
//!
//! The page-event fixture (`fixtures/oracle/page-events.jsonl`, the pinned
//! `FretboardWeb.FretboardLive.handle_event/3`) records, for every case, the
//! parameters the page started from, the state after every step that pushed a
//! patch, the page it ended on, and how many patches the baseline sent. This
//! file drives the families `C13` owns and leaves every other case to its own
//! task: the identity events are `C09`'s (`reducer_identity.rs`), the piano keys
//! are `C14`'s, the key and progression application is `C18`'s.
//!
//! ## The two readers
//!
//! The fretted transitions split in two, exactly as the baseline does:
//!
//! * [`page_event`] reads the steps that change the *page* — one position
//!   toggled, the selection cleared, the tab switched, the instrument changed —
//!   into a [`PageEvent`], which [`apply_event`] applies to the page state;
//! * [`draft_event`] reads the steps of the **tuning modal**, which are UI-only:
//!   the baseline assigns `show_tuning_modal` / `modal_tuning_state` and pushes
//!   no patch at all. `02-core-contract.md` section 8 states the rule this file
//!   pins: *"Tuning draft operations live outside `PageState`; callers commit
//!   only on Apply."* The draft is therefore a plain [`TuningState`] beside the
//!   page, edited by [`select_tuning_preset`] and [`change_tuning_string`], and
//!   committed by [`PageEvent::CommitTuning`].
//!
//! `page_event` still returns `None` for the waves of a step it does not know
//! (`C14`'s piano keys, `C18`'s key modal), and that is not "the state stays the
//! same" for those events.
//!
//! ## The patch count
//!
//! [`patch_count`](Case::patches) is the baseline's own statement of which steps
//! *pushed*: the exporter counts a step as patched when the handler called
//! `push_patch` (`tools/oracle/page_events_test.exs`). For every event that
//! carries page state that is the same as "the state changed", but the tuning
//! modal's Apply is the one exception the baseline spells out: `apply_tuning`
//! pushes unconditionally on a fretted page — it also closes the modal, which
//! is UI-only — so an Apply whose draft equals the committed tuning still pushes
//! and still counts (`tuning-draft-invalid-note-ignored` is exactly that case,
//! with `patch_count: 1` and `patch_query: {}`). This driver counts it that way
//! and asserts the count per step, so neither half of the rule can drift.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod common;

use common::oracle_records;
use fretboard_core::{
    InstrumentId, InstrumentState, PageEvent, PageState, Position, Tab, TuningState, apply_event,
    change_tuning_string, decode_page_params, decoded_page, draft_event, encode_page_params,
    open_tuning_draft, page_event, select_tuning_preset, validate_state,
};
use serde_json::{Map, Value};

/// The frozen fixture, the cases this task owns, and the whole case count.
const FIXTURE: &str = "fixtures/oracle/page-events.jsonl";
const CASES: usize = 49;
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

/// One owned case: its identifier, its recorded steps and the baseline's page.
struct Case {
    id: String,
    steps: Vec<Value>,
    step_outputs: Vec<Value>,
    initial_params: Map<String, Value>,
    initial_state: Value,
    final_state: Value,
    final_params: Value,
    patches: u64,
}

/// The owned cases, read from the fixture.
fn cases() -> Vec<Case> {
    let records = oracle_records(FIXTURE);
    assert_eq!(
        records.len(),
        CASES,
        "the frozen fixture carries {CASES} cases"
    );

    OWNED
        .iter()
        .map(|wanted| {
            let record = records
                .iter()
                .find(|record| record["case_id"].as_str() == Some(wanted))
                .unwrap_or_else(|| panic!("the fixture has no case {wanted}"));
            let output = &record["output"];
            Case {
                id: (*wanted).to_owned(),
                steps: record["input"]["steps"]
                    .as_array()
                    .expect("a case has steps")
                    .clone(),
                step_outputs: output["steps"]
                    .as_array()
                    .expect("a case has step outputs")
                    .clone(),
                initial_params: output["initial_page_params"]
                    .as_object()
                    .expect("a case has initial parameters")
                    .clone(),
                initial_state: output["initial_state"].clone(),
                final_state: output["final_state"].clone(),
                final_params: output["final_page_params"].clone(),
                patches: output["patch_count"]
                    .as_u64()
                    .expect("a case has a patch count"),
            }
        })
        .collect()
}

/// The state a case starts from, which the parameters codec of `C08` builds from
/// the baseline's own initial parameters.
fn initial(case: &Case) -> PageState {
    let state = decode_page_params(&case.initial_params);
    assert_eq!(
        decoded_page(&state),
        case.initial_state,
        "{}: the initial parameters must decode to the recorded page",
        case.id
    );
    state
}

/// A parameter map from string pairs, the shape the fixture records.
fn params(pairs: &[(&str, &str)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), Value::from(*value)))
        .collect()
}

/// What one step did to the page: whether the baseline pushed, and the state it
/// pushed.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StepOutcome {
    patched: bool,
    state: PageState,
}

/// The instrument of a page, whichever kind it is.
const fn instrument_of(state: &PageState) -> InstrumentId {
    match &state.instrument {
        InstrumentState::Fretted { instrument, .. } => *instrument,
        InstrumentState::Piano { .. } => InstrumentId::Piano,
    }
}

/// Drive a case through the two readers, the way a client does.
///
/// The draft is deliberately a local value: it is not part of the page state, so
/// a draft edit can only ever return a new draft. An instrument change closes
/// the modal — the baseline assigns `show_tuning_modal: false` in
/// `handle_event("change_instrument", …)` — which for a caller of this core is
/// dropping the draft.
fn drive(case: &Case) -> Vec<StepOutcome> {
    let mut state = initial(case);
    let mut draft: Option<TuningState> = None;
    let mut outcomes = Vec::new();

    for step in &case.steps {
        if let Some(event) = draft_event(step) {
            let instrument = instrument_of(&state);
            match event {
                fretboard_core::DraftEvent::Open => {
                    draft = open_tuning_draft(&state);
                }
                fretboard_core::DraftEvent::SelectPreset { name } => {
                    draft = draft.map(|current| select_tuning_preset(instrument, &current, &name));
                }
                fretboard_core::DraftEvent::ChangeString { string, note } => {
                    draft = draft
                        .map(|current| change_tuning_string(instrument, &current, &string, &note));
                }
                fretboard_core::DraftEvent::Close => {
                    draft = None;
                }
                fretboard_core::DraftEvent::Apply => {
                    let Some(tuning) = draft.take() else {
                        // The piano's tuning events are ignored entirely: no
                        // draft was ever opened, so nothing pushes.
                        outcomes.push(StepOutcome {
                            patched: false,
                            state: state.clone(),
                        });
                        continue;
                    };
                    state = apply_event(&state, &PageEvent::CommitTuning(tuning));
                    // `apply_tuning` pushes unconditionally on a fretted page.
                    outcomes.push(StepOutcome {
                        patched: true,
                        state: state.clone(),
                    });
                    continue;
                }
            }
            outcomes.push(StepOutcome {
                patched: false,
                state: state.clone(),
            });
            continue;
        }

        let event = page_event(step)
            .unwrap_or_else(|| panic!("{}: the step is not a page event: {step}", case.id));
        let next = apply_event(&state, &event);
        let patched = next != state;
        if matches!(event, PageEvent::SetInstrument(_)) {
            draft = None;
        }
        state = next;
        outcomes.push(StepOutcome {
            patched,
            state: state.clone(),
        });
    }

    outcomes
}

/// Every owned step is read by exactly one of the two readers, so no recorded
/// step is silently skipped and none is read twice.
#[test]
fn every_owned_step_is_read_by_exactly_one_reader() {
    for case in cases() {
        for step in &case.steps {
            let ours = page_event(step).is_some();
            let draft = draft_event(step).is_some();
            assert!(
                ours ^ draft,
                "{}: the recorded step is not read by exactly one reader: {step}",
                case.id
            );
        }
    }
}

/// Applying the recorded steps reaches the recorded page after every patched
/// step and at the end, and the final page encodes to the recorded parameters.
#[test]
fn every_case_reaches_the_frozen_state_after_every_patch() {
    for case in cases() {
        let outcomes = drive(&case);
        assert_eq!(
            outcomes.len(),
            case.steps.len(),
            "{}: every step must be driven",
            case.id
        );

        for (index, (outcome, recorded)) in outcomes.iter().zip(&case.step_outputs).enumerate() {
            assert_eq!(
                outcome.patched,
                recorded["patched"].as_bool().expect("a patched flag"),
                "{}: step {index} patched differently from the baseline",
                case.id
            );
            if outcome.patched {
                assert_eq!(
                    decoded_page(&outcome.state),
                    recorded["state"],
                    "{}: step {index} pushed a different page",
                    case.id
                );
            } else {
                assert!(
                    recorded["state"].is_null(),
                    "{}: an unpatched step carries no state",
                    case.id
                );
            }
        }

        let Some(last) = outcomes.last() else {
            panic!("{}: the case has steps", case.id);
        };
        let state = &last.state;
        assert_eq!(
            decoded_page(state),
            case.final_state,
            "{}: the final page differs from the baseline",
            case.id
        );
        assert!(
            validate_state(state).is_ok(),
            "{}: the final state is not valid",
            case.id
        );
        assert_eq!(
            Value::Object(encode_page_params(state)),
            case.final_params,
            "{}: the final parameters differ from the baseline",
            case.id
        );
    }
}

/// The baseline's patch count is reproduced step by step, including the tuning
/// modal's unconditional Apply.
#[test]
fn every_case_pushes_as_often_as_the_baseline_says() {
    for case in cases() {
        let outcomes = drive(&case);
        let patches = outcomes.iter().filter(|outcome| outcome.patched).count() as u64;
        assert_eq!(
            patches, case.patches,
            "{}: the baseline reports {} patches, the reducer made {patches}",
            case.id, case.patches
        );
    }
}

/// One position per string: tapping the same position again removes it, a
/// different fret replaces that string and never adds a second position, and a
/// second string keeps the first. The cases are the fixture's own.
#[test]
fn a_position_is_toggled_by_string() {
    let cases = cases();
    let find = |id: &str| {
        cases
            .iter()
            .find(|case| case.id == id)
            .unwrap_or_else(|| panic!("the fixture has no case {id}"))
    };

    let added = drive(find("page_event/toggle-note-adds-position"));
    let InstrumentState::Fretted { selected, .. } = &added.last().expect("a step").state.instrument
    else {
        panic!("the case is fretted");
    };
    assert_eq!(selected.len(), 1);
    assert_eq!(u8::from(selected[0].string), 0);
    assert_eq!(u8::from(selected[0].fret), 3);

    let removed = drive(find("page_event/toggle-note-same-position-removes"));
    let InstrumentState::Fretted { selected, .. } =
        &removed.last().expect("a step").state.instrument
    else {
        panic!("the case is fretted");
    };
    assert!(selected.is_empty(), "the same position toggles off");

    let replaced = drive(find(
        "page_event/toggle-note-different-fret-replaces-string",
    ));
    let InstrumentState::Fretted { selected, .. } =
        &replaced.last().expect("a step").state.instrument
    else {
        panic!("the case is fretted");
    };
    assert_eq!(selected.len(), 2, "a string keeps exactly one position");
    assert_eq!(u8::from(selected[0].fret), 5, "the tapped string moved");
    assert_eq!(u8::from(selected[1].fret), 2, "the other string stayed");

    let second = drive(find("page_event/toggle-note-second-string-keeps-first"));
    let InstrumentState::Fretted { selected, .. } =
        &second.last().expect("a step").state.instrument
    else {
        panic!("the case is fretted");
    };
    assert_eq!(selected.len(), 2, "a second string adds a position");
}

/// An out-of-range string index is not a crash and not a position: the state is
/// left exactly as it was, so the client counts no patch (`Contract.D10`: the
/// safer typed rejection of a malformed action, never a panic or a truncated
/// list).
#[test]
fn an_out_of_range_string_leaves_the_state_untouched() {
    let params = params(&[("tab", "analyzer")]);
    let state = decode_page_params(&params);

    for string in [6u8, 9u8, 200u8] {
        let event = PageEvent::ToggleNote(Position {
            string: fretboard_core::StringIndex::try_from(string).expect("a string index"),
            fret: fretboard_core::Fret::try_from(3).expect("a fret"),
        });
        assert_eq!(
            apply_event(&state, &event),
            state,
            "string {string} is not a guitar string"
        );
    }
}

/// Clearing the selection is independent of the rest of the page: the chords,
/// the highlight, the tuning and the tab survive, and clearing an already empty
/// selection changes nothing.
#[test]
fn clearing_the_selection_is_independent_of_the_chords_and_the_tuning() {
    let params: Map<String, Value> = [
        ("chords".to_owned(), Value::from("Cmaj,Amin")),
        ("highlight".to_owned(), Value::from("Cmaj")),
        ("marked".to_owned(), Value::from("0-3,4-2")),
        ("tab".to_owned(), Value::from("analyzer")),
    ]
    .into_iter()
    .collect();
    let state = decode_page_params(&params);

    let cleared = apply_event(&state, &PageEvent::ClearSelection);
    assert_eq!(cleared.chords, state.chords, "the chords survive");
    assert_eq!(cleared.highlight, state.highlight, "the highlight survives");
    assert_eq!(cleared.tab, state.tab, "the tab survives");
    let InstrumentState::Fretted {
        instrument,
        tuning,
        selected,
    } = &cleared.instrument
    else {
        panic!("the case is fretted");
    };
    let InstrumentState::Fretted {
        instrument: before_instrument,
        tuning: before_tuning,
        ..
    } = &state.instrument
    else {
        panic!("the case is fretted");
    };
    assert_eq!(instrument, before_instrument, "the instrument survives");
    assert_eq!(tuning, before_tuning, "the tuning survives");
    assert!(selected.is_empty(), "the selection is cleared");

    assert_eq!(
        apply_event(&cleared, &PageEvent::ClearSelection),
        cleared,
        "clearing an empty selection changes nothing"
    );
}

/// A draft edit never touches the committed state, and the values the baseline
/// ignores leave the draft unchanged: an unknown preset, an invalid string
/// index and a note outside the chromatic scale.
#[test]
fn a_draft_never_touches_the_committed_state() {
    let params = params(&[("marked", "0-3")]);
    let state = decode_page_params(&params);
    let InstrumentState::Fretted { tuning, .. } = &state.instrument else {
        panic!("the case is fretted");
    };

    let draft = open_tuning_draft(&state).expect("a fretted page opens a draft");
    assert_eq!(&draft, tuning, "the draft starts from the committed tuning");

    let edited = change_tuning_string(InstrumentId::Guitar, &draft, "0", "D");
    assert_eq!(
        edited.pitches[0],
        fretboard_core::OpenPitch::try_from(38).expect("38"),
        "the recorded edit of `tuning-draft-edit-string-then-apply`"
    );
    assert_eq!(
        draft.pitches[0],
        fretboard_core::OpenPitch::try_from(40).expect("40"),
        "the draft the edit started from is untouched"
    );
    let InstrumentState::Fretted { tuning, .. } = &state.instrument else {
        panic!("the case is fretted");
    };
    assert_eq!(
        tuning.pitches[0],
        fretboard_core::OpenPitch::try_from(40).expect("40"),
        "the committed state is untouched by a draft edit"
    );

    // The baseline ignores these: an index the instrument does not have, a
    // preset that is not the instrument's, a note outside the chromatic scale.
    assert_eq!(
        change_tuning_string(InstrumentId::Guitar, &draft, "9", "D"),
        draft
    );
    assert_eq!(
        change_tuning_string(InstrumentId::Guitar, &draft, "0", "H"),
        draft
    );
    assert_eq!(
        change_tuning_string(InstrumentId::Guitar, &draft, "0", "Db"),
        draft
    );
    assert_eq!(
        select_tuning_preset(InstrumentId::Guitar, &draft, "Nope"),
        draft
    );
    assert_eq!(
        select_tuning_preset(InstrumentId::Guitar, &draft, "Low G"),
        draft,
        "Low G is a ukulele preset, not a guitar one"
    );

    // A known preset replaces the draft's pitches and keeps its reference.
    let drop_d = select_tuning_preset(InstrumentId::Guitar, &draft, "Drop D");
    assert_eq!(u8::from(drop_d.pitches[0]), 38);
    assert_eq!(drop_d.reference.to_string(), "Drop D");
}

/// A commit preserves the marked positions, the chords, the highlight and the
/// tab: it sets the tuning and nothing else (`handle_event("apply_tuning", …)`).
#[test]
fn a_commit_preserves_the_marked_positions() {
    let params: Map<String, Value> = [
        ("chords".to_owned(), Value::from("Cmaj,Amin")),
        ("highlight".to_owned(), Value::from("Amin")),
        ("marked".to_owned(), Value::from("0-3,2-5")),
        ("tab".to_owned(), Value::from("analyzer")),
    ]
    .into_iter()
    .collect();
    let state = decode_page_params(&params);

    let draft = open_tuning_draft(&state).expect("a draft");
    let edited = change_tuning_string(InstrumentId::Guitar, &draft, "0", "D");
    let committed = apply_event(&state, &PageEvent::CommitTuning(edited));

    assert_eq!(committed.chords, state.chords);
    assert_eq!(committed.highlight, state.highlight);
    assert_eq!(committed.tab, state.tab);
    let InstrumentState::Fretted {
        tuning, selected, ..
    } = &committed.instrument
    else {
        panic!("the case is fretted");
    };
    assert_eq!(u8::from(tuning.pitches[0]), 38, "the draft is committed");
    assert_eq!(
        tuning.reference.to_string(),
        "Standard",
        "the reference is kept"
    );
    let InstrumentState::Fretted {
        selected: before, ..
    } = &state.instrument
    else {
        panic!("the case is fretted");
    };
    assert_eq!(selected, before, "the marked positions survive the commit");
    assert!(validate_state(&committed).is_ok());
}

/// The tuning modal is the fretted instruments' own: on the piano nothing opens,
/// nothing is edited and nothing is committed.
#[test]
fn the_tuning_modal_ignores_the_piano() {
    let params = params(&[("instrument", "piano")]);
    let state = decode_page_params(&params);

    assert_eq!(open_tuning_draft(&state), None);
    assert_eq!(
        apply_event(&state, &PageEvent::ClearSelection).instrument,
        state.instrument,
        "clearing the keys keeps the piano"
    );
    assert_eq!(state.tab, Tab::Visualizer);
}
