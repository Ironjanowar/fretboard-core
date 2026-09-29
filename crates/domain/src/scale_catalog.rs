//! The frozen scale catalog: identifiers, display labels, formulas and display
//! groups (the pinned `lib/fretboard/music/scale.ex` `@scale_formulas`,
//! `@labels` and `@grouped_scale_types`).
//!
//! This file belongs to task `C15`. The tables carry the pinned source's own
//! values in the frozen identifier order ([`ScaleId::ALL`]); no formula is
//! transcribed from the plan. `tests/scales.rs` pins every entry against the
//! frozen oracle (`fixtures/oracle/catalogs.json` `scale_types` and
//! `grouped_scale_types`, plus the `scale_label/1` records of
//! `fixtures/oracle/scales.jsonl`), record by record, and checks each formula
//! against the fixture's own `notes_from_tonic`.
//!
//! Nothing here computes notes; [`crate::scale_notes`] does that with the real
//! note math. A formula is only the list of semitones above the tonic.

use crate::chord::str_eq;
use crate::types::ScaleId;

/// One scale type of the frozen catalog: its stable identifier, its display
/// label and its semitone formula above the tonic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ScaleEntry {
    /// The stable catalog identifier the scale type parses from.
    id: &'static str,
    /// The display label.
    label: &'static str,
    /// The semitones above the tonic, in returned-note order.
    formula: &'static [u8],
}

/// The fifteen scale types of the frozen catalog, in the frozen identifier
/// order.
///
/// The order is the same one [`ScaleId::ALL`] enumerates, so the catalog
/// carries exactly one entry per accepted identifier.
const CATALOG: [ScaleEntry; 15] = [
    ScaleEntry {
        id: "major",
        label: "Major",
        formula: &[0, 2, 4, 5, 7, 9, 11],
    },
    ScaleEntry {
        id: "minor",
        label: "Minor",
        formula: &[0, 2, 3, 5, 7, 8, 10],
    },
    ScaleEntry {
        id: "harmonic_minor",
        label: "Harmonic Minor",
        formula: &[0, 2, 3, 5, 7, 8, 11],
    },
    ScaleEntry {
        id: "melodic_minor",
        label: "Melodic Minor",
        formula: &[0, 2, 3, 5, 7, 9, 11],
    },
    ScaleEntry {
        id: "pentatonic_major",
        label: "Major Pentatonic",
        formula: &[0, 2, 4, 7, 9],
    },
    ScaleEntry {
        id: "pentatonic_minor",
        label: "Minor Pentatonic",
        formula: &[0, 3, 5, 7, 10],
    },
    ScaleEntry {
        id: "blues",
        label: "Blues",
        formula: &[0, 3, 5, 6, 7, 10],
    },
    ScaleEntry {
        id: "dorian",
        label: "Dorian",
        formula: &[0, 2, 3, 5, 7, 9, 10],
    },
    ScaleEntry {
        id: "phrygian",
        label: "Phrygian",
        formula: &[0, 1, 3, 5, 7, 8, 10],
    },
    ScaleEntry {
        id: "lydian",
        label: "Lydian",
        formula: &[0, 2, 4, 6, 7, 9, 11],
    },
    ScaleEntry {
        id: "mixolydian",
        label: "Mixolydian",
        formula: &[0, 2, 4, 5, 7, 9, 10],
    },
    ScaleEntry {
        id: "locrian",
        label: "Locrian",
        formula: &[0, 1, 3, 5, 6, 8, 10],
    },
    ScaleEntry {
        id: "phrygian_dominant",
        label: "Phrygian Dominant",
        formula: &[0, 1, 4, 5, 7, 8, 10],
    },
    ScaleEntry {
        id: "whole_tone",
        label: "Whole Tone",
        formula: &[0, 2, 4, 6, 8, 10],
    },
    ScaleEntry {
        id: "chromatic",
        label: "Chromatic",
        formula: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    },
];

/// The catalog and the accepted identifiers are the same list in the same
/// order.
///
/// This is what makes [`catalog_entry`] total: a mistyped, missing or reordered
/// catalog entry fails the build instead of silently answering with another
/// scale's notes. It compares the strings with the crate's one const string
/// comparison, because the `PartialEq` operator is not available in const
/// evaluation.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn assert_catalog_matches_identifiers() {
    let ids = ScaleId::ALL;
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

/// Compile-time proof of the invariant [`catalog_entry`] relies on.
const _: () = assert_catalog_matches_identifiers();

/// The catalog entry of a validated scale identifier.
///
/// `ScaleId` accepts exactly the frozen catalog identifiers and [`CATALOG`]
/// carries exactly one entry per identifier in the same order — proved at
/// compile time — so the lookup cannot miss. The first entry is returned only to
/// keep the function total.
#[allow(clippy::indexing_slicing)]
fn catalog_entry(scale: ScaleId) -> ScaleEntry {
    ScaleId::ALL
        .iter()
        .position(|known| *known == scale)
        .map_or(CATALOG[0], |index| CATALOG[index])
}

/// The display label of a scale type, e.g. `Major Pentatonic`.
pub fn scale_label(scale: ScaleId) -> &'static str {
    catalog_entry(scale).label
}

/// The semitone formula of a scale type, in returned-note order.
///
/// The formula always starts at its tonic (`0`), and the note at each offset is
/// computed by [`crate::scale_notes`]; the offsets themselves are the pinned
/// source's own table.
pub fn scale_formula(scale: ScaleId) -> &'static [u8] {
    catalog_entry(scale).formula
}

/// One display group of scale types, in catalog display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleGroup {
    /// The group's display name.
    pub group: &'static str,
    /// The group's scale types, in display order.
    pub scales: &'static [ScaleId],
}

/// The scale types of the `Standard` group.
const STANDARD: &[ScaleId] = &[
    ScaleId::from_catalog("major"),
    ScaleId::from_catalog("minor"),
];

/// The scale types of the `Minor Variants` group.
const MINOR_VARIANTS: &[ScaleId] = &[
    ScaleId::from_catalog("harmonic_minor"),
    ScaleId::from_catalog("melodic_minor"),
];

/// The scale types of the `Pentatonic` group.
const PENTATONIC: &[ScaleId] = &[
    ScaleId::from_catalog("pentatonic_major"),
    ScaleId::from_catalog("pentatonic_minor"),
];

/// The scale types of the `Blues` group.
const BLUES: &[ScaleId] = &[ScaleId::from_catalog("blues")];

/// The scale types of the `Modes` group.
const MODES: &[ScaleId] = &[
    ScaleId::from_catalog("dorian"),
    ScaleId::from_catalog("phrygian"),
    ScaleId::from_catalog("lydian"),
    ScaleId::from_catalog("mixolydian"),
    ScaleId::from_catalog("locrian"),
];

/// The scale types of the `Exotic` group.
const EXOTIC: &[ScaleId] = &[
    ScaleId::from_catalog("phrygian_dominant"),
    ScaleId::from_catalog("whole_tone"),
];

/// The scale types of the `Other` group.
const OTHER: &[ScaleId] = &[ScaleId::from_catalog("chromatic")];

/// The seven display groups of the frozen catalog, in display order.
const GROUPED_SCALE_TYPES: [ScaleGroup; 7] = [
    ScaleGroup {
        group: "Standard",
        scales: STANDARD,
    },
    ScaleGroup {
        group: "Minor Variants",
        scales: MINOR_VARIANTS,
    },
    ScaleGroup {
        group: "Pentatonic",
        scales: PENTATONIC,
    },
    ScaleGroup {
        group: "Blues",
        scales: BLUES,
    },
    ScaleGroup {
        group: "Modes",
        scales: MODES,
    },
    ScaleGroup {
        group: "Exotic",
        scales: EXOTIC,
    },
    ScaleGroup {
        group: "Other",
        scales: OTHER,
    },
];

/// Every display group of the frozen catalog, in display order.
///
/// The seven groups partition the catalog: every scale type appears in exactly
/// one group, and the groups together carry all fifteen identifiers (pinned by
/// `tests/scales.rs`).
pub const fn grouped_scale_types() -> &'static [ScaleGroup] {
    &GROUPED_SCALE_TYPES
}
