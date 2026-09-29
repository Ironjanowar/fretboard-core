//! The page-event reducer: what a tap does to the page state
//! (`02-core-contract.md` section 9, and the pinned
//! `lib/fretboard_web/live/fretboard_live.ex`, `handle_event/3`).
//!
//! `C09` wrote the identity families (the chord list and the highlight); `C13`
//! added the fretted ones — the marked position toggles, clearing the selection,
//! the tab, the instrument change and the tuning commit — to this same module,
//! because they are the same kind of rule: a pure function from the state the
//! page holds to the state it holds next. `C14` added the piano keys
//! ([`PageEvent::TogglePianoKey`]) and completed the instrument boundary: the
//! same [`PageEvent::SetInstrument`] now crosses the piano edge as well as the
//! fretted ones. Nothing is reparsed and nothing is recomputed from the URL.
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
//!   notes — stay two different chords while an exact duplicate is refused. The
//!   fretted families keep the rule (an out-of-range string index is a typed
//!   no-op, never a truncated list), and the one baseline step that pushes without
//!   changing page state is the tuning modal's Apply, described below.
//!
//! ## The tuning modal is UI-only
//!
//! The tuning draft is deliberately **not** part of [`PageState`]. The contract
//! states it (`02-core-contract.md` section 8: *"Tuning draft operations live
//! outside `PageState`; callers commit only on Apply"*), and the baseline behaves
//! that way: `open_tuning_modal`, `select_preset`, `change_string` and
//! `close_tuning_modal` only assign UI fields and push **no** page patch, while
//! `apply_tuning` commits the draft. This module therefore exposes:
//!
//! * [`open_tuning_draft`], [`select_tuning_preset`] and [`change_tuning_string`]
//!   — the pure draft math, whose guards are the handler's own (an unknown preset,
//!   an index the instrument does not have and a note outside the chromatic scale
//!   all leave the draft unchanged, and the piano has no draft at all);
//! * [`DraftEvent`] and [`draft_event`], which read those recorded steps;
//! * [`PageEvent::CommitTuning`], the commit itself.
//!
//! A draft edit can only ever return a new draft, so it cannot mutate the
//! committed state it started from.
//!
//! The event families of the later tasks extend this module in turn: the piano
//! keys and the instrument boundary arrived with `C14` (`reducer_piano`), and the
//! key/progression application arrives with `C18` (`reducer_keys`).
//! [`page_event`] returns `None` for an event it does
//! not know, and a caller must not read that as "the state stays the same" for an
//! event the baseline defines elsewhere.

use std::str::FromStr;

use serde_json::Value;

use crate::instrument_catalog::{instrument_strings, keyboard_pitch_range};
use crate::pitch::change_tuning_note;
use crate::state::{ChordSpec, InstrumentState, PageState, Position, TuningState, preset_tuning};
use crate::types::{
    Fret, InstrumentId, OpenPitch, PitchClass, PresetName, QualityId, StringIndex, Tab,
};

/// One page event that changes the page state.
#[derive(Debug, Clone, PartialEq, Eq)]
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
    /// Mark a position: add it, remove it when it already carries that fret, or
    /// replace the fret of that string.
    ToggleNote(Position),
    /// Toggle one absolute key of the piano, on its analyzer tab: add it, or
    /// remove it when it is already selected.
    ///
    /// A key outside the frozen keyboard range (`48..=83`) is a typed no-op, and
    /// so is a toggle on a page that is not the piano or is not on the analyzer
    /// tab: the pinned handler answers `{:noreply, socket}` for every one of
    /// those, pushing no patch.
    TogglePianoKey(OpenPitch),
    /// Clear the selection, keeping the instrument, its tuning and the chords.
    ClearSelection,
    /// Switch to this tab.
    SetTab(Tab),
    /// Change the instrument, keeping the parts of the selection that still fit.
    SetInstrument(InstrumentId),
    /// Commit a tuning draft: set the committed tuning, and nothing else.
    CommitTuning(TuningState),
}

/// One recorded step of the tuning modal: the UI-only half of the fretted
/// transitions.
///
/// None of these changes the page state; [`DraftEvent::Apply`] is the moment the
/// draft is *committed*, which the caller does with
/// [`PageEvent::CommitTuning`] — the draft itself is not part of the recorded
/// step, so [`page_event`] cannot read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftEvent {
    /// Open the modal: the draft starts from the committed tuning, or does not
    /// open at all on the piano.
    Open,
    /// Select a preset for the draft; an unknown preset leaves it unchanged.
    SelectPreset {
        /// The preset's wire name, which may name no preset of the instrument.
        name: String,
    },
    /// Edit one string of the draft; an invalid index or note leaves it
    /// unchanged.
    ChangeString {
        /// The recorded string index, as text (the handler parses it).
        string: String,
        /// The recorded note name, as the handler receives it.
        note: String,
    },
    /// Close the modal, discarding the draft.
    Close,
    /// Commit the draft (see [`PageEvent::CommitTuning`]).
    Apply,
}

/// Read one recorded step into a page event, when this module implements it.
///
/// A step is the baseline's own shape: an optional `event` name, the `kind` of
/// interaction, and the `value` the handler received. The mapping is:
///
/// * `add_chord` and a submit of the chord form both add one occurrence.
/// * `remove_chord` and `highlight_chord` carry an occurrence index.
/// * `clear_all_chords` carries nothing.
/// * `toggle_note` carries the marked `string` and `fret`.
/// * `toggle_piano_key` carries the absolute `pitch`; the recorded activation
///   `key` gates it — only `Enter` or a space (or no key at all) is a toggle.
/// * `clear_notes` clears the selection.
/// * `toggle_tab` carries the target tab; an unknown value falls back to the
///   visualizer, exactly as the pinned decoder does.
/// * A `change` of `#instrument-form` carries the new instrument.
/// * The tuning modal's steps are [`draft_event`]'s, not page events: the
///   baseline pushes no patch for them, and `apply_tuning` cannot be read from
///   the step because the draft it commits is not in it.
/// * A field change touches no page state here: the chord form's own change only
///   validates, and the field changes that do move the page — the instrument and
///   the tab selects — belong to their tasks (`C13`, `C14`).
pub fn page_event(step: &Value) -> Option<PageEvent> {
    match step.get("event").and_then(Value::as_str) {
        Some("add_chord") => chord_of(step).map(PageEvent::AddChord),
        Some("remove_chord") => index_of(step).map(|index| PageEvent::RemoveChord { index }),
        Some("clear_all_chords") => Some(PageEvent::ClearAllChords),
        Some("highlight_chord") => index_of(step).map(|index| PageEvent::HighlightChord { index }),
        Some("toggle_note") => position_of(step).map(PageEvent::ToggleNote),
        Some("toggle_piano_key") => piano_key_of(step).map(PageEvent::TogglePianoKey),
        Some("clear_notes") => Some(PageEvent::ClearSelection),
        Some("toggle_tab") => Some(PageEvent::SetTab(tab_of(step))),
        Some(_) => None,
        None => submitted_chord(step)
            .map(PageEvent::AddChord)
            .or_else(|| instrument_change(step)),
    }
}

/// Read one recorded step of the tuning modal.
///
/// These steps never change the page state, which is why they are not
/// [`PageEvent`]s: the baseline assigns `show_tuning_modal` / `modal_tuning_state`
/// and pushes no patch. `None` means the step is not a tuning-modal step this
/// module knows.
pub fn draft_event(step: &Value) -> Option<DraftEvent> {
    match step.get("event").and_then(Value::as_str)? {
        "open_tuning_modal" => Some(DraftEvent::Open),
        "close_tuning_modal" => Some(DraftEvent::Close),
        "apply_tuning" => Some(DraftEvent::Apply),
        "select_preset" => Some(DraftEvent::SelectPreset {
            name: step.get("value")?.get("preset")?.as_str()?.to_owned(),
        }),
        "change_string" => Some(DraftEvent::ChangeString {
            string: text_of(step.get("value")?.get("string")?)?,
            note: step.get("value")?.get("note")?.as_str()?.to_owned(),
        }),
        _ => None,
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
        PageEvent::ToggleNote(position) => toggle_note(state, *position),
        PageEvent::TogglePianoKey(pitch) => toggle_piano_key(state, *pitch),
        PageEvent::ClearSelection => clear_selection(state),
        PageEvent::SetTab(tab) => set_tab(state, *tab),
        PageEvent::SetInstrument(instrument) => set_instrument(state, *instrument),
        PageEvent::CommitTuning(tuning) => commit_tuning(state, tuning),
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

/// Mark one position on a fretted page.
///
/// The string keeps exactly one position: the tapped position's fret replaces
/// the string's own when it differs, and removes it when it is the same. A string
/// the instrument does not have is not a position at all — the state is left
/// exactly as it was (`Contract.D10`: the safer typed rejection of a malformed
/// action, never a panic and never a position the page cannot hold).
fn toggle_note(state: &PageState, position: Position) -> PageState {
    let InstrumentState::Fretted { instrument, .. } = &state.instrument else {
        return state.clone();
    };
    let strings = instrument_strings(*instrument).unwrap_or(0);
    if u8::from(position.string) >= strings {
        return state.clone();
    }

    let mut next = state.clone();
    let InstrumentState::Fretted { selected, .. } = &mut next.instrument else {
        return state.clone();
    };

    let current = selected
        .iter()
        .find(|entry| entry.string == position.string)
        .map(|entry| entry.fret);
    match current {
        Some(fret) if fret == position.fret => {
            selected.retain(|entry| entry.string != position.string);
        }
        Some(_) => {
            if let Some(entry) = selected
                .iter_mut()
                .find(|entry| entry.string == position.string)
            {
                entry.fret = position.fret;
            }
        }
        None => {
            let index = selected
                .iter()
                .position(|entry| entry.string > position.string)
                .unwrap_or(selected.len());
            selected.insert(index, position);
        }
    }
    next
}

/// Toggle one absolute key on the piano's analyzer tab.
///
/// The pinned `handle_event("toggle_piano_key", …)` applies only on the piano's
/// analyzer tab and only when the recorded activation key is one it accepts, so
/// every other page is left exactly as it was. The key keeps the page's
/// canonical selection: unique and ascending, so the parameters encode the way
/// [`crate::encode_page_params`] writes them. A value outside the frozen
/// keyboard range is not a key of the keyboard and is a typed no-op
/// (`Contract.D10`).
fn toggle_piano_key(state: &PageState, pitch: OpenPitch) -> PageState {
    if !matches!(state.instrument, InstrumentState::Piano { .. }) || state.tab != Tab::Analyzer {
        return state.clone();
    }
    let (lowest, highest) = keyboard_pitch_range();
    if pitch < lowest || pitch > highest {
        return state.clone();
    }

    let mut next = state.clone();
    let InstrumentState::Piano { selected } = &mut next.instrument else {
        return state.clone();
    };
    if let Some(index) = selected.iter().position(|key| *key == pitch) {
        selected.remove(index);
    } else {
        let index = selected
            .iter()
            .position(|key| *key > pitch)
            .unwrap_or(selected.len());
        selected.insert(index, pitch);
    }
    next
}

/// Clear the selection, whichever kind the instrument is.
fn clear_selection(state: &PageState) -> PageState {
    let mut next = state.clone();
    match &mut next.instrument {
        InstrumentState::Fretted { selected, .. } => selected.clear(),
        InstrumentState::Piano { selected } => selected.clear(),
    }
    next
}

/// Switch the tab; the tab the page already shows changes nothing.
fn set_tab(state: &PageState, tab: Tab) -> PageState {
    if state.tab == tab {
        return state.clone();
    }
    let mut next = state.clone();
    next.tab = tab;
    next
}

/// Change the instrument, exactly as `handle_event("change_instrument", …)`
/// does: reset to the new instrument's Standard tuning, clear the highlight, and
/// keep the parts of the selection the new instrument can hold.
///
/// Switching between fretted instruments keeps the marked positions whose string
/// index still exists; crossing the piano boundary converts nothing — piano keys
/// are never string positions and string positions are never keys.
fn set_instrument(state: &PageState, instrument: InstrumentId) -> PageState {
    if instrument_of(state) == instrument {
        return state.clone();
    }

    let mut next = state.clone();
    next.highlight = None;
    next.instrument = match instrument {
        // Crossing to the piano converts nothing: a string position is never a
        // key, so the new keyboard begins empty and there is no tuning to carry.
        InstrumentId::Piano => InstrumentState::Piano {
            selected: Vec::new(),
        },
        fretted => {
            let selection = match &state.instrument {
                InstrumentState::Piano { .. } => Vec::new(),
                InstrumentState::Fretted { selected, .. } => {
                    let strings = instrument_strings(fretted).unwrap_or(0);
                    selected
                        .iter()
                        .copied()
                        .filter(|position| u8::from(position.string) < strings)
                        .collect()
                }
            };
            let Ok(tuning) = preset_tuning(fretted, &PresetName::from_catalog("Standard")) else {
                // Every fretted instrument of the frozen catalog has a Standard
                // preset; the fallback keeps the function total without an
                // invalid state.
                return state.clone();
            };
            InstrumentState::Fretted {
                instrument: fretted,
                tuning,
                selected: selection,
            }
        }
    };
    next
}

/// Commit a tuning draft: the tuning changes and nothing else does.
///
/// A draft whose pitch count is not the instrument's is not committed — the
/// state would be invalid, and `Contract.D10` prefers a typed no-op to an
/// impossible page.
fn commit_tuning(state: &PageState, tuning: &TuningState) -> PageState {
    let InstrumentState::Fretted { instrument, .. } = &state.instrument else {
        return state.clone();
    };
    if tuning.pitches.len() != usize::from(instrument_strings(*instrument).unwrap_or(0)) {
        return state.clone();
    }

    let mut next = state.clone();
    let InstrumentState::Fretted {
        tuning: committed, ..
    } = &mut next.instrument
    else {
        return state.clone();
    };
    *committed = tuning.clone();
    next
}

/// The draft a page opens: its committed tuning, or none on the piano, which has
/// no tuning and no modal.
#[must_use]
pub fn open_tuning_draft(state: &PageState) -> Option<TuningState> {
    match &state.instrument {
        InstrumentState::Fretted { tuning, .. } => Some(tuning.clone()),
        InstrumentState::Piano { .. } => None,
    }
}

/// Select a preset for a draft.
///
/// The draft is replaced only when the name is a preset of that instrument; an
/// unknown name, or one that belongs to another instrument, leaves the draft
/// exactly as it was, which is what the baseline's `nil` branch does.
#[must_use]
pub fn select_tuning_preset(
    instrument: InstrumentId,
    draft: &TuningState,
    name: &str,
) -> TuningState {
    let Ok(preset) = PresetName::parse(name) else {
        return draft.clone();
    };
    preset_tuning(instrument, &preset).unwrap_or_else(|_error| draft.clone())
}

/// Edit one string of a draft.
///
/// The guards are the handler's own: the string index is parsed from its recorded
/// text and must be a string of the instrument, and the note must be one of the
/// twelve sharp names (`Music.chromatic_scale()`), so a flat spelling the chord
/// note lookup would widen is still rejected here. Anything else leaves the draft
/// unchanged. The edit resolves against the draft's own reference, never against
/// its current pitches (`C11`), so repeated edits cannot drift the anchor.
#[must_use]
pub fn change_tuning_string(
    instrument: InstrumentId,
    draft: &TuningState,
    string: &str,
    note: &str,
) -> TuningState {
    let Ok(index) = string.parse::<i64>() else {
        return draft.clone();
    };
    let Ok(index) = u8::try_from(index) else {
        return draft.clone();
    };
    let Ok(string_index) = StringIndex::try_from(index) else {
        return draft.clone();
    };
    if PitchClass::from_str(note).is_err() {
        return draft.clone();
    }
    change_tuning_note(instrument, draft, string_index, note).unwrap_or_else(|_error| draft.clone())
}

/// The instrument of a page, whichever kind it is.
const fn instrument_of(state: &PageState) -> InstrumentId {
    match &state.instrument {
        InstrumentState::Fretted { instrument, .. } => *instrument,
        InstrumentState::Piano { .. } => InstrumentId::Piano,
    }
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

/// The instrument of a recorded `change` of the instrument select.
fn instrument_change(step: &Value) -> Option<PageEvent> {
    if step.get("kind").and_then(Value::as_str) != Some("change") {
        return None;
    }
    if step.get("selector").and_then(Value::as_str) != Some("#instrument-form") {
        return None;
    }
    let instrument = step
        .get("value")?
        .get("instrument")?
        .as_str()?
        .parse()
        .ok()?;
    Some(PageEvent::SetInstrument(instrument))
}

/// The marked position of a step's value, as text or as a number.
fn position_of(step: &Value) -> Option<Position> {
    let value = step.get("value")?;
    Some(Position {
        string: StringIndex::try_from(u8_of(value.get("string")?)?).ok()?,
        fret: Fret::try_from(u8_of(value.get("fret")?)?).ok()?,
    })
}

/// The absolute key of a `toggle_piano_key` step.
///
/// The pinned handler applies the toggle only when the recorded activation key
/// is one it accepts (`Enter` or a space) or when no key is recorded at all; a
/// recorded `Tab` is not a toggle at all, so this answers `None`. The pitch is
/// read as text or as a number, exactly like the marked positions, and a value
/// the open-pitch type cannot hold is not a key either. The keyboard-range guard
/// is the reducer's, not the reader's, so a representable key outside `48..=83`
/// still reads as a toggle and is refused when it is applied.
fn piano_key_of(step: &Value) -> Option<OpenPitch> {
    let value = step.get("value")?;
    if !activation_accepted(value) {
        return None;
    }
    OpenPitch::try_from(u8_of(value.get("pitch")?)?).ok()
}

/// Whether a recorded step carries an activation key the handler accepts
/// (`valid_activation?/1`: `Enter` or a space, or no key at all).
fn activation_accepted(value: &Value) -> bool {
    match value.get("key") {
        None => true,
        Some(Value::String(key)) => key == "Enter" || key == " ",
        Some(_) => false,
    }
}

/// The target tab of a `toggle_tab` step.
///
/// The pinned decoder answers `:visualizer` for anything outside the two names,
/// including a missing value, so this cannot fail.
fn tab_of(step: &Value) -> Tab {
    match step
        .get("value")
        .and_then(|value| value.get("tab"))
        .and_then(Value::as_str)
    {
        Some("analyzer") => Tab::Analyzer,
        _ => Tab::Visualizer,
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

/// A small unsigned value as text or as a number.
fn u8_of(value: &Value) -> Option<u8> {
    match value {
        Value::String(text) => text.parse::<u8>().ok(),
        Value::Number(number) => number.as_u64().and_then(|value| u8::try_from(value).ok()),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}

/// A value as text, whatever JSON type the step carries it in.
fn text_of(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_) => None,
    }
}
