//! Typed foreign boundary for the Fretboard domain.
//!
//! This crate owns the DTOs, the conversions and the exported API that native
//! clients call through generated bindings. It contains no musical rule: every
//! semantic decision belongs to `fretboard_core`.
//!
//! P1 (task C04) exports the first typed surface, deliberately small:
//! [`default_state`], [`validate_state`] and [`chord_details`], with the DTOs of
//! [`dto`]. There is no string-in/string-out `execute` dispatcher — strings
//! cross the boundary only as encoded URLs or snapshots, in later tasks — and
//! no reducer yet.
//!
//! The generated binding package is configured by `uniffi.toml`
//! (`dev.ironjanowar.fretboard.core`); generated Kotlin is a build output of a
//! later task (`C05`) and is never hand-written or committed.
//!
//! The adapter is exportable through the pinned UniFFI (0.32.2): every entry
//! point carries `#[uniffi::export]`, the DTOs carry `uniffi::Record` /
//! `uniffi::Enum`, [`AdapterError`] carries `uniffi::Error` as an enum (the
//! pinned derive is implemented for enums only, so the struct error of the
//! first delivery could not be exported), and `uniffi::setup_scaffolding!()`
//! emits the metadata and FFI symbols the binding generator reads from
//! `libfretboard_mobile_ffi.so`.

#![forbid(unsafe_code)]

mod api;
mod convert;
mod dto;

// The UniFFI namespace is the crate's module path (`fretboard_mobile_ffi`),
// which is what the pinned generator expects for this cdylib; the generated
// package name is configured separately in `uniffi.toml`.
uniffi::setup_scaffolding!();

pub use api::{chord_details, default_state, validate_state};
pub use dto::{
    AdapterError, ChordDetailsDto, ChordDto, ErrorCode, InstrumentDto, InstrumentStateDto,
    PageStateDto, PositionDto, TabDto, TuningDto,
};
