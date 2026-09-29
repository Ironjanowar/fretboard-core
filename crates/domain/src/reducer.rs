//! The page-event reducer: what a tap does to the page state
//! (`02-core-contract.md` section 9, and the pinned
//! `lib/fretboard_web/live/fretboard_live.ex`, `handle_event/3`).
//!
//! This file belongs to task `C09`. The reducer is pure: it takes the current
//! [`PageState`] and one page event and returns the next state. Nothing is
//! reparsed and nothing is recomputed from the URL — a tap acts on the state the
//! page already holds, which is what the baseline's own socket does.
//!
//! Two rules shape every branch:
//!
//! * **An occurrence is not an identity.** The chord list keeps repeats (a URL can
//!   import `Cmaj,Cmaj`, and the pinned decoder keeps both), so events address
//!   occurrences by index while the *highlight* is held as the identity it points
//!   at. That is why removing one of two `Cmaj` occurrences keeps the highlight
//!   (the surviving copy still carries the identity) and why removing the last one
//!   clears it.
//! * **A tap that changes nothing must leave the state exactly as it was.** The
//!   baseline counts patches, so a no-op add, a same-chip highlight toggle and an
//!   out-of-range index all return the state untouched; the identity comparison is
//!   `(root, quality)`, never the pitch set, so `C6` and `Amin7` — the same four
//!   notes — stay two different chords while an exact duplicate is refused.
//!
//! The event families of the later tasks extend this module: the fretted
//! selection and tuning drafts arrive with `C13` (`reducer_fretted`), the piano
//! keys with `C14` (`reducer_piano`) and the key/progression application with
//! `C18` (`reducer_keys`). [`page_event`] returns `None` for an event it does not
//! know, and a caller must not read that as "the state stays the same" for an
//! event the baseline defines elsewhere.

use std::str::FromStr;

use serde_json::Value;

use crate::state::{ChordSpec, PageState};
use crate::types::{PitchClass, QualityId};

/// One page event that changes the page state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageEvent {
    /// Add one occurrence of a chord, unless that identity is already present.
    AddChord(ChordSpec),
    /// Remove the occurrence at this index.
    RemoveChord {
        /// The index of the occurrence to remove.
        index: usize,
    },
    /// Remove every chord and clear the highlight, keeping everything else.
    ClearAllChords,
    /// Highlight the occurrence at this index, or clear the highlight when that
    /// occurrence already carries it.
    HighlightChord {
        /// The index of the occurrence the tap landed on.
        index: usize,
    },
}

/// Read one recorded step into a page event, when this task implements it.
///
/// A step is the baseline's own shape: an optional `event` name, the `kind` of
/// interaction, and the `value` the handler received. The mapping is:
///
/// * `add_chord` and a submit of the chord form both add one occurrence.
/// * `remove_chord` and `highlight_chord` carry an occurrence index.
/// * `clear_all_chords` carries nothing.
/// * A field change touches no page state here: the chord form's own change only
///   validates, and the field changes that do move the page — the instrument and
///   the tab selects — belong to their tasks (`C13`, `C14`).
pub fn page_event(step: &Value) -> Option<PageEvent> {
    match step.get("event").and_then(Value::as_str) {
        Some("add_chord") => chord_of(step).map(PageEvent::AddChord),
        Some("remove_chord") => index_of(step).map(|index| PageEvent::RemoveChord { index }),
        Some("clear_all_chords") => Some(PageEvent::ClearAllChords),
        Some("highlight_chord") => index_of(step).map(|index| PageEvent::HighlightChord { index }),
        Some(_) => None,
        None => submitted_chord(step).map(PageEvent::AddChord),
    }
}

/// Apply one page event to the state.
///
/// The result is the whole next state, and an event that changes nothing returns
/// the state it was given.
#[must_use]
pub fn apply_event(state: &PageState, event: &PageEvent) -> PageState {
    match event {
        PageEvent::AddChord(spec) => add_chord(state, *spec),
        PageEvent::RemoveChord { index } => remove_chord(state, *index),
        PageEvent::ClearAllChords => clear_chords(state),
        PageEvent::HighlightChord { index } => highlight_chord(state, *index),
    }
}

/// Add one occurrence, unless the identity is already there.
fn add_chord(state: &PageState, spec: ChordSpec) -> PageState {
    if state.chords.contains(&spec) {
        return state.clone();
    }

    let mut next = state.clone();
    next.chords.push(spec);
    next
}

/// Remove one occurrence, and clear the highlight when its last occurrence goes.
fn remove_chord(state: &PageState, index: usize) -> PageState {
    if index >= state.chords.len() {
        return state.clone();
    }

    let mut next = state.clone();
    next.chords.remove(index);
    if let Some(highlight) = next.highlight
        && !next.chords.contains(&highlight)
    {
        next.highlight = None;
    }
    next
}

/// Clear the chords and the highlight; the instrument, its selection and the tab
/// stay as they are.
fn clear_chords(state: &PageState) -> PageState {
    let mut next = state.clone();
    next.chords = Vec::new();
    next.highlight = None;
    next
}

/// Highlight the tapped occurrence, or clear the highlight when that occurrence
/// already carries it.
///
/// The comparison is by identity, so for a repeated chord either copy toggles the
/// highlight off, exactly as the baseline's single stored identity does.
fn highlight_chord(state: &PageState, index: usize) -> PageState {
    let Some(spec) = state.chords.get(index).copied() else {
        return state.clone();
    };

    let mut next = state.clone();
    next.highlight = if state.highlight == Some(spec) {
        None
    } else {
        Some(spec)
    };
    next
}

/// The chord of a step's value, when it names one of the frozen identities.
fn chord_of(step: &Value) -> Option<ChordSpec> {
    let chord = step.get("value")?.get("chord")?;
    Some(ChordSpec {
        root: PitchClass::from_str(chord.get("root")?.as_str()?).ok()?,
        quality: QualityId::parse(chord.get("quality")?.as_str()?).ok()?,
    })
}

/// The chord of a submitted chord form.
///
/// A submit names its form in `selector`; a `change` on the same form only
/// validates what was typed, so it is not an event at all.
fn submitted_chord(step: &Value) -> Option<ChordSpec> {
    match step.get("kind").and_then(Value::as_str) {
        Some("submit") => chord_of(step),
        _ => None,
    }
}

/// The occurrence index of a step's value, as a number or as a decimal string.
fn index_of(step: &Value) -> Option<usize> {
    match step.get("value")?.get("index")? {
        Value::String(text) => text.parse::<usize>().ok(),
        Value::Number(number) => number
            .as_u64()
            .and_then(|value| usize::try_from(value).ok()),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}
