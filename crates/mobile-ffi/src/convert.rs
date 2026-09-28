//! Conversions between the adapter DTOs and the domain types (P1, task C04).
//!
//! Every musical decision stays in `fretboard_core`: this module only moves
//! values across the boundary and translates the domain's failure into the
//! adapter's typed one. A root or note name is looked up in the domain's note
//! table ([`note_index`]) and a quality, preset or instrument identifier in the
//! domain's catalog identifiers ([`QualityId::parse`], [`PresetName::parse`]) —
//! the adapter never carries a second table of its own.
//!
//! Conversion and validation are separate steps on purpose: the boundary maps
//! a DTO into the domain's typed state, asks the domain to validate *that*
//! value ([`fretboard_core::validate_state`]) and only then projects it back,
//! so the value a client gets is exactly the domain's canonical value.

use fretboard_core::{
    ChordDetails, ChordSpec, CoreError, Fret, InstrumentId, InstrumentState, OpenPitch, PageState,
    Position, PresetName, QualityId, StringIndex, Tab, TuningState, note_index,
};

use crate::dto::{
    AdapterError, ChordDetailsDto, ChordDto, ErrorCode, InstrumentDto, InstrumentStateDto,
    PageStateDto, PositionDto, TabDto, TuningDto,
};

impl From<CoreError> for AdapterError {
    /// Map a domain failure onto the adapter's typed failure.
    ///
    /// The code is the matching [`ErrorCode`] and the message is the domain's
    /// own English sentence; the domain's field name is carried through as
    /// diagnostic detail. The mapping matches on the domain *variant*, not on
    /// its string code, so a new domain variant fails this build instead of
    /// being silently mis-mapped. An unimplemented capability keeps its own
    /// code and can therefore never surface as `InvalidAction`.
    fn from(error: CoreError) -> Self {
        let code = match &error {
            CoreError::InvalidState(_) => ErrorCode::InvalidState,
            CoreError::UnknownIdentifier(_) => ErrorCode::UnknownIdentifier,
            CoreError::OutOfRange(_) => ErrorCode::OutOfRange,
            CoreError::InvalidAction(_) => ErrorCode::InvalidAction,
            CoreError::InvalidUrl(_) => ErrorCode::InvalidUrl,
            CoreError::UnsupportedOrigin(_) => ErrorCode::UnsupportedOrigin,
            CoreError::InputTooLarge(_) => ErrorCode::InputTooLarge,
            CoreError::InvalidSnapshot(_) => ErrorCode::InvalidSnapshot,
            CoreError::UnsupportedSchemaVersion(_) => ErrorCode::UnsupportedSchemaVersion,
            CoreError::UnsupportedCapability(_) => ErrorCode::UnsupportedCapability,
        };
        Self::new(code, error.to_string(), error.field().map(str::to_string))
    }
}

/// The domain chord identity of a chord DTO.
///
/// The root is looked up through the domain's note table (which also
/// recognizes the flat aliases) and the quality through the domain's stable
/// quality identifiers.
pub(crate) fn chord_from_dto(chord: &ChordDto) -> Result<ChordSpec, AdapterError> {
    Ok(ChordSpec {
        root: note_index(&chord.root).map_err(AdapterError::from)?,
        quality: QualityId::parse(&chord.quality).map_err(AdapterError::from)?,
    })
}

/// The chord DTO of a domain chord identity.
pub(crate) fn chord_to_dto(chord: &ChordSpec) -> ChordDto {
    ChordDto {
        root: chord.root.name().to_string(),
        quality: chord.quality.as_str().to_string(),
    }
}

/// The details DTO of domain chord details.
pub(crate) fn chord_details_to_dto(details: &ChordDetails) -> ChordDetailsDto {
    ChordDetailsDto {
        root: details.root.name().to_string(),
        quality: details.quality.as_str().to_string(),
        label: details.label.clone(),
        notes: details
            .notes
            .iter()
            .map(|note| note.name().to_string())
            .collect(),
        interval_labels: details
            .intervals
            .iter()
            .map(|label| (*label).to_string())
            .collect(),
    }
}

/// The domain page state of a page DTO, converted field by field and not yet
/// validated.
pub(crate) fn page_state_from_dto(state: &PageStateDto) -> Result<PageState, AdapterError> {
    Ok(PageState {
        instrument: instrument_state_from_dto(&state.instrument)?,
        chords: state
            .chords
            .iter()
            .map(chord_from_dto)
            .collect::<Result<Vec<_>, _>>()?,
        highlight: state.highlight.as_ref().map(chord_from_dto).transpose()?,
        tab: tab_from_dto(state.tab),
    })
}

/// The page DTO of a domain page state.
pub(crate) fn page_state_to_dto(state: &PageState) -> PageStateDto {
    PageStateDto {
        instrument: instrument_state_to_dto(&state.instrument),
        chords: state.chords.iter().map(chord_to_dto).collect(),
        highlight: state.highlight.as_ref().map(chord_to_dto),
        tab: tab_to_dto(state.tab),
    }
}

/// The domain instrument state of an instrument DTO.
fn instrument_state_from_dto(state: &InstrumentStateDto) -> Result<InstrumentState, AdapterError> {
    match state {
        InstrumentStateDto::Fretted {
            instrument,
            tuning,
            selected,
        } => Ok(InstrumentState::Fretted {
            instrument: instrument_from_dto(*instrument),
            tuning: tuning_from_dto(tuning)?,
            selected: selected
                .iter()
                .copied()
                .map(position_from_dto)
                .collect::<Result<Vec<_>, _>>()?,
        }),
        InstrumentStateDto::Piano { selected } => Ok(InstrumentState::Piano {
            selected: pitches_from_dto(selected)?,
        }),
    }
}

/// The instrument DTO of a domain instrument state.
fn instrument_state_to_dto(state: &InstrumentState) -> InstrumentStateDto {
    match state {
        InstrumentState::Fretted {
            instrument,
            tuning,
            selected,
        } => InstrumentStateDto::Fretted {
            instrument: instrument_to_dto(*instrument),
            tuning: tuning_to_dto(tuning),
            selected: selected.iter().copied().map(position_to_dto).collect(),
        },
        InstrumentState::Piano { selected } => InstrumentStateDto::Piano {
            selected: selected.iter().map(|pitch| u8::from(*pitch)).collect(),
        },
    }
}

/// The domain instrument identifier of an instrument DTO.
const fn instrument_from_dto(instrument: InstrumentDto) -> InstrumentId {
    match instrument {
        InstrumentDto::Guitar => InstrumentId::Guitar,
        InstrumentDto::Bass4 => InstrumentId::Bass4,
        InstrumentDto::Bass5 => InstrumentId::Bass5,
        InstrumentDto::Ukelele => InstrumentId::Ukelele,
        InstrumentDto::Piano => InstrumentId::Piano,
    }
}

/// The instrument DTO of a domain instrument identifier.
const fn instrument_to_dto(instrument: InstrumentId) -> InstrumentDto {
    match instrument {
        InstrumentId::Guitar => InstrumentDto::Guitar,
        InstrumentId::Bass4 => InstrumentDto::Bass4,
        InstrumentId::Bass5 => InstrumentDto::Bass5,
        InstrumentId::Ukelele => InstrumentDto::Ukelele,
        InstrumentId::Piano => InstrumentDto::Piano,
    }
}

/// The domain tuning state of a tuning DTO.
fn tuning_from_dto(tuning: &TuningDto) -> Result<TuningState, AdapterError> {
    Ok(TuningState {
        pitches: pitches_from_dto(&tuning.pitches)?,
        reference: PresetName::parse(&tuning.reference).map_err(AdapterError::from)?,
    })
}

/// The tuning DTO of a domain tuning state.
fn tuning_to_dto(tuning: &TuningState) -> TuningDto {
    TuningDto {
        pitches: tuning
            .pitches
            .iter()
            .map(|pitch| u8::from(*pitch))
            .collect(),
        reference: tuning.reference.as_str().to_string(),
    }
}

/// The domain position of a position DTO.
fn position_from_dto(position: PositionDto) -> Result<Position, AdapterError> {
    Ok(Position {
        string: StringIndex::try_from(position.string).map_err(AdapterError::from)?,
        fret: Fret::try_from(position.fret).map_err(AdapterError::from)?,
    })
}

/// The position DTO of a domain position.
fn position_to_dto(position: Position) -> PositionDto {
    PositionDto {
        string: u8::from(position.string),
        fret: u8::from(position.fret),
    }
}

/// The domain open pitches of a wire pitch list.
fn pitches_from_dto(pitches: &[u8]) -> Result<Vec<OpenPitch>, AdapterError> {
    pitches
        .iter()
        .map(|pitch| OpenPitch::try_from(*pitch))
        .collect::<Result<Vec<_>, CoreError>>()
        .map_err(AdapterError::from)
}

/// The domain tab of a tab DTO.
const fn tab_from_dto(tab: TabDto) -> Tab {
    match tab {
        TabDto::Visualizer => Tab::Visualizer,
        TabDto::Analyzer => Tab::Analyzer,
    }
}

/// The tab DTO of a domain tab.
const fn tab_to_dto(tab: Tab) -> TabDto {
    match tab {
        Tab::Visualizer => TabDto::Visualizer,
        Tab::Analyzer => TabDto::Analyzer,
    }
}
