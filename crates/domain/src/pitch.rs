//! Fixed-reference pitch editing (`02-core-contract.md` section 2, and the
//! pinned `Fretboard.Music.Pitch.string_pitches/2` with
//! `Fretboard.Music.change_tuning_note/4`, `detect_preset/2` and `tuning_notes/1`).
//!
//! This file belongs to task `C11`. It owns one rule and the two edit
//! operations built on it:
//!
//! * [`closest_pitch`] — the pitch of a named class nearest a reference pitch,
//!   with a six-semitone tie resolving deterministically downward. This is the
//!   **one** copy of that rule in the crate: the page-params codec (`C08`)
//!   resolves the legacy `tuning` notes through it as well, because the plan's
//!   handoff says the nearest-reference subpart of `C11` serves `C08` and must
//!   never be duplicated as a second codec algorithm.
//! * [`change_tuning_note`] — editing one string. The anchor is **that string's
//!   pitch in the reference preset**, not the string's current pitch and not a
//!   detected preset: an edit therefore never drifts, and a state whose pitches
//!   have already moved away from its preset still resolves against the preset
//!   it is anchored to. `02-core-contract.md` states this rule explicitly, and
//!   the frozen `edit-sequence/…` records exercise it.
//! * [`tuning_notes`] — the note names of a tuning state, derived from its
//!   pitches (never stored beside them), and [`detect_preset`] — exact-pitch
//!   preset detection in catalog order, answering `Custom` for anything that is
//!   not exactly a preset. `Custom` is not a catalog identifier, so it is the
//!   `None` of the typed surface rather than a [`PresetName`].
//!
//! Everything here is pure: every function takes a tuning state by reference and
//! returns a new value, so a draft calculation cannot mutate the committed state
//! it started from, and a caller commits only on Apply.
//!
//! Fixture: every record of `fixtures/oracle/tunings.jsonl` (1316 cases) is
//! pinned by `tests/tuning.rs`; the nearest-pitch rule itself is also pinned by
//! the page-params tests through the legacy tuning decode.

use crate::error::CoreError;
use crate::instrument_catalog::{instrument_strings, pitch_presets, preset_pitches};
use crate::note::note_index;
use crate::state::TuningState;
use crate::types::{InstrumentId, OpenPitch, PitchClass, PresetName, StringIndex};

/// The note names of a tuning state, in physical string order.
///
/// The names are derived from the exact pitches — the pitch class of each, per
/// `Pitch.note_name/1` — and are never an independent state.
pub fn tuning_notes(state: &TuningState) -> Vec<PitchClass> {
    state.pitches.iter().copied().map(note_of).collect()
}

/// The named preset a set of exact pitches is, in catalog order, or `None` when
/// it is not exactly one of them — the baseline's `Custom`.
///
/// The comparison is by exact absolute pitches, never by note names or pitch
/// classes: an octave-shifted or semitone-shifted standard tuning is not a
/// preset, and the first matching preset in catalog order wins.
pub fn detect_preset(instrument: InstrumentId, pitches: &[OpenPitch]) -> Option<PresetName> {
    pitch_presets(instrument)
        .iter()
        .find(|preset| preset.pitches == pitches)
        .map(|preset| preset.name)
}

/// The pitch of one pitch class that is closest to a reference pitch.
///
/// The candidate octaves are the reference's own octave and its neighbours; the
/// pinned source lists them in ascending order and takes the first minimum, so a
/// tie of six semitones deterministically resolves downwards. The result is the
/// requested class and is never further than six semitones from the reference.
///
/// `None` only when a pitch of that class next to the reference does not fit in
/// an [`OpenPitch`] (a reference above 121 with a class far above the
/// reference's own).
pub fn closest_pitch(note: PitchClass, reference: OpenPitch) -> Option<OpenPitch> {
    let reference = i16::from(u8::from(reference));
    #[allow(clippy::integer_division)]
    let remainder = reference % 12;
    let anchor = reference.saturating_sub(remainder);
    let base = anchor.saturating_add(i16::from(u8::from(note)));
    let candidates = [base.saturating_sub(12), base, base.saturating_add(12)];

    let chosen = candidates
        .into_iter()
        .min_by_key(|candidate| candidate.saturating_sub(reference).unsigned_abs())?;
    let byte = u8::try_from(chosen).ok()?;
    OpenPitch::try_from(byte).ok()
}

/// Resolve note names against the standard pitches of the instrument: every name
/// becomes the pitch of its class nearest that string's standard pitch.
///
/// This is the legacy `tuning` decode of the page-params codec (`C08`), which
/// asks `C11`'s rule instead of keeping a second copy of it.
pub(crate) fn anchored_pitches(notes: &[PitchClass], instrument: InstrumentId) -> Vec<OpenPitch> {
    let standard = standard_of(instrument);
    notes
        .iter()
        .zip(standard.iter())
        .filter_map(|(note, reference)| closest_pitch(*note, *reference))
        .collect()
}

/// The note name of one absolute pitch: its pitch class.
///
/// Every pitch is within `0..=127`, so the reduction to a pitch class is exact;
/// the unison is returned only to keep the function total.
#[allow(clippy::arithmetic_side_effects)]
pub(crate) fn note_of(pitch: OpenPitch) -> PitchClass {
    let index = u8::from(pitch) % 12;
    PitchClass::try_from(index).unwrap_or(PitchClass::from_catalog(0))
}

/// The standard pitches of an instrument, or none for a keyboard.
fn standard_of(instrument: InstrumentId) -> &'static [OpenPitch] {
    preset_pitches(instrument, PresetName::from_catalog("Standard")).unwrap_or(&[])
}

/// Edit one string's note, resolved against the fixed reference preset.
///
/// The note name is turned into the pitch of its class nearest **that string's
/// pitch in the preset the state is anchored to** — never the string's current
/// pitch, so repeated edits cannot drift the anchor — and the returned state
/// carries the same reference and the other strings untouched. The operation is
/// pure: `state` is never mutated.
///
/// # Errors
///
/// * [`CoreError::InvalidState`] naming `instrument` when the instrument is the
///   piano, which has no strings and no tuning.
/// * [`CoreError::InvalidState`] naming `tuning` when the state's pitch count is
///   not the instrument's string count.
/// * [`CoreError::InvalidState`] naming `reference` when the state's reference is
///   not a preset of that instrument — a committed state's structural violation,
///   per `docs/contracts.md`, not a lookup failure.
/// * [`CoreError::OutOfRange`] naming `string` when the instrument has no such
///   string.
/// * [`CoreError::UnknownIdentifier`] naming `note` when the value is neither one
///   of the twelve sharp names nor one of the seven flat aliases the baseline's
///   note lookup accepts (`Note.note_index/1`).
pub fn change_tuning_note(
    instrument: InstrumentId,
    state: &TuningState,
    string: StringIndex,
    note: &str,
) -> Result<TuningState, CoreError> {
    let strings = instrument_strings(instrument)?;
    if state.pitches.len() != usize::from(strings) {
        return Err(CoreError::invalid_state("tuning"));
    }

    let index = usize::from(u8::from(string));
    if index >= usize::from(strings) {
        return Err(CoreError::out_of_range("string"));
    }

    let reference = preset_pitches(instrument, state.reference)
        .ok_or_else(|| CoreError::invalid_state("reference"))?;
    let anchor = reference
        .get(index)
        .copied()
        .ok_or_else(|| CoreError::out_of_range("string"))?;

    let pitch =
        closest_pitch(note_index(note)?, anchor).ok_or_else(|| CoreError::out_of_range("note"))?;

    let mut pitches = state.pitches.clone();
    match pitches.get_mut(index) {
        Some(slot) => *slot = pitch,
        None => return Err(CoreError::out_of_range("string")),
    }

    Ok(TuningState {
        pitches,
        reference: state.reference,
    })
}
