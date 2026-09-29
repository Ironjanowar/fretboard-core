//! Full ordered identification of a note set (`02-core-contract.md` section 3;
//! the pinned `lib/fretboard/music/chord.ex` `identify/1` and `identify/2`).
//!
//! Given a set of pitch classes, [`identify_notes`] answers with every chord the
//! frozen baseline recognizes, in the baseline's order, and
//! [`identify_notes_with_bass`] additionally annotates each interpretation with
//! its inversion and slash label. The whole ordered list is the answer: a caller
//! never sees a set or a first match.
//!
//! ## The rule
//!
//! The input's distinct pitch classes are seen from each of the twelve roots in
//! chromatic order, and every one of the 47 catalog qualities is tried. The
//! formula is the catalog's own ([`chord_formula`], `C06`), so nothing is
//! retyped here. A candidate is classified exactly as the baseline classifies
//! it:
//!
//! * **exact** — the input set equals the formula's interval set;
//! * **incomplete** — the input is a subset of the formula and at most two
//!   formula tones are missing;
//! * **partial** — at least three formula tones are covered and the formula is
//!   shorter than the input. The baseline's prose calls a partial a "strict
//!   subset", but the code requires only the covered count
//!   (`Contract.D02`, still open); this port implements the code, which is what
//!   the fixture carries.
//!
//! Candidates are ordered by the baseline's own sort key ([`MatchSortKey`]):
//! exact first, then incomplete by missing count, missing-priority sum and root,
//! then partial by coverage descending and root. The sort is explicit — never a
//! `HashMap`'s iteration order.
//!
//! ## The tie-break (`Contract.D03`)
//!
//! The baseline's `Enum.sort_by` is stable, so candidates that compare equal on
//! the sort key inherit `Map.to_list(@formulas)`'s enumeration order — an
//! Elixir map order that follows the VM's atom table rather than the source
//! text (the measurement is in `docs/decisions.md`). The approved native rule
//! keeps those keys unchanged and breaks a tie by the **frozen catalog
//! identifier order** — the position of the quality in [`QualityId::ALL`],
//! which is `QUALITY_IDS` in `types.rs` — and then by the frozen pitch-class
//! order. In one line: *the baseline's keys, then the catalog identifier order,
//! then the pitch-class order*. The sort key already carries the root index, so
//! the pitch-class term never decides anything in practice; it is stated for
//! completeness. `tests/identify.rs` compares the full ordered result of all
//! 15,693 fixture records and accepts a difference only inside a group of equal
//! sort keys, printing how many records that affects.
//!
//! ## Notes and intervals stay separate (`Contract.D01`)
//!
//! An [`Interpretation`] carries the formula-order notes and the independently
//! ordered interval labels without zipping them, exactly as
//! [`crate::ChordDetails`] does. A consumer that zips the two reproduces the
//! baseline's raw pairs.
//!
//! Flat note spellings never reach this surface: like every wire surface
//! (`CORE-D06`), it takes validated pitch classes, and
//! [`crate::note_index`] is the lookup that widens a *name* (`Db` to `C#`)
//! before it becomes a [`PitchClass`].

use crate::chord::{
    chord_formula, chord_interval_labels, chord_quality_label, contextual_interval_label,
};
use crate::note::note_at;
use crate::types::{PitchClass, QualityId};

/// The fewest distinct pitch classes an identification needs: anything smaller
/// is an empty answer, exactly like the baseline's `length(notes) < 3` guard.
const MINIMUM_INPUT: i32 = 3;

/// The most tones an incomplete match may be missing.
const MAXIMUM_INCOMPLETE_GAP: i32 = 2;

/// The fewest formula tones a partial match must cover.
const MINIMUM_PARTIAL_COVERAGE: i32 = 3;

/// The priority of a missing semitone, lower first: omitting the fifth is
/// extremely common in guitar voicings, omitting the third or seventh less so.
///
/// This is the baseline's `@missing_interval_priority`, indexed by semitone.
/// The baseline's map has no entry for semitone 0 (a missing root), so index 0
/// carries its `|| 9` default; [`DEFAULT_MISSING_PRIORITY`] is that same value
/// for a lookup that cannot miss.
const MISSING_PRIORITY: [u8; 12] = [9, 5, 2, 3, 3, 1, 5, 0, 6, 2, 4, 4];

/// The priority of a semitone the priority table does not name.
const DEFAULT_MISSING_PRIORITY: u8 = 9;

/// The number of pitch classes: the roots the identification iterates.
const ROOT_COUNT: u8 = 12;

/// Which of the baseline's three match classes an interpretation belongs to.
///
/// The variants are declared in the order the baseline's sort keys order them,
/// so the derived `Ord` is the baseline's first sort component: [`Self::Exact`]
/// before [`Self::Incomplete`] before [`Self::Partial`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MatchKind {
    /// The input set equals the formula's interval set.
    Exact,
    /// The input is a subset of the formula, missing at most two tones.
    Incomplete,
    /// The formula is shorter than the input and at least three tones are
    /// covered.
    Partial,
}

/// The baseline's own sort key for one candidate interpretation.
///
/// Ordering these keys reproduces the baseline's `Enum.sort_by` exactly: the
/// [`MatchKind`] orders the three classes, the secondary component is the
/// missing count (incomplete) or the negated coverage (partial), and the
/// tertiary is the missing-priority sum (incomplete only). Two candidates whose
/// keys compare equal are a *tie group*: their relative order is the baseline's
/// map enumeration order, which is not reproducible, so the native order breaks
/// it by the frozen catalog identifier order (see the module documentation).
///
/// [`match_sort_key`] is the public way to ask the baseline's key for a root
/// and quality without building the interpretation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MatchSortKey {
    /// The match class, which orders first.
    pub kind: MatchKind,
    /// The second component: `0` for exact, the missing-note count for
    /// incomplete, the negated coverage for partial.
    pub secondary: i32,
    /// The third component: `0` for exact and partial, the missing-priority sum
    /// for incomplete.
    pub tertiary: i32,
    /// The root's position in the chromatic scale, `0..=11`.
    pub root_index: usize,
    /// The number of semitones in the catalog formula.
    pub formula_len: usize,
}

/// One identification: the chord identity, the notes it names, what the input
/// is missing, and the bass annotation of a bass-aware call.
///
/// The notes are in formula order and the interval labels in their own order,
/// deliberately not zipped (`Contract.D01`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interpretation {
    /// The chord root.
    pub root: PitchClass,
    /// The chord quality identifier.
    pub quality: QualityId,
    /// Whether the input set equals the formula's interval set.
    pub exact: bool,
    /// Whether the input is a subset of the formula missing at most two tones.
    pub incomplete: bool,
    /// The chord members as pitch classes, in formula order.
    pub notes: Vec<PitchClass>,
    /// The contextual role of each member, in label order.
    pub intervals: Vec<&'static str>,
    /// The contextual labels of the formula tones the input lacks, in formula
    /// order; empty unless the match is incomplete.
    pub missing_intervals: Vec<&'static str>,
    /// The bass note of a bass-aware call, `None` otherwise.
    pub bass: Option<PitchClass>,
    /// The inversion `0..=6`, or `None` when the bass is not a chord tone the
    /// baseline maps to an inversion.
    pub inversion: Option<u8>,
    /// The slash label of a bass-aware call: the plain label for root position
    /// or a non-chord bass, and the label with `/{bass}` appended otherwise.
    pub slash_label: Option<String>,
}

/// One candidate before it reaches the caller: its sort key, the catalog
/// position of its quality, its root index and the interpretation itself.
struct Candidate {
    /// The baseline sort key.
    key: MatchSortKey,
    /// The position of the quality in [`QualityId::ALL`]: the tie-break's first
    /// term.
    quality_index: usize,
    /// The root's position in the chromatic scale: the tie-break's second term.
    root_index: usize,
    /// The interpretation, without any bass annotation yet.
    interpretation: Interpretation,
}

/// The classification of one root and quality against an input, with the
/// information the sort key and the interpretation both need.
struct Match {
    /// The baseline sort key of the match.
    key: MatchSortKey,
    /// The contextual labels of the missing formula tones, in formula order.
    missing_intervals: Vec<&'static str>,
}

/// Every chord the baseline recognizes for a set of pitch classes, in the
/// baseline's order and with the approved tie-break.
///
/// Fewer than three distinct pitch classes identify nothing.
pub fn identify_notes(notes: &[PitchClass]) -> Vec<Interpretation> {
    identify(notes, None)
}

/// Every chord the baseline recognizes for a set of pitch classes and a bass
/// note, in the baseline's order and with the approved tie-break.
///
/// Each interpretation carries the bass, its inversion and its slash label.
/// Fewer than three distinct pitch classes identify nothing, exactly as the
/// one-argument call does.
pub fn identify_notes_with_bass(notes: &[PitchClass], bass: PitchClass) -> Vec<Interpretation> {
    identify(notes, Some(bass))
}

/// The baseline's own sort key for one root and quality against a note set, or
/// `None` when that root and quality is not a match.
///
/// This is the public form of the sort key every [`Interpretation`] is ordered
/// by; the parity tests use it to detect the groups where the baseline's keys
/// cannot distinguish two interpretations.
pub fn match_sort_key(
    notes: &[PitchClass],
    root: PitchClass,
    quality: QualityId,
) -> Option<MatchSortKey> {
    let input_mask = pitch_class_mask(notes);
    let input_size = as_i32(input_mask.count_ones());
    if input_size < MINIMUM_INPUT {
        return None;
    }
    let intervals_mask = interval_mask(notes, root);
    classify_match(root, quality, input_size, intervals_mask).map(|found| found.key)
}

/// The identification of a note set, optionally seen from a bass note.
fn identify(notes: &[PitchClass], bass: Option<PitchClass>) -> Vec<Interpretation> {
    candidates(notes)
        .into_iter()
        .map(|candidate| annotate(candidate.interpretation, bass))
        .collect()
}

/// Every candidate of a note set, sorted by the baseline's key and then by the
/// approved tie-break: the frozen catalog identifier order, then the frozen
/// pitch-class order.
///
/// The sort is explicit; no map iteration order reaches the result.
fn candidates(notes: &[PitchClass]) -> Vec<Candidate> {
    let input_mask = pitch_class_mask(notes);
    let input_size = as_i32(input_mask.count_ones());
    if input_size < MINIMUM_INPUT {
        return Vec::new();
    }

    let mut found: Vec<Candidate> = Vec::new();
    for root_value in 0..ROOT_COUNT {
        // The newtype's invariant is exactly `0..12`, so this cannot fail; the
        // `continue` keeps the loop total without a panic.
        let Ok(root) = PitchClass::try_from(root_value) else {
            continue;
        };
        let root_index = usize::from(root_value);
        let intervals_mask = interval_mask(notes, root);
        for (quality_index, quality) in QualityId::ALL.iter().copied().enumerate() {
            let Some(found_match) = classify_match(root, quality, input_size, intervals_mask)
            else {
                continue;
            };
            let kind = found_match.key.kind;
            found.push(Candidate {
                key: found_match.key,
                quality_index,
                root_index,
                interpretation: Interpretation {
                    root,
                    quality,
                    exact: kind == MatchKind::Exact,
                    incomplete: kind == MatchKind::Incomplete,
                    notes: chord_formula(quality)
                        .iter()
                        .map(|semitone| note_at(root, i32::from(*semitone)))
                        .collect(),
                    intervals: chord_interval_labels(quality),
                    missing_intervals: found_match.missing_intervals,
                    bass: None,
                    inversion: None,
                    slash_label: None,
                },
            });
        }
    }

    found.sort_by_key(|candidate| (candidate.key, candidate.quality_index, candidate.root_index));
    found
}

/// Classify one root and quality against the input and build its key.
///
/// `None` means the root and quality is not a match at all.
fn classify_match(
    root: PitchClass,
    quality: QualityId,
    input_size: i32,
    intervals_mask: u16,
) -> Option<Match> {
    let formula = chord_formula(quality);
    let formula_mask = formula_mask(formula);
    let formula_size = as_i32(formula_mask.count_ones());
    let coverage = as_i32((formula_mask & intervals_mask).count_ones());
    let kind = classify(input_size, formula_size, coverage)?;

    let (missing_intervals, secondary, tertiary) = match kind {
        MatchKind::Exact => (Vec::new(), 0, 0),
        MatchKind::Incomplete => {
            let (labels, count, priority) = missing_of(formula, intervals_mask);
            (labels, count, priority)
        }
        // The coverage is a `0..=12` count, so the negation cannot overflow.
        MatchKind::Partial => (Vec::new(), coverage.checked_neg().unwrap_or(coverage), 0),
    };

    Some(Match {
        key: MatchSortKey {
            kind,
            secondary,
            tertiary,
            root_index: usize::from(u8::from(root)),
            formula_len: formula.len(),
        },
        missing_intervals,
    })
}

/// The baseline's classification of one candidate.
///
/// The `formula_size > input_size` and `formula_size < input_size` comparisons
/// are the baseline's own; the subtraction is bounded by the seven-tone formula
/// and the twelve pitch classes, so `arithmetic_side_effects` is allowed for
/// exactly that reason.
#[allow(clippy::arithmetic_side_effects)]
const fn classify(input_size: i32, formula_size: i32, coverage: i32) -> Option<MatchKind> {
    if coverage == input_size && formula_size == input_size {
        Some(MatchKind::Exact)
    } else if coverage == input_size
        && formula_size > input_size
        && formula_size - input_size <= MAXIMUM_INCOMPLETE_GAP
    {
        Some(MatchKind::Incomplete)
    } else if coverage >= MINIMUM_PARTIAL_COVERAGE && formula_size < input_size {
        Some(MatchKind::Partial)
    } else {
        None
    }
}

/// The contextual labels of the formula tones the input lacks, in formula
/// order, with their count and their priority sum.
///
/// The count is a `0..=7` value, so the increment cannot overflow.
#[allow(clippy::arithmetic_side_effects)]
fn missing_of(formula: &[u8], intervals_mask: u16) -> (Vec<&'static str>, i32, i32) {
    let mut labels = Vec::new();
    let mut count = 0i32;
    let mut priority = 0i32;
    for semitone in formula {
        if !mask_has(intervals_mask, *semitone) {
            labels.push(contextual_interval_label(*semitone, formula));
            count += 1;
            priority += i32::from(priority_of(*semitone));
        }
    }
    (labels, count, priority)
}

/// The sort priority of a formula semitone.
fn priority_of(semitone: u8) -> u8 {
    MISSING_PRIORITY
        .get(usize::from(semitone))
        .copied()
        .unwrap_or(DEFAULT_MISSING_PRIORITY)
}

/// Annotate one interpretation with a bass note, its inversion and its slash
/// label.
fn annotate(interpretation: Interpretation, bass: Option<PitchClass>) -> Interpretation {
    let Some(bass) = bass else {
        return interpretation;
    };
    let bass_interval = relative_semitone(bass, interpretation.root);
    let formula = chord_formula(interpretation.quality);
    let inversion = if formula.contains(&bass_interval) {
        inversion_for_interval(bass_interval)
    } else {
        None
    };
    let label = format!(
        "{}{}",
        interpretation.root.name(),
        chord_quality_label(interpretation.quality)
    );
    let slash_label = match inversion {
        Some(0) | None => label,
        Some(_) => format!("{label}/{}", bass.name()),
    };
    Interpretation {
        bass: Some(bass),
        inversion,
        slash_label: Some(slash_label),
        ..interpretation
    }
}

/// The inversion number of a bass interval that is a chord tone.
///
/// Ported from the pinned `bass_inversion/1` and `inversion_for_interval/1`:
/// the root is position `0`, the third and the augmented third/fourth are `1`,
/// the fifth and augmented fifth are `2`, the seventh is `3`, and a ninth,
/// eleventh or thirteenth in the bass is `4` to `6`. The tritone (`6`) and the
/// minor second (`1`) are chord tones of the diminished and altered qualities
/// but are deliberately not mapped — the surprising diminished/altered
/// inversion behavior of `Contract.D04`, which is the baseline's behavior and
/// what the fixture carries.
const fn inversion_for_interval(bass_interval: u8) -> Option<u8> {
    match bass_interval {
        0 => Some(0),
        3 | 4 => Some(1),
        7 | 8 => Some(2),
        10 | 11 => Some(3),
        2 => Some(4),
        5 => Some(5),
        9 => Some(6),
        _ => None,
    }
}

/// Set the bits of the distinct pitch classes in the input.
///
/// The shift amount is a `PitchClass`, whose invariant is `0..=11`, so the
/// `u16` result cannot overflow and `arithmetic_side_effects` is allowed for
/// exactly that reason.
#[allow(clippy::arithmetic_side_effects)]
fn pitch_class_mask(notes: &[PitchClass]) -> u16 {
    notes
        .iter()
        .map(|note| 1u16 << u8::from(*note))
        .fold(0u16, |mask, bit| mask | bit)
}

/// Set the bits of the distinct semitones above one root.
#[allow(clippy::arithmetic_side_effects)]
fn interval_mask(notes: &[PitchClass], root: PitchClass) -> u16 {
    notes
        .iter()
        .map(|note| relative_semitone(*note, root))
        .map(|semitone| 1u16 << semitone)
        .fold(0u16, |mask, bit| mask | bit)
}

/// Set the bits of a formula's semitones.
#[allow(clippy::arithmetic_side_effects)]
fn formula_mask(formula: &[u8]) -> u16 {
    formula
        .iter()
        .map(|semitone| 1u16 << *semitone)
        .fold(0u16, |mask, bit| mask | bit)
}

/// Whether a semitone mask carries one semitone.
#[allow(clippy::arithmetic_side_effects)]
const fn mask_has(mask: u16, semitone: u8) -> bool {
    mask & (1u16 << semitone) != 0
}

/// The semitone from one pitch class up to another, `0..=11`.
///
/// Both values are `0..=11`, so the intermediate sum stays below `u8::MAX` and
/// the subtraction cannot go negative; `arithmetic_side_effects` is allowed for
/// exactly that reason.
#[allow(clippy::arithmetic_side_effects)]
fn relative_semitone(note: PitchClass, root: PitchClass) -> u8 {
    (u8::from(note) + 12 - u8::from(root)) % 12
}

/// A small unsigned count as `i32`.
///
/// Every count here is bounded by the twelve pitch classes, so the conversion
/// cannot fail; the saturating fallback keeps the function total without a
/// panic.
fn as_i32(value: u32) -> i32 {
    i32::try_from(value).unwrap_or(i32::MAX)
}
