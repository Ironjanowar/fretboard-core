//! Task `C18`: the key and progression drafts and their apply actions.
//!
//! The frozen oracle for this target is the seven `page_event` cases of
//! `fixtures/oracle/page-events.jsonl` that own the keys/progressions screen:
//!
//! * `apply-key-triads-replaces-chords`, `apply-key-sevenths` — the key apply in
//!   both modes of the modal, on a page whose selection must survive;
//! * `apply-key-modal-resets-preview-on-open` — opening the modal resets the
//!   draft, and none of these steps patches the page;
//! * `apply-suggested-key-triads-from-existing-mode`,
//!   `apply-suggested-key-inherits-seventh-mode` — applying a *suggested* key
//!   inherits the mode the current chords already imply;
//! * `apply-progression-replaces-chords`,
//!   `apply-progression-after-piano-instrument` — the progression apply on a
//!   fretted page with a highlight, and on the piano with a live key selection.
//!
//! The shapes follow `tests/reducer_fretted.rs`: a recorded step is read by
//! exactly one of the two readers, the modal's own steps are UI-only (they never
//! touch the committed page) and the apply is the moment the *draft* a client
//! holds is committed. The draft is deliberately not part of [`PageState`]: it is
//! the client's, exactly as the tuning modal's is, so an app that drops it cannot
//! corrupt the committed page.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used,
    variant_size_differences
)]

mod common;

use common::{oracle_records, pitch_class};
use fretboard_core::{
    ChordMode, ChordSpec, EvaluationDraftEvent, InstrumentState, KeysDraft, PageEvent, PageState,
    ProgressionDraft, ProgressionId, Tab, apply_event, decode_page_params, decoded_page,
    encode_page_params, evaluation_draft_event, infer_chord_mode, note_index, page_event,
};
use serde_json::{Map, Value};

/// The frozen fixture, the cases this task owns, and the whole case count.
const FIXTURE: &str = "fixtures/oracle/page-events.jsonl";
const CASES: usize = 49;
const OWNED: [&str; 7] = [
    "page_event/apply-key-modal-resets-preview-on-open",
    "page_event/apply-key-sevenths",
    "page_event/apply-key-triads-replaces-chords",
    "page_event/apply-progression-after-piano-instrument",
    "page_event/apply-progression-replaces-chords",
    "page_event/apply-suggested-key-inherits-seventh-mode",
    "page_event/apply-suggested-key-triads-from-existing-mode",
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

/// The state a case starts from, through the page-parameters codec of `C08`.
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

/// The state the baseline starts a page from, with no parameters at all.
fn default_page() -> PageState {
    decode_page_params(&params(&[]))
}

/// A chord list as `root:quality` text, so a key's diatonic chords (the
/// `DiatonicChord` the scale module answers) compare with the page's own
/// `ChordSpec` list.
fn rendered(chords: &[fretboard_core::DiatonicChord]) -> Vec<String> {
    chords
        .iter()
        .map(|chord| format!("{}:{}", chord.root.name(), chord.quality.as_str()))
        .collect()
}

/// The same rendering for the page's own chord list.
fn rendered_chords(chords: &[ChordSpec]) -> Vec<String> {
    chords
        .iter()
        .map(|chord| format!("{}:{}", chord.root.name(), chord.quality.as_str()))
        .collect()
}

/// What one step did to the page.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StepOutcome {
    patched: bool,
    state: PageState,
}

/// Drive a case through the two readers, the way a client does.
///
/// The key and progression drafts are local values: the modal's steps return a
/// new draft, `apply_key` / `apply_progression` commit the draft the client was
/// holding, and a reader that cannot read a draft ignores the step instead of
/// guessing what the user had selected.
fn drive(case: &Case) -> Vec<StepOutcome> {
    let mut state = initial(case);
    let mut keys: Option<KeysDraft> = None;
    let mut progression: Option<ProgressionDraft> = None;
    let mut outcomes = Vec::new();

    for step in &case.steps {
        if let Some(event) = evaluation_draft_event(step) {
            match event {
                EvaluationDraftEvent::OpenKeys => {
                    keys = Some(KeysDraft::default());
                }
                EvaluationDraftEvent::UpdateKeys(draft) => {
                    keys = Some(draft);
                }
                EvaluationDraftEvent::CloseKeys => {
                    keys = None;
                }
                EvaluationDraftEvent::ApplyKeys => {
                    let Some(draft) = keys else {
                        outcomes.push(StepOutcome {
                            patched: false,
                            state: state.clone(),
                        });
                        continue;
                    };
                    state = apply_event(&state, &PageEvent::CommitKeys(draft));
                    outcomes.push(StepOutcome {
                        patched: true,
                        state: state.clone(),
                    });
                    continue;
                }
                EvaluationDraftEvent::OpenProgressions => {
                    progression = Some(ProgressionDraft::default());
                }
                EvaluationDraftEvent::UpdateProgressions(draft) => {
                    progression = Some(draft);
                }
                EvaluationDraftEvent::CloseProgressions => {
                    progression = None;
                }
                EvaluationDraftEvent::ApplyProgressions => {
                    let Some(draft) = progression else {
                        outcomes.push(StepOutcome {
                            patched: false,
                            state: state.clone(),
                        });
                        continue;
                    };
                    state = apply_event(&state, &PageEvent::CommitProgression(draft));
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
        state = next;
        outcomes.push(StepOutcome {
            patched,
            state: state.clone(),
        });
    }

    outcomes
}

#[test]
fn every_owned_step_is_read_by_exactly_one_reader() {
    for case in cases() {
        for step in &case.steps {
            let ours = page_event(step).is_some();
            let draft = evaluation_draft_event(step).is_some();
            assert!(
                ours ^ draft,
                "{}: the recorded step is not read by exactly one reader: {step}",
                case.id
            );
        }
    }
}

#[test]
fn every_case_pushes_as_often_as_the_baseline_says() {
    for case in cases() {
        let outcomes = drive(&case);
        assert_eq!(
            outcomes.iter().filter(|outcome| outcome.patched).count() as u64,
            case.patches,
            "{}: the patch count differs from the baseline",
            case.id
        );
        for (index, (outcome, recorded)) in
            outcomes.iter().zip(case.step_outputs.iter()).enumerate()
        {
            assert_eq!(
                outcome.patched,
                recorded["patched"].as_bool().unwrap_or(false),
                "{}: step {index} pushes differently from the baseline",
                case.id
            );
        }
    }
}

#[test]
fn every_case_reaches_the_frozen_page_after_every_patch() {
    for case in cases() {
        for (index, outcome) in drive(&case).into_iter().enumerate() {
            let recorded = &case.step_outputs[index];
            if outcome.patched {
                assert_eq!(
                    decoded_page(&outcome.state),
                    recorded["state"],
                    "{}: step {index} does not reach the recorded page",
                    case.id
                );
            }
        }
        let final_state = drive(&case).last().expect("a case has steps").state.clone();
        assert_eq!(
            decoded_page(&final_state),
            case.final_state,
            "{}: the final page differs from the baseline",
            case.id
        );
        assert_eq!(
            Value::Object(encode_page_params(&final_state)),
            case.final_params,
            "{}: the final page encodes differently from the baseline",
            case.id
        );
    }
}

#[test]
fn an_apply_replaces_the_chords_and_keeps_everything_else() {
    // A page with two chords, a highlight and a live fretted selection.
    let mut state = decode_page_params(&params(&[
        ("chords", "Cmaj,Amin"),
        ("highlight", "Cmaj"),
        ("marked", "0-3"),
        ("tab", "analyzer"),
    ]));
    assert!(state.highlight.is_some(), "the page starts highlighted");
    let selection = match &state.instrument {
        InstrumentState::Fretted { selected, .. } => selected.clone(),
        InstrumentState::Piano { .. } => panic!("the fixture page is fretted"),
    };
    assert!(
        !selection.is_empty(),
        "the page starts with a marked position"
    );

    let draft = KeysDraft {
        tonic: pitch_class(7),
        scale: "major".parse().expect("a catalog scale"),
        mode: ChordMode::Triad,
    };
    state = apply_event(&state, &PageEvent::CommitKeys(draft));

    let expected: Vec<String> = [
        "G:major", "A:minor", "B:minor", "C:major", "D:major", "E:minor", "F#:dim",
    ]
    .iter()
    .map(ToString::to_string)
    .collect();
    let ours: Vec<String> = state
        .chords
        .iter()
        .map(|chord| format!("{}:{}", chord.root.name(), chord.quality.as_str()))
        .collect();
    assert_eq!(
        ours, expected,
        "the apply replaces the chords with the key's own"
    );
    assert!(state.highlight.is_none(), "the apply clears the highlight");
    assert_eq!(state.tab, Tab::Analyzer, "the apply keeps the tab");
    match &state.instrument {
        InstrumentState::Fretted {
            selected, tuning, ..
        } => {
            assert_eq!(selected, &selection, "the apply keeps the marked positions");
            assert_eq!(
                tuning.reference.as_str(),
                "Standard",
                "the apply keeps the committed tuning"
            );
        }
        InstrumentState::Piano { .. } => panic!("the apply must not change the instrument"),
    }
}

#[test]
fn the_suggested_key_inherits_the_mode_of_the_current_chords() {
    // The two frozen suggested-key cases, driven from their own pages.
    for case in cases() {
        if !case.id.contains("apply-suggested-key") {
            continue;
        }
        let state = initial(&case);
        let expected_mode = infer_chord_mode(&state.chords);
        let step = case
            .steps
            .iter()
            .find(|step| page_event(step).is_some())
            .expect("a suggested-key case carries its step");
        let PageEvent::CommitSuggestedKeys { tonic, scale } =
            page_event(step).expect("the step is the suggested-key page event")
        else {
            panic!("{}: the step must be the suggested-key event", case.id);
        };
        let next = apply_event(&state, &PageEvent::CommitSuggestedKeys { tonic, scale });
        assert_eq!(
            rendered_chords(&next.chords),
            rendered(&fretboard_core::diatonic_chords(
                tonic,
                scale,
                expected_mode
            )),
            "{}: the suggested key uses the mode the current chords imply",
            case.id
        );
        // And the same key applied in the other mode is a different chord list,
        // so the recorded answer really is mode-dependent.
        let other = match expected_mode {
            ChordMode::Triad => ChordMode::Seventh,
            ChordMode::Seventh => ChordMode::Triad,
        };
        assert_ne!(
            rendered_chords(&next.chords),
            rendered(&fretboard_core::diatonic_chords(tonic, scale, other)),
            "{}: the two modes must differ for this key",
            case.id
        );
    }

    // The rule itself: a seventh quality in the list opts into seventh mode, and
    // an extended or suspended quality does not.
    let triads = [common::chord(0, "major"), common::chord(9, "minor")];
    assert_eq!(infer_chord_mode(&triads), ChordMode::Triad);
    let sevenths = [common::chord(0, "maj7"), common::chord(2, "min7")];
    assert_eq!(infer_chord_mode(&sevenths), ChordMode::Seventh);
    let suspended = [common::chord(0, "sus4"), common::chord(0, "add9")];
    assert_eq!(
        infer_chord_mode(&suspended),
        ChordMode::Triad,
        "an extended or suspended quality does not opt into seventh mode"
    );
    assert_eq!(infer_chord_mode(&[]), ChordMode::Triad);
}

#[test]
fn a_progression_apply_keeps_every_occurrence() {
    let repeated: ProgressionId = "pop_i_v_vi_iv".parse().expect("a catalog id");
    let draft = ProgressionDraft {
        tonic: pitch_class(7),
        progression: repeated,
    };
    let state = apply_event(&default_page(), &PageEvent::CommitProgression(draft));
    assert_eq!(
        state.chords.len(),
        fretboard_core::progression(repeated).degrees.len(),
        "the chord list has one entry per degree"
    );

    // A progression whose degrees repeat a chord keeps both occurrences: the
    // apply replaces the list with the catalog's own, it does not deduplicate it.
    let repeating = fretboard_core::all_progressions()
        .iter()
        .find(|progression| {
            let chords = fretboard_core::progression_chords(pitch_class(0), progression.id);
            let mut seen: Vec<&ChordSpec> = Vec::new();
            chords.iter().any(|chord| {
                let repeat = seen.contains(&chord);
                seen.push(chord);
                repeat
            })
        })
        .expect("the catalog carries a progression that repeats a chord");
    let draft = ProgressionDraft {
        tonic: pitch_class(0),
        progression: repeating.id,
    };
    let state = apply_event(&default_page(), &PageEvent::CommitProgression(draft));
    let mut distinct: Vec<&ChordSpec> = Vec::new();
    for chord in &state.chords {
        if !distinct.contains(&chord) {
            distinct.push(chord);
        }
    }
    assert!(
        distinct.len() < state.chords.len(),
        "{}: the repeated chord is kept as two occurrences",
        repeating.id.as_str()
    );
}

#[test]
fn a_draft_never_touches_the_committed_page() {
    let cases = cases();
    let reset = cases
        .iter()
        .find(|case| case.id.ends_with("apply-key-modal-resets-preview-on-open"))
        .expect("the reset case is owned");
    let state = initial(reset);
    assert_eq!(
        drive(reset).last().expect("a case has steps").state.clone(),
        state,
        "opening, editing and reopening the modal without applying changes nothing"
    );
    assert_eq!(
        reset.patches, 0,
        "the baseline pushes no patch for these steps"
    );
}

#[test]
fn an_unreadable_step_leaves_the_draft_alone() {
    // A mode the modal does not offer, a scale the catalog does not carry, a
    // tonic that is not a note name and a progression that does not exist are all
    // rejected by the reader instead of becoming a draft a client could commit.
    let unknown = [
        serde_json::json!({"kind": "change", "selector": "#key-form",
            "value": {"key": {"tonic": "C", "scale_type": "major", "chord_mode": "ninth"}}}),
        serde_json::json!({"kind": "change", "selector": "#key-form",
            "value": {"key": {"tonic": "C", "scale_type": "no_such_scale", "chord_mode": "triad"}}}),
        serde_json::json!({"kind": "change", "selector": "#key-form",
            "value": {"key": {"tonic": "H", "scale_type": "major", "chord_mode": "triad"}}}),
        serde_json::json!({"kind": "change", "selector": "#progression-form",
            "value": {"progression": {"id": "no_such_progression", "tonic": "C"}}}),
    ];
    for step in &unknown {
        assert!(
            evaluation_draft_event(step).is_none(),
            "the step must not become a draft: {step}"
        );
    }

    // The filter is by selector: a chord-form change is not a key draft, and a
    // key-form change is not a page event.
    let chord_form = serde_json::json!({"kind": "change", "selector": "#chord-form",
        "value": {"chord": {"root": "C", "quality": "major"}}});
    assert!(evaluation_draft_event(&chord_form).is_none());
    assert!(page_event(&chord_form).is_none());

    // The tonic of a draft is a real note name, flats included through the
    // domain's note lookup (`Bb` is the catalog's own `example_key`).
    let flat = note_index("Bb").expect("Bb is a note name");
    assert_eq!(flat.name(), "A#", "the domain answers the sharp spelling");
}

#[test]
fn the_two_modes_are_the_ones_the_modal_offers() {
    let modes: Vec<&str> = [ChordMode::Triad, ChordMode::Seventh]
        .iter()
        .map(|mode| mode.as_str())
        .collect();
    assert_eq!(modes, vec!["triad", "seventh"]);
    for (wire, mode) in [("triad", ChordMode::Triad), ("seventh", ChordMode::Seventh)] {
        assert_eq!(
            wire.parse::<ChordMode>().expect("a mode"),
            mode,
            "the wire spelling of {wire} must parse back"
        );
        assert_eq!(mode.as_str(), wire);
    }
    assert!(
        "ninth".parse::<ChordMode>().is_err(),
        "a mode the modal does not offer is rejected"
    );
}
