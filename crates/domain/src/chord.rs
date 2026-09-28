//! Chord details: the notes and contextual interval labels of a chord quality
//! (`02-core-contract.md` section 3, and the pinned
//! `lib/fretboard/music/chord.ex`).
//!
//! This file belongs to task `C06` (the complete 47-quality catalog); P1 reaches
//! it through the documented `C03` handoff, which authorizes exactly one
//! quality: `major`. Every other catalog quality is a
//! [`CoreError::UnsupportedCapability`] — a known feature this build does not
//! implement yet — never an empty chord and never another quality's formula.
//!
//! The notes are computed with the real note math ([`note_at`]); no fixture
//! answer is stored. The contextual labels are the chord-local vocabulary:
//! semitone 0 is the chord's `Root`, and every other offset follows the pinned
//! `interval_label_for/2` rules (an extension is named as a compound interval
//! only when the formula actually carries a seventh, which is also when the
//! label list of the plan's section 3 is sorted by chord-member rank — that sort
//! arrives with `C06` together with the first quality that reaches it).

use crate::error::CoreError;
use crate::interval::interval_name;
use crate::note::note_at;
use crate::state::ChordSpec;
use crate::types::{PitchClass, QualityId};

/// One chord quality this build implements: its stable catalog identifier, its
/// display/wire suffix and its semitone formula in returned-note order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Quality {
    /// The stable catalog identifier the quality parses from.
    id: &'static str,
    /// The display/wire suffix concatenated after the root.
    suffix: &'static str,
    /// The semitone offsets of the chord members, in returned-note order.
    formula: &'static [u8],
}

/// The `major` triad: identifier `major`, suffix `maj`, formula `[0, 4, 7]`.
const MAJOR: Quality = Quality {
    id: "major",
    suffix: "maj",
    formula: &[0, 4, 7],
};

/// The contextual label of a chord's root member.
const ROOT_LABEL: &str = "Root";

/// The quality behind a catalog identifier, when this build implements it.
///
/// P1 implements `major` only; the other 46 catalog qualities are `C06`.
fn implemented_quality(quality: QualityId) -> Option<Quality> {
    match quality.as_str() {
        "major" => Some(MAJOR),
        _ => None,
    }
}

/// The derived details of one chord: what it is called, which notes it names and
/// which role each of those notes plays in the chord.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChordDetails {
    /// The chord root the details were derived from.
    pub root: PitchClass,
    /// The chord quality identifier the details were derived from.
    pub quality: QualityId,
    /// The full label: the root's sharp name concatenated with the quality's
    /// wire suffix, e.g. `Cmaj`.
    pub label: String,
    /// The chord members as pitch classes, in formula order.
    pub notes: Vec<PitchClass>,
    /// The contextual role of each member, in the same order as `notes`.
    pub intervals: Vec<&'static str>,
}

/// The details of one chord: its label, its notes and their interval roles.
///
/// The notes are the root transposed by the quality's formula with the real
/// modulo-twelve note math, so every root is computed, not looked up.
///
/// # Errors
///
/// [`CoreError::UnsupportedCapability`] naming `quality` when the quality is a
/// known catalog identifier that this build does not implement yet. No
/// unimplemented quality ever answers with another quality's chord.
pub fn chord_details(spec: &ChordSpec) -> Result<ChordDetails, CoreError> {
    let quality = implemented_quality(spec.quality)
        .ok_or_else(|| CoreError::unsupported_capability("quality"))?;
    let notes = quality
        .formula
        .iter()
        .map(|semitone| note_at(spec.root, i32::from(*semitone)))
        .collect();
    let intervals = quality
        .formula
        .iter()
        .map(|semitone| contextual_interval_label(*semitone, quality.formula))
        .collect();
    let root_name = spec.root.name();
    let suffix = quality.suffix;
    Ok(ChordDetails {
        root: spec.root,
        quality: spec.quality,
        label: format!("{root_name}{suffix}"),
        notes,
        intervals,
    })
}

/// The chord-local label of one formula offset inside its own formula.
///
/// Ported from the pinned `Fretboard.Music.Chord.interval_label_for/2`: zero is
/// the chord's `Root`; the third, fifth and seventh keep their simple names; a
/// ninth, eleventh or thirteenth offset is named as that compound interval only
/// when the formula carries a seventh; and the altered offsets follow the
/// source's contextual rules for the tones they replace.
fn contextual_interval_label(semitone: u8, formula: &[u8]) -> &'static str {
    let seventh = has_seventh(formula);
    match semitone {
        0 => ROOT_LABEL,
        1 => extension(seventh, "Flat 9th", semitone),
        2 => extension(seventh, "Major 9th", semitone),
        5 => extension(seventh, "Perfect 11th", semitone),
        9 => extension(seventh, "Major 13th", semitone),
        3 => {
            if seventh && formula.contains(&4) {
                "Sharp 9th"
            } else {
                simple_interval_label(semitone)
            }
        }
        6 => {
            if formula.contains(&7) {
                "Augmented 11th"
            } else {
                simple_interval_label(semitone)
            }
        }
        8 => {
            if formula.contains(&6) || formula.contains(&7) {
                "Minor 13th"
            } else {
                simple_interval_label(semitone)
            }
        }
        // Every remaining offset — including the always-simple 4, 7, 10 and 11 —
        // keeps the simple interval name of its distance.
        _ => simple_interval_label(semitone),
    }
}

/// The compound name of an extension offset when the formula carries a seventh,
/// otherwise its simple interval name.
fn extension(seventh: bool, compound: &'static str, semitone: u8) -> &'static str {
    if seventh {
        compound
    } else {
        simple_interval_label(semitone)
    }
}

/// Whether a formula carries a seventh (semitone 10 or 11, never the diminished
/// seventh 9).
fn has_seventh(formula: &[u8]) -> bool {
    formula.contains(&10) || formula.contains(&11)
}

/// The simple interval name of one formula offset.
fn simple_interval_label(semitone: u8) -> &'static str {
    interval_name(u32::from(semitone))
}
