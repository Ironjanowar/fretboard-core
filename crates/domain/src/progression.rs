//! Task `C18`: resolving a progression's degrees into chords.
//!
//! Ported from the pinned `Fretboard.Music.Progression.progression_chords/2`:
//!
//! * the diatonic chords of the **progression's own scale type** are the base
//!   (`Scale.diatonic_chords/2`, which is the triad classifier here);
//! * each degree takes the diatonic chord of its degree number, moves the root by
//!   its `accidental` semitones and keeps the quality, unless
//! * the degree names an explicit quality, which always wins, or
//! * the degree is flattened by one semitone in a major-flavoured context and no
//!   explicit quality is given: ♭II (Neapolitan), ♭III, ♭VI and ♭VII are
//!   conventionally major triads (`infer_altered_quality/3`).
//!
//! The two rules are the whole port; `tests/progressions.rs` pins the answer
//! against the 1062 frozen records, which sweep all 59 progressions over the
//! twelve chromatic tonics and the six example keys.
//!
//! The apply side lives in the reducer: [`crate::PageEvent::CommitProgression`]
//! replaces the page's chords with exactly this list, occurrences and all.

use crate::chord::ChordMode;
use crate::note::note_at;
use crate::progression_catalog::{Degree, Progression, progression};
use crate::scale::DiatonicChord;
use crate::scale::diatonic_chords;
use crate::state::ChordSpec;
use crate::types::{PitchClass, ProgressionId, QualityId};

/// The triad the pinned rule falls back to for a flattened degree.
const fn major() -> QualityId {
    QualityId::from_catalog("major")
}

/// The chords of one progression in one tonic, in playing order.
///
/// The list has one entry per degree, and a progression that repeats a chord
/// repeats it here too: the catalog is data, and the caller replaces the page's
/// chords with this list rather than merging into it.
///
/// A degree whose number has no diatonic chord is skipped; the frozen catalog
/// only carries degrees 1-7 and the three scale types it uses are heptatonic, so
/// the answer always has one chord per degree — a fact `tests/progressions.rs`
/// asserts over every frozen record instead of trusting it.
pub fn progression_chords(tonic: PitchClass, id: ProgressionId) -> Vec<ChordSpec> {
    let definition: &'static Progression = progression(id);
    let diatonic = diatonic_chords(tonic, definition.scale_type, ChordMode::Triad);

    definition
        .degrees
        .iter()
        .filter_map(|degree| {
            let index = usize::from(degree.degree.checked_sub(1)?);
            let base = diatonic.get(index)?;
            Some(resolve_degree(degree, base))
        })
        .collect()
}

/// The display label of a progression, which is the catalog's own name.
///
/// `Contract.D07` ports the label verbatim, so this is a field read and never a
/// recomputation from the degrees: six of the fifty-nine labels contradict the
/// degrees they carry, and the decision is to inherit that.
pub fn progression_label(id: ProgressionId) -> &'static str {
    progression(id).name
}

/// One degree resolved against the diatonic chord of its number.
fn resolve_degree(degree: &Degree, base: &DiatonicChord) -> ChordSpec {
    ChordSpec {
        root: note_at(base.root, i32::from(degree.accidental)),
        quality: degree_quality(degree, base.quality),
    }
}

/// The quality of one resolved degree: the explicit one, the pinned altered-degree
/// rule, or the scale's own.
const fn degree_quality(degree: &Degree, diatonic_quality: QualityId) -> QualityId {
    if let Some(explicit) = degree.quality {
        return explicit;
    }
    if degree.accidental == 0 {
        return diatonic_quality;
    }
    // Altered degrees flattened by one semitone in major-flavoured contexts
    // (♭II Neapolitan, ♭III, ♭VI, ♭VII) are conventionally major triads.
    if degree.accidental == -1 && matches!(degree.degree, 2 | 3 | 6 | 7) {
        return major();
    }
    diatonic_quality
}
