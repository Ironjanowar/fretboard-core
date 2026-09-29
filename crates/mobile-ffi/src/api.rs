//! The exported P1 adapter entry points (task C04).
//!
//! Every function here is a thin projection over `fretboard_core`: it converts
//! the DTOs, calls the domain, and converts back. No musical rule lives in this
//! module — a missing capability is reported by the domain as
//! `UnsupportedCapability` and reaches the client as the same typed code.
//!
//! Each entry point is `#[uniffi::export]`ed and takes its DTOs by value, which
//! is the shape a generated binding calls: the foreign side owns its argument
//! and the Rust side consumes it. `setup_scaffolding!()` in the crate root
//! emits the metadata and FFI symbols `C05` reads from the compiled library.

// The exported entry points take their DTOs by value on purpose: that is the
// shape a generated UniFFI binding calls, so the foreign side hands ownership
// over. The Rust body only reads the argument, which is exactly what
// `needless_pass_by_value` would otherwise flag.
#![allow(clippy::needless_pass_by_value)]

use crate::convert::{
    chord_details_to_dto, chord_from_dto, chord_mode_from_dto, chord_slots_to_dto, chord_to_dto,
    diatonic_chord_to_dto, fretted_surface_to_dto, instrument_definitions_to_dto, key_row_to_dto,
    key_suggestion_from_dto, keyboard_surface_to_dto, multi_key_group_to_dto, page_event_from_dto,
    page_state_from_dto, page_state_to_dto, progression_from_wire, progression_group_to_dto,
    quality_groups_to_dto, scale_from_wire, tonic_from_wire,
};
use fretboard_core::{SoundingPitch, StringIndex};

use crate::convert::{analysis_to_dto, instrument_from_dto, tuning_from_dto};
use crate::dto::{
    AdapterError, AnalysisDto, ChordDetailsDto, ChordDto, ChordModeDto, FrettedSurfaceDto,
    InstrumentDefinitionDto, InstrumentDto, KeyRowDto, KeySuggestionDto, KeyboardSurfaceDto,
    MultiKeyGroupDto, PageEventDto, PageStateDto, ProgressionGroupDto, QualityGroupDto, TuningDto,
};

/// The default page: guitar, Standard tuning, no chords, no highlight, the
/// visualizer tab.
///
/// The value is the domain's default, projected onto the adapter's DTOs.
#[uniffi::export]
pub fn default_state() -> PageStateDto {
    page_state_to_dto(&fretboard_core::default_state())
}

/// Validate a page DTO and return its canonical form.
///
/// The DTO is converted into the domain's typed state, the domain validates it
/// and the validated state is projected back. A state that the domain accepts
/// therefore comes back unchanged in meaning: repetitions and their order are
/// kept, an absent highlight stays absent, and no value is invented.
///
/// # Errors
///
/// [`AdapterError`] with the domain's stable code: `InvalidState` for a
/// structurally impossible state (a highlight that does not occur in the
/// chords, a wrong pitch count, duplicate strings), `OutOfRange` for a value
/// outside its instrument's range, `UnknownIdentifier` for an identifier
/// outside the frozen catalog.
#[uniffi::export]
pub fn validate_state(state: PageStateDto) -> Result<PageStateDto, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    fretboard_core::validate_state(&domain).map_err(AdapterError::from)?;
    Ok(page_state_to_dto(&domain))
}

/// The derived details of one chord: its label, its notes and their interval
/// roles.
///
/// # Errors
///
/// [`AdapterError`] with the domain's stable code: `UnknownIdentifier` when the
/// root is not one of the twelve sharp wire names or the quality is not a frozen
/// catalog identifier.
/// The signature keeps the typed failure because the contract carries a code
/// across the boundary; since `C06` implements the whole chord catalog, no
/// quality is pending any more, so `UnsupportedCapability` is reserved for the
/// capabilities a later task adds — never a plausible wrong chord.
#[uniffi::export]
pub fn chord_details(chord: ChordDto) -> Result<ChordDetailsDto, AdapterError> {
    let spec = chord_from_dto(&chord)?;
    let details = fretboard_core::chord_details(&spec).map_err(AdapterError::from)?;
    Ok(chord_details_to_dto(&details))
}

/// The catalog instruments, in catalog order.
///
/// The picker shows exactly these five, with the labels, string counts, fret
/// counts and standard pitches the frozen catalog carries; no client curates its
/// own list.
#[uniffi::export]
#[must_use]
pub fn instruments() -> Vec<InstrumentDefinitionDto> {
    instrument_definitions_to_dto()
}

/// The chord qualities, grouped as the page's picker shows them, in catalog
/// order.
#[uniffi::export]
#[must_use]
pub fn quality_groups() -> Vec<QualityGroupDto> {
    quality_groups_to_dto()
}

/// The preset names of an instrument, in catalog order; empty for the piano.
///
/// This is the enumeration a client's preset picker reads instead of carrying a
/// hand-written list of its own: the names are the frozen catalog's, in the
/// frozen order, per instrument, so a name a client offers is one
/// [`detect_tuning_preset`](crate::detect_tuning_preset) can answer and
/// [`select_tuning_preset`](crate::select_tuning_preset) can select. It closes
/// the gap `A09` hit: the adapter exported `detect_tuning_preset` but no way to
/// enumerate the names it could detect.
#[uniffi::export]
#[must_use]
pub fn presets(instrument: InstrumentDto) -> Vec<String> {
    fretboard_core::preset_names(instrument_from_dto(instrument))
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// Apply one page event and return the page it produces.
///
/// A tap acts on the state the client already holds: the event is converted, the
/// domain applies it, and the result is projected back. An event that changes
/// nothing returns the page it was given, unchanged in meaning, so a client can
/// compare the two and count its own patches.
///
/// # Errors
///
/// [`AdapterError`] with the domain's stable code and the same mapping
/// [`validate_state`] uses: `UnknownIdentifier` for an identifier outside the
/// frozen catalog, `OutOfRange` for an occurrence index this platform cannot
/// hold, `InvalidState` for a state the domain rejects.
#[uniffi::export]
pub fn apply_page_event(
    state: PageStateDto,
    event: PageEventDto,
) -> Result<PageStateDto, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    let next = fretboard_core::apply_event(&domain, &page_event_from_dto(&event)?);
    Ok(page_state_to_dto(&next))
}

/// The fretted surface of a page: one row per string, one cell per fret, each
/// carrying its note, the colour slot of every chord that claims it, and its
/// fill.
///
/// # Errors
///
/// [`AdapterError`] with `InvalidState` when the page is the piano, which has no
/// fretted surface, and the state conversion's own failures.
#[uniffi::export]
pub fn fretted_surface(state: PageStateDto) -> Result<FrettedSurfaceDto, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    fretted_surface_to_dto(&domain)
}

/// The keyboard surface of a page: one key per pitch of the instrument's range,
/// with the same memberships and fill a fretted cell carries.
///
/// # Errors
///
/// [`AdapterError`] with the state conversion's own failures: `UnknownIdentifier`
/// for an identifier outside the frozen catalog, `OutOfRange` for a value outside
/// its range, `InvalidState` for a state the domain rejects.
#[uniffi::export]
pub fn keyboard_surface(state: PageStateDto) -> Result<KeyboardSurfaceDto, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    Ok(keyboard_surface_to_dto(&domain))
}

/// The colour slot of every active chord occurrence: a repeated identity shares
/// the slot of its first occurrence.
///
/// # Errors
///
/// [`AdapterError`] with the state conversion's own failures, exactly as
/// [`validate_state`] reports them.
#[uniffi::export]
pub fn chord_color_slots(state: PageStateDto) -> Result<Vec<u64>, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    Ok(chord_slots_to_dto(&domain))
}

/// The frozen baseline's label for a tuning that matches no preset.
const CUSTOM_PRESET_LABEL: &str = "Custom";

/// The note names of a tuning, one per physical string, in string order.
///
/// # Errors
///
/// [`AdapterError`] with `OutOfRange` when a pitch is outside `0..=127` and
/// `InvalidState` when the tuning is not valid for a fretted instrument.
#[uniffi::export]
pub fn tuning_notes(tuning: TuningDto) -> Result<Vec<String>, AdapterError> {
    let domain = tuning_from_dto(&tuning)?;
    Ok(crate::convert::tuning_notes_to_dto(&domain))
}

/// The preset a set of pitches matches exactly, or the baseline's own label for
/// a tuning that matches none.
///
/// Detection is exact: a tuning that differs from every preset of the instrument
/// by a single semitone is not the nearest preset. The frozen baseline answers
/// the literal `Custom` in that case (`Fretboard.Music.detect_preset/2`, pinned
/// by `fixtures/oracle/tunings.jsonl`, case
/// `detect_preset/guitar/Standard-semitone-shifted`), and this entry point
/// answers the same string, so a client never invents the label. The domain's own
/// API keeps the typed meaning (`Option<PresetName>`); only the wire carries it.
///
/// # Errors
///
/// [`AdapterError`] with `OutOfRange` for a pitch outside `0..=127`.
#[uniffi::export]
pub fn detect_tuning_preset(
    instrument: InstrumentDto,
    tuning: TuningDto,
) -> Result<String, AdapterError> {
    let domain = tuning_from_dto(&tuning)?;
    Ok(
        fretboard_core::detect_preset(instrument_from_dto(instrument), &domain.pitches)
            .map_or_else(
                || CUSTOM_PRESET_LABEL.to_owned(),
                |preset| preset.to_string(),
            ),
    )
}

/// Edit one string's note against the fixed reference of the tuning.
///
/// The note resolves to the pitch of its class nearest **that string's pitch in
/// the preset the tuning is anchored to**, never the string's current pitch, so
/// repeated edits cannot drift the anchor. The returned tuning is a new value:
/// nothing is mutated, which is what lets a client hold a draft and apply or
/// discard it without touching the page it came from.
///
/// # Errors
///
/// [`AdapterError`] with the domain's stable codes: `OutOfRange` for a string the
/// instrument does not have, `UnknownIdentifier` for a note name outside the
/// chromatic scale, `InvalidState` for the piano, a wrong pitch count or a
/// reference that is not a preset of the instrument.
#[uniffi::export]
pub fn change_tuning_note(
    instrument: InstrumentDto,
    tuning: TuningDto,
    string: u8,
    note: String,
) -> Result<TuningDto, AdapterError> {
    let domain = tuning_from_dto(&tuning)?;
    let edited = fretboard_core::change_tuning_note(
        instrument_from_dto(instrument),
        &domain,
        StringIndex::try_from(string).map_err(AdapterError::from)?,
        &note,
    )
    .map_err(AdapterError::from)?;
    Ok(crate::convert::tuning_to_dto(&edited))
}

/// The analysis of a page, or nothing when the page is on the visualizer tab.
///
/// The analysis is derived from the committed selection — a fretted position
/// through the committed tuning, a piano key as its absolute pitch — and never
/// from the stored chords, so the same answer comes back whatever the
/// visualizer holds. `None` is the contract's *absent* analysis
/// (`Evaluation.analysis` is optional and absent on visualizer); an empty
/// selection on the analyzer tab answers
/// [`AnalysisDto::Empty`](crate::AnalysisDto::Empty) instead.
///
/// # Errors
///
/// [`AdapterError`] with the state conversion's own failures: `UnknownIdentifier`
/// for an identifier outside the frozen catalog, `OutOfRange` for a value outside
/// its range, `InvalidState` for a state the domain rejects.
#[uniffi::export]
pub fn analyze_page(state: PageStateDto) -> Result<Option<AnalysisDto>, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    Ok(fretboard_core::analyze_page(&domain)
        .as_ref()
        .map(analysis_to_dto))
}

/// The analysis of absolute sounding pitches, independent of instrument.
///
/// This is the same analyzer the page reaches through [`analyze_page`]: the
/// piano's selected keys and a fretted selection reduced to their sounding
/// pitches analyze identically. A pitch outside `0..=151` is rejected rather
/// than clamped.
///
/// # Errors
///
/// [`AdapterError`] with `OutOfRange` for a pitch outside the sounding-pitch
/// range.
#[uniffi::export]
pub fn analyze_pitches(pitches: Vec<u16>) -> Result<AnalysisDto, AdapterError> {
    let sounding = pitches
        .into_iter()
        .map(SoundingPitch::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(AdapterError::from)?;
    Ok(analysis_to_dto(&fretboard_core::analyze_pitches(&sounding)))
}

/// The tuning draft a page opens, or nothing on the piano.
///
/// The draft is UI-only (`02-core-contract.md` section 8) and starts from the
/// committed tuning; editing it can never touch the page. A fretted page always
/// answers a draft, the piano never does — its tuning modal is ignored entirely.
///
/// # Errors
///
/// [`AdapterError`] with the state conversion's own failures.
#[uniffi::export]
pub fn open_tuning_draft(state: PageStateDto) -> Result<Option<TuningDto>, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    Ok(fretboard_core::open_tuning_draft(&domain)
        .as_ref()
        .map(crate::convert::tuning_to_dto))
}

/// Select a preset for a tuning draft.
///
/// The draft changes only when the name is a preset of that instrument; an
/// unknown name, or one that belongs to another instrument, comes back
/// unchanged, exactly as the baseline's ignored event does. The committed page
/// is not involved: this is draft math.
///
/// # Errors
///
/// [`AdapterError`] with `OutOfRange` for a pitch outside `0..=127`, and
/// `UnknownIdentifier` for a reference the catalog does not know.
#[uniffi::export]
pub fn select_tuning_preset(
    instrument: InstrumentDto,
    draft: TuningDto,
    preset: String,
) -> Result<TuningDto, AdapterError> {
    let domain = tuning_from_dto(&draft)?;
    Ok(crate::convert::tuning_to_dto(
        &fretboard_core::select_tuning_preset(instrument_from_dto(instrument), &domain, &preset),
    ))
}

/// Edit one string of a tuning draft.
///
/// The guards are the baseline handler's: the string index is the recorded text
/// and must be a string of the instrument, and the note must be one of the
/// twelve sharp names. An invalid value comes back as the unchanged draft, never
/// as an error and never as a wrong pitch; the edit resolves against the draft's
/// own reference, so repeated edits cannot drift.
///
/// # Errors
///
/// [`AdapterError`] with `OutOfRange` for a pitch outside `0..=127`, and
/// `UnknownIdentifier` for a reference the catalog does not know.
#[uniffi::export]
pub fn change_tuning_string(
    instrument: InstrumentDto,
    draft: TuningDto,
    string: String,
    note: String,
) -> Result<TuningDto, AdapterError> {
    let domain = tuning_from_dto(&draft)?;
    Ok(crate::convert::tuning_to_dto(
        &fretboard_core::change_tuning_string(
            instrument_from_dto(instrument),
            &domain,
            &string,
            &note,
        ),
    ))
}

/// The key rows of a page: the suggestions for the active chords, grouped the
/// way the panel shows them.
///
/// The whole decision is the domain's and crosses whole: the page's two-chord
/// gate (`KEY_PAGE_MIN_CHORDS`) answers nothing below two active chords, the
/// remaining suggestions are ordered by score and the collapsed groups are built
/// by the same grouping the pinned panel uses. Kotlin renders the rows it gets
/// and never re-scores, re-groups or re-sorts them.
///
/// # Errors
///
/// [`AdapterError`] with the state conversion's own failures: `UnknownIdentifier`
/// for a chord root outside the twelve sharp wire names, `OutOfRange` for a value
/// outside its range, `InvalidState` for a state the domain rejects.
#[uniffi::export]
pub fn key_suggestions(state: PageStateDto) -> Result<Vec<KeyRowDto>, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    Ok(
        fretboard_core::group_key_suggestions(&fretboard_core::page_key_suggestions(
            &domain.chords,
        ))
        .iter()
        .map(key_row_to_dto)
        .collect(),
    )
}

/// The key rows of a suggestion list a client already holds.
///
/// This is the pure grouping of [`key_suggestions`] without the page's own gate
/// and scoring: the input is a suggestion list (the answer of an earlier
/// [`key_suggestions`] call, or a cached one) and the output is the rows the
/// panel renders. A client that keeps a suggestion list therefore never
/// re-implements the grouping to redraw it.
///
/// # Errors
///
/// [`AdapterError`] with `UnknownIdentifier` for a tonic or scale outside the
/// catalog, and `OutOfRange` for a count that does not fit this platform.
#[uniffi::export]
pub fn group_key_suggestions(
    suggestions: Vec<KeySuggestionDto>,
) -> Result<Vec<KeyRowDto>, AdapterError> {
    let domain = suggestions
        .iter()
        .map(key_suggestion_from_dto)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(fretboard_core::group_key_suggestions(&domain)
        .iter()
        .map(key_row_to_dto)
        .collect())
}

/// The multi-key groups of a page, each with its full displayed chord
/// membership.
///
/// The domain's multi-key page gate decides whether the panel exists at all: the
/// raw operation answers nothing below three active chords, and the page shows it
/// only when the single-key operation answered nothing, so a page whose chords
/// share any key answers no groups. A group's `chords` is the key's full
/// membership — every input occurrence whose notes fit — so one chord can appear
/// in two groups, exactly as the pinned panel shows it.
///
/// # Errors
///
/// [`AdapterError`] with the state conversion's own failures, exactly as
/// [`key_suggestions`] reports them.
#[uniffi::export]
pub fn multi_key_suggestions(state: PageStateDto) -> Result<Vec<MultiKeyGroupDto>, AdapterError> {
    let domain = page_state_from_dto(&state)?;
    let single_keys = fretboard_core::page_key_suggestions(&domain.chords);
    Ok(
        fretboard_core::page_multi_key_suggestions(&single_keys, &domain.chords)
            .iter()
            .map(multi_key_group_to_dto)
            .collect(),
    )
}

/// The progression catalog, grouped as the picker shows it, in the frozen order.
///
/// Every definition crosses whole — identifier, name, category, genre,
/// description, example key, scale, degrees and notable songs — so the picker
/// and the modal read the catalog from the engine instead of carrying a copy.
#[uniffi::export]
#[must_use]
pub fn progressions() -> Vec<ProgressionGroupDto> {
    fretboard_core::grouped_progressions()
        .iter()
        .map(progression_group_to_dto)
        .collect()
}

/// The chords of one progression in one tonic, in playing order.
///
/// The list has one entry per degree, and a progression that repeats a chord
/// repeats it here too: the apply replaces the page's chords with this list
/// rather than merging into it. This is both the modal preview and the exact
/// list [`apply_page_event`] commits for
/// [`PageEventDto::CommitProgression`](crate::PageEventDto::CommitProgression).
///
/// # Errors
///
/// [`AdapterError`] with `UnknownIdentifier` for either argument: a tonic that
/// is not a note name, or a progression identifier the catalog does not carry.
#[uniffi::export]
pub fn progression_chords(
    tonic: String,
    progression: String,
) -> Result<Vec<ChordDto>, AdapterError> {
    let tonic = tonic_from_wire(&tonic)?;
    let progression = progression_from_wire(&progression)?;
    Ok(fretboard_core::progression_chords(tonic, progression)
        .iter()
        .map(chord_to_dto)
        .collect())
}

/// The diatonic chords of one key in the chosen mode, in scale order.
///
/// This is the key modal's preview: the same chords the key apply commits for
/// [`PageEventDto::CommitKeys`](crate::PageEventDto::CommitKeys) in the same
/// mode. The mode is the modal's own — triads or seventh chords.
///
/// # Errors
///
/// [`AdapterError`] with `UnknownIdentifier` for either a tonic that is not a
/// note name or a scale identifier the catalog does not carry.
#[uniffi::export]
pub fn diatonic_chords(
    tonic: String,
    scale: String,
    mode: ChordModeDto,
) -> Result<Vec<ChordDto>, AdapterError> {
    let tonic = tonic_from_wire(&tonic)?;
    let scale = scale_from_wire(&scale)?;
    Ok(
        fretboard_core::diatonic_chords(tonic, scale, chord_mode_from_dto(mode))
            .iter()
            .map(diatonic_chord_to_dto)
            .collect(),
    )
}
