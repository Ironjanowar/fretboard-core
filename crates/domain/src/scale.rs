//! The fifteen scales and their inferred diatonic chords (the pinned
//! `lib/fretboard/music/scale.ex` `scale_notes/2` and `diatonic_chords/3`).
//!
//! This file belongs to task `C15`. It answers two questions about a tonic and
//! a scale type:
//!
//! * [`scale_notes`] — the notes of the scale, as the formula's semitones above
//!   the tonic;
//! * [`diatonic_chords`] — the chord the scale builds on each of those notes.
//!
//! ## The inference is the baseline's own classifier
//!
//! The baseline does **not** stack thirds by scale index. For every degree it
//! collects the intervals the scale has above that degree and classifies them:
//!
//! * the first matching triad rule wins, in the source's order — major (`4`,
//!   `7`), minor (`3`, `7`), dim (`3`, `6`), aug (`4`, `8`), sus2 (`2`, `7`),
//!   sus4 (`5`, `7`) — and when no rule matches, the fallback answers major
//!   when the degree has a major third, minor when it has a minor third, and
//!   major when it has neither;
//! * seventh mode tries the source's own seventh rules first, in its order —
//!   dim7 (`3`, `6`, `9`), m7b5 (`3`, `6`, `10`), min7 (`3`, `7`, `10`), `7`
//!   (`4`, `7`, `10`), maj7 (`4`, `7`, `11`), min_maj7 (`3`, `7`, `11`),
//!   aug_maj7 (`4`, `8`, `11`), aug7 (`4`, `8`, `10`) — and falls back to the
//!   triad classifier when none matches.
//!
//! Two details decide answers and are therefore explicit, not incidental:
//!
//! * **The required tones are the catalog's own formulas.** Each rule's
//!   required set is [`chord_formula`] without its root, so the intervals are
//!   not retyped here; the catalog is the one place a quality's tones live.
//! * **dim7 and m7b5 are vetoed by the perfect fifth.** The source excludes
//!   semitone `7` from those two rules, so a dense scale that carries both the
//!   diminished and the perfect fifth above a degree answers min7 (or `7`),
//!   never dim7. This is why the chromatic scale answers min7 on all twelve
//!   degrees and the harmonic minor keeps dim7 where the perfect fifth is
//!   absent.
//!
//! This is a *degree classifier*, not a second recognizer: it never enumerates
//! candidates, ranks them or looks at an input set. The ordered note-set
//! recognition of `C12` keeps exactly one home ([`crate::identify_notes`]), and
//! this module reuses the catalog and the small interval-mask helpers that
//! recognizer already uses instead of growing its own copy of either.
//!
//! Fixture: every record of `fixtures/oracle/scales.jsonl` is pinned by
//! `tests/scales.rs`, including the dense and non-heptatonic scales where the
//! exclusions and the fallback decide the answer.

use crate::chord::{ChordMode, chord_formula};
use crate::identify::{formula_mask, mask_has};
use crate::note::note_at;
use crate::scale_catalog::scale_formula;
use crate::types::{PitchClass, QualityId, ScaleId};

/// The number of pitch classes: the interval wheel a degree is measured on.
const SEMITONE_COUNT: u8 = 12;

/// The major third, the fallback's first question.
const MAJOR_THIRD: u8 = 4;

/// The minor third, the fallback's second question.
const MINOR_THIRD: u8 = 3;

/// The triad rules in the source's priority order: the first rule whose
/// required tones the scale has above the degree wins.
const TRIAD_RULES: [QualityId; 6] = [
    QualityId::from_catalog("major"),
    QualityId::from_catalog("minor"),
    QualityId::from_catalog("dim"),
    QualityId::from_catalog("aug"),
    QualityId::from_catalog("sus2"),
    QualityId::from_catalog("sus4"),
];

/// The seventh rules in the source's priority order, each with the semitones
/// that veto it.
///
/// The exclusion is the source's own rule and it is load-bearing: dim7 and m7b5
/// sound a diminished fifth, and when the scale also carries the perfect fifth
/// above the same degree the baseline prefers the non-diminished reading.
const SEVENTH_RULES: [(QualityId, &[u8]); 8] = [
    (QualityId::from_catalog("dim7"), &[7]),
    (QualityId::from_catalog("m7b5"), &[7]),
    (QualityId::from_catalog("min7"), &[]),
    (QualityId::from_catalog("7"), &[]),
    (QualityId::from_catalog("maj7"), &[]),
    (QualityId::from_catalog("min_maj7"), &[]),
    (QualityId::from_catalog("aug_maj7"), &[]),
    (QualityId::from_catalog("aug7"), &[]),
];

/// One inferred diatonic chord: the degree's root note and the quality the
/// classifier chose for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiatonicChord {
    /// The chord root: the scale's own note at this degree.
    pub root: PitchClass,
    /// The inferred chord quality.
    pub quality: QualityId,
}

/// The notes of a scale, in the formula's order.
///
/// The tonic is a validated pitch class, so the answer is sharp-only like every
/// other wire surface (`CORE-D06`); a flat spelling resolves through
/// [`crate::note_index`] before it becomes a pitch class.
pub fn scale_notes(tonic: PitchClass, scale: ScaleId) -> Vec<PitchClass> {
    scale_formula(scale)
        .iter()
        .map(|semitone| note_at(tonic, i32::from(*semitone)))
        .collect()
}

/// The chord the scale builds on every one of its notes, in formula order.
///
/// The mode selects the classifier family: [`ChordMode::Triad`] answers a triad
/// quality (or the fallback) and [`ChordMode::Seventh`] tries the seventh rules
/// first. The answer is one chord per scale note, which is what makes the
/// degree's root and the scale's own note the same pitch class.
pub fn diatonic_chords(tonic: PitchClass, scale: ScaleId, mode: ChordMode) -> Vec<DiatonicChord> {
    let formula = scale_formula(scale);
    let scale_mask = formula_mask(formula);
    formula
        .iter()
        .map(|root_semitone| DiatonicChord {
            root: note_at(tonic, i32::from(*root_semitone)),
            quality: classify(degree_mask(scale_mask, *root_semitone), mode),
        })
        .collect()
}

/// The intervals the scale has above one degree, as a semitone mask.
///
/// This is the source's `rem(s - root_semitone + 12, 12)` over the scale's
/// semitones: every scale semitone is rotated so that the degree becomes `0`.
/// Both operands are bounded by the twelve pitch classes, so the `u8` sum stays
/// below `u8::MAX` and `arithmetic_side_effects` is allowed for exactly that
/// reason.
#[allow(clippy::arithmetic_side_effects)]
const fn degree_mask(scale_mask: u16, root_semitone: u8) -> u16 {
    let mut intervals = 0u16;
    let mut semitone = 0u8;
    while semitone < SEMITONE_COUNT {
        if mask_has(scale_mask, semitone) {
            let rotated = (semitone + SEMITONE_COUNT - root_semitone) % SEMITONE_COUNT;
            intervals |= 1u16 << rotated;
        }
        semitone += 1;
    }
    intervals
}

/// The quality of one degree, in the requested mode.
fn classify(intervals: u16, mode: ChordMode) -> QualityId {
    match mode {
        ChordMode::Triad => classify_triad(intervals),
        ChordMode::Seventh => classify_seventh(intervals),
    }
}

/// The first matching triad rule, or the fallback when none matches.
fn classify_triad(intervals: u16) -> QualityId {
    TRIAD_RULES
        .iter()
        .copied()
        .find(|quality| covers(*quality, intervals))
        .unwrap_or_else(|| partial(intervals))
}

/// The first matching seventh rule that no exclusion vetoes, or the triad
/// classifier when none matches.
fn classify_seventh(intervals: u16) -> QualityId {
    SEVENTH_RULES
        .iter()
        .copied()
        .find(|(quality, excluded)| covers(*quality, intervals) && !vetoed(excluded, intervals))
        .map_or_else(|| classify_triad(intervals), |(quality, _)| quality)
}

/// Whether the scale carries every tone of a quality above one degree.
///
/// The required tones are the catalog's own formula without its root, so the
/// classifier and the chord catalog cannot drift apart.
fn covers(quality: QualityId, intervals: u16) -> bool {
    chord_formula(quality)
        .iter()
        .filter(|semitone| **semitone != 0)
        .all(|semitone| mask_has(intervals, *semitone))
}

/// Whether the scale carries one of the semitones that veto a rule.
fn vetoed(excluded: &[u8], intervals: u16) -> bool {
    excluded
        .iter()
        .any(|semitone| mask_has(intervals, *semitone))
}

/// The fallback of the source's triad classifier: the major third answers
/// major, otherwise the minor third answers minor, otherwise major.
const fn partial(intervals: u16) -> QualityId {
    if mask_has(intervals, MAJOR_THIRD) || !mask_has(intervals, MINOR_THIRD) {
        QualityId::from_catalog("major")
    } else {
        QualityId::from_catalog("minor")
    }
}
