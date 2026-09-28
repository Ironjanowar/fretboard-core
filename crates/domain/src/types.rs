//! Validated primitives and stable identifiers of the typed page-state
//! contract (`02-core-contract.md` sections 2 and 8).
//!
//! Every value of a [`crate::PageState`] is a validated type: the numeric
//! newtypes are built with `TryFrom`, the identifier types with `FromStr`, and
//! the `serde` implementations go through those same constructors. A
//! deserialised snapshot therefore can never carry a value the public API could
//! not have built.
//!
//! Type-level bounds are the catalog-independent absolute limits of the
//! contract (twelve pitch classes, open pitch 127, fret 24). Limits that depend
//! on the instrument (its string count, the 48..=83 piano range) belong to
//! validation, not to a primitive.
//!
//! Strings on the wire are the frozen identifiers of
//! `fixtures/oracle/catalogs.json`; no platform enum ordinal is ever a wire
//! value. Flat note names and the domain note lookup are `note.rs` (C03).

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::CoreError;

/// The number of pitch classes (contract section 2).
pub(crate) const PITCH_CLASS_COUNT: u8 = 12;

/// The twelve pitch-class names in contract order; display is sharp-only
/// (contract section 2).
pub(crate) const PITCH_CLASS_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// The highest absolute open pitch of the page boundary (contract section 2).
pub(crate) const MAX_OPEN_PITCH: u8 = 127;

/// The highest fret of every fretted instrument (contract section 2).
pub(crate) const MAX_FRET: u8 = 24;

/// The highest sounding pitch: open pitch 127 plus fret 24. Sounding pitches
/// are wider than 127 and are never clamped.
const MAX_SOUNDING_PITCH: u16 = 151;

/// A pitch class: an index into `C C# D D# E F F# G G# A A# B`, 0..=11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PitchClass(u8);

impl PitchClass {
    /// The sharp name of this pitch class.
    pub const fn name(self) -> &'static str {
        // The newtype's invariant is 0..=11, so this index cannot be out of
        // bounds; `indexing_slicing` is allowed for exactly that reason.
        #[allow(clippy::indexing_slicing)]
        let name = PITCH_CLASS_NAMES[self.0 as usize];
        name
    }

    /// The pitch class of one of the twelve sharp names; `None` for anything
    /// else (URL parsing accepts only sharp names, contract section 2).
    fn from_name(name: &str) -> Option<Self> {
        PITCH_CLASS_NAMES
            .iter()
            .position(|known| *known == name)
            .and_then(|index| u8::try_from(index).ok())
            .map(Self)
    }
}

impl TryFrom<u8> for PitchClass {
    type Error = CoreError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < PITCH_CLASS_COUNT {
            Ok(Self(value))
        } else {
            Err(CoreError::out_of_range("pitch_class"))
        }
    }
}

impl From<PitchClass> for u8 {
    fn from(pitch_class: PitchClass) -> Self {
        pitch_class.0
    }
}

impl Serialize for PitchClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for PitchClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let name = String::deserialize(deserializer)?;
        Self::from_name(&name)
            .ok_or_else(|| serde::de::Error::custom(CoreError::unknown_identifier("root")))
    }
}

/// An absolute open-string (MIDI) pitch, 0..=127.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct OpenPitch(u8);

impl OpenPitch {
    /// The table-internal constructor for the frozen catalog constants: the
    /// bound is asserted during const evaluation, so a mistranscribed pitch
    /// fails the build instead of becoming a runtime error.
    pub(crate) const fn from_catalog(value: u8) -> Self {
        assert!(value <= MAX_OPEN_PITCH, "open pitch out of range");
        Self(value)
    }
}

impl TryFrom<u8> for OpenPitch {
    type Error = CoreError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= MAX_OPEN_PITCH {
            Ok(Self(value))
        } else {
            Err(CoreError::out_of_range("open_pitch"))
        }
    }
}

impl From<OpenPitch> for u8 {
    fn from(open_pitch: OpenPitch) -> Self {
        open_pitch.0
    }
}

/// A sounding fret pitch: an open pitch plus a fret, which may exceed 127.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct SoundingPitch(u16);

impl TryFrom<u16> for SoundingPitch {
    type Error = CoreError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if value <= MAX_SOUNDING_PITCH {
            Ok(Self(value))
        } else {
            Err(CoreError::out_of_range("sounding_pitch"))
        }
    }
}

impl From<SoundingPitch> for u16 {
    fn from(sounding_pitch: SoundingPitch) -> Self {
        sounding_pitch.0
    }
}

/// The physical string of a fretted instrument, by index.
///
/// Every `u8` is a possible index; whether the instrument has that string is
/// decided against the instrument during validation (contract section 8), not
/// by the primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct StringIndex(u8);

impl TryFrom<u8> for StringIndex {
    type Error = CoreError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(Self(value))
    }
}

impl From<StringIndex> for u8 {
    fn from(string: StringIndex) -> Self {
        string.0
    }
}

/// A fret, 0..=24 on every fretted instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub struct Fret(u8);

impl TryFrom<u8> for Fret {
    type Error = CoreError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value <= MAX_FRET {
            Ok(Self(value))
        } else {
            Err(CoreError::out_of_range("fret"))
        }
    }
}

impl From<Fret> for u8 {
    fn from(fret: Fret) -> Self {
        fret.0
    }
}

/// A catalog instrument identifier, in catalog display order.
///
/// `Ukelele` keeps the frozen single-`e` spelling on the wire as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstrumentId {
    /// Guitar.
    Guitar,
    /// Bass (4-string).
    Bass4,
    /// Bass (5-string).
    Bass5,
    /// Ukulele.
    Ukelele,
    /// Piano.
    Piano,
}

impl InstrumentId {
    /// Every instrument of the frozen catalog, in catalog display order.
    pub(crate) const ALL: [Self; 5] = [
        Self::Guitar,
        Self::Bass4,
        Self::Bass5,
        Self::Ukelele,
        Self::Piano,
    ];

    /// The stable wire identifier.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guitar => "guitar",
            Self::Bass4 => "bass_4",
            Self::Bass5 => "bass_5",
            Self::Ukelele => "ukelele",
            Self::Piano => "piano",
        }
    }

    /// Whether this instrument has strings, frets and a tuning. The piano has
    /// none of them (contract section 2).
    pub(crate) const fn is_fretted(self) -> bool {
        !matches!(self, Self::Piano)
    }
}

impl FromStr for InstrumentId {
    type Err = CoreError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .iter()
            .copied()
            .find(|instrument| instrument.as_str() == value)
            .ok_or_else(|| CoreError::unknown_identifier("instrument"))
    }
}

impl fmt::Display for InstrumentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl Serialize for InstrumentId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for InstrumentId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let identifier = String::deserialize(deserializer)?;
        identifier
            .parse()
            .map_err(|error: CoreError| serde::de::Error::custom(error))
    }
}

/// The page tab, in the contract order (visualizer before analyzer).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tab {
    /// Visualizer.
    Visualizer,
    /// Analyzer.
    Analyzer,
}

/// The stable chord-quality identifiers in catalog order (`available_qualities`).
#[rustfmt::skip]
const QUALITY_IDS: [&str; 47] = [
    "11",
    "13",
    "13b9",
    "7",
    "7#11",
    "7#9",
    "7b13",
    "7b5",
    "7b9",
    "7b9b13",
    "7sus4",
    "9",
    "9#5",
    "9b5",
    "add9",
    "aug",
    "aug7",
    "aug_maj7",
    "dim",
    "dim7",
    "dim7b13",
    "dim_maj7",
    "m11b5",
    "m7b5",
    "m_add9",
    "maj11",
    "maj13",
    "maj6",
    "maj6_9",
    "maj7",
    "maj7#11",
    "maj9",
    "major",
    "min11",
    "min13",
    "min6",
    "min6_9",
    "min7",
    "min7b13",
    "min9",
    "min_maj7",
    "minor",
    "sus13",
    "sus2",
    "sus4",
    "sus9",
    "susb9",
];

/// The stable scale identifiers in catalog order (`available_scale_types`).
#[rustfmt::skip]
const SCALE_IDS: [&str; 15] = [
    "major",
    "minor",
    "harmonic_minor",
    "melodic_minor",
    "pentatonic_major",
    "pentatonic_minor",
    "blues",
    "dorian",
    "phrygian",
    "lydian",
    "mixolydian",
    "locrian",
    "phrygian_dominant",
    "whole_tone",
    "chromatic",
];

/// The canonical catalog string of a chord-quality identifier.
fn canonical_quality_id(value: &str) -> Option<&'static str> {
    QUALITY_IDS.iter().copied().find(|known| *known == value)
}

/// The canonical catalog string of a scale identifier.
fn canonical_scale_id(value: &str) -> Option<&'static str> {
    SCALE_IDS.iter().copied().find(|known| *known == value)
}

/// Declare one validated identifier type over the frozen catalog strings.
///
/// The type keeps the canonical catalog string of the frozen tables (so a
/// parsed value is never a caller-owned string), displays it unchanged, and
/// serialises to it. Parsing anything outside the table is
/// [`CoreError::UnknownIdentifier`] naming `$code_field`.
macro_rules! stable_identifier {
    ($(#[$doc:meta])* $name:ident, $code_field:literal, $canonical:path) => {
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(&'static str);

        impl $name {
            /// The stable catalog string of this identifier.
            pub const fn as_str(self) -> &'static str {
                self.0
            }

            /// Parse one of the stable catalog strings.
            ///
            /// # Errors
            ///
            /// [`CoreError::UnknownIdentifier`] when the value is not one of the
            /// stable strings of the frozen catalog.
            pub fn parse(value: &str) -> Result<Self, CoreError> {
                match $canonical(value) {
                    Some(known) => Ok(Self(known)),
                    None => Err(CoreError::unknown_identifier($code_field)),
                }
            }
        }

        impl FromStr for $name {
            type Err = CoreError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.0)
            }
        }

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.0)
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let text = String::deserialize(deserializer)?;
                Self::parse(&text).map_err(serde::de::Error::custom)
            }
        }
    };
}

/// Declare the exhaustive identifier list of a catalog-backed identifier type.
///
/// [`ALL`](Self::ALL) is generated from the same table the parser searches, so
/// the enumerated accepted set and the parsed accepted set cannot drift.
macro_rules! catalog_identifier_list {
    ($name:ident, $ids:ident) => {
        impl $name {
            /// Every accepted catalog identifier, in catalog order: `parse`
            /// accepts exactly these strings.
            pub const ALL: [Self; $ids.len()] = {
                let mut all = [Self($ids[0]); $ids.len()];
                let mut index = 0;
                while index < $ids.len() {
                    all[index] = Self($ids[index]);
                    index += 1;
                }
                all
            };
        }
    };
}

stable_identifier!(
    /// A validated chord-quality identifier; the display label of a quality is
    /// a catalog concern and is not this identifier.
    QualityId,
    "quality",
    canonical_quality_id
);

catalog_identifier_list!(QualityId, QUALITY_IDS);

stable_identifier!(
    /// A validated scale identifier.
    ScaleId,
    "scale",
    canonical_scale_id
);

catalog_identifier_list!(ScaleId, SCALE_IDS);

stable_identifier!(
    /// A validated instrument pitch-preset name.
    ///
    /// The accepted names are the union of the frozen catalog's instrument
    /// presets, so a name that is a real preset of one instrument parses even
    /// when it is not a preset of another; whether it belongs to a given
    /// instrument is a validation question (contract section 8).
    PresetName,
    "reference",
    crate::state::canonical_preset_name
);

impl PresetName {
    /// The table-internal constructor for the frozen catalog constants.
    ///
    /// The name is one of the `fixtures/oracle/catalogs.json` preset names; the
    /// catalog tables and the accepted-name lookup share this one list, so a
    /// mistyped entry changes both together and is caught by the oracle-pinned
    /// contract tests for the presets they exercise.
    pub(crate) const fn from_catalog(name: &'static str) -> Self {
        Self(name)
    }
}
