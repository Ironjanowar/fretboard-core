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
    chord_details_to_dto, chord_from_dto, chord_slots_to_dto, fretted_surface_to_dto,
    instrument_definitions_to_dto, keyboard_surface_to_dto, page_event_from_dto,
    page_state_from_dto, page_state_to_dto, quality_groups_to_dto,
};
use crate::dto::{
    AdapterError, ChordDetailsDto, ChordDto, FrettedSurfaceDto, InstrumentDefinitionDto,
    KeyboardSurfaceDto, PageEventDto, PageStateDto, QualityGroupDto,
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
