//! The visualizer surface: what a fretboard or a keyboard shows
//! (`02-core-contract.md` section 10, and the pinned
//! `lib/fretboard/music.ex`, `lib/fretboard/music/keyboard.ex` and the
//! `note_fill/4` of `lib/fretboard_web/components/fretboard_svg.ex`).
//!
//! This file belongs to task `C10`. It answers *which* note each position or key
//! carries and *which active chords* claim it, and it decides the fill of a note
//! as a *slot*, never as a colour: the palette belongs to the platform, so the
//! domain says "the first chord's colour" and the client owns the hex values.
//!
//! Three surfaces, all pure:
//!
//! * [`fretted_rows`] — one row per string in physical order, one cell per fret
//!   from the open string to the last fret of that instrument. The note of a cell
//!   is the open note moved by the fret with the real modulo-twelve note math, and
//!   the memberships are the active chords that name it.
//! * [`keyboard_keys`] — one key per pitch of the instrument's keyboard range, in
//!   pitch order, with the same memberships. Every octave shares them, because a
//!   chord names pitch classes, not octaves.
//! * [`note_fill`] — the fill of one note: the slot of the chord it belongs to, or
//!   the overlap. A highlighted chord takes the fill of its own slot when it
//!   claims the note, and the overlap otherwise.
//!
//! The memberships keep the active order **and the repeats**: two occurrences of
//! `Cmaj` claim a note twice, exactly as the frozen lookup does, because the
//! surface is showing occurrences, not identities.
//!
//! `identity_slots` answers the colour half of the same question: which slot each
//! occurrence fills. A repeated identity shares the slot of its first occurrence,
//! which is why two `Cmaj` chips are drawn in one colour and why a client can
//! index its own palette with the slot.

use crate::chord::chord_details;
use crate::error::CoreError;
use crate::instrument_catalog::{instrument_frets, instrument_strings, keyboard_pitch_range};
use crate::note::note_at;
use crate::pitch::note_of;
use crate::state::{ChordSpec, InstrumentState, PageState};
use crate::types::{Fret, InstrumentId, OpenPitch, PitchClass};

/// One position of a fretted surface: its fret, the note it carries and the
/// active chords that claim that note, in active order and with repeats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceCell {
    /// The fret of the position, from the open string to the last fret.
    pub fret: Fret,
    /// The note the position carries.
    pub note: PitchClass,
    /// The active chords that name that note, in active order with repeats.
    pub memberships: Vec<ChordSpec>,
}

/// One key of a keyboard surface: its absolute pitch, its note and the active
/// chords that claim it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardKey {
    /// The absolute pitch of the key.
    pub pitch: OpenPitch,
    /// The note the key carries.
    pub note: PitchClass,
    /// The active chords that name that note, in active order with repeats.
    pub memberships: Vec<ChordSpec>,
}

/// What fills one note of a surface.
///
/// The domain answers with a slot, not a colour: the platform indexes its own
/// palette with it. The overlap is its own answer because it is not a colour the
/// chords own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteFill {
    /// The note is filled with the colour of this slot, which is the index of an
    /// occurrence in the active list; a palette shorter than the list wraps.
    Slot(usize),
    /// The note is filled with the overlap colour: it belongs to no active chord,
    /// to more than one, or the highlighted chord does not claim it.
    Overlap,
}

/// The fretted surface of one tuning: one row per string, in physical order, with
/// one cell per fret up to the instrument's last fret.
///
/// # Errors
///
/// [`CoreError::InvalidState`] when the tuning does not carry exactly one note per
/// string of the instrument, and whatever [`instrument_frets`] and
/// [`instrument_strings`] report for an instrument that is not in the catalog.
pub fn fretted_rows(
    instrument: InstrumentId,
    tuning: &[PitchClass],
    chords: &[ChordSpec],
) -> Result<Vec<Vec<SurfaceCell>>, CoreError> {
    let strings = usize::from(instrument_strings(instrument)?);
    if tuning.len() != strings {
        return Err(CoreError::invalid_state("tuning"));
    }

    let last_fret = instrument_frets(instrument)?;
    let claimed = claims(chords);

    tuning
        .iter()
        .map(|open| {
            (0..=last_fret)
                .map(|fret| {
                    let note = note_at(*open, i32::from(fret));
                    Ok(SurfaceCell {
                        fret: Fret::try_from(fret)?,
                        note,
                        memberships: claimed_by(&claimed, note),
                    })
                })
                .collect::<Result<Vec<SurfaceCell>, CoreError>>()
        })
        .collect::<Result<Vec<Vec<SurfaceCell>>, CoreError>>()
}

/// The keyboard surface: one key per pitch of the instrument's range, in pitch
/// order.
///
/// Every octave of a note carries the same memberships, because a chord names
/// pitch classes.
pub fn keyboard_keys(chords: &[ChordSpec]) -> Vec<KeyboardKey> {
    let (low, high) = keyboard_pitch_range();
    let claimed = claims(chords);

    (u8::from(low)..=u8::from(high))
        .filter_map(|pitch| {
            let pitch = OpenPitch::try_from(pitch).ok()?;
            let note = note_of(pitch);
            Some(KeyboardKey {
                pitch,
                note,
                memberships: claimed_by(&claimed, note),
            })
        })
        .collect()
}

/// The fretted surface of a page state: one row per string of its tuning.
///
/// The state owns the instrument, its tuning and its chords, so this is the
/// surface the visualizer tab draws. The note of every string comes from that
/// string's own pitch, not from a preset name: a tuning edited string by string
/// is still drawn correctly.
///
/// # Errors
///
/// Whatever [`fretted_rows`] reports, and [`CoreError::InvalidState`] when the
/// page is the piano, which has no fretted surface.
pub fn fretted_surface(state: &PageState) -> Result<Vec<Vec<SurfaceCell>>, CoreError> {
    let InstrumentState::Fretted {
        instrument, tuning, ..
    } = &state.instrument
    else {
        return Err(CoreError::invalid_state("instrument"));
    };

    let notes = tuning
        .pitches
        .iter()
        .copied()
        .map(note_of)
        .collect::<Vec<PitchClass>>();

    fretted_rows(*instrument, &notes, &state.chords)
}

/// The keyboard surface of a page state: one key per pitch of the instrument's
/// range, claiming the page's chords.
///
/// The piano has a surface and no tuning, so this answers for the piano page; the
/// chords of a fretted page still claim keys, because those notes exist on a
/// keyboard as well.
#[must_use]
pub fn keyboard_surface(state: &PageState) -> Vec<KeyboardKey> {
    keyboard_keys(&state.chords)
}

/// The colour slot of every occurrence of the active list: the index of the first
/// occurrence of the same identity.
///
/// A repeated identity shares one slot, so the two chips of `Cmaj,Cmaj` are drawn
/// in one colour, and a client can index its palette with these slots.
#[must_use]
pub fn identity_slots(chords: &[ChordSpec]) -> Vec<usize> {
    chords
        .iter()
        .enumerate()
        .map(|(index, spec)| slot_of(chords, spec).unwrap_or(index))
        .collect()
}

/// The colour slot of one identity: the index of its first occurrence.
///
/// This is the one rule behind a repeated chord being drawn in one colour, and it
/// is the domain's answer so that a consumer never re-derives it.
#[must_use]
pub fn slot_of(chords: &[ChordSpec], spec: &ChordSpec) -> Option<usize> {
    chords.iter().position(|candidate| candidate == spec)
}

/// The fill of a note of one page: the slot of its single claim, the highlighted
/// chord's slot when that chord claims it, and the overlap otherwise.
///
/// The highlight is the page's own, so the caller passes only the note's
/// memberships.
#[must_use]
pub fn note_fill_of(state: &PageState, memberships: &[ChordSpec]) -> NoteFill {
    let highlighted = state
        .highlight
        .and_then(|highlight| slot_of(&state.chords, &highlight));
    note_fill(&state.chords, memberships, highlighted)
}

/// The fill of one note: the slot of the chord it belongs to, or the overlap.
///
/// `chords` is the active list and `memberships` are the identities that claim
/// this note, in active order. With a highlighted chord the note takes that
/// chord's slot when the highlighted chord claims it, and the overlap otherwise —
/// including when other chords claim it too. Without a highlight the note takes
/// the slot of its single distinct chord, and the overlap when it has none, when
/// several different chords claim it, or when a membership names no active chord
/// at all.
#[must_use]
pub fn note_fill(
    chords: &[ChordSpec],
    memberships: &[ChordSpec],
    highlight: Option<usize>,
) -> NoteFill {
    if let Some(highlighted) = highlight {
        let Some(spec) = chords.get(highlighted) else {
            return NoteFill::Overlap;
        };
        return if memberships.contains(spec) {
            NoteFill::Slot(highlighted)
        } else {
            NoteFill::Overlap
        };
    }

    let mut distinct = Vec::new();
    for membership in memberships {
        if !distinct.contains(membership) {
            distinct.push(*membership);
        }
    }

    match distinct.as_slice() {
        [only] => slot_of(chords, only).map_or(NoteFill::Overlap, NoteFill::Slot),
        _ => NoteFill::Overlap,
    }
}

/// The notes of every active chord, computed once per surface.
fn claims(chords: &[ChordSpec]) -> Vec<(ChordSpec, Vec<PitchClass>)> {
    chords
        .iter()
        .map(|spec| {
            let notes =
                chord_details(spec).map_or_else(|_error| Vec::new(), |details| details.notes);
            (*spec, notes)
        })
        .collect()
}

/// The active chords that name one note, in active order and with repeats.
fn claimed_by(claimed: &[(ChordSpec, Vec<PitchClass>)], note: PitchClass) -> Vec<ChordSpec> {
    claimed
        .iter()
        .filter(|(_, notes)| notes.contains(&note))
        .map(|(spec, _)| *spec)
        .collect()
}
