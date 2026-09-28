//! Chord details and the complete chord-quality catalog
//! (`02-core-contract.md` section 3, and the pinned
//! `lib/fretboard/music/chord.ex`).
//!
//! This file belongs to task `C06`. P1 shipped exactly one quality — `major`,
//! through the documented `C03` handoff — and reported every other catalog
//! quality as an explicit pending capability. C06 replaces that single-entry
//! table with the whole frozen catalog: all 47 qualities answer with their own
//! formula, display suffix and contextual interval labels, and nothing is
//! pending any more.
//!
//! The table below is the pinned source's own `@formulas`/`@labels` maps, in the
//! frozen identifier order (`QualityId::ALL`); no formula is transcribed from
//! the plan. `tests/chord_catalog.rs` pins every entry, every root output and
//! every group against the frozen oracle (`fixtures/oracle/catalogs.json` and
//! `fixtures/oracle/chords.jsonl`), record by record.
//!
//! The notes are computed with the real note math ([`note_at`]); no fixture
//! answer is stored. The interval labels are the chord-local vocabulary:
//! semitone 0 is the chord's `Root`, the basic tones keep the simple interval
//! names, an extension is named as a compound interval only when the formula
//! actually carries a seventh, and only then are the labels sorted into
//! chord-member order (root, 3rd, 5th, 7th, 9th, 11th, 13th).
//!
//! `Contract.D01` — the baseline zips formula-order notes with that
//! independently ordered label list — is deliberately *not* resolved here:
//! [`ChordDetails`] exposes the two orders separately, and a consumer that zips
//! them reproduces the baseline's raw pairs exactly.

use crate::error::CoreError;
use crate::interval::interval_name;
use crate::note::note_at;
use crate::state::ChordSpec;
use crate::types::{PitchClass, QualityId};

/// One chord quality of the frozen catalog: its stable identifier, its
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

/// The 47 chord qualities of the frozen catalog, in the frozen identifier
/// order.
///
/// The order is the same one [`QualityId::ALL`] enumerates, so the catalog
/// carries exactly one entry per accepted identifier.
const CATALOG: [Quality; 47] = [
    Quality {
        id: "11",
        suffix: "11",
        formula: &[0, 2, 4, 5, 7, 10],
    },
    Quality {
        id: "13",
        suffix: "13",
        formula: &[0, 2, 4, 5, 7, 9, 10],
    },
    Quality {
        id: "13b9",
        suffix: "13b9",
        formula: &[0, 1, 4, 5, 7, 9, 10],
    },
    Quality {
        id: "7",
        suffix: "7",
        formula: &[0, 4, 7, 10],
    },
    Quality {
        id: "7#11",
        suffix: "7#11",
        formula: &[0, 4, 6, 7, 10],
    },
    Quality {
        id: "7#9",
        suffix: "7#9",
        formula: &[0, 3, 4, 7, 10],
    },
    Quality {
        id: "7b13",
        suffix: "7b13",
        formula: &[0, 4, 7, 8, 10],
    },
    Quality {
        id: "7b5",
        suffix: "7b5",
        formula: &[0, 4, 6, 10],
    },
    Quality {
        id: "7b9",
        suffix: "7b9",
        formula: &[0, 1, 4, 7, 10],
    },
    Quality {
        id: "7b9b13",
        suffix: "7b9b13",
        formula: &[0, 1, 4, 7, 8, 10],
    },
    Quality {
        id: "7sus4",
        suffix: "7sus",
        formula: &[0, 5, 7, 10],
    },
    Quality {
        id: "9",
        suffix: "9",
        formula: &[0, 2, 4, 7, 10],
    },
    Quality {
        id: "9#5",
        suffix: "9#5",
        formula: &[0, 2, 4, 8, 10],
    },
    Quality {
        id: "9b5",
        suffix: "9b5",
        formula: &[0, 2, 4, 6, 10],
    },
    Quality {
        id: "add9",
        suffix: "add9",
        formula: &[0, 2, 4, 7],
    },
    Quality {
        id: "aug",
        suffix: "aug",
        formula: &[0, 4, 8],
    },
    Quality {
        id: "aug7",
        suffix: "aug7",
        formula: &[0, 4, 8, 10],
    },
    Quality {
        id: "aug_maj7",
        suffix: "augMaj7",
        formula: &[0, 4, 8, 11],
    },
    Quality {
        id: "dim",
        suffix: "dim",
        formula: &[0, 3, 6],
    },
    Quality {
        id: "dim7",
        suffix: "dim7",
        formula: &[0, 3, 6, 9],
    },
    Quality {
        id: "dim7b13",
        suffix: "dim7b13",
        formula: &[0, 3, 6, 8, 9],
    },
    Quality {
        id: "dim_maj7",
        suffix: "dimMaj7",
        formula: &[0, 3, 6, 11],
    },
    Quality {
        id: "m11b5",
        suffix: "m11b5",
        formula: &[0, 2, 3, 5, 6, 10],
    },
    Quality {
        id: "m7b5",
        suffix: "m7b5",
        formula: &[0, 3, 6, 10],
    },
    Quality {
        id: "m_add9",
        suffix: "madd9",
        formula: &[0, 2, 3, 7],
    },
    Quality {
        id: "maj11",
        suffix: "maj11",
        formula: &[0, 2, 4, 5, 7, 11],
    },
    Quality {
        id: "maj13",
        suffix: "maj13",
        formula: &[0, 2, 4, 5, 7, 9, 11],
    },
    Quality {
        id: "maj6",
        suffix: "6",
        formula: &[0, 4, 7, 9],
    },
    Quality {
        id: "maj6_9",
        suffix: "6/9",
        formula: &[0, 2, 4, 7, 9],
    },
    Quality {
        id: "maj7",
        suffix: "maj7",
        formula: &[0, 4, 7, 11],
    },
    Quality {
        id: "maj7#11",
        suffix: "maj7#11",
        formula: &[0, 4, 6, 7, 11],
    },
    Quality {
        id: "maj9",
        suffix: "maj9",
        formula: &[0, 2, 4, 7, 11],
    },
    Quality {
        id: "major",
        suffix: "maj",
        formula: &[0, 4, 7],
    },
    Quality {
        id: "min11",
        suffix: "m11",
        formula: &[0, 2, 3, 5, 7, 10],
    },
    Quality {
        id: "min13",
        suffix: "m13",
        formula: &[0, 2, 3, 5, 7, 9, 10],
    },
    Quality {
        id: "min6",
        suffix: "m6",
        formula: &[0, 3, 7, 9],
    },
    Quality {
        id: "min6_9",
        suffix: "m6/9",
        formula: &[0, 2, 3, 7, 9],
    },
    Quality {
        id: "min7",
        suffix: "min7",
        formula: &[0, 3, 7, 10],
    },
    Quality {
        id: "min7b13",
        suffix: "m7b13",
        formula: &[0, 3, 7, 8, 10],
    },
    Quality {
        id: "min9",
        suffix: "m9",
        formula: &[0, 2, 3, 7, 10],
    },
    Quality {
        id: "min_maj7",
        suffix: "mMaj7",
        formula: &[0, 3, 7, 11],
    },
    Quality {
        id: "minor",
        suffix: "min",
        formula: &[0, 3, 7],
    },
    Quality {
        id: "sus13",
        suffix: "sus13",
        formula: &[0, 5, 7, 9, 10],
    },
    Quality {
        id: "sus2",
        suffix: "sus2",
        formula: &[0, 2, 7],
    },
    Quality {
        id: "sus4",
        suffix: "sus4",
        formula: &[0, 5, 7],
    },
    Quality {
        id: "sus9",
        suffix: "sus9",
        formula: &[0, 2, 5, 7, 10],
    },
    Quality {
        id: "susb9",
        suffix: "susb9",
        formula: &[0, 1, 5, 7, 10],
    },
];

/// The contextual label of a chord's root member.
const ROOT_LABEL: &str = "Root";

/// The compound name of a flattened ninth.
const FLAT_NINTH: &str = "Flat 9th";

/// The compound name of a major ninth.
const MAJOR_NINTH: &str = "Major 9th";

/// The compound name of a sharpened ninth.
const SHARP_NINTH: &str = "Sharp 9th";

/// The compound name of a perfect eleventh.
const PERFECT_ELEVENTH: &str = "Perfect 11th";

/// The compound name of an augmented eleventh.
const AUGMENTED_ELEVENTH: &str = "Augmented 11th";

/// The compound name of a minor thirteenth.
const MINOR_THIRTEENTH: &str = "Minor 13th";

/// The compound name of a major thirteenth.
const MAJOR_THIRTEENTH: &str = "Major 13th";

/// The eight classic seventh qualities: the ones that make a key application
/// use seventh-chord qualities.
///
/// Extended and suspended qualities deliberately do not opt into seventh mode
/// (the source's `infer_chord_mode/1` set is exactly these eight).
const SEVENTH_QUALITIES: [QualityId; 8] = [
    QualityId::from_catalog("7"),
    QualityId::from_catalog("maj7"),
    QualityId::from_catalog("min7"),
    QualityId::from_catalog("dim7"),
    QualityId::from_catalog("m7b5"),
    QualityId::from_catalog("min_maj7"),
    QualityId::from_catalog("aug_maj7"),
    QualityId::from_catalog("aug7"),
];

/// Compare two strings during const evaluation, where the `PartialEq` operator
/// is not available.
///
/// The loop indexes both byte slices behind an explicit length equality check
/// and only increments a counter bounded by that length, so
/// `indexing_slicing` and `arithmetic_side_effects` are allowed here: it is
/// compile-time table comparison, not runtime arithmetic on caller data.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn str_eq(left: &str, right: &str) -> bool {
    let (left, right) = (left.as_bytes(), right.as_bytes());
    if left.len() != right.len() {
        return false;
    }
    let mut index = 0;
    while index < left.len() {
        if left[index] != right[index] {
            return false;
        }
        index += 1;
    }
    true
}

/// The catalog and the accepted identifiers are the same list in the same
/// order.
///
/// This is what makes [`catalog_quality`] total: a mistyped, missing or
/// reordered catalog entry fails the build instead of silently answering with
/// another quality's chord.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn assert_catalog_matches_identifiers() {
    let ids = QualityId::ALL;
    assert!(
        CATALOG.len() == ids.len(),
        "the catalog and the identifier list differ in length"
    );
    let mut index = 0;
    while index < ids.len() {
        assert!(
            str_eq(CATALOG[index].id, ids[index].as_str()),
            "the catalog is not in the frozen identifier order"
        );
        index += 1;
    }
}

/// Compile-time proof of the invariant [`catalog_quality`] relies on.
const _: () = assert_catalog_matches_identifiers();

/// The catalog entry of a validated quality identifier.
///
/// `QualityId` accepts exactly the frozen catalog identifiers and [`CATALOG`]
/// carries exactly one entry per identifier in the same order — proved at
/// compile time — so the lookup cannot miss. The first entry is returned only to
/// keep the function total.
#[allow(clippy::indexing_slicing)]
fn catalog_quality(quality: QualityId) -> Quality {
    QualityId::ALL
        .iter()
        .position(|known| *known == quality)
        .map_or(CATALOG[0], |index| CATALOG[index])
}

/// Whether a formula carries a seventh: semitone 10 or 11, never the diminished
/// seventh 9.
fn has_seventh(formula: &[u8]) -> bool {
    formula.contains(&10) || formula.contains(&11)
}

/// The chord-member rank of one formula offset inside its own formula: root
/// first, then 3rds, 5ths, 7ths, 9ths, 11ths, 13ths.
///
/// Semitone 8 is contextual: it is a 13th-level extension — the rank of the
/// `Minor 13th` it is named after — exactly when the formula carries the sixth
/// or the fifth that renames it, and a fifth-level chord tone otherwise. The
/// offsets outside `0..=11` cannot occur; the last arm keeps the function total.
fn member_rank(semitone: u8, formula: &[u8]) -> u8 {
    if semitone == 8 && (formula.contains(&6) || formula.contains(&7)) {
        return 6;
    }
    match semitone {
        0 => 0,
        3..=4 => 1,
        6..=8 => 2,
        10..=11 => 3,
        1..=2 => 4,
        5 => 5,
        9 => 6,
        _ => 7,
    }
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
        2 => extension(seventh, MAJOR_NINTH, semitone),
        5 => extension(seventh, PERFECT_ELEVENTH, semitone),
        9 => extension(seventh, MAJOR_THIRTEENTH, semitone),
        1 => extension(seventh, FLAT_NINTH, semitone),
        3 => {
            if seventh && formula.contains(&4) {
                SHARP_NINTH
            } else {
                interval_name(u32::from(semitone))
            }
        }
        6 => {
            if formula.contains(&7) {
                AUGMENTED_ELEVENTH
            } else {
                interval_name(u32::from(semitone))
            }
        }
        8 => {
            if formula.contains(&6) || formula.contains(&7) {
                MINOR_THIRTEENTH
            } else {
                interval_name(u32::from(semitone))
            }
        }
        // Every remaining offset — including the always-simple 4, 7, 10 and 11 —
        // keeps the simple interval name of its distance.
        _ => interval_name(u32::from(semitone)),
    }
}

/// The compound name of an extension offset when the formula carries a seventh,
/// otherwise its simple interval name.
fn extension(seventh: bool, compound: &'static str, semitone: u8) -> &'static str {
    if seventh {
        compound
    } else {
        interval_name(u32::from(semitone))
    }
}

/// The formula offsets of a quality in label order.
///
/// Without a seventh the offsets keep formula order; with one they are sorted by
/// chord-member rank and then by semitone, which is the source's
/// `Enum.sort_by(&{rank, interval})`.
fn label_order(formula: &[u8]) -> Vec<u8> {
    let mut offsets = formula.to_vec();
    if has_seventh(formula) {
        offsets.sort_by_key(|semitone| (member_rank(*semitone, formula), *semitone));
    }
    offsets
}

/// The semitone formula of a catalog quality, in returned-note order.
pub fn chord_formula(quality: QualityId) -> &'static [u8] {
    catalog_quality(quality).formula
}

/// The display/wire suffix of a catalog quality, e.g. `maj` or `7sus`.
pub fn chord_quality_label(quality: QualityId) -> &'static str {
    catalog_quality(quality).suffix
}

/// The contextual interval labels of a catalog quality, in label order.
///
/// The labels are the chord-local vocabulary of the pinned source; see the
/// module documentation for the rule and for `Contract.D01`.
pub fn chord_interval_labels(quality: QualityId) -> Vec<&'static str> {
    let formula = chord_formula(quality);
    label_order(formula)
        .iter()
        .map(|semitone| contextual_interval_label(*semitone, formula))
        .collect()
}

/// One UI group of chord qualities, in catalog display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualityGroup {
    /// The group's display name.
    pub group: &'static str,
    /// The group's qualities, in display order.
    pub qualities: &'static [QualityId],
}

/// The qualities of the `Triads` group.
const TRIADS: &[QualityId] = &[
    QualityId::from_catalog("major"),
    QualityId::from_catalog("minor"),
    QualityId::from_catalog("dim"),
    QualityId::from_catalog("aug"),
    QualityId::from_catalog("sus2"),
    QualityId::from_catalog("sus4"),
];

/// The qualities of the `Sixths` group.
const SIXTHS: &[QualityId] = &[
    QualityId::from_catalog("maj6"),
    QualityId::from_catalog("min6"),
    QualityId::from_catalog("maj6_9"),
    QualityId::from_catalog("min6_9"),
];

/// The qualities of the `Added tones` group.
const ADDED_TONES: &[QualityId] = &[
    QualityId::from_catalog("add9"),
    QualityId::from_catalog("m_add9"),
];

/// The qualities of the `Sevenths` group.
const SEVENTHS: &[QualityId] = &[
    QualityId::from_catalog("7"),
    QualityId::from_catalog("maj7"),
    QualityId::from_catalog("min7"),
    QualityId::from_catalog("dim7"),
    QualityId::from_catalog("m7b5"),
    QualityId::from_catalog("min_maj7"),
    QualityId::from_catalog("aug_maj7"),
    QualityId::from_catalog("aug7"),
    QualityId::from_catalog("7sus4"),
    QualityId::from_catalog("dim_maj7"),
];

/// The qualities of the `Ninths` group.
const NINTHS: &[QualityId] = &[
    QualityId::from_catalog("9"),
    QualityId::from_catalog("maj9"),
    QualityId::from_catalog("min9"),
    QualityId::from_catalog("7b9"),
    QualityId::from_catalog("7#9"),
    QualityId::from_catalog("9#5"),
    QualityId::from_catalog("9b5"),
    QualityId::from_catalog("7b5"),
];

/// The qualities of the `Elevenths` group.
const ELEVENTHS: &[QualityId] = &[
    QualityId::from_catalog("11"),
    QualityId::from_catalog("maj11"),
    QualityId::from_catalog("min11"),
    QualityId::from_catalog("m11b5"),
    QualityId::from_catalog("maj7#11"),
    QualityId::from_catalog("7#11"),
];

/// The qualities of the `Thirteenths` group.
const THIRTEENTHS: &[QualityId] = &[
    QualityId::from_catalog("13"),
    QualityId::from_catalog("maj13"),
    QualityId::from_catalog("min13"),
    QualityId::from_catalog("13b9"),
];

/// The qualities of the `Suspended (extended)` group.
const SUSPENDED_EXTENDED: &[QualityId] = &[
    QualityId::from_catalog("sus9"),
    QualityId::from_catalog("susb9"),
    QualityId::from_catalog("sus13"),
    QualityId::from_catalog("7b13"),
    QualityId::from_catalog("7b9b13"),
    QualityId::from_catalog("min7b13"),
    QualityId::from_catalog("dim7b13"),
];

/// The eight UI groups of the frozen catalog, in display order.
const GROUPED_QUALITIES: [QualityGroup; 8] = [
    QualityGroup {
        group: "Triads",
        qualities: TRIADS,
    },
    QualityGroup {
        group: "Sixths",
        qualities: SIXTHS,
    },
    QualityGroup {
        group: "Added tones",
        qualities: ADDED_TONES,
    },
    QualityGroup {
        group: "Sevenths",
        qualities: SEVENTHS,
    },
    QualityGroup {
        group: "Ninths",
        qualities: NINTHS,
    },
    QualityGroup {
        group: "Elevenths",
        qualities: ELEVENTHS,
    },
    QualityGroup {
        group: "Thirteenths",
        qualities: THIRTEENTHS,
    },
    QualityGroup {
        group: "Suspended (extended)",
        qualities: SUSPENDED_EXTENDED,
    },
];

/// Every UI group of the frozen catalog, in display order.
///
/// The eight groups partition the catalog: every quality appears in exactly one
/// group, and the groups together carry all 47 identifiers.
pub const fn grouped_qualities() -> &'static [QualityGroup] {
    &GROUPED_QUALITIES
}

/// The diatonic chord mode of a key application: triads or seventh chords.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChordMode {
    /// Triad qualities.
    Triad,
    /// Seventh qualities.
    Seventh,
}

impl ChordMode {
    /// The stable wire string of this mode.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Triad => "triad",
            Self::Seventh => "seventh",
        }
    }
}

/// The mode a suggested key should be applied in, inferred from the active
/// chords.
///
/// Ported from the pinned `Fretboard.Music.Chord.infer_chord_mode/1`: the mode is
/// `Seventh` when the list carries at least one of the eight classic seventh
/// qualities and `Triad` otherwise. Extended and suspended qualities do not
/// implicitly opt into seventh mode, and an empty list is a triad list.
pub fn infer_chord_mode(chords: &[ChordSpec]) -> ChordMode {
    if chords
        .iter()
        .any(|chord| SEVENTH_QUALITIES.contains(&chord.quality))
    {
        ChordMode::Seventh
    } else {
        ChordMode::Triad
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
    /// The contextual role of each member, in label order.
    pub intervals: Vec<&'static str>,
}

/// The details of one chord: its label, its notes and their interval roles.
///
/// The notes are the root transposed by the quality's formula with the real
/// modulo-twelve note math, so every root is computed, not looked up. The label
/// uses the root's canonical sharp spelling and the quality's wire suffix, which
/// is exactly what the frozen oracle pins for the twelve sharp and natural
/// spellings.
///
/// # Errors
///
/// The signature is fallible because the frozen adapter contract carries a
/// stable error code across the boundary, and later capabilities (the analyzer,
/// the key and progression resolution) report through the same code. Since C06
/// implements the whole 47-quality catalog, this build raises no error here: a
/// quality outside the catalog is already rejected when the identifier is parsed
/// ([`QualityId::parse`]), never at this call.
pub fn chord_details(spec: &ChordSpec) -> Result<ChordDetails, CoreError> {
    let quality = catalog_quality(spec.quality);
    let notes = quality
        .formula
        .iter()
        .map(|semitone| note_at(spec.root, i32::from(*semitone)))
        .collect();
    let intervals = label_order(quality.formula)
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
