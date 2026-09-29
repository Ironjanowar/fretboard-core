//! The absolute analyzer (`02-core-contract.md` section 8, and the pinned
//! `Fretboard.Music.Analyzer`).
//!
//! This file belongs to task `C13`. It answers one question — *what is this
//! selection?* — from absolute sounding pitches, and it does so independently
//! of the instrument that produced them:
//!
//! * [`analyze_pitches`] is the instrument-independent analyzer. Any selection
//!   that can be expressed as absolute pitches analyzes the same way, which is
//!   why the piano reaches it with its selected keys and `C14` needs no
//!   keyboard-specific chord algorithm.
//! * [`analyze_page`] is the page's own view of it: the committed selection
//!   mapped through the instrument's tuning, gated by the tab.
//!
//! ## The rule
//!
//! The sounding pitches are sorted ascending and reduced to one representative
//! per pitch class — the **lowest height** of each class, because the sort comes
//! first. The result is then:
//!
//! * `empty` — nothing selected;
//! * `single` — one pitch class at one height only (a repeated pitch is still a
//!   single note);
//! * `interval` — one class at distinct heights (the baseline's `Octave`), or
//!   two classes ordered by their lowest height, labelled by the simple interval
//!   between those lowest occurrences;
//! * `chords` — three or more classes; the notes are the representatives in
//!   ascending order, the bass is the lowest sounding pitch, and the ordered
//!   identifications are [`identify_notes_with_bass`].
//!
//! Two consequences the baseline's own documentation calls out, and which this
//! port keeps: the bass of a shape is its **lowest sounding pitch**, so a
//! reentrant tuning (the ukulele's high G) and a real pitch crossing both work;
//! and two notes a pitch class apart are reported as an `Octave` rather than
//! collapsing into one note.
//!
//! ## One copy of the recognition rules
//!
//! The chord recognition is **not** reimplemented here: [`analyze_pitches`]
//! calls [`crate::identify_notes_with_bass`], so the ordered note-set rules have
//! exactly one home (`C12`). The analyzer adds the pitch reduction, the bass and
//! the interval vocabulary ([`crate::interval_name`]) around it.
//!
//! ## The tab gate
//!
//! `Evaluation.analysis` is optional and **absent on the visualizer tab**
//! (`02-core-contract.md` section 8): [`analyze_page`] answers `None` there, and
//! an empty selection on the analyzer tab is `Some(Analysis::Empty)` — a
//! different thing from "not computed". On the analyzer tab the analysis depends
//! on the *selection* alone; the stored visualizer chords and the highlight
//! never reach it.
//!
//! Fixture: every record of `fixtures/oracle/analyzer.jsonl` (34 cases, both
//! `analyze_pitches/1` and `analyzer_state/2`) is pinned by `tests/analyzer.rs`.

use crate::identify::identify_notes_with_bass;
use crate::interval::interval_name;
use crate::state::{InstrumentState, PageState};
use crate::types::{PITCH_CLASS_COUNT, PitchClass, SoundingPitch, Tab};

/// What a selection is: the baseline's `analyze_pitches/1` answer.
///
/// The variants are the baseline's own tuples, in its own order, and the
/// [`Chords`](Self::Chords) list is the **ordered** identification of
/// [`crate::identify_notes_with_bass`] — the whole list is the answer, never a
/// set or a first match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Analysis {
    /// Nothing is selected.
    Empty,
    /// One pitch class at one height.
    Single {
        /// The note.
        note: PitchClass,
    },
    /// One class at distinct heights, or two classes seen from their lowest
    /// heights.
    Interval {
        /// The lower note name.
        low: PitchClass,
        /// The higher note name. Equal to `low` for the one-class case.
        high: PitchClass,
        /// The simple interval name between them (`Octave` across octaves).
        label: &'static str,
    },
    /// Three or more pitch classes.
    Chords {
        /// The representatives, in ascending sounding order.
        notes: Vec<PitchClass>,
        /// The note of the lowest sounding pitch.
        bass: PitchClass,
        /// Every identification the baseline recognizes, in its order.
        interpretations: Vec<crate::Interpretation>,
    },
}

/// The analysis of a selection of absolute sounding pitches.
///
/// Input order and repeated pitches do not affect the answer: the pitches are
/// sorted and reduced first.
#[must_use]
pub fn analyze_pitches(pitches: &[SoundingPitch]) -> Analysis {
    let mut sorted: Vec<u16> = pitches.iter().map(|pitch| u16::from(*pitch)).collect();
    sorted.sort_unstable();

    // Sorting before deduplicating keeps the lowest height of each class.
    let mut classes: Vec<u16> = Vec::new();
    for pitch in &sorted {
        let class = pitch_class_of(*pitch);
        if classes.iter().all(|kept| pitch_class_of(*kept) != class) {
            classes.push(*pitch);
        }
    }

    match classes.as_slice() {
        [] => Analysis::Empty,
        [only] => {
            let repeated = sorted.windows(2).any(|pair| pair.first() != pair.get(1));
            if !repeated {
                return Analysis::Single {
                    note: pitch_class_of(*only),
                };
            }
            let last = sorted.last().copied().unwrap_or(*only);
            Analysis::Interval {
                low: pitch_class_of(*only),
                high: pitch_class_of(*only),
                label: interval_name(u32::from(last.saturating_sub(*only))),
            }
        }
        [low, high] => Analysis::Interval {
            low: pitch_class_of(*low),
            high: pitch_class_of(*high),
            label: interval_name(u32::from(high.saturating_sub(*low))),
        },
        _ => {
            let Some(bass_pitch) = classes.first().copied() else {
                // `classes` has three or more entries here, so the fallback only
                // keeps the function total without a panic.
                return Analysis::Empty;
            };
            let notes: Vec<PitchClass> = classes.iter().copied().map(pitch_class_of).collect();
            let bass = pitch_class_of(bass_pitch);
            Analysis::Chords {
                interpretations: identify_notes_with_bass(&notes, bass),
                notes,
                bass,
            }
        }
    }
}

/// The analysis of a page, or `None` when the page is on the visualizer tab.
///
/// The selection is mapped to absolute pitches through the committed tuning:
/// each marked position sounds at its string's open pitch plus the fret, and the
/// piano's selected keys are already absolute. The stored chords and the
/// highlight are never consulted.
#[must_use]
pub fn analyze_page(state: &PageState) -> Option<Analysis> {
    if state.tab != Tab::Analyzer {
        return None;
    }
    Some(analyze_pitches(&selected_pitches(state)))
}

/// The absolute sounding pitches of a page's committed selection.
///
/// A fretted position outside the tuning's string count cannot occur in a
/// validated state; the `filter_map` keeps the function total without a panic.
fn selected_pitches(state: &PageState) -> Vec<SoundingPitch> {
    match &state.instrument {
        InstrumentState::Fretted {
            tuning, selected, ..
        } => selected
            .iter()
            .filter_map(|position| {
                let open = *tuning.pitches.get(usize::from(u8::from(position.string)))?;
                #[allow(clippy::arithmetic_side_effects)]
                // An open pitch is at most 127 and a fret at most 24, so the
                // sum is at most the sounding-pitch maximum of 151.
                let sounding = u16::from(u8::from(open)) + u16::from(u8::from(position.fret));
                SoundingPitch::try_from(sounding).ok()
            })
            .collect(),
        InstrumentState::Piano { selected } => selected
            .iter()
            .filter_map(|pitch| SoundingPitch::try_from(u16::from(u8::from(*pitch))).ok())
            .collect(),
    }
}

/// The pitch class of an absolute sounding pitch: `Pitch.note_name/1`'s class.
///
/// The reduction is exact for every sounding pitch, so the fallback only keeps
/// the function total.
fn pitch_class_of(pitch: u16) -> PitchClass {
    // The divisor is the non-zero pitch-class count, so the remainder cannot
    // overflow or divide by zero.
    #[allow(clippy::arithmetic_side_effects)]
    let index = pitch % u16::from(PITCH_CLASS_COUNT);
    u8::try_from(index)
        .ok()
        .and_then(|value| PitchClass::try_from(value).ok())
        .unwrap_or_else(low_pitch_class)
}

/// The pitch class the reduction falls back to; unreachable for a `u16`.
const fn low_pitch_class() -> PitchClass {
    PitchClass::from_catalog(0)
}
