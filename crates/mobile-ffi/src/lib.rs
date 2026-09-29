//! Typed foreign boundary for the Fretboard domain.
//!
//! This crate owns the DTOs, the conversions and the exported API that native
//! clients call through generated bindings. It contains no musical rule: every
//! semantic decision belongs to `fretboard_core`.
//!
//! P1 (task C04) exported the first typed surface, deliberately small:
//! [`default_state`], [`validate_state`] and [`chord_details`], with the DTOs of
//! [`dto`]. The P2 integration added what the visualizer phase needs — the
//! catalogs ([`instruments`], [`quality_groups`]), the page events
//! ([`apply_page_event`]) and the surfaces ([`fretted_surface`],
//! [`keyboard_surface`], [`chord_color_slots`]). There is still no
//! string-in/string-out `execute` dispatcher, and no URL or snapshot codec:
//! strings cross the boundary only through those codecs, in later tasks.
//!
//! The generated binding package is configured by `uniffi.toml`
//! (`dev.ironjanowar.fretboard.core`); generated Kotlin is a build output of a
//! later task (`C05`) and is never hand-written or committed.
//!
//! The P3 integration added the tuning edits ([`change_tuning_note`],
//! [`tuning_notes`], [`detect_tuning_preset`]) and then the analyzer slice
//! [`C13`] needs: the page analysis ([`analyze_page`]), the
//! instrument-independent analyzer ([`analyze_pitches`]) and the UI-only tuning
//! draft ([`open_tuning_draft`], [`select_tuning_preset`],
//! [`change_tuning_string`]) that [`apply_page_event`] commits through
//! [`PageEventDto::CommitTuning`].
//!
//! The P5 integration added the key and progression surfaces: the key panel
//! ([`key_suggestions`], [`group_key_suggestions`]), the multi-key panel
//! ([`multi_key_suggestions`]), the catalog and its chords ([`progressions`],
//! [`progression_chords`]) and the key modal's preview ([`diatonic_chords`]),
//! committed through [`PageEventDto::CommitKeys`],
//! [`PageEventDto::CommitSuggestedKeys`] and [`PageEventDto::CommitProgression`].
//!
//! The P6 integration added the two string codecs the durable session needs:
//! the URL import transport ([`import_url`], with the caller-supplied
//! [`UrlPolicyDto`] and the domain's frozen error codes) and the snapshot
//! envelope ([`encode_snapshot`], [`decode_snapshot`],
//! [`snapshot_schema_version`]).
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

pub use api::{
    analyze_page, analyze_pitches, apply_page_event, change_tuning_note, change_tuning_string,
    chord_color_slots, chord_details, decode_snapshot, default_state, detect_tuning_preset,
    diatonic_chords, encode_snapshot, fretted_surface, group_key_suggestions, import_url,
    instruments, key_suggestions, keyboard_surface, multi_key_suggestions, open_tuning_draft,
    presets, progression_chords, progressions, quality_groups, select_tuning_preset,
    snapshot_schema_version, tuning_notes, validate_state,
};
pub use dto::{
    AdapterError, AnalysisDto, ChordDetailsDto, ChordDto, ChordModeDto, DegreeDto, ErrorCode,
    FrettedSurfaceDto, InstrumentDefinitionDto, InstrumentDto, InstrumentKindDto,
    InstrumentStateDto, InterpretationDto, KeyRowDto, KeySuggestionDto, KeyboardKeyDto,
    KeyboardSurfaceDto, MultiKeyGroupDto, NoteFillDto, PageEventDto, PageStateDto, PositionDto,
    ProgressionDto, ProgressionGroupDto, QualityDto, QualityGroupDto, SurfaceCellDto,
    SurfaceRowDto, TabDto, TuningDto, UrlPolicyDto,
};
