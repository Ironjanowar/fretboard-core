//! Typed foreign boundary for the Fretboard domain.
//!
//! This crate owns the DTOs, the conversions and the exported API that native
//! clients call through generated bindings. It contains no musical rule: every
//! semantic decision belongs to `fretboard_core`.
//!
//! Status: the crate skeleton exists as of the P0 bootstrap task; the first
//! exported API (P1 task C04) arrives with its host-side contract tests.

#![forbid(unsafe_code)]
