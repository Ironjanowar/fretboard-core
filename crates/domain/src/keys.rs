//! Single-key suggestion scoring (task `C16`; the pinned
//! `lib/fretboard/music/scale.ex` `suggest_keys/1`).
//!
//! This module answers one question: *which keys contain every chord of the
//! input, and how many of the input chords does each key's own diatonic triad
//! agree with?*
//!
//! Three details decide the answer and are therefore explicit, not incidental:
//!
//! * **The triad base is a hand-written table, not a third derived from the
//!   chord.** `maj9` and `13` both score as `major`, `7#9`, `7b9`, `13b9` and
//!   `7b13` as `major`, `9b5`, `7b5`, `m11b5`, `dim_maj7` and `dim7b13` as
//!   `dim`. [`TRIAD_BASE`] is that table and [`triad_base`] its lookup.
//! * **Containment resolves a flat root; the score does not.** The baseline
//!   builds a chord's notes through `Note.note_index/1`, so a flat-rooted chord
//!   passes containment, but it scores by comparing the input's *raw root
//!   string* against a map whose keys are sharp names, so it can never match.
//!   That asymmetry is `Contract.D05`, approved as a deviation (option A):
//!   the native root is a [`PitchClass`] with no spelling, so the one affected
//!   record (`suggest_keys/flat-root-eb-major`) answers the score of its sharp
//!   spelling. Membership and order are unaffected, and no client can reach the
//!   case — the URL codec rejects non-sharp roots and every wire surface is
//!   sharp-only (`CORE-D06`). `tests/keys.rs` pins the deviation's size.
//! * **The empty input is answered, not gated.** With no chord at all,
//!   containment is vacuously true for every candidate, so the raw operation
//!   answers all 168 candidates at `0:0`. The page's two-chord gate lives here
//!   in [`page_key_suggestions`] so that no client re-implements it.
//!
//! Fixture: every record of `fixtures/oracle/keys.jsonl` is pinned by
//! `tests/keys.rs`, including the three records that answer nothing, the tied
//! score groups and the two flat-rooted records.

use std::cmp::Ordering;
use std::collections::BTreeSet;

use crate::chord::{ChordMode, chord_details, str_eq};
use crate::scale::{DiatonicChord, diatonic_chords, scale_notes};
use crate::state::ChordSpec;
use crate::types::{PITCH_CLASS_COUNT, PitchClass, QualityId, ScaleId};

/// The triad-base map of the pinned `@quality_to_triad`, in one table: every one
/// of the 47 catalog qualities and the triad quality it is scored as.
///
/// The order is the source table's own (alphabetical by identifier) and the
/// coverage is proved at compile time below, so the lookup in [`triad_base`]
/// cannot miss.
const TRIAD_BASE: [(&str, &str); 47] = [
    ("11", "major"),
    ("13", "major"),
    ("13b9", "major"),
    ("7", "major"),
    ("7#11", "major"),
    ("7#9", "major"),
    ("7b13", "major"),
    ("7b5", "dim"),
    ("7b9", "major"),
    ("7b9b13", "major"),
    ("7sus4", "sus4"),
    ("9", "major"),
    ("9#5", "aug"),
    ("9b5", "dim"),
    ("add9", "major"),
    ("aug", "aug"),
    ("aug7", "aug"),
    ("aug_maj7", "aug"),
    ("dim", "dim"),
    ("dim7", "dim"),
    ("dim7b13", "dim"),
    ("dim_maj7", "dim"),
    ("m11b5", "dim"),
    ("m7b5", "dim"),
    ("m_add9", "minor"),
    ("maj11", "major"),
    ("maj13", "major"),
    ("maj6", "major"),
    ("maj6_9", "major"),
    ("maj7", "major"),
    ("maj7#11", "major"),
    ("maj9", "major"),
    ("major", "major"),
    ("min11", "minor"),
    ("min13", "minor"),
    ("min6", "minor"),
    ("min6_9", "minor"),
    ("min7", "minor"),
    ("min7b13", "minor"),
    ("min9", "minor"),
    ("min_maj7", "minor"),
    ("minor", "minor"),
    ("sus13", "sus4"),
    ("sus2", "sus2"),
    ("sus4", "sus4"),
    ("sus9", "sus2"),
    ("susb9", "sus2"),
];

/// The map carries exactly one entry per catalog quality.
///
/// This is what makes the lookup in [`triad_base`] total: a quality the table
/// misses — or a table entry the catalog does not have — fails the build instead
/// of silently scoring a chord as another quality's triad.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn assert_triad_base_covers_the_catalog() {
    let ids = QualityId::ALL;
    assert!(
        TRIAD_BASE.len() == ids.len(),
        "the triad-base map and the catalog differ in length"
    );
    let mut index = 0;
    while index < ids.len() {
        let id = ids[index].as_str();
        let mut found = 0;
        let mut entry = 0;
        while entry < TRIAD_BASE.len() {
            if str_eq(TRIAD_BASE[entry].0, id) {
                found += 1;
            }
            entry += 1;
        }
        assert!(
            found == 1,
            "the triad-base map does not carry a catalog quality exactly once"
        );
        index += 1;
    }
}

/// Compile-time proof of the invariant [`triad_base`] relies on.
const _: () = assert_triad_base_covers_the_catalog();

/// The triad base of one chord quality: the triad the pinned table scores it as.
///
/// `QualityId` accepts exactly the 47 frozen catalog identifiers and
/// [`TRIAD_BASE`] carries exactly one entry per identifier — proved at compile
/// time — so the lookup cannot miss. The first entry is returned only to keep
/// the function total.
#[allow(clippy::indexing_slicing)]
pub fn triad_base(quality: QualityId) -> QualityId {
    let base = TRIAD_BASE
        .iter()
        .find(|(id, _base)| *id == quality.as_str())
        .map_or(TRIAD_BASE[0].1, |(_id, base)| *base);
    QualityId::from_catalog(base)
}

/// One candidate key and how well it explains the input chords.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySuggestion {
    /// The key's tonic.
    pub tonic: PitchClass,
    /// The key's scale type.
    pub scale_type: ScaleId,
    /// How many input occurrences the key's diatonic triads explain.
    pub score: usize,
    /// How many chords the input carried, occurrences included.
    pub total: usize,
    /// The key's diatonic triads, in scale order: the chords the score was
    /// computed against, returned with the suggestion so a client renders what
    /// the engine scored instead of re-deriving it.
    pub diatonic_chords: Vec<DiatonicChord>,
}

/// The page's own gate: the pinned LiveView computes suggestions only from two
/// active chords.
///
/// The raw operation has no such gate (see the module documentation); this
/// constant states the page's rule once, in the domain, so a client branches on
/// the value instead of carrying its own copy.
pub const KEY_PAGE_MIN_CHORDS: usize = 2;

/// Every candidate key: each of the twelve tonics times each of the fourteen
/// scale types, in the frozen catalog order — the chromatic scale excluded,
/// exactly as the baseline deletes it.
///
/// The order is load-bearing for `C17`'s greedy tie-break, which takes the
/// *first* of two equally good candidates, so this one enumeration is shared
/// rather than rebuilt.
pub(crate) fn candidate_keys() -> Vec<(PitchClass, ScaleId)> {
    let mut keys = Vec::new();
    // Bounded by the twelve pitch classes, so the conversion cannot fail; the
    // filter keeps the function total instead of unwrapping.
    for tonic in (0..PITCH_CLASS_COUNT).filter_map(|index| PitchClass::try_from(index).ok()) {
        for scale_type in ScaleId::ALL {
            if scale_type.as_str() == "chromatic" {
                continue;
            }
            keys.push((tonic, scale_type));
        }
    }
    keys
}

/// The pitch classes one chord names, as the domain spells them.
///
/// A catalog quality always resolves, so the empty fallback is unreachable; it
/// keeps the function total instead of panicking on a value the public API
/// cannot have built.
pub(crate) fn chord_notes(chord: &ChordSpec) -> Vec<PitchClass> {
    chord_details(chord).map_or_else(|_error| Vec::new(), |details| details.notes)
}

/// Whether a key's diatonic triads explain one chord: the key builds the chord's
/// triad base on the chord's own root.
pub(crate) fn explains(diatonic: &[DiatonicChord], chord: &ChordSpec) -> bool {
    diatonic
        .iter()
        .any(|degree| degree.root == chord.root && degree.quality == triad_base(chord.quality))
}

/// The candidate keys that contain all the notes of every input chord.
///
/// This is the baseline's `valid_candidate?/3`: a candidate whose scale notes are
/// a superset of every chord's note set. Both operands are resolved pitch
/// classes, so a flat-rooted input passes exactly as the baseline's
/// `Chord.notes/2` made it pass.
fn contains_every_chord(scale: &BTreeSet<PitchClass>, chords: &[ChordSpec]) -> bool {
    chords
        .iter()
        .all(|chord| chord_notes(chord).iter().all(|note| scale.contains(note)))
}

/// The baseline's order: score descending, then the tonic name and the scale
/// type lexically, both ascending.
fn compare(left: &KeySuggestion, right: &KeySuggestion) -> Ordering {
    right
        .score
        .cmp(&left.score)
        .then_with(|| left.tonic.name().cmp(right.tonic.name()))
        .then_with(|| left.scale_type.as_str().cmp(right.scale_type.as_str()))
}

/// Suggests the keys that contain every input chord, ordered by how many of
/// them their diatonic triads explain.
///
/// The input is a list of occurrences, so a chord repeated twice is contained,
/// scored and counted twice. A key that contains no chord is never suggested,
/// and an input with no chord at all is contained by every candidate — that is
/// the raw answer the page's two-chord gate ([`page_key_suggestions`]) exists to
/// keep away from the panel.
pub fn suggest_keys(chords: &[ChordSpec]) -> Vec<KeySuggestion> {
    let mut suggestions: Vec<KeySuggestion> = Vec::new();
    for (tonic, scale_type) in candidate_keys() {
        let scale: BTreeSet<PitchClass> = scale_notes(tonic, scale_type).into_iter().collect();
        if !contains_every_chord(&scale, chords) {
            continue;
        }
        let triads = diatonic_chords(tonic, scale_type, ChordMode::Triad);
        let score = chords
            .iter()
            .filter(|chord| explains(&triads, chord))
            .count();
        suggestions.push(KeySuggestion {
            tonic,
            scale_type,
            score,
            total: chords.len(),
            diatonic_chords: triads,
        });
    }
    suggestions.sort_by(compare);
    suggestions
}

/// [`suggest_keys`] behind the page's two-chord gate.
///
/// The pinned LiveView computes key suggestions only when at least two chords
/// are active and answers nothing below that; the check lives here so the rule
/// has one home.
pub fn page_key_suggestions(chords: &[ChordSpec]) -> Vec<KeySuggestion> {
    if chords.len() < KEY_PAGE_MIN_CHORDS {
        Vec::new()
    } else {
        suggest_keys(chords)
    }
}
