//! Note lookup, the chromatic scale and modulo-twelve transposition
//! (`02-core-contract.md` section 2).
//!
//! Display is sharp-only: the twelve names of the chromatic scale are the only
//! spellings this module produces, and the flat aliases of the pinned
//! `Fretboard.Music.Note` source only widen a *lookup*. URL chord and tuning
//! parsing stays sharp-only — it parses [`crate::PitchClass`] directly and never
//! goes through [`note_index`].
//!
//! The rules are ported from the pinned sources
//! (`lib/fretboard/music/{note,pitch}.ex`); no fixture answer is transcribed
//! here.

use crate::error::CoreError;
use crate::types::{PITCH_CLASS_COUNT, PITCH_CLASS_NAMES, PitchClass};

/// The seven flat spellings the note lookup recognizes and the sharp name each
/// one resolves to, in the source's own order.
const FLAT_ALIASES: [(&str, &str); 7] = [
    ("Db", "C#"),
    ("Eb", "D#"),
    ("Fb", "E"),
    ("Gb", "F#"),
    ("Ab", "G#"),
    ("Bb", "A#"),
    ("Cb", "B"),
];

/// The twelve pitch-class names in pitch-class order: the sharp-only display
/// spelling of a note.
///
/// This is the same table the pitch-class primitive itself displays, so the
/// scale and [`PitchClass::name`] cannot drift apart.
pub const fn chromatic_scale() -> [&'static str; 12] {
    PITCH_CLASS_NAMES
}

/// The pitch class of a note name.
///
/// The twelve sharp names are recognized directly and the seven flat aliases
/// (`Db Eb Fb Gb Ab Bb Cb`) resolve to their sharp equivalents; nothing else is
/// a note name — no trimming, no case folding, no double accidentals.
///
/// # Errors
///
/// [`CoreError::UnknownIdentifier`] naming `note` when the name is neither one
/// of the twelve sharp names nor one of the seven flat aliases.
pub fn note_index(name: &str) -> Result<PitchClass, CoreError> {
    sharp_index(name)
        .or_else(|| flat_index(name))
        .and_then(|index| u8::try_from(index).ok())
        .and_then(|value| PitchClass::try_from(value).ok())
        .ok_or_else(|| CoreError::unknown_identifier("note"))
}

/// The position of one of the sharp names in the chromatic scale.
fn sharp_index(name: &str) -> Option<usize> {
    PITCH_CLASS_NAMES.iter().position(|known| *known == name)
}

/// The position of a flat alias's sharp equivalent in the chromatic scale.
fn flat_index(name: &str) -> Option<usize> {
    FLAT_ALIASES
        .iter()
        .find(|(alias, _)| *alias == name)
        .and_then(|(_, sharp)| sharp_index(sharp))
}

/// The pitch class `semitones` away from `base`, modulo twelve.
///
/// The offset is signed and wraps in both directions with the same circular
/// indexing as the pinned `rem` plus `Enum.at` (`note_at(C, -1)` is `B`), which
/// is `rem_euclid`, not Rust's truncated `%` on a negative dividend.
///
/// `arithmetic_side_effects` is allowed for the one addition, and the `i64`
/// intermediate is deliberate: it cannot overflow for any `u8` base plus any
/// `i32` offset, so the arithmetic the release profile checks is bounded by
/// construction.
#[allow(clippy::arithmetic_side_effects)]
pub fn note_at(base: PitchClass, semitones: i32) -> PitchClass {
    let offset = i64::from(i32::from(u8::from(base))) + i64::from(semitones);
    // `rem_euclid` of a positive modulus is always `0..=11`, so both conversions
    // below are exact and the fallback is unreachable for any input.
    u8::try_from(offset.rem_euclid(i64::from(PITCH_CLASS_COUNT)))
        .ok()
        .and_then(|value| PitchClass::try_from(value).ok())
        .unwrap_or(base)
}
