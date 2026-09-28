//! Portable music domain for Fretboard.
//!
//! This crate owns the musical rules, the catalogs, the canonical page state,
//! the state transitions and the URL/snapshot codecs for every Fretboard
//! client. It is deliberately pure: no UniFFI, Android, JNI, network, storage,
//! clock, randomness or rendering dependency.
//!
//! Its only dependencies are `serde` (with `derive`) and `serde_json`, approved
//! by `CORE-D04` in `docs/decisions.md`: the contract needs data-only JSON
//! codecs in the domain, and serde is data-only. Any further dependency needs a
//! new recorded decision.
//!
//! Baseline behavior is frozen from the Elixir web application at commit
//! `2daa8c665efa268942dda352691f39d78db42512`; the web application remains
//! unchanged and is the migration oracle.
//!
//! Status: `C02` froze the typed state contract — [`PageState`], its validated
//! primitives ([`PitchClass`], [`OpenPitch`], [`SoundingPitch`], [`StringIndex`],
//! [`Fret`]), its stable identifiers ([`InstrumentId`], [`QualityId`],
//! [`ScaleId`], [`PresetName`]), [`default_state`], [`preset_tuning`] and
//! [`validate_state`], with the frozen schema examples in
//! `fixtures/contract/`. `C03` added the note and interval primitives
//! ([`chromatic_scale`], [`note_index`], [`note_at`], [`interval_name`]) and the
//! `major` chord ([`chord_details`]); every other chord quality is
//! [`CoreError::UnsupportedCapability`] until `C06` implements the catalog.
//! Catalogs, codecs, transitions and derivation arrive in later tasks, each of
//! them tests first.

#![forbid(unsafe_code)]

mod chord;
mod error;
mod interval;
mod note;
mod state;
mod types;

pub use chord::{ChordDetails, chord_details};
pub use error::CoreError;
pub use interval::interval_name;
pub use note::{chromatic_scale, note_at, note_index};
pub use state::{
    ChordSpec, InstrumentState, PageState, Position, TuningState, default_state, preset_tuning,
    validate_state,
};
pub use types::{
    Fret, InstrumentId, OpenPitch, PitchClass, PresetName, QualityId, ScaleId, SoundingPitch,
    StringIndex, Tab,
};
