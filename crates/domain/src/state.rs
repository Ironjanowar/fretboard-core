//! The canonical page state of a Fretboard page (`02-core-contract.md`
//! sections 6, 8 and 9).
//!
//! The state is a value: owned, validated, comparable and serialisable. It
//! holds only durable page fields, never derived data and never a UI-only
//! transient. Serialising a [`PageState`] produces exactly the `page` value of
//! the `{"schema_version": 1, "page": …}` snapshot envelope, so the same type
//! serves the in-memory state and the stored session.
//!
//! Validation is separate from construction ([`validate_state`]) because the
//! contract's invariants are about *combinations* of fields; the field types
//! themselves are already validated in [`crate::types`].
//!
//! Deserialisation is strict and validating, not just shape-checking: unknown
//! fields anywhere in the page are an error, and each state type rejects by
//! itself the same violations [`validate_state`] rejects, so no code path can
//! build a value the public API could not have built.
//!
//! The instrument catalog tables below are the minimal, literal transcription
//! of `fixtures/oracle/catalogs.json` that this contract needs (string counts,
//! named pitch presets, the piano range). The full catalog record — labels,
//! fret counts, groups — is C07's `instrument_catalog.rs`, which takes over
//! these tables.

use serde::de;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::CoreError;
use crate::types::{
    Fret, InstrumentId, OpenPitch, PitchClass, PresetName, QualityId, StringIndex, Tab,
};

/// The inclusive absolute pitch range of the piano keyboard, 36 keys, from the
/// oracle's `instrument_definitions.piano.pitch_range`.
const PIANO_PITCH_RANGE: (u8, u8) = (48, 83);

/// The preset a fretted instrument's default state is anchored to (contract
/// section 6: guitar Standard, reference Standard).
const GUITAR_STANDARD: PitchPreset =
    PitchPreset::new("Standard", &pitches([40, 45, 50, 55, 59, 64]));

/// The number of distinct preset names of the frozen catalog: the union of the
/// four per-instrument preset tables below.
const PRESET_NAME_COUNT: usize = 13;

/// One named pitch preset: the exact absolute pitch of every physical string,
/// in physical string order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PitchPreset {
    name: PresetName,
    pitches: &'static [OpenPitch],
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

    /// The committed tuning state of this preset: its exact pitches plus the
    /// reference every later editing operation is anchored to.
    fn tuning(self) -> TuningState {
        TuningState {
            pitches: self.pitches.to_vec(),
            reference: self.name,
        }
    }
}

/// One instrument of the frozen catalog: its string count and its named pitch
/// presets in catalog order. The piano has no strings and no presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InstrumentDefinition {
    strings: u8,
    presets: &'static [PitchPreset],
}

impl InstrumentDefinition {
    /// The preset of this instrument with that name, when it has one.
    fn preset(self, name: PresetName) -> Option<PitchPreset> {
        self.presets
            .iter()
            .copied()
            .find(|preset| preset.name == name)
    }
}

/// The guitar presets, in catalog order.
const GUITAR_PRESETS: &[PitchPreset] = &[
    GUITAR_STANDARD,
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

/// The ukulele presets, in catalog order. Standard is reentrant.
const UKELELE_PRESETS: &[PitchPreset] = &[
    PitchPreset::new("Standard", &pitches([67, 60, 64, 69])),
    PitchPreset::new("Low G", &pitches([55, 60, 64, 69])),
    PitchPreset::new("D tuning", &pitches([69, 62, 66, 71])),
    PitchPreset::new("Baritone", &pitches([50, 55, 59, 64])),
    PitchPreset::new("Half Step Down", &pitches([66, 59, 63, 68])),
];

/// The guitar definition. Guitar, bass and ukulele share fret 0..=24.
const GUITAR: InstrumentDefinition = InstrumentDefinition {
    strings: 6,
    presets: GUITAR_PRESETS,
};
/// The bass (4-string) definition.
const BASS_4: InstrumentDefinition = InstrumentDefinition {
    strings: 4,
    presets: BASS_4_PRESETS,
};
/// The bass (5-string) definition.
const BASS_5: InstrumentDefinition = InstrumentDefinition {
    strings: 5,
    presets: BASS_5_PRESETS,
};
/// The ukulele definition.
const UKELELE: InstrumentDefinition = InstrumentDefinition {
    strings: 4,
    presets: UKELELE_PRESETS,
};
/// The piano definition: no strings, no presets, a fixed key range.
const PIANO: InstrumentDefinition = InstrumentDefinition {
    strings: 0,
    presets: &[],
};

/// The frozen catalog record of one instrument.
const fn definition_of(instrument: InstrumentId) -> InstrumentDefinition {
    match instrument {
        InstrumentId::Guitar => GUITAR,
        InstrumentId::Bass4 => BASS_4,
        InstrumentId::Bass5 => BASS_5,
        InstrumentId::Ukelele => UKELELE,
        InstrumentId::Piano => PIANO,
    }
}

/// The canonical catalog string of a preset name, when it is one of the frozen
/// catalog's preset names. This is the single source of truth for the accepted
/// [`PresetName`] values.
pub(crate) fn canonical_preset_name(value: &str) -> Option<&'static str> {
    [
        GUITAR_PRESETS,
        BASS_4_PRESETS,
        BASS_5_PRESETS,
        UKELELE_PRESETS,
    ]
    .iter()
    .flat_map(|presets| presets.iter())
    .map(|preset| preset.name.as_str())
    .find(|name| *name == value)
}

/// Compare two strings during const evaluation, where the `PartialEq` operator
/// is not available.
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

/// Every distinct preset name of the frozen catalog, in catalog order: the
/// union of the four per-instrument preset tables with the first occurrence
/// keeping its place. This is the source of [`PresetName::ALL`], so the
/// enumerated accepted names and the parsed accepted names cannot drift.
const DISTINCT_PRESET_NAMES: [PresetName; PRESET_NAME_COUNT] = distinct_preset_names();

/// Collect the distinct preset names of the four preset tables at compile
/// time, asserting that the frozen catalog still declares exactly thirteen.
const fn distinct_preset_names() -> [PresetName; PRESET_NAME_COUNT] {
    const TABLES: [&[PitchPreset]; 4] = [
        GUITAR_PRESETS,
        BASS_4_PRESETS,
        BASS_5_PRESETS,
        UKELELE_PRESETS,
    ];
    let mut names = [PresetName::from_catalog(""); PRESET_NAME_COUNT];
    let mut count = 0;
    let mut table_index = 0;
    while table_index < TABLES.len() {
        let table = TABLES[table_index];
        let mut preset_index = 0;
        while preset_index < table.len() {
            let name = table[preset_index].name;
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
        table_index += 1;
    }
    assert!(
        count == PRESET_NAME_COUNT,
        "the frozen catalog has 13 distinct preset names"
    );
    names
}

impl PresetName {
    /// Every distinct preset name of the frozen catalog, in catalog order:
    /// [`PresetName::parse`] accepts exactly these names.
    pub const ALL: [Self; 13] = DISTINCT_PRESET_NAMES;
}

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

/// A chord identity: root plus quality, not the pitch set it names. C6 and
/// Amin7 stay different chords.
///
/// `Copy`, because an occurrence is a small value that callers copy as freely
/// as the identity they compare; the active list still keeps occurrences, so
/// two copies of the same identity remain two entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChordSpec {
    /// The chord root.
    pub root: PitchClass,
    /// The chord quality identifier.
    pub quality: QualityId,
}

/// A committed tuning: the exact pitch of every string plus the preset every
/// editing operation is anchored to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TuningState {
    /// The exact absolute pitch of every string, in physical string order.
    pub pitches: Vec<OpenPitch>,
    /// The preset the pitches are anchored to.
    pub reference: PresetName,
}

/// The serialised shape of [`TuningState`]; unknown fields are an error.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct TuningStateWire {
    pitches: Vec<OpenPitch>,
    reference: PresetName,
}

impl<'de> Deserialize<'de> for TuningState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TuningStateWire::deserialize(deserializer)?;
        let tuning = Self {
            pitches: wire.pitches,
            reference: wire.reference,
        };
        validate_unbound_tuning(&tuning).map_err(de::Error::custom)?;
        Ok(tuning)
    }
}

/// One fretted selection entry: a string and the fret marked on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Position {
    /// The physical string index.
    pub string: StringIndex,
    /// The fret on that string.
    pub fret: Fret,
}

/// The instrument part of the page: a fretted instrument with a tuning and one
/// position per string, or the piano with its absolute keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstrumentState {
    /// A fretted instrument: guitar, bass or ukulele.
    Fretted {
        /// The instrument.
        instrument: InstrumentId,
        /// The committed tuning.
        tuning: TuningState,
        /// The marked positions, unique and ascending by physical string.
        selected: Vec<Position>,
    },
    /// The piano, selected by absolute pitch.
    Piano {
        /// The selected keys, unique and ascending, within 48..=83.
        selected: Vec<OpenPitch>,
    },
}

/// The serialised shape of [`InstrumentState`]: a tagged object whose `kind`
/// selects one strict variant schema. `Fretted` carries `id`, `tuning` and
/// `selection`; `Piano` carries `id` and `selection` and deliberately declares
/// no `tuning` field, so a stray `tuning` key — null or not — is an unknown
/// field of the piano schema.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum InstrumentStateWire {
    Fretted(FrettedWire),
    Piano(PianoWire),
}

/// The wire fields of a fretted instrument; unknown fields are an error.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FrettedWire {
    #[serde(rename = "id")]
    instrument: InstrumentId,
    tuning: TuningState,
    #[serde(rename = "selection")]
    selected: Vec<Position>,
}

/// The wire fields of the piano; unknown fields are an error.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PianoWire {
    #[serde(rename = "id")]
    instrument: InstrumentId,
    #[serde(rename = "selection")]
    selected: Vec<OpenPitch>,
}

impl Serialize for InstrumentState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut object = serializer.serialize_map(None)?;
        match self {
            Self::Fretted {
                instrument,
                tuning,
                selected,
            } => {
                object.serialize_entry("kind", "fretted")?;
                object.serialize_entry("id", instrument)?;
                object.serialize_entry("tuning", tuning)?;
                object.serialize_entry("selection", selected)?;
            }
            Self::Piano { selected } => {
                object.serialize_entry("kind", "piano")?;
                object.serialize_entry("id", &InstrumentId::Piano)?;
                object.serialize_entry("selection", selected)?;
            }
        }
        object.end()
    }
}

impl<'de> Deserialize<'de> for InstrumentState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match InstrumentStateWire::deserialize(deserializer)? {
            InstrumentStateWire::Fretted(wire) => {
                // A fretted state may not carry the piano: that is the only
                // representation that could attach a tuning to a keyboard
                // instrument.
                if !wire.instrument.is_fretted() {
                    return Err(de::Error::custom(CoreError::invalid_state("instrument")));
                }
                validate_fretted(wire.instrument, &wire.tuning, &wire.selected)
                    .map_err(de::Error::custom)?;
                Ok(Self::Fretted {
                    instrument: wire.instrument,
                    tuning: wire.tuning,
                    selected: wire.selected,
                })
            }
            InstrumentStateWire::Piano(wire) => {
                if wire.instrument != InstrumentId::Piano {
                    return Err(de::Error::custom(CoreError::invalid_state("instrument")));
                }
                validate_piano_selection(&wire.selected).map_err(de::Error::custom)?;
                Ok(Self::Piano {
                    selected: wire.selected,
                })
            }
        }
    }
}

/// The durable state of one page: what gets stored and restored (contract
/// sections 6 and 9). Derived data is recomputed, never persisted, and never
/// part of this type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PageState {
    /// The instrument and its kind-specific selection.
    pub instrument: InstrumentState,
    /// The active chords as ordered occurrences, not a set.
    pub chords: Vec<ChordSpec>,
    /// The highlighted chord identity, which must occur in `chords`.
    pub highlight: Option<ChordSpec>,
    /// The active tab.
    pub tab: Tab,
}

/// The serialised shape of [`PageState`]; unknown fields are an error.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageStateWire {
    instrument: InstrumentState,
    chords: Vec<ChordSpec>,
    highlight: Option<ChordSpec>,
    tab: Tab,
}

impl<'de> Deserialize<'de> for PageState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PageStateWire::deserialize(deserializer)?;
        let state = Self {
            instrument: wire.instrument,
            chords: wire.chords,
            highlight: wire.highlight,
            tab: wire.tab,
        };
        validate_state(&state).map_err(de::Error::custom)?;
        Ok(state)
    }
}

/// The default page (contract section 6): guitar, Standard tuning anchored to
/// the Standard preset, no chords, no highlight, the visualizer tab and an
/// empty selection.
pub fn default_state() -> PageState {
    PageState {
        instrument: InstrumentState::Fretted {
            instrument: InstrumentId::Guitar,
            tuning: GUITAR_STANDARD.tuning(),
            selected: Vec::new(),
        },
        chords: Vec::new(),
        highlight: None,
        tab: Tab::Visualizer,
    }
}

/// The committed tuning of one named preset of a fretted instrument.
///
/// The piano has no presets and no tuning, and a fretted instrument has only the
/// presets of the frozen catalog, so both are typed errors rather than a
/// function-clause crash.
///
/// # Errors
///
/// [`CoreError::InvalidState`] when the instrument is the piano, and
/// [`CoreError::UnknownIdentifier`] when the name is not a preset of that
/// instrument.
pub fn preset_tuning(
    instrument: InstrumentId,
    name: &PresetName,
) -> Result<TuningState, CoreError> {
    if !instrument.is_fretted() {
        return Err(CoreError::invalid_state("instrument"));
    }
    definition_of(instrument)
        .preset(*name)
        .map(PitchPreset::tuning)
        .ok_or_else(|| CoreError::unknown_identifier("reference"))
}

/// Check every invariant the state types cannot express themselves
/// (contract section 8).
///
/// Structural violations are [`CoreError::InvalidState`]; numeric values outside
/// the range their position allows are [`CoreError::OutOfRange`]. The input is
/// borrowed and never mutated, so a rejected state is left exactly as it was.
///
/// # Errors
///
/// Returns the first violation found: [`CoreError::InvalidState`] for a
/// structurally impossible state (a fretted state carrying the piano, a wrong
/// pitch count, duplicate string indices, a highlight absent from the chords) or
/// [`CoreError::OutOfRange`] for a position outside its instrument's range.
pub fn validate_state(state: &PageState) -> Result<(), CoreError> {
    validate_instrument(&state.instrument)?;
    if let Some(highlight) = &state.highlight
        && !state.chords.contains(highlight)
    {
        return Err(CoreError::invalid_state("highlight"));
    }
    Ok(())
}

/// Check the instrument part: kind, pitch count, preset reference and the
/// instrument-specific selection.
fn validate_instrument(instrument: &InstrumentState) -> Result<(), CoreError> {
    match instrument {
        InstrumentState::Fretted {
            instrument,
            tuning,
            selected,
        } => {
            // A fretted state may not carry the piano: that is the only
            // representation that could attach a tuning to a keyboard
            // instrument.
            if !instrument.is_fretted() {
                return Err(CoreError::invalid_state("instrument"));
            }
            validate_fretted(*instrument, tuning, selected)
        }
        InstrumentState::Piano { selected } => validate_piano_selection(selected),
    }
}

/// Check a fretted instrument's tuning and selection against its definition:
/// the pitch count must match the instrument's string count, the reference must
/// be one of its own presets, and every position must be on a string it has.
fn validate_fretted(
    instrument: InstrumentId,
    tuning: &TuningState,
    selected: &[Position],
) -> Result<(), CoreError> {
    let definition = definition_of(instrument);
    if tuning.pitches.len() != usize::from(definition.strings) {
        return Err(CoreError::invalid_state("tuning"));
    }
    if definition.preset(tuning.reference).is_none() {
        return Err(CoreError::invalid_state("reference"));
    }
    validate_positions(definition, selected)
}

/// Check a bare [`TuningState`] against the frozen catalog.
///
/// A bare tuning carries no instrument id, so the strongest decidable rule is:
/// its pitch count must be the string count of some fretted instrument, and its
/// reference must be a preset of an instrument with that same string count.
/// Requiring only a known name would be too weak — `Low G` is a real preset,
/// but not one for a six-string instrument.
fn validate_unbound_tuning(tuning: &TuningState) -> Result<(), CoreError> {
    let string_count = tuning.pitches.len();
    let no_fretted_instrument_has_this_many_strings =
        !InstrumentId::ALL.iter().copied().any(|instrument| {
            instrument.is_fretted()
                && usize::from(definition_of(instrument).strings) == string_count
        });
    if no_fretted_instrument_has_this_many_strings {
        return Err(CoreError::invalid_state("tuning"));
    }
    let reference_belongs_to_a_same_count_instrument =
        InstrumentId::ALL.iter().copied().any(|instrument| {
            instrument.is_fretted()
                && usize::from(definition_of(instrument).strings) == string_count
                && definition_of(instrument).preset(tuning.reference).is_some()
        });
    if !reference_belongs_to_a_same_count_instrument {
        return Err(CoreError::invalid_state("reference"));
    }
    Ok(())
}

/// Check a piano selection: every key within 48..=83, unique and ascending.
fn validate_piano_selection(selected: &[OpenPitch]) -> Result<(), CoreError> {
    let (lowest, highest) = PIANO_PITCH_RANGE;
    let mut previous: Option<OpenPitch> = None;
    for pitch in selected {
        let value = u8::from(*pitch);
        if value < lowest || value > highest {
            return Err(CoreError::out_of_range("selection"));
        }
        if previous.is_some_and(|previous| *pitch <= previous) {
            return Err(CoreError::invalid_state("selection"));
        }
        previous = Some(*pitch);
    }
    Ok(())
}

/// Check the fretted selection: the string must exist on the instrument, and the
/// positions must be unique and ascending by physical string (never by pitch;
/// the ukulele Standard tuning is reentrant).
fn validate_positions(
    definition: InstrumentDefinition,
    selected: &[Position],
) -> Result<(), CoreError> {
    let mut previous: Option<StringIndex> = None;
    for position in selected {
        if u8::from(position.string) >= definition.strings {
            return Err(CoreError::out_of_range("selection"));
        }
        if previous.is_some_and(|previous| position.string <= previous) {
            return Err(CoreError::invalid_state("selection"));
        }
        previous = Some(position.string);
    }
    Ok(())
}
