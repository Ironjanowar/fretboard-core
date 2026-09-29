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
//! first chord quality through the documented C03 handoff. `C06` completed the
//! chord catalog: all 47 qualities answer with their own formula, display suffix
//! and contextual interval labels ([`chord_formula`], [`chord_quality_label`],
//! [`chord_interval_labels`], [`chord_details`]), the eight UI groups
//! ([`grouped_qualities`]) and the triad/seventh mode inference
//! ([`infer_chord_mode`], [`ChordMode`]). `C07` added the instrument and preset
//! catalog ([`instruments`], [`fretted_instruments`], [`piano`],
//! [`instrument_strings`], [`instrument_frets`], [`keyboard_pitch_range`],
//! [`pitch_presets`], [`preset_pitches`], [`standard_pitches`],
//! [`standard_tuning_notes`], [`tuning_presets`] and the guitar note-name
//! aliases). `C08` added the page-params codec: [`decode_page_params`],
//! [`encode_page_params`] and the baseline's decoded form ([`decoded_page`]) for
//! every current page field, plus [`quality_from_label`], the wire-label lookup
//! its chord tokens need. `C09` added the identity reducer ([`apply_event`] and
//! the recorded-step reader [`page_event`]) for the chord and highlight events.
//! `C10` added the visualizer surface ([`fretted_rows`], [`keyboard_keys`],
//! [`note_fill`] and its colour half [`identity_slots`]). Scale, progression,
//! derivation and analyzer support arrive in later tasks, each of them tests
//! first.
//!
//! `C11` added fixed-reference pitch editing ([`change_tuning_note`],
//! [`detect_preset`], [`tuning_notes`]) and the one copy of the nearest-pitch
//! rule ([`closest_pitch`]) that the page-params codec resolves the legacy
//! tuning notes through.
//!
//! `C12` added full ordered identification ([`identify_notes`],
//! [`identify_notes_with_bass`] and the [`Interpretation`] they return) with
//! the approved deterministic tie-break of `Contract.D03`.
//!
//! `C13` added the absolute analyzer ([`analyze_pitches`], [`analyze_page`] and
//! the [`Analysis`] they answer with) and the fretted reductions of the page
//! reducer ([`PageEvent::ToggleNote`], [`PageEvent::ClearSelection`],
//! [`PageEvent::SetTab`], [`PageEvent::SetInstrument`],
//! [`PageEvent::CommitTuning`], and the UI-only tuning draft
//! [`open_tuning_draft`], [`select_tuning_preset`], [`change_tuning_string`]
//! with the [`DraftEvent`] reader).

#![forbid(unsafe_code)]

mod analyzer;
mod chord;
mod error;
mod identify;
mod instrument_catalog;
mod interval;
mod note;
mod page_params;
mod pitch;
mod reducer;
mod state;
mod surface;
mod types;

pub use analyzer::{Analysis, analyze_page, analyze_pitches};
pub use chord::{
    ChordDetails, ChordMode, QualityGroup, chord_details, chord_formula, chord_interval_labels,
    chord_quality_label, grouped_qualities, infer_chord_mode, quality_from_label,
};
pub use error::CoreError;
pub use identify::{
    Interpretation, MatchKind, MatchSortKey, identify_notes, identify_notes_with_bass,
    match_sort_key,
};
pub use instrument_catalog::{
    Instrument, InstrumentKind, PitchPreset, fretted_instruments, guitar_standard_tuning,
    guitar_tuning_preset_names, guitar_tuning_presets, instrument_frets, instrument_kind,
    instrument_label, instrument_strings, instruments, keyboard_pitch_range, named_preset_pitches,
    piano, pitch_presets, preset_pitches, standard_pitches, standard_tuning_notes, tuning_presets,
};
pub use interval::interval_name;
pub use note::{chromatic_scale, note_at, note_index};
pub use page_params::{decode_page_params, decoded_page, encode_page_params};
pub use pitch::{change_tuning_note, closest_pitch, detect_preset, tuning_notes};
pub use reducer::{
    DraftEvent, PageEvent, apply_event, change_tuning_string, draft_event, open_tuning_draft,
    page_event, select_tuning_preset,
};
pub use state::{
    ChordSpec, InstrumentState, PageState, Position, TuningState, default_state, preset_tuning,
    validate_state,
};
pub use surface::{
    KeyboardKey, NoteFill, SurfaceCell, fretted_rows, fretted_surface, identity_slots,
    keyboard_keys, keyboard_surface, note_fill, note_fill_of, slot_of,
};
pub use types::{
    Fret, InstrumentId, OpenPitch, PitchClass, PresetName, QualityId, ScaleId, SoundingPitch,
    StringIndex, Tab,
};
