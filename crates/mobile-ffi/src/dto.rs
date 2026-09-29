//! The adapter's own typed values (P1, task C04).
//!
//! These records and enums are the values a native client sees; they are
//! described independently of the domain's types on purpose, so a generated
//! binding never depends on an internal domain rewrite. Every musical field
//! crosses the boundary as a plain wire value — a frozen identifier string, an
//! integer pitch — and never as a platform enum ordinal. The adapter owns no
//! note table and no formula table: [`crate::convert`] moves every value
//! between these types and `fretboard_core`.
//!
//! The frozen wire strings are the catalog values of
//! `fixtures/oracle/catalogs.json`, not display labels: the identifier enums
//! round-trip byte-for-byte ([`InstrumentDto::as_str`],
//! [`InstrumentDto::parse`], [`TabDto::as_str`], [`TabDto::parse`]).
//!
//! Scope is the P1 slice only: `default_state`, `chord_details` and
//! `validate_state`. There is no string-in/string-out dispatcher here
//! (`04-core-phases.md` task C04 forbids a JSON `execute` blob), and the reducer
//! arrives with a later task.
//!
//! Every DTO that crosses the boundary carries its UniFFI metadata:
//! [`uniffi::Record`] on the plain structs, [`uniffi::Enum`] on the enums and
//! [`uniffi::Error`] on [`AdapterError`]. That is what lets `C05` generate
//! Kotlin from the compiled library instead of from hand-written definitions.

use std::fmt;

/// A frozen adapter error code (`02-core-contract.md` section 8).
///
/// The variant name *is* the wire code: [`ErrorCode::as_str`] returns exactly
/// the string a client branches on, so no client parses an English message to
/// find out what happened. The set matches the domain's stable codes one for
/// one, including `UnsupportedCapability`: a capability a later task has not
/// implemented yet reports that explicit pending code instead of masquerading as
/// an invalid request or answering with a plausible wrong result. The chord
/// catalog is complete since `C06`, so no chord quality reaches a client through
/// it any more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// The value is structurally inconsistent.
    InvalidState,
    /// The string is not one of the stable catalog identifiers.
    UnknownIdentifier,
    /// A numeric value is outside the range its position allows.
    OutOfRange,
    /// The action cannot apply to the current state.
    InvalidAction,
    /// The input is not a syntactically valid URL.
    InvalidUrl,
    /// The URL origin is outside the configured share policy.
    UnsupportedOrigin,
    /// The input is larger than the accepted import limit.
    InputTooLarge,
    /// The snapshot is corrupt, truncated or of the wrong shape.
    InvalidSnapshot,
    /// The snapshot declares a schema version this build cannot read.
    UnsupportedSchemaVersion,
    /// A known catalog capability this engine build does not implement yet.
    UnsupportedCapability,
}

impl ErrorCode {
    /// The frozen wire string of this code.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidState => "InvalidState",
            Self::UnknownIdentifier => "UnknownIdentifier",
            Self::OutOfRange => "OutOfRange",
            Self::InvalidAction => "InvalidAction",
            Self::InvalidUrl => "InvalidUrl",
            Self::UnsupportedOrigin => "UnsupportedOrigin",
            Self::InputTooLarge => "InputTooLarge",
            Self::InvalidSnapshot => "InvalidSnapshot",
            Self::UnsupportedSchemaVersion => "UnsupportedSchemaVersion",
            Self::UnsupportedCapability => "UnsupportedCapability",
        }
    }
}

/// A typed adapter failure: one frozen code plus an English message.
///
/// The variants are the codes, so the error *is* the code a client branches on
/// — [`AdapterError::code`] returns the matching [`ErrorCode`] and
/// [`AdapterError::message`] the domain's own English sentence. UniFFI 0.32.2
/// derives error metadata for enums only, which is why this type is an enum
/// rather than the three-field struct of the first delivery: a struct error has
/// no UniFFI metadata, so `#[uniffi::export]` refuses it and the binding
/// generator would have nothing to read.
///
/// `field` names the offending part of the input when the domain can name one
/// (`"quality"`, `"selection"`, `"highlight"`, …). It is diagnostic detail for
/// logs and messages, never a value a client branches on — the code is.
///
/// The `Debug` rendering is the frozen code plus the message; the variant name
/// is never parsed by a client.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Error)]
pub enum AdapterError {
    /// The value is structurally inconsistent.
    InvalidState {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// The string is not one of the stable catalog identifiers.
    UnknownIdentifier {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// A numeric value is outside the range its position allows.
    OutOfRange {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// The action cannot apply to the current state.
    InvalidAction {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// The input is not a syntactically valid URL.
    InvalidUrl {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// The URL origin is outside the configured share policy.
    UnsupportedOrigin {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// The input is larger than the accepted import limit.
    InputTooLarge {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// The snapshot is corrupt, truncated or of the wrong shape.
    InvalidSnapshot {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// The snapshot declares a schema version this build cannot read.
    UnsupportedSchemaVersion {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
    /// A known catalog capability this engine build does not implement yet.
    UnsupportedCapability {
        /// The English sentence of the failure, as the domain phrased it.
        sentence: String,
        /// The offending field, when the domain names one.
        field: Option<String>,
    },
}

impl AdapterError {
    /// The frozen stable code of this failure.
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::InvalidState { .. } => ErrorCode::InvalidState,
            Self::UnknownIdentifier { .. } => ErrorCode::UnknownIdentifier,
            Self::OutOfRange { .. } => ErrorCode::OutOfRange,
            Self::InvalidAction { .. } => ErrorCode::InvalidAction,
            Self::InvalidUrl { .. } => ErrorCode::InvalidUrl,
            Self::UnsupportedOrigin { .. } => ErrorCode::UnsupportedOrigin,
            Self::InputTooLarge { .. } => ErrorCode::InputTooLarge,
            Self::InvalidSnapshot { .. } => ErrorCode::InvalidSnapshot,
            Self::UnsupportedSchemaVersion { .. } => ErrorCode::UnsupportedSchemaVersion,
            Self::UnsupportedCapability { .. } => ErrorCode::UnsupportedCapability,
        }
    }

    /// The English sentence of this failure, as the domain phrased it.
    ///
    /// The field is called `sentence` rather than `message` on purpose: the
    /// generated Kotlin subclass of an error inherits `Throwable.message`, and a
    /// variant field named `message` makes the generated binding ambiguous and
    /// un-compilable under the pinned UniFFI 0.32.2. UniFFI still renders the
    /// fields into Kotlin's own `message` property.
    pub fn sentence(&self) -> &str {
        match self {
            Self::InvalidState { sentence, .. }
            | Self::UnknownIdentifier { sentence, .. }
            | Self::OutOfRange { sentence, .. }
            | Self::InvalidAction { sentence, .. }
            | Self::InvalidUrl { sentence, .. }
            | Self::UnsupportedOrigin { sentence, .. }
            | Self::InputTooLarge { sentence, .. }
            | Self::InvalidSnapshot { sentence, .. }
            | Self::UnsupportedSchemaVersion { sentence, .. }
            | Self::UnsupportedCapability { sentence, .. } => sentence,
        }
    }

    /// The offending field, when the domain names one.
    pub fn field(&self) -> Option<&str> {
        match self {
            Self::InvalidState { field, .. }
            | Self::UnknownIdentifier { field, .. }
            | Self::OutOfRange { field, .. }
            | Self::InvalidAction { field, .. }
            | Self::InvalidUrl { field, .. }
            | Self::UnsupportedOrigin { field, .. }
            | Self::InputTooLarge { field, .. }
            | Self::InvalidSnapshot { field, .. }
            | Self::UnsupportedSchemaVersion { field, .. }
            | Self::UnsupportedCapability { field, .. } => field.as_deref(),
        }
    }

    /// Build a failure from its stable code, its English message and an
    /// optional field name.
    ///
    /// The code selects the variant, so a call site cannot pair a variant with
    /// a code that contradicts it.
    pub fn new(code: ErrorCode, sentence: impl Into<String>, field: Option<String>) -> Self {
        let sentence = sentence.into();
        match code {
            ErrorCode::InvalidState => Self::InvalidState { sentence, field },
            ErrorCode::UnknownIdentifier => Self::UnknownIdentifier { sentence, field },
            ErrorCode::OutOfRange => Self::OutOfRange { sentence, field },
            ErrorCode::InvalidAction => Self::InvalidAction { sentence, field },
            ErrorCode::InvalidUrl => Self::InvalidUrl { sentence, field },
            ErrorCode::UnsupportedOrigin => Self::UnsupportedOrigin { sentence, field },
            ErrorCode::InputTooLarge => Self::InputTooLarge { sentence, field },
            ErrorCode::InvalidSnapshot => Self::InvalidSnapshot { sentence, field },
            ErrorCode::UnsupportedSchemaVersion => {
                Self::UnsupportedSchemaVersion { sentence, field }
            }
            ErrorCode::UnsupportedCapability => Self::UnsupportedCapability { sentence, field },
        }
    }
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code().as_str(), self.sentence())
    }
}

impl std::error::Error for AdapterError {}

/// A chord identity crossing the boundary: a root name and a quality
/// identifier.
///
/// Equal note sets are still different chords (`C6` versus `Amin7`), so the
/// identity is the pair, not the pitches it names.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ChordDto {
    /// The chord root as a note name, e.g. `C#`.
    ///
    /// The wire spelling is one of the twelve sharp names; the flat aliases are
    /// a domain lookup and are rejected here (`CORE-D06`).
    pub root: String,
    /// The chord quality identifier, e.g. `major`.
    pub quality: String,
}

/// A committed tuning crossing the boundary: the exact pitch of every string
/// plus the preset it is anchored to.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TuningDto {
    /// The absolute pitch of every physical string, in physical string order.
    pub pitches: Vec<u8>,
    /// The preset name the pitches are anchored to.
    pub reference: String,
}

/// One fretted selection entry: a string and the fret marked on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct PositionDto {
    /// The physical string index.
    pub string: u8,
    /// The fret on that string.
    pub fret: u8,
}

/// A catalog instrument, in catalog display order.
///
/// `Ukelele` keeps the frozen single-`e` wire spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum InstrumentDto {
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

impl InstrumentDto {
    /// The stable wire identifier of this instrument.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Guitar => "guitar",
            Self::Bass4 => "bass_4",
            Self::Bass5 => "bass_5",
            Self::Ukelele => "ukelele",
            Self::Piano => "piano",
        }
    }

    /// Parse one of the stable catalog wire identifiers.
    ///
    /// # Errors
    ///
    /// [`ErrorCode::UnknownIdentifier`] when the value is not one of the five
    /// catalog identifiers.
    pub fn parse(value: &str) -> Result<Self, AdapterError> {
        match value {
            "guitar" => Ok(Self::Guitar),
            "bass_4" => Ok(Self::Bass4),
            "bass_5" => Ok(Self::Bass5),
            "ukelele" => Ok(Self::Ukelele),
            "piano" => Ok(Self::Piano),
            _ => Err(AdapterError::new(
                ErrorCode::UnknownIdentifier,
                "unknown catalog identifier: instrument",
                Some("instrument".to_string()),
            )),
        }
    }
}

/// The active page tab, in the contract order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum TabDto {
    /// The visualizer tab.
    Visualizer,
    /// The analyzer tab.
    Analyzer,
}

impl TabDto {
    /// The stable wire string of this tab.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Visualizer => "visualizer",
            Self::Analyzer => "analyzer",
        }
    }

    /// Parse one of the two stable tab wire strings.
    ///
    /// # Errors
    ///
    /// [`ErrorCode::UnknownIdentifier`] when the value is not `visualizer` or
    /// `analyzer`.
    pub fn parse(value: &str) -> Result<Self, AdapterError> {
        match value {
            "visualizer" => Ok(Self::Visualizer),
            "analyzer" => Ok(Self::Analyzer),
            _ => Err(AdapterError::new(
                ErrorCode::UnknownIdentifier,
                "unknown catalog identifier: tab",
                Some("tab".to_string()),
            )),
        }
    }
}

/// The instrument part of a page: a fretted instrument with its tuning and
/// positions, or the piano with its absolute keys.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum InstrumentStateDto {
    /// A fretted instrument: guitar, bass or ukulele.
    Fretted {
        /// The instrument.
        instrument: InstrumentDto,
        /// The committed tuning.
        tuning: TuningDto,
        /// The marked positions, unique and ascending by physical string.
        selected: Vec<PositionDto>,
    },
    /// The piano, selected by absolute pitch.
    Piano {
        /// The selected keys, unique and ascending, within 48..=83.
        selected: Vec<u8>,
    },
}

/// The durable state of one page crossing the boundary.
///
/// `chords` is a list of occurrences, not a set: repetitions and their order
/// are part of the contract, and `highlight` is an identity that must occur in
/// `chords` when it is present.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PageStateDto {
    /// The instrument and its kind-specific selection.
    pub instrument: InstrumentStateDto,
    /// The active chords as ordered occurrences.
    pub chords: Vec<ChordDto>,
    /// The highlighted chord identity, which must occur in `chords`.
    pub highlight: Option<ChordDto>,
    /// The active tab.
    pub tab: TabDto,
}

/// The derived details of one chord, with chord-root and note names as strings
/// so the value compares byte-for-byte with the frozen oracle.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ChordDetailsDto {
    /// The chord root the details were derived from.
    pub root: String,
    /// The chord quality identifier the details were derived from.
    pub quality: String,
    /// The full label, e.g. `Cmaj`.
    pub label: String,
    /// The chord members as note names, in formula order.
    pub notes: Vec<String>,
    /// The contextual role of each member, in the same order as `notes`.
    pub interval_labels: Vec<String>,
}

/// The kind of a catalog instrument: a fretted one or the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum InstrumentKindDto {
    /// A fretted instrument: strings, frets and a tuning.
    Fretted,
    /// The keyboard: absolute pitches and no tuning.
    Keyboard,
}

/// One catalog instrument definition crossing the boundary: what the picker
/// lists and what a surface may assume.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct InstrumentDefinitionDto {
    /// The instrument, as the stable catalog identifier.
    pub instrument: InstrumentDto,
    /// The display name of the instrument.
    pub name: String,
    /// The kind of instrument.
    pub kind: InstrumentKindDto,
    /// The number of physical strings, and zero for the keyboard.
    pub strings: u8,
    /// The number of frets, and absent for the keyboard.
    pub frets: Option<u8>,
    /// The pitches of the instrument's standard tuning, in physical string order,
    /// and empty for the keyboard.
    pub standard_pitches: Vec<u8>,
}

/// One chord quality of the frozen catalog: its stable identifier and the label
/// the page shows for it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QualityDto {
    /// The quality identifier, e.g. `major`.
    pub quality: String,
    /// The display/wire label of the quality, e.g. `maj`.
    pub label: String,
}

/// One group of chord qualities, in catalog display order.
///
/// The groups are the page's own; no client curates a subset of them.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct QualityGroupDto {
    /// The group's display name.
    pub group: String,
    /// The group's qualities, in display order.
    pub qualities: Vec<QualityDto>,
}

/// One page event crossing the boundary.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PageEventDto {
    /// Add one occurrence of a chord, unless that identity is already present.
    AddChord {
        /// The chord to add.
        chord: ChordDto,
    },
    /// Remove the occurrence at this index.
    RemoveChord {
        /// The index of the occurrence to remove.
        index: u64,
    },
    /// Remove every chord and clear the highlight, keeping everything else.
    ClearAllChords,
    /// Highlight the occurrence at this index, or clear the highlight when that
    /// occurrence already carries it.
    HighlightChord {
        /// The index of the occurrence the tap landed on.
        index: u64,
    },
    /// Mark a position: add it, remove it when it already carries that fret, or
    /// replace the fret of that string.
    ToggleNote {
        /// The marked position.
        position: PositionDto,
    },
    /// Clear the selection, keeping the instrument, its tuning and the chords.
    ClearSelection,
    /// Switch to this tab.
    SetTab {
        /// The target tab.
        tab: TabDto,
    },
    /// Change the instrument, keeping the parts of the selection that still fit.
    SetInstrument {
        /// The new instrument.
        instrument: InstrumentDto,
    },
    /// Commit a tuning draft: set the committed tuning, and nothing else.
    ///
    /// The draft itself is UI-only (`02-core-contract.md` section 8: *"Tuning
    /// draft operations live outside `PageState`; callers commit only on
    /// Apply"*). A client holds it with [`crate::open_tuning_draft`],
    /// [`crate::select_tuning_preset`] and [`crate::change_tuning_string`], and
    /// hands the edited value over here on Apply.
    CommitTuning {
        /// The edited tuning to commit.
        tuning: TuningDto,
    },
}

/// One identification of a chord answer, as the analyzer shows it.
///
/// The notes are in formula order and the interval labels in their own order,
/// deliberately not zipped (`Contract.D01`); the list of interval labels with a
/// member the input lacks is [`Self::missing_intervals`], and
/// [`Self::incomplete`] says whether the missing-member styling applies.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct InterpretationDto {
    /// The chord root.
    pub root: String,
    /// The chord quality identifier.
    pub quality: String,
    /// Whether the input set equals the formula's interval set.
    pub exact: bool,
    /// Whether the input is a subset of the formula missing at most two tones.
    pub incomplete: bool,
    /// The chord members, in formula order.
    pub notes: Vec<String>,
    /// The contextual role of each member, in label order.
    pub intervals: Vec<String>,
    /// The contextual labels of the formula tones the input lacks.
    pub missing_intervals: Vec<String>,
    /// The bass note the answer was seen from.
    pub bass: String,
    /// The inversion `0..=6`, or `None` when the bass is not a chord tone the
    /// baseline maps to an inversion.
    pub inversion: Option<u8>,
    /// The slash label: the plain label in root position or for a non-chord
    /// bass, and the label with `/{bass}` appended otherwise.
    pub slash_label: String,
}

/// What a selection is: the analyzer's answer (`02-core-contract.md` section 8).
///
/// `Empty` is a computed empty analysis; a missing analysis is the *absent*
/// `None` the analyzer tab gate answers on the visualizer tab, which is a
/// different thing (`Evaluation.analysis` is optional and absent on visualizer).
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum AnalysisDto {
    /// Nothing is selected.
    Empty,
    /// One pitch class at one height.
    Single {
        /// The note.
        note: String,
    },
    /// One class at distinct heights, or two classes seen from their lowest
    /// heights.
    Interval {
        /// The lower note name.
        low: String,
        /// The higher note name. Equal to `low` for the one-class case.
        high: String,
        /// The simple interval name between them (`Octave` across octaves).
        label: String,
    },
    /// Three or more pitch classes.
    Chords {
        /// The representatives, in ascending sounding order.
        notes: Vec<String>,
        /// The note of the lowest sounding pitch.
        bass: String,
        /// Every identification, in the baseline's order.
        interpretations: Vec<InterpretationDto>,
    },
}

/// What fills one note of a surface.
///
/// The slot is an index into the active chord list; the client owns the palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum NoteFillDto {
    /// The note is filled with the colour of this slot.
    Slot {
        /// The active-list index whose colour fills the note.
        slot: u64,
    },
    /// The note is filled with the overlap colour.
    Overlap,
}

/// One position of a fretted surface.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SurfaceCellDto {
    /// The fret of the position, from the open string to the last fret.
    pub fret: u8,
    /// The note the position carries.
    pub note: String,
    /// The colour slot of every active chord that claims the note, in active
    /// order and with repeats.
    pub memberships: Vec<u64>,
    /// What fills the note.
    pub fill: NoteFillDto,
}

/// One row of a fretted surface: the cells of one physical string.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct SurfaceRowDto {
    /// The cells of the row, in fret order.
    pub cells: Vec<SurfaceCellDto>,
}

/// The fretted surface of a page: one row per string, in physical string order.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FrettedSurfaceDto {
    /// The rows of the surface.
    pub rows: Vec<SurfaceRowDto>,
}

/// One key of a keyboard surface.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct KeyboardKeyDto {
    /// The absolute pitch of the key.
    pub pitch: u8,
    /// The note the key carries.
    pub note: String,
    /// The colour slot of every active chord that claims the note.
    pub memberships: Vec<u64>,
    /// What fills the key.
    pub fill: NoteFillDto,
}

/// The keyboard surface of a page: one key per pitch of the range, in pitch order.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct KeyboardSurfaceDto {
    /// The keys of the surface.
    pub keys: Vec<KeyboardKeyDto>,
}
