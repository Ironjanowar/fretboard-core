//! Interval names (`02-core-contract.md` section 2, and the pinned
//! `lib/fretboard/music/{intervals,pitch}.ex`).
//!
//! This module owns the *simple-table* vocabulary of an interval distance. The
//! chord-local contextual vocabulary (`Root`, `Major 3rd`, …) is a different
//! function of a different question — a chord member's role inside one formula —
//! and lives in [`crate::chord`]; the two tables are deliberately not shared.

use crate::types::PITCH_CLASS_COUNT;

/// The simple interval names of semitones `0..=11`, in pitch-class order:
/// `Perfect Unison`, `Minor 2nd`, … `Major 7th`.
const SIMPLE_INTERVAL_NAMES: [&str; 12] = [
    "Perfect Unison",
    "Minor 2nd",
    "Major 2nd",
    "Minor 3rd",
    "Major 3rd",
    "Perfect 4th",
    "Tritone",
    "Perfect 5th",
    "Augmented 5th",
    "Major 6th",
    "Minor 7th",
    "Major 7th",
];

/// The name of the interval of `semitones` semitones.
///
/// Zero is the `Perfect Unison`; a positive multiple of twelve is an `Octave`
/// (two distinct pitches told apart by their absolute height are never
/// collapsed into the deeper note); every other distance, however many octaves
/// it spans, reduces to its simple interval `0..=11`.
///
/// `arithmetic_side_effects` is allowed for the one bounded remainder: the
/// divisor is the non-zero pitch-class count, so the operation cannot overflow
/// or divide by zero.
#[allow(clippy::arithmetic_side_effects)]
pub fn interval_name(semitones: u32) -> &'static str {
    if semitones == 0 {
        return simple_interval_name(0);
    }
    let simple = semitones % u32::from(PITCH_CLASS_COUNT);
    if simple == 0 {
        return "Octave";
    }
    simple_interval_name(simple)
}

/// The simple interval name of a distance that is already reduced to `0..=11`.
///
/// The lookup cannot miss for a reduced distance, and the fallback is the
/// unison name; the `.get` form keeps the caller free of an unchecked index.
fn simple_interval_name(reduced: u32) -> &'static str {
    u8::try_from(reduced)
        .ok()
        .map(usize::from)
        .and_then(|index| SIMPLE_INTERVAL_NAMES.get(index).copied())
        .unwrap_or("Perfect Unison")
}
