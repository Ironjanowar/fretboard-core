//! Task `C09`: the identity reducer against the frozen oracle.
//!
//! The page-event fixture (`fixtures/oracle/page-events.jsonl`, the pinned
//! `FretboardWeb.FretboardLive.handle_event/3`) records, for every case, the
//! parameters the page started from, the page it ended on, and how many patches
//! the baseline sent. This file drives the identity events of that record — the
//! chord list and the highlight — and leaves each later task the cases it owns:
//! the fretted selection and tuning drafts (`C13`), the piano keys (`C14`) and
//! the key/progression application (`C18`).
//!
//! Each case is checked in both directions and against the patch count, so a
//! "no-op" cannot be implemented as "the same state by luck": the count is the
//! baseline's own statement of which steps changed anything.

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
    ChordSpec, PageEvent, QualityId, apply_event, decode_page_params, decoded_page,
    encode_page_params, page_event, validate_state,
};
use serde_json::{Map, Value};

/// The frozen fixture, the cases this task owns, and the whole case count.
const FIXTURE: &str = "fixtures/oracle/page-events.jsonl";
const CASES: usize = 49;
const OWNED: [&str; 14] = [
    "page_event/add-chord-by-form-submit",
    "page_event/add-chord-form-change-only-validates",
    "page_event/add-duplicate-chord-is-noop",
    "page_event/add-new-chord",
    "page_event/add-two-chords-in-sequence",
    "page_event/clear-all-chords-keeps-selection",
    "page_event/highlight-duplicate-canonicalizes-to-first-occurrence",
    "page_event/highlight-moves-to-other-chip",
    "page_event/highlight-off-on-same-chip",
    "page_event/highlight-on",
    "page_event/remove-first-of-duplicates-moves-highlight-to-first-remaining",
    "page_event/remove-last-chord-clears-highlight",
    "page_event/remove-middle-occurrence",
    "page_event/remove-unhighlighted-occurrence-keeps-highlight",
];

/// One owned case: its identifier, its recorded steps, and the baseline's page.
struct Case {
    id: String,
    steps: Vec<Value>,
    initial_state: Value,
    initial_params: Map<String, Value>,
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
                initial_state: output["initial_state"].clone(),
                initial_params: output["initial_page_params"]
                    .as_object()
                    .expect("a case has initial parameters")
                    .clone(),
                final_state: output["final_state"].clone(),
                final_params: output["final_page_params"].clone(),
                patches: output["patch_count"]
                    .as_u64()
                    .expect("a case has a patch count"),
            }
        })
        .collect()
}

/// A parameter map from string pairs, the shape the fixture records.
fn params(pairs: &[(&str, &str)]) -> Map<String, Value> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_owned(), Value::from(*value)))
        .collect()
}

/// The state a case starts from, which the parameters codec of `C08` builds from
/// the baseline's own initial parameters.
fn initial(case: &Case) -> fretboard_core::PageState {
    let state = decode_page_params(&case.initial_params);
    assert_eq!(
        decoded_page(&state),
        case.initial_state,
        "{}: the initial parameters must decode to the recorded page",
        case.id
    );
    state
}

/// The identity events of this task are the ones the fixture records for them:
/// no owned step goes unread.
#[test]
fn every_owned_step_is_an_event_this_task_reads() {
    for case in cases() {
        for step in &case.steps {
            let is_change_only = step.get("event").is_none()
                && step.get("kind").and_then(Value::as_str) != Some("submit");
            if is_change_only {
                assert!(
                    page_event(step).is_none(),
                    "{}: a field change must not be a page event: {step}",
                    case.id
                );
            } else {
                assert!(
                    page_event(step).is_some(),
                    "{}: the recorded step is not read: {step}",
                    case.id
                );
            }
        }
    }
}

/// Applying the recorded steps reaches the recorded page.
#[test]
fn every_case_reaches_the_frozen_final_state() {
    for case in cases() {
        let mut state = initial(&case);
        for step in &case.steps {
            if let Some(event) = page_event(step) {
                state = apply_event(&state, &event);
            }
        }

        assert_eq!(
            decoded_page(&state),
            case.final_state,
            "{}: the final page differs from the baseline",
            case.id
        );
        assert!(
            validate_state(&state).is_ok(),
            "{}: the final state is not valid",
            case.id
        );
        assert_eq!(
            Value::Object(encode_page_params(&state)),
            case.final_params,
            "{}: the final parameters differ from the baseline",
            case.id
        );
    }
}

/// The baseline's patch count is the number of steps that changed the page: a
/// no-op must leave the state exactly as it was.
#[test]
fn every_case_changes_the_page_as_often_as_the_baseline_says() {
    for case in cases() {
        let mut state = initial(&case);
        let mut patches = 0_u64;
        for step in &case.steps {
            if let Some(event) = page_event(step) {
                let next = apply_event(&state, &event);
                if next != state {
                    patches += 1;
                }
                state = next;
            }
        }

        assert_eq!(
            patches, case.patches,
            "{}: the baseline reports {} patches, the reducer made {patches}",
            case.id, case.patches
        );
    }
}

/// The identity, not the pitch set, decides a duplicate: `Cmaj6` and `Amin7` are
/// the same four notes and stay two chords, while an exact duplicate is refused.
#[test]
fn identity_is_the_root_and_quality_not_the_pitch_set() {
    let c6 = ChordSpec {
        root: "C".parse().expect("C is a note"),
        quality: QualityId::parse("maj6").expect("maj6 is a catalog quality"),
    };
    let amin7 = ChordSpec {
        root: "A".parse().expect("A is a note"),
        quality: QualityId::parse("min7").expect("min7 is a catalog quality"),
    };

    let state = decode_page_params(&Map::new());
    let with_amin7 = apply_event(&state, &PageEvent::AddChord(amin7));
    let with_both = apply_event(&with_amin7, &PageEvent::AddChord(c6));

    assert_eq!(
        with_both.chords,
        vec![amin7, c6],
        "both identities must stay, in the order they were added"
    );
    assert_eq!(
        apply_event(&with_both, &PageEvent::AddChord(c6)).chords,
        vec![amin7, c6],
        "an exact duplicate must not be added twice"
    );
}

/// A repeat imported from the URL survives every event that does not remove it:
/// the reducer never deduplicates what the decoder kept.
#[test]
fn imported_repeats_are_kept_and_removed_one_occurrence_at_a_time() {
    let params = params(&[("chords", "Cmaj,Cmaj,Amin")]);
    let state = decode_page_params(&params);
    assert_eq!(state.chords.len(), 3, "the imported repeat must be kept");

    let after_one = apply_event(&state, &PageEvent::RemoveChord { index: 0 });
    assert_eq!(
        after_one.chords,
        vec![state.chords[1], state.chords[2]],
        "the repeat that was not removed survives, in order"
    );

    let after_both = apply_event(&after_one, &PageEvent::RemoveChord { index: 0 });
    assert_eq!(
        after_both.chords,
        vec![state.chords[2]],
        "the second removal takes the repeat away and keeps the other chord"
    );
}

/// Removing the highlighted occurrence of a repeated chord keeps the highlight:
/// the surviving copy carries the same identity. Removing the last one clears it.
#[test]
fn the_highlight_follows_the_identity_through_removals() {
    let params = params(&[("chords", "Cmaj,Cmaj,Amin"), ("highlight", "Cmaj")]);
    let state = decode_page_params(&params);
    let cmaj = state.chords[0];

    let after_one = apply_event(&state, &PageEvent::RemoveChord { index: 0 });
    assert_eq!(
        after_one.highlight,
        Some(cmaj),
        "the surviving copy keeps it"
    );

    let after_two = apply_event(&after_one, &PageEvent::RemoveChord { index: 0 });
    assert_eq!(
        after_two.highlight, None,
        "the last occurrence takes it away"
    );
}

/// Tapping a chip that already carries the highlight clears it, and a repeated
/// chord means either copy does.
#[test]
fn the_highlight_toggles_on_the_identity() {
    let params = params(&[("chords", "Cmaj,Amin,Cmaj")]);
    let state = decode_page_params(&params);

    let on_first = apply_event(&state, &PageEvent::HighlightChord { index: 0 });
    assert_eq!(on_first.highlight, Some(state.chords[0]));
    assert_eq!(
        apply_event(&on_first, &PageEvent::HighlightChord { index: 0 }).highlight,
        None,
        "the same chip toggles it off"
    );
    assert_eq!(
        apply_event(&on_first, &PageEvent::HighlightChord { index: 2 }).highlight,
        None,
        "the other copy of the same identity toggles it off too"
    );
}

/// An index outside the chord list changes nothing, and so does clearing an
/// already empty page.
#[test]
fn an_out_of_range_index_and_a_redundant_clear_change_nothing() {
    let params = params(&[("chords", "Cmaj")]);
    let state = decode_page_params(&params);

    for event in [
        PageEvent::RemoveChord { index: 1 },
        PageEvent::HighlightChord { index: 9 },
    ] {
        assert_eq!(
            apply_event(&state, &event),
            state,
            "{event:?} must change nothing"
        );
    }

    let empty = decode_page_params(&Map::new());
    assert_eq!(
        apply_event(&empty, &PageEvent::ClearAllChords),
        empty,
        "clearing an empty page changes nothing"
    );
}
