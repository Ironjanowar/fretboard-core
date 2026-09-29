//! Task `C14`: the piano reducer families and the instrument boundary against
//! the frozen oracle.
//!
//! The page-event fixture (`fixtures/oracle/page-events.jsonl`, the pinned
//! `FretboardWeb.FretboardLive.handle_event/3`) records, for every case, the
//! parameters the page started from, the state after every step that pushed a
//! patch, the page it ended on, and how many patches the baseline sent. This
//! file drives the families `C14` owns and leaves every other case to its own
//! task: the identity events are `C09`'s (`reducer_identity.rs`), the fretted
//! families are `C13`'s (`reducer_fretted.rs`), the key and progression
//! application is `C18`'s.
//!
//! ## The piano families
//!
//! * [`PageEvent::TogglePianoKey`] is one tap on one absolute key, read from a
//!   recorded `toggle_piano_key` step. The pinned handler applies it only on the
//!   **piano's analyzer tab** and only when the recorded activation key is one
//!   it accepts (`Enter` or a space); a key outside `48..=83` is not a key at
//!   all. Every branch it refuses leaves the page exactly as it was, which is
//!   the same *"a tap that changes nothing must leave the state exactly as it
//!   was"* rule the identity and fretted families keep (`patch_count: 0`).
//! * [`PageEvent::ClearSelection`] clears the piano's keys the way it clears a
//!   fretted selection — the chords, the highlight, the tab and the instrument
//!   all survive.
//! * [`PageEvent::SetInstrument`] is the **instrument boundary**: the pinned
//!   `switch_selection/2` converts nothing between kinds — switching to the
//!   piano drops the marked positions (a keyboard has none) and switching away
//!   from the piano drops the keys (a fretted instrument has no absolute
//!   selection) — while the chords and the tab are carried over and the
//!   highlight is cleared.
//!
//! The recorded `patch_count` is the baseline's own statement of which steps
//! pushed, so it is asserted for every case and every step, exactly as
//! `reducer_fretted.rs` does for `C13`.

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
    InstrumentId, InstrumentState, PageEvent, PageState, Position, Tab, apply_event,
    decode_page_params, decoded_page, encode_page_params, keyboard_pitch_range, page_event,
    validate_state,
};
use serde_json::{Map, Value};

/// The frozen fixture, the cases this task owns, and the whole case count.
const FIXTURE: &str = "fixtures/oracle/page-events.jsonl";
const CASES: usize = 49;
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

/// The pinned handler's own activation guard (`valid_activation?/1`): a recorded
/// step whose `key` is neither `Enter` nor a space is not a toggle at all.
fn rejected_activation(step: &Value) -> bool {
    match step.get("value").and_then(|value| value.get("key")) {
        None => false,
        Some(Value::String(key)) => key != "Enter" && key != " ",
        Some(_) => true,
    }
}

/// Drive a case through the page-event reader and the reducer, the way a client
/// does.
///
/// A recorded step the reader refuses must be the handler's own activation
/// rejection — the reader answers `None` for exactly that step, and treating any
/// other `None` as "the state stays the same" would be the mistake the module
/// documentation warns about. The step is therefore checked, not silently
/// skipped.
fn drive(case: &Case) -> Vec<StepOutcome> {
    let mut state = initial(case);
    let mut outcomes = Vec::new();

    for step in &case.steps {
        let event = page_event(step);
        if event.is_none() {
            assert!(
                rejected_activation(step),
                "{}: the recorded step is not a page event: {step}",
                case.id
            );
        }
        let next = event.map_or_else(|| state.clone(), |event| apply_event(&state, &event));
        let patched = next != state;
        state = next;
        outcomes.push(StepOutcome {
            patched,
            state: state.clone(),
        });
    }

    outcomes
}

/// The owner of a case's final page, whichever kind it is.
const fn instrument_of(state: &PageState) -> InstrumentId {
    match &state.instrument {
        InstrumentState::Fretted { instrument, .. } => *instrument,
        InstrumentState::Piano { .. } => InstrumentId::Piano,
    }
}

/// The keys of a piano page.
fn keys_of(state: &PageState) -> Vec<u8> {
    match &state.instrument {
        InstrumentState::Piano { selected } => selected.iter().map(|key| u8::from(*key)).collect(),
        InstrumentState::Fretted { .. } => panic!("the page is not the piano"),
    }
}

/// Applying the recorded steps reaches the recorded page after every patched
/// step and at the end, the final page is valid and encodes to the recorded
/// parameters.
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

/// The baseline's patch count is reproduced case by case.
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

/// One absolute key is toggled by pitch: tapping an unselected key adds it,
/// tapping the same key again removes it.
#[test]
fn a_key_is_toggled_by_pitch() {
    let all = cases();
    let find = |id: &str| {
        all.iter()
            .find(|case| case.id == id)
            .unwrap_or_else(|| panic!("the fixture has no case {id}"))
    };

    let added = drive(find("page_event/toggle-piano-key-adds-pitch"));
    let last = added.last().expect("a step");
    assert_eq!(instrument_of(&last.state), InstrumentId::Piano);
    assert_eq!(keys_of(&last.state), vec![60], "the tapped key was added");
    assert!(last.patched, "an add pushes");

    let removed = drive(find("page_event/toggle-piano-key-removes-pitch"));
    let last = removed.last().expect("a step");
    assert_eq!(keys_of(&last.state), vec![64], "the tapped key was removed");
    assert!(last.patched, "a remove pushes");
}

/// A key outside the frozen keyboard range is not a key: the step is a toggle of
/// a value the page cannot hold, and the state is left exactly as it was, so the
/// client counts no patch (`Contract.D10`: the safer typed rejection, never a
/// panic and never a key the keyboard does not have).
#[test]
fn a_key_outside_the_keyboard_is_a_typed_no_op() {
    let case = cases()
        .into_iter()
        .find(|case| case.id == "page_event/toggle-piano-key-outside-range-ignored")
        .expect("the fixture carries the case");
    let step = case.steps.first().expect("a step");

    // The step *is* a toggle — its pitch is a number, 47 — so it reads as one and
    // the range guard, not the reader, is what makes it a no-op.
    let event = page_event(step).expect("the recorded step is a toggle");
    assert_eq!(
        event,
        PageEvent::TogglePianoKey(common::open_pitch(47)),
        "the recorded pitch is read as written"
    );

    let state = initial(&case);
    assert_eq!(
        keys_of(&state),
        vec![60],
        "the case starts from the recorded key"
    );
    assert_eq!(
        apply_event(&state, &event),
        state,
        "a key below the keyboard cannot be selected"
    );

    let (lowest, highest) = keyboard_pitch_range();
    let above = PageEvent::TogglePianoKey(
        fretboard_core::OpenPitch::try_from(u8::from(highest) + 1)
            .expect("one above the keyboard is still an open pitch"),
    );
    assert_eq!(
        apply_event(&state, &above),
        state,
        "a key above the keyboard cannot be selected"
    );
    assert_eq!(
        u8::from(lowest),
        48,
        "the keyboard's own lower bound is the frozen one"
    );
}

/// The recorded activation key `Tab` is not an activation the handler accepts,
/// so the step is not a toggle at all: the reader refuses it and the page is
/// untouched.
#[test]
fn a_tab_activation_key_is_not_a_toggle() {
    let case = cases()
        .into_iter()
        .find(|case| case.id == "page_event/toggle-piano-key-without-activation-key-ignored")
        .expect("the fixture carries the case");
    let step = case.steps.first().expect("a step");

    assert!(
        rejected_activation(step),
        "the case records a rejected activation key"
    );
    assert_eq!(
        page_event(step),
        None,
        "a rejected activation key is not a toggle"
    );

    let state = initial(&case);
    assert!(
        keys_of(&state).is_empty(),
        "the case starts with no key selected"
    );
    let outcomes = drive(&case);
    assert!(
        outcomes.iter().all(|outcome| !outcome.patched),
        "the ignored toggle pushes nothing"
    );
    assert_eq!(outcomes.last().expect("a step").state, state);
}

/// The piano's toggle applies on its analyzer tab only: the same recorded event
/// on the visualizer tab is a toggle whose page is not the analyzer's, so it
/// changes nothing.
#[test]
fn the_piano_toggle_is_gated_by_the_analyzer_tab() {
    let case = cases()
        .into_iter()
        .find(|case| case.id == "page_event/toggle-piano-key-ignored-in-visualizer")
        .expect("the fixture carries the case");
    let step = case.steps.first().expect("a step");

    let event = page_event(step).expect("the recorded step is a toggle");
    let state = initial(&case);
    assert_eq!(state.tab, Tab::Visualizer, "the case is on the visualizer");
    assert_eq!(keys_of(&state), vec![60]);
    assert_eq!(
        apply_event(&state, &event),
        state,
        "the visualizer tab has no key toggling"
    );

    // The very same event on the analyzer tab does select the key.
    let analyzer = decode_page_params(&params(&[
        ("instrument", "piano"),
        ("tab", "analyzer"),
        ("keys", "60"),
    ]));
    let selected = apply_event(&analyzer, &event);
    assert_eq!(keys_of(&selected), vec![60, 64]);
    assert_eq!(selected.tab, Tab::Analyzer);
}

/// A wrong-kind toggle is not a crash and not a selection: a fretted tap on a
/// piano page leaves the keys alone, and a key tap on a fretted page leaves the
/// positions alone.
#[test]
fn a_wrong_kind_toggle_is_a_typed_no_op() {
    let piano = decode_page_params(&params(&[
        ("instrument", "piano"),
        ("tab", "analyzer"),
        ("keys", "60,64"),
    ]));
    let guitar = decode_page_params(&params(&[("tab", "analyzer"), ("marked", "0-3")]));

    let note = PageEvent::ToggleNote(Position {
        string: common::string_index(0),
        fret: common::fret(5),
    });
    assert_eq!(
        apply_event(&piano, &note),
        piano,
        "the piano has no string positions"
    );

    let key = PageEvent::TogglePianoKey(common::open_pitch(60));
    assert_eq!(
        apply_event(&guitar, &key),
        guitar,
        "a fretted instrument has no absolute keys"
    );
}

/// Clearing the piano's selection is independent of the rest of the page: the
/// chords, the highlight and the tab survive, and clearing an already empty
/// selection changes nothing.
#[test]
fn clearing_the_piano_selection_keeps_the_chords() {
    let state = decode_page_params(&params(&[
        ("instrument", "piano"),
        ("keys", "48,60,64"),
        ("chords", "Cmaj,Amin"),
        ("highlight", "Cmaj"),
        ("tab", "analyzer"),
    ]));
    assert_eq!(keys_of(&state), vec![48, 60, 64]);

    let cleared = apply_event(&state, &PageEvent::ClearSelection);
    assert_eq!(cleared.chords, state.chords, "the chords survive");
    assert_eq!(cleared.highlight, state.highlight, "the highlight survives");
    assert_eq!(cleared.tab, state.tab, "the tab survives");
    assert!(keys_of(&cleared).is_empty(), "the keys are cleared");
    assert_eq!(
        apply_event(&cleared, &PageEvent::ClearSelection),
        cleared,
        "clearing an empty selection changes nothing"
    );

    // The fixture's own case drives the same rule.
    let case = cases()
        .into_iter()
        .find(|case| case.id == "page_event/clear-notes-piano")
        .expect("the fixture carries the case");
    assert_eq!(
        keys_of(&drive(&case).last().expect("a step").state),
        Vec::<u8>::new()
    );
}

/// The instrument boundary converts nothing between kinds: every crossing of the
/// piano edge lands on an empty selection of the new kind.
#[test]
fn the_piano_boundary_clears_the_selection_in_both_directions() {
    let guitar = decode_page_params(&params(&[
        ("chords", "Cmaj"),
        ("marked", "0-3,4-5"),
        ("tab", "analyzer"),
    ]));
    let piano = apply_event(&guitar, &PageEvent::SetInstrument(InstrumentId::Piano));
    assert_eq!(instrument_of(&piano), InstrumentId::Piano);
    assert!(
        keys_of(&piano).is_empty(),
        "a string position is never a piano key"
    );
    assert!(validate_state(&piano).is_ok());

    let back = apply_event(&piano, &PageEvent::SetInstrument(InstrumentId::Guitar));
    assert_eq!(instrument_of(&back), InstrumentId::Guitar);
    let InstrumentState::Fretted { selected, .. } = &back.instrument else {
        panic!("the page is fretted again");
    };
    assert!(
        selected.is_empty(),
        "a piano key is never a string position"
    );
    assert!(validate_state(&back).is_ok());

    // Crossing to a differently-sized fretted instrument is empty too.
    let uke = apply_event(&piano, &PageEvent::SetInstrument(InstrumentId::Ukelele));
    assert_eq!(instrument_of(&uke), InstrumentId::Ukelele);
    assert!(validate_state(&uke).is_ok());

    // The fixture's own crossings reach the same pages, which the case test
    // already compares record by record.
    for id in [
        "page_event/change-instrument-fretted-to-piano-clears-selection",
        "page_event/change-instrument-piano-to-fretted-clears-keys",
        "page_event/change-instrument-piano-to-ukelele",
    ] {
        let case = cases()
            .into_iter()
            .find(|case| case.id == id)
            .expect("the fixture carries the case");
        assert_eq!(
            drive(&case)
                .iter()
                .filter(|outcome| outcome.patched)
                .count(),
            1,
            "{id}: the crossing pushes once"
        );
    }
}

/// The crossing carries the chords and the tab over and clears the highlight: it
/// is the instrument that changes, not the rest of the page.
#[test]
fn the_piano_boundary_keeps_the_chords_and_the_tab_and_clears_the_highlight() {
    let guitar = decode_page_params(&params(&[
        ("chords", "Cmaj,Amin"),
        ("highlight", "Amin"),
        ("marked", "0-3"),
        ("tab", "analyzer"),
    ]));
    assert!(guitar.highlight.is_some(), "the case starts highlighted");

    let piano = apply_event(&guitar, &PageEvent::SetInstrument(InstrumentId::Piano));
    assert_eq!(piano.chords, guitar.chords, "the chords survive");
    assert_eq!(piano.tab, guitar.tab, "the tab survives");
    assert_eq!(piano.highlight, None, "the highlight is cleared");

    let back = apply_event(&piano, &PageEvent::SetInstrument(InstrumentId::Guitar));
    assert_eq!(back.chords, guitar.chords, "the chords still survive");
    assert_eq!(back.tab, guitar.tab, "the tab still survives");
    assert_eq!(back.highlight, None, "the highlight stays cleared");
}

/// Selecting the instrument the page already is changes nothing, whichever
/// instrument that is.
#[test]
fn selecting_the_same_instrument_is_a_no_op() {
    let piano = decode_page_params(&params(&[
        ("instrument", "piano"),
        ("keys", "60"),
        ("chords", "Cmaj"),
    ]));
    assert_eq!(
        apply_event(&piano, &PageEvent::SetInstrument(InstrumentId::Piano)),
        piano
    );

    let guitar = decode_page_params(&params(&[("chords", "Cmaj"), ("marked", "0-3")]));
    assert_eq!(
        apply_event(&guitar, &PageEvent::SetInstrument(InstrumentId::Guitar)),
        guitar
    );
}
