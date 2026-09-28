//! The instrument and preset catalog (`02-core-contract.md` sections 2 and 6,
//! and the pinned `lib/fretboard/music/instrument.ex`).
//!
//! This module belongs to task `C07` and takes over the minimal tables the typed
//! state contract needed to validate itself. It is the single source of the
//! instrument catalog: the five stable identifiers with their English labels,
//! the string and fret counts, the fixed piano range, and every named pitch
//! preset with the exact MIDI pitch of each physical string.
//!
//! MIDI pitches are the only tuning data; the note-name ("legacy") views the
//! baseline keeps for the guitar are *derived* from them, exactly as the source
//! derives them (`Pitch.note_name/1` is the pitch class of the absolute pitch),
//! so a preset cannot disagree with itself.
//!
//! String order is physical, never pitch order: the ukulele's Standard tuning is
//! reentrant (its first string is the highest), and sorting the strings would
//! change the instrument.
//!
//! The tables are the pinned source's own catalog, verified record by record
//! against `fixtures/oracle/catalogs.json` and `fixtures/oracle/tunings.jsonl`
//! by `tests/instrument_catalog.rs`. Nothing is transcribed from the plan.
//!
//! Tuning *analysis* — detecting which preset a set of pitches matches, and
//! editing one string to its closest pitch — belongs to a later task and is not
//! here.

use crate::error::CoreError;
use crate::types::{InstrumentId, OpenPitch, PitchClass, PresetName};

/// Whether an instrument has strings and frets, or is a keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentKind {
    /// An instrument with strings, frets and a tuning.
    Fretted,
    /// A keyboard: absolute keys, no strings and no tuning.
    Keyboard,
}

/// One named pitch preset: the exact absolute pitch of every physical string, in
/// physical string order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PitchPreset {
    /// The preset name, one of the frozen catalog's names.
    pub name: PresetName,
    /// The absolute pitch of every physical string, in physical string order.
    pub pitches: &'static [OpenPitch],
}

impl PitchPreset {
    /// The table-internal constructor: `name` is one of the frozen catalog's
    /// preset names, and the entry is what makes it a known [`PresetName`].
    const fn new(name: &'static str, pitches: &'static [OpenPitch]) -> Self {
        Self {
            name: PresetName::from_catalog(name),
            pitches,
        }
    }
}

/// One instrument of the frozen catalog.
///
/// The kind-specific fields are optional because the two kinds carry different
/// data: a fretted instrument has strings, frets and named presets, while the
/// piano has a fixed key range and no tuning at all. Prefer the typed accessors
/// ([`instrument_strings`], [`instrument_frets`], [`keyboard_pitch_range`]),
/// which reject the wrong kind instead of reporting a plausible wrong number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Instrument {
    /// The stable catalog identifier.
    pub id: InstrumentId,
    /// The English display label.
    pub label: &'static str,
    /// Whether this instrument is fretted or a keyboard.
    pub kind: InstrumentKind,
    /// The number of strings, for a fretted instrument.
    pub strings: Option<u8>,
    /// The number of frets, for a fretted instrument.
    pub frets: Option<u8>,
    /// The inclusive key range, for a keyboard.
    pub pitch_range: Option<(OpenPitch, OpenPitch)>,
    /// The named pitch presets, in catalog order. Empty for a keyboard.
    pub presets: &'static [PitchPreset],
}

impl Instrument {
    /// Whether this instrument is the piano.
    pub const fn is_keyboard(self) -> bool {
        matches!(self.kind, InstrumentKind::Keyboard)
    }
}

/// The number of distinct preset names of the frozen catalog: the union of the
/// four fretted instruments' preset tables.
pub(crate) const PRESET_NAME_COUNT: usize = 13;

/// The inclusive absolute pitch range of the piano keyboard, 36 keys, from the
/// oracle's `instrument_definitions.piano.pitch_range`.
const PIANO_PITCH_RANGE: (u8, u8) = (48, 83);

/// Convert one transcribed pitch table into the typed representation, asserting
/// every value during const evaluation so a mistyped catalog value fails the
/// build.
///
/// The index is bounded by `COUNT` and every assignment is validated, so
/// `indexing_slicing` and `arithmetic_side_effects` are allowed here: this is
/// compile-time table conversion, not runtime arithmetic on user input.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn pitches<const COUNT: usize>(values: [u8; COUNT]) -> [OpenPitch; COUNT] {
    let mut converted = [OpenPitch::from_catalog(0); COUNT];
    let mut index = 0;
    while index < COUNT {
        converted[index] = OpenPitch::from_catalog(values[index]);
        index += 1;
    }
    converted
}

/// The guitar presets, in catalog order.
const GUITAR_PRESETS: &[PitchPreset] = &[
    PitchPreset::new("Standard", &pitches([40, 45, 50, 55, 59, 64])),
    PitchPreset::new("Drop D", &pitches([38, 45, 50, 55, 59, 64])),
    PitchPreset::new("DADGAD", &pitches([38, 45, 50, 55, 57, 62])),
    PitchPreset::new("Open G", &pitches([38, 43, 50, 55, 59, 62])),
    PitchPreset::new("Open D", &pitches([38, 45, 50, 54, 57, 62])),
    PitchPreset::new("Open E", &pitches([40, 47, 52, 56, 59, 64])),
    PitchPreset::new("Half Step Down", &pitches([39, 44, 49, 54, 58, 63])),
    PitchPreset::new("Full Step Down", &pitches([38, 43, 48, 53, 57, 62])),
    PitchPreset::new("Drop C", &pitches([36, 43, 48, 53, 57, 62])),
];

/// The bass (4-string) presets, in catalog order.
const BASS_4_PRESETS: &[PitchPreset] = &[
    PitchPreset::new("Standard", &pitches([28, 33, 38, 43])),
    PitchPreset::new("Drop D", &pitches([26, 33, 38, 43])),
    PitchPreset::new("Half Step Down", &pitches([27, 32, 37, 42])),
];

/// The bass (5-string) presets, in catalog order.
const BASS_5_PRESETS: &[PitchPreset] = &[
    PitchPreset::new("Standard", &pitches([23, 28, 33, 38, 43])),
    PitchPreset::new("Half Step Down", &pitches([22, 27, 32, 37, 42])),
    PitchPreset::new("Drop A", &pitches([21, 28, 33, 38, 43])),
];

/// The ukulele presets, in catalog order. Standard is reentrant: the first
/// string is the highest.
const UKELELE_PRESETS: &[PitchPreset] = &[
    PitchPreset::new("Standard", &pitches([67, 60, 64, 69])),
    PitchPreset::new("Low G", &pitches([55, 60, 64, 69])),
    PitchPreset::new("D tuning", &pitches([69, 62, 66, 71])),
    PitchPreset::new("Baritone", &pitches([50, 55, 59, 64])),
    PitchPreset::new("Half Step Down", &pitches([66, 59, 63, 68])),
];

/// The five instruments of the frozen catalog, in display order.
///
/// The display order is the catalog's own: guitar, bass (4-string), bass
/// (5-string), ukulele, then the piano.
const CATALOG: [Instrument; 5] = [
    Instrument {
        id: InstrumentId::Guitar,
        label: "Guitar",
        kind: InstrumentKind::Fretted,
        strings: Some(6),
        frets: Some(24),
        pitch_range: None,
        presets: GUITAR_PRESETS,
    },
    Instrument {
        id: InstrumentId::Bass4,
        label: "Bass (4-string)",
        kind: InstrumentKind::Fretted,
        strings: Some(4),
        frets: Some(24),
        pitch_range: None,
        presets: BASS_4_PRESETS,
    },
    Instrument {
        id: InstrumentId::Bass5,
        label: "Bass (5-string)",
        kind: InstrumentKind::Fretted,
        strings: Some(5),
        frets: Some(24),
        pitch_range: None,
        presets: BASS_5_PRESETS,
    },
    Instrument {
        id: InstrumentId::Ukelele,
        label: "Ukulele",
        kind: InstrumentKind::Fretted,
        strings: Some(4),
        frets: Some(24),
        pitch_range: None,
        presets: UKELELE_PRESETS,
    },
    Instrument {
        id: InstrumentId::Piano,
        label: "Piano",
        kind: InstrumentKind::Keyboard,
        strings: None,
        frets: None,
        pitch_range: Some((
            OpenPitch::from_catalog(PIANO_PITCH_RANGE.0),
            OpenPitch::from_catalog(PIANO_PITCH_RANGE.1),
        )),
        presets: &[],
    },
];

/// Every instrument of the frozen catalog, in display order.
pub const fn instruments() -> &'static [Instrument] {
    &CATALOG
}

/// Every fretted instrument of the frozen catalog, in display order.
///
/// The piano is not among them: it has no strings, no frets and no tuning.
#[allow(clippy::indexing_slicing)]
pub const fn fretted_instruments() -> &'static [Instrument] {
    // The catalog's display order puts the four fretted instruments first, so
    // the fretted view is its prefix. The split indices are checked during const
    // evaluation by `assert_catalog_split`, and by the catalog tests, so this
    // cannot silently drop or include an instrument.
    CATALOG.split_at(FRETTED_INSTRUMENT_COUNT).0
}

/// The number of fretted instruments of the frozen catalog.
const FRETTED_INSTRUMENT_COUNT: usize = 4;

/// The catalog's display order really is "the fretted instruments, then the
/// keyboard".
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn assert_catalog_split() {
    assert!(
        FRETTED_INSTRUMENT_COUNT <= CATALOG.len(),
        "the fretted prefix cannot exceed the catalog"
    );
    let mut index = 0;
    while index < FRETTED_INSTRUMENT_COUNT {
        assert!(
            matches!(CATALOG[index].kind, InstrumentKind::Fretted),
            "every instrument of the fretted prefix must be fretted"
        );
        index += 1;
    }
    while index < CATALOG.len() {
        assert!(
            matches!(CATALOG[index].kind, InstrumentKind::Keyboard),
            "every instrument after the fretted prefix must be a keyboard"
        );
        index += 1;
    }
}

/// Compile-time proof of the display order [`fretted_instruments`] relies on.
const _: () = assert_catalog_split();

/// The keyboard instrument of the frozen catalog.
///
/// There is exactly one keyboard today; the accessor keeps the call sites honest
/// instead of indexing the catalog by hand.
#[allow(clippy::indexing_slicing)]
pub const fn piano() -> &'static Instrument {
    &CATALOG[FRETTED_INSTRUMENT_COUNT]
}

/// The catalog entry of an instrument identifier.
///
/// Every [`InstrumentId`] has exactly one entry — `assert_catalog_split` and the
/// catalog tests prove the catalog carries all five, in order — so the lookup
/// cannot miss. The first entry is returned only to keep the function total.
fn instrument_of(id: InstrumentId) -> &'static Instrument {
    CATALOG
        .iter()
        .find(|instrument| instrument.id == id)
        .unwrap_or(&CATALOG[0])
}

/// The English display label of an instrument.
pub fn instrument_label(id: InstrumentId) -> &'static str {
    instrument_of(id).label
}

/// Whether an instrument is fretted or a keyboard.
pub fn instrument_kind(id: InstrumentId) -> InstrumentKind {
    instrument_of(id).kind
}

/// The number of strings of a fretted instrument.
///
/// # Errors
///
/// [`CoreError::InvalidState`] naming `instrument` for the piano, which has no
/// strings.
pub fn instrument_strings(id: InstrumentId) -> Result<u8, CoreError> {
    instrument_of(id)
        .strings
        .ok_or_else(|| CoreError::invalid_state("instrument"))
}

/// The number of frets of a fretted instrument (0..=24 on every one of them).
///
/// # Errors
///
/// [`CoreError::InvalidState`] naming `instrument` for the piano, which has no
/// frets.
pub fn instrument_frets(id: InstrumentId) -> Result<u8, CoreError> {
    instrument_of(id)
        .frets
        .ok_or_else(|| CoreError::invalid_state("instrument"))
}

/// The inclusive absolute pitch range of the piano keyboard.
pub fn keyboard_pitch_range() -> (OpenPitch, OpenPitch) {
    // The keyboard entry always carries its range: `assert_catalog_split` proves
    // the last catalog entry is the keyboard, and the catalog tests compare the
    // range with the oracle's. The constant keeps the function total.
    piano().pitch_range.unwrap_or((
        OpenPitch::from_catalog(PIANO_PITCH_RANGE.0),
        OpenPitch::from_catalog(PIANO_PITCH_RANGE.1),
    ))
}

/// The named pitch presets of an instrument, in catalog order.
///
/// Empty for the piano, which has no tuning.
pub fn pitch_presets(id: InstrumentId) -> &'static [PitchPreset] {
    instrument_of(id).presets
}

/// The exact pitches of one named preset, or `None` when the instrument has no
/// such preset.
pub fn preset_pitches(id: InstrumentId, name: PresetName) -> Option<&'static [OpenPitch]> {
    pitch_presets(id)
        .iter()
        .find(|preset| preset.name == name)
        .map(|preset| preset.pitches)
}

/// The canonical catalog string of a preset name, when it is one of the frozen
/// catalog's preset names: the union of the four fretted instruments' presets.
///
/// This is the single source of truth for the accepted [`PresetName`] values, so
/// a name that belongs to one instrument parses even when another instrument
/// does not have it; whether a given instrument has it is a lookup (see
/// [`preset_pitches`]).
pub(crate) fn canonical_preset_name(value: &str) -> Option<&'static str> {
    CATALOG
        .iter()
        .flat_map(|instrument| instrument.presets.iter())
        .map(|preset| preset.name.as_str())
        .find(|name| *name == value)
}

/// The exact pitches of the named preset, or
/// [`CoreError::UnknownIdentifier`] naming `reference` when this instrument has
/// no such preset.
///
/// # Errors
///
/// [`CoreError::UnknownIdentifier`] naming `reference` when the name is not a
/// preset of *this* instrument (`Low G` is a real preset, but not a guitar one).
pub fn named_preset_pitches(
    id: InstrumentId,
    name: PresetName,
) -> Result<&'static [OpenPitch], CoreError> {
    preset_pitches(id, name).ok_or_else(|| CoreError::unknown_identifier("reference"))
}

/// The exact open-string pitches of a fretted instrument's Standard tuning.
///
/// # Errors
///
/// [`CoreError::InvalidState`] naming `instrument` for the piano, which has no
/// tuning at all.
pub fn standard_pitches(id: InstrumentId) -> Result<&'static [OpenPitch], CoreError> {
    if instrument_kind(id) == InstrumentKind::Keyboard {
        return Err(CoreError::invalid_state("instrument"));
    }
    named_preset_pitches(id, PresetName::from_catalog("Standard"))
}

/// The note names of a fretted instrument's Standard tuning, in physical string
/// order.
///
/// The names are derived from the exact pitches ([`Pitch::note_name`]'s rule is
/// the pitch class), never stored beside them.
///
/// # Errors
///
/// [`CoreError::InvalidState`] naming `instrument` for the piano, which has no
/// tuning at all.
pub fn standard_tuning_notes(id: InstrumentId) -> Result<Vec<PitchClass>, CoreError> {
    Ok(standard_pitches(id)?.iter().copied().map(note_of).collect())
}

/// The named presets of a fretted instrument with their derived note names, in
/// catalog order.
///
/// This is the baseline's legacy note-name view of the same catalog: an
/// instrument with no presets (the piano) has an empty list, exactly as the
/// source's `instrument_tuning_presets/1` returns `[]` for it.
pub fn tuning_presets(id: InstrumentId) -> Vec<(PresetName, Vec<PitchClass>)> {
    pitch_presets(id)
        .iter()
        .map(|preset| {
            (
                preset.name,
                preset
                    .pitches
                    .iter()
                    .copied()
                    .map(note_of)
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

/// The note name of one absolute pitch: its pitch class.
///
/// Every pitch of the catalog is within `0..=127`, so the reduction to a pitch
/// class is exact; the unison is returned only to keep the function total (the
/// `% 12` remainder of any `u8` is already a pitch class).
#[allow(clippy::arithmetic_side_effects)]
fn note_of(pitch: OpenPitch) -> PitchClass {
    let index = u8::from(pitch) % 12;
    PitchClass::try_from(index).unwrap_or(PitchClass::from_catalog(0))
}

/// Every distinct preset name of the frozen catalog, in catalog order: the
/// union of the four fretted instruments' preset tables, with the first
/// occurrence keeping its place.
///
/// This is the source of [`PresetName::ALL`], so the enumerated accepted names
/// and the parsed accepted names cannot drift.
pub(crate) const DISTINCT_PRESET_NAMES: [PresetName; PRESET_NAME_COUNT] = distinct_preset_names();

/// Collect the distinct preset names of the catalog at compile time, asserting
/// that the frozen catalog still declares exactly thirteen.
///
/// Every index is bounded by the length of the table it walks and every counter
/// is asserted before use, so `indexing_slicing` and `arithmetic_side_effects`
/// are allowed: this runs during const evaluation over frozen tables, never on
/// caller input.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn distinct_preset_names() -> [PresetName; PRESET_NAME_COUNT] {
    let mut names = [PresetName::from_catalog(""); PRESET_NAME_COUNT];
    let mut count = 0;
    let mut instrument_index = 0;
    while instrument_index < CATALOG.len() {
        let presets = CATALOG[instrument_index].presets;
        let mut preset_index = 0;
        while preset_index < presets.len() {
            let name = presets[preset_index].name;
            let mut seen = false;
            let mut scan = 0;
            while scan < count {
                if str_eq(names[scan].as_str(), name.as_str()) {
                    seen = true;
                    break;
                }
                scan += 1;
            }
            if !seen {
                assert!(
                    count < PRESET_NAME_COUNT,
                    "more than 13 distinct preset names"
                );
                names[count] = name;
                count += 1;
            }
            preset_index += 1;
        }
        instrument_index += 1;
    }
    assert!(
        count == PRESET_NAME_COUNT,
        "the frozen catalog has 13 distinct preset names"
    );
    names
}

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

/// The guitar's Standard tuning, as note names, in physical string order.
///
/// The baseline keeps three guitar-only aliases for backward compatibility
/// (`Fretboard.Music.standard_tuning/0` and friends); they are the guitar entry
/// of this catalog.
pub fn guitar_standard_tuning() -> Vec<&'static str> {
    standard_tuning_notes(InstrumentId::Guitar).map_or_else(
        |_| Vec::new(),
        |notes| notes.iter().map(|note| note.name()).collect(),
    )
}

/// The guitar's preset names, in catalog order (the baseline's
/// `tuning_preset_names/0` alias).
pub fn guitar_tuning_preset_names() -> Vec<PresetName> {
    pitch_presets(InstrumentId::Guitar)
        .iter()
        .map(|preset| preset.name)
        .collect()
}

/// The guitar's presets with their derived note names (the baseline's
/// `tuning_presets/0` alias).
pub fn guitar_tuning_presets() -> Vec<(PresetName, Vec<PitchClass>)> {
    tuning_presets(InstrumentId::Guitar)
}
