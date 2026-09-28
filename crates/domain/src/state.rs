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
//! fields anywhere in the page are an error, a key whose field is misspelled is
//! a missing-field error rather than a silent default, and each state type
//! rejects by itself the same violations [`validate_state`] rejects, so no
//! *deserialisation* path can build a value the public API could not have
//! built. The state types have public fields, so a caller can still hand-build
//! such a value; [`validate_state`] is the entry point that rejects it.
//!
//! The instrument catalog lives in [`crate::instrument_catalog`] (task C07):
//! this module asks it for a string count, a named preset or the piano range
//! instead of keeping a second copy of the frozen tables. Validation asks the
//! catalog, the catalog owns the numbers.

use serde::de;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::CoreError;
use crate::instrument_catalog::{
    Instrument, InstrumentKind, fretted_instruments, instrument_kind, instrument_strings,
    keyboard_pitch_range, named_preset_pitches, preset_pitches,
};
use crate::types::{
    Fret, InstrumentId, OpenPitch, PitchClass, PresetName, QualityId, StringIndex, Tab,
};

// The instrument catalog — string counts, named pitch presets, the piano range
// — lives in `crate::instrument_catalog` since C07. This module asks it instead
// of keeping a second copy of the frozen tables.

impl PresetName {
    /// Every distinct preset name of the frozen catalog, in catalog order:
    /// [`PresetName::parse`] accepts exactly these names.
    pub const ALL: [Self; 13] = crate::instrument_catalog::DISTINCT_PRESET_NAMES;
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

/// A JSON field that must be **present** but may be `null`.
///
/// `Option<T>` alone cannot say this: serde's derive treats an `Option` field
/// specially and turns a missing key into `None`, so a snapshot truncated after
/// the `chords` key would silently lose its `highlight` — the same defect class
/// as a misspelled key. Wrapping the field in this type takes it out of that
/// special case: the derive still demands the key, and only a genuinely absent
/// key is serde's standard ``missing field `highlight` `` error.
///
/// The manual impl reads the value through [`serde_json::Value`] — that is,
/// through `deserialize_any` — on purpose. For an absent key serde calls this
/// impl on its internal *missing-field* deserializer, whose `deserialize_option`
/// succeeds with `None` (it exists so that `Option` fields can default), while
/// every `deserialize_any`-based read fails with the missing-field error.
/// Delegating to `Option::<T>::deserialize` would therefore reintroduce the
/// silent default this type exists to prevent. JSON is the crate's only wire
/// format (`CORE-D04`), so buffering the present value there is not a loss.
///
/// Deliberately not `Default`: a default would let the derive fill in the
/// missing key again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RequiredOption<T>(Option<T>);

impl<'de, T> Deserialize<'de> for RequiredOption<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_null() {
            return Ok(Self(None));
        }
        // `Value` is a deserializer itself, so the buffered value runs through
        // `T`'s own strict reader; only its error type is translated.
        T::deserialize(value)
            .map(|value| Self(Some(value)))
            .map_err(de::Error::custom)
    }
}

/// The serialised shape of [`PageState`]; unknown fields are an error.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PageStateWire {
    instrument: InstrumentState,
    chords: Vec<ChordSpec>,
    highlight: RequiredOption<ChordSpec>,
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
            highlight: wire.highlight.0,
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
            tuning: guitar_standard_tuning(),
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
    if instrument_kind(instrument) == InstrumentKind::Keyboard {
        return Err(CoreError::invalid_state("instrument"));
    }
    Ok(TuningState {
        pitches: named_preset_pitches(instrument, *name)?.to_vec(),
        reference: *name,
    })
}

/// The committed tuning state of the guitar's Standard preset, which the default
/// page is anchored to (contract section 6).
///
/// The catalog's guitar Standard preset always exists, and the catalog tests pin
/// it against the oracle; the empty fallback only keeps the function total.
fn guitar_standard_tuning() -> TuningState {
    let reference = PresetName::from_catalog("Standard");
    TuningState {
        pitches: preset_pitches(InstrumentId::Guitar, reference)
            .map_or_else(Vec::new, <[OpenPitch]>::to_vec),
        reference,
    }
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
            if instrument_kind(*instrument) == InstrumentKind::Keyboard {
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
    let strings = instrument_strings(instrument)?;
    if tuning.pitches.len() != usize::from(strings) {
        return Err(CoreError::invalid_state("tuning"));
    }
    if preset_pitches(instrument, tuning.reference).is_none() {
        return Err(CoreError::invalid_state("reference"));
    }
    validate_positions(strings, selected)
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
    let same_arity: Vec<&Instrument> = fretted_instruments()
        .iter()
        .filter(|instrument| instrument.strings.map(usize::from) == Some(string_count))
        .collect();
    if same_arity.is_empty() {
        return Err(CoreError::invalid_state("tuning"));
    }
    let reference_belongs_to_a_same_count_instrument = same_arity
        .iter()
        .any(|instrument| preset_pitches(instrument.id, tuning.reference).is_some());
    if !reference_belongs_to_a_same_count_instrument {
        return Err(CoreError::invalid_state("reference"));
    }
    Ok(())
}

/// Check a piano selection: every key within 48..=83, unique and ascending.
fn validate_piano_selection(selected: &[OpenPitch]) -> Result<(), CoreError> {
    let (lowest, highest) = keyboard_pitch_range();
    let lowest = u8::from(lowest);
    let highest = u8::from(highest);
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
fn validate_positions(strings: u8, selected: &[Position]) -> Result<(), CoreError> {
    let mut previous: Option<StringIndex> = None;
    for position in selected {
        if u8::from(position.string) >= strings {
            return Err(CoreError::out_of_range("selection"));
        }
        if previous.is_some_and(|previous| position.string <= previous) {
            return Err(CoreError::invalid_state("selection"));
        }
        previous = Some(position.string);
    }
    Ok(())
}
