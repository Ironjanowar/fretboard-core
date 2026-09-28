// Test target: `expect`/`unwrap`, panicking assertions and direct indexing are the
// idiom in tests, so the restriction lints that forbid them in the library are
// relaxed here only. Every other lint, including `pedantic`, still applies.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

//! First typed `UniFFI` adapter surface (P1, task C04).
//!
//! These tests pin the adapter's own typed DTOs and entry points from
//! `02-core-contract.md` section 8 and `04-core-phases.md` task C04: the
//! `default_state` projection, `chord_details` against the frozen chord oracle,
//! the instrument/tab wire conversions, ordered occurrence lists, a `None`
//! highlight, the error mapping of every domain variant, and typed errors
//! carrying the domain's stable code.
//!
//! The FFI boundary dictates the shape: `UniFFI` 0.32.2 derives its error type
//! only from an enum, so `AdapterError` is an enum and its code, message and
//! field are read through accessors (`AdapterError::code`, `::message`,
//! `::field`) rather than struct fields. The exported entry points likewise take
//! owned DTOs, so the tests build every argument and pass it by value.
//!
//! Scope is deliberately the P1 slice only. There is no JSON-blob `execute`
//! entry point and no reducer here: `04-core-phases.md` forbids a generic
//! string-in/string-out dispatcher, and the reducer belongs to later tasks.
//!
//! Expected musical values are read from the frozen oracle
//! (`fixtures/oracle/chords.jsonl` and `fixtures/oracle/tunings.jsonl`), never
//! retyped from the plan. This test target has no JSON dependency of its own and
//! the fixture format is one flat JSON object per line, so the small extractors
//! below read the exact fields the contract needs instead of pulling a parser
//! into the adapter's dependency set.
//!
//! First observation for C04 is the expected failure to compile: the adapter API
//! (`crates/mobile-ffi/src/{api,dto,convert}.rs`) does not exist yet.

use std::path::{Path, PathBuf};

use fretboard_core::CoreError;
use fretboard_mobile_ffi::{
    AdapterError, ChordDetailsDto, ChordDto, ErrorCode, InstrumentDto, InstrumentStateDto,
    PageStateDto, PositionDto, TabDto, TuningDto, chord_details, default_state, validate_state,
};

// ---------------------------------------------------------------------------
// Frozen-oracle readers
// ---------------------------------------------------------------------------

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join(relative)
}

/// Read a frozen fixture. A missing or unreadable fixture fails loudly instead
/// of silently skipping the ORACLE check.
fn read_fixture(relative: &str) -> String {
    let path = fixture_path(relative);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

/// One record of a JSONL fixture by its `case_id`.
fn oracle_line(relative: &str, case_id: &str) -> String {
    let text = read_fixture(relative);
    let needle = format!("\"case_id\":\"{case_id}\"");
    text.lines()
        .find(|line| line.contains(&needle))
        .unwrap_or_else(|| panic!("{relative} has no record {case_id}"))
        .to_string()
}

/// The body of a flat JSON array field, e.g. `"notes":["C","E","G"]` -> `"C","E","G"`.
/// The oracle's array elements are plain scalars, so no nesting has to be tracked.
fn array_body<'a>(fragment: &'a str, key: &str) -> &'a str {
    let needle = format!("\"{key}\":[");
    let start = fragment
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} array in {fragment}"))
        + needle.len();
    let rest = &fragment[start..];
    let end = rest
        .find(']')
        .unwrap_or_else(|| panic!("unterminated {key} array in {fragment}"));
    &rest[..end]
}

/// The string items of a flat JSON array field, in order.
fn string_array_after(fragment: &str, key: &str) -> Vec<String> {
    array_body(fragment, key)
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            item.strip_prefix('"')
                .and_then(|inner| inner.strip_suffix('"'))
                .unwrap_or_else(|| panic!("{item} is not a JSON string"))
                .to_string()
        })
        .collect()
}

/// The integer items of a flat JSON array field, in order.
fn int_array_after(fragment: &str, key: &str) -> Vec<u8> {
    array_body(fragment, key)
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| {
            item.parse::<u8>()
                .unwrap_or_else(|error| panic!("{item} is not a u8: {error}"))
        })
        .collect()
}

/// One flat JSON string field, e.g. `"label":"Cmaj"` -> `Cmaj`.
fn string_value_after(fragment: &str, key: &str) -> String {
    let needle = format!("\"{key}\":\"");
    let start = fragment
        .find(&needle)
        .unwrap_or_else(|| panic!("no {key} string in {fragment}"))
        + needle.len();
    let rest = &fragment[start..];
    let end = rest
        .find('"')
        .unwrap_or_else(|| panic!("unterminated {key} string in {fragment}"));
    rest[..end].to_string()
}

/// The frozen instrument ids of `fixtures/oracle/catalogs.json`, in catalog
/// display order (`instrument_models.instruments`).
fn oracle_instrument_ids() -> Vec<String> {
    let text = read_fixture("fixtures/oracle/catalogs.json");
    let needle = "\"instruments\":[";
    let start = text
        .find(needle)
        .unwrap_or_else(|| panic!("the catalog has no instruments array"))
        + needle.len();
    let rest = &text[start..];
    let body = &rest[..rest
        .find(']')
        .unwrap_or_else(|| panic!("the instruments array is unterminated"))];

    let mut ids = Vec::new();
    let mut cursor = 0;
    while let Some(position) = body[cursor..].find("\"id\":\"") {
        let value_start = cursor + position + "\"id\":\"".len();
        let value_end = value_start
            + body[value_start..]
                .find('"')
                .unwrap_or_else(|| panic!("an instrument id is unterminated"));
        ids.push(body[value_start..value_end].to_string());
        cursor = value_end + 1;
    }
    ids
}

/// The `chord_notes` names of one root and quality, from the frozen oracle.
fn oracle_chord_notes(root: &str, quality: &str) -> Vec<String> {
    let line = oracle_line(
        "fixtures/oracle/chords.jsonl",
        &format!("chord_notes/{root}:{quality}"),
    );
    string_array_after(&line, "notes")
}

/// The full `chord_label` of one root and quality, from the frozen oracle.
fn oracle_chord_label(root: &str, quality: &str) -> String {
    let line = oracle_line(
        "fixtures/oracle/chords.jsonl",
        &format!("chord_label/{root}:{quality}"),
    );
    string_value_after(&line, "label")
}

/// The `chord_interval_labels` of one quality, from the frozen oracle.
fn oracle_interval_labels(quality: &str) -> Vec<String> {
    let line = oracle_line(
        "fixtures/oracle/chords.jsonl",
        &format!("chord_interval_labels/{quality}"),
    );
    string_array_after(&line, "interval_labels")
}

/// The guitar Standard preset as `(pitches, reference)`, from the frozen oracle
/// `preset_tuning` export.
fn oracle_guitar_standard() -> (Vec<u8>, String) {
    let line = oracle_line(
        "fixtures/oracle/tunings.jsonl",
        "preset_tuning/guitar/Standard",
    );
    (
        int_array_after(&line, "pitches"),
        string_value_after(&line, "reference"),
    )
}

fn read_repo_file(relative: &str) -> String {
    read_fixture(relative)
}

// ---------------------------------------------------------------------------
// Adapter DTO builders and assertions
// ---------------------------------------------------------------------------

fn chord(root: &str, quality: &str) -> ChordDto {
    ChordDto {
        root: root.to_string(),
        quality: quality.to_string(),
    }
}

fn standard_guitar_tuning() -> TuningDto {
    let (pitches, reference) = oracle_guitar_standard();
    TuningDto { pitches, reference }
}

/// A fretted guitar page with the oracle Standard tuning and empty selection.
fn guitar_state(chords: Vec<ChordDto>, highlight: Option<ChordDto>) -> PageStateDto {
    PageStateDto {
        instrument: InstrumentStateDto::Fretted {
            instrument: InstrumentDto::Guitar,
            tuning: standard_guitar_tuning(),
            selected: Vec::new(),
        },
        chords,
        highlight,
        tab: TabDto::Visualizer,
    }
}

/// Assert that a fallible adapter call failed with exactly `expected`, and
/// return the typed error for further checks.
fn assert_error_code<T: std::fmt::Debug>(
    result: Result<T, AdapterError>,
    expected: ErrorCode,
) -> AdapterError {
    match result {
        Ok(value) => panic!("expected {}, got Ok({value:?})", expected.as_str()),
        Err(error) => {
            assert_eq!(
                error.code().as_str(),
                expected.as_str(),
                "wrong error code in {error:?}"
            );
            assert_eq!(
                error.code(),
                expected,
                "wrong typed error code in {error:?}"
            );
            assert!(
                !error.message().is_empty(),
                "the typed error must carry an English message: {error:?}"
            );
            error
        }
    }
}

// ---------------------------------------------------------------------------
// default_state
// ---------------------------------------------------------------------------

#[test]
fn default_state_is_guitar_standard_visualizer_from_the_oracle() {
    let (expected_pitches, expected_reference) = oracle_guitar_standard();
    let state = default_state();

    let InstrumentStateDto::Fretted {
        instrument,
        tuning,
        selected,
    } = &state.instrument
    else {
        panic!(
            "the default instrument must be fretted, got {:?}",
            state.instrument
        );
    };
    assert_eq!(*instrument, InstrumentDto::Guitar);
    assert_eq!(
        tuning.pitches, expected_pitches,
        "default pitches must equal the oracle guitar Standard preset"
    );
    assert_eq!(
        tuning.reference, expected_reference,
        "default reference must be the oracle Standard preset name"
    );
    assert!(selected.is_empty(), "the default selection must be empty");

    assert!(
        state.chords.is_empty(),
        "the default chord list must be empty"
    );
    assert!(
        state.highlight.is_none(),
        "the default highlight must be None, not an empty value"
    );
    assert_eq!(state.tab, TabDto::Visualizer);
}

#[test]
fn default_state_survives_validation_unchanged() {
    // The boundary conversion the FFI performs (DTO -> domain -> DTO) must accept
    // the default state and give it back unchanged: no highlight invented, no
    // selection invented, no chord materialised.
    let state = default_state();
    let validated = validate_state(state.clone()).expect("the default state is valid");
    assert_eq!(
        validated, state,
        "validation must not rewrite the default state"
    );
    assert!(validated.highlight.is_none());
}

// ---------------------------------------------------------------------------
// chord_details
// ---------------------------------------------------------------------------

#[test]
fn chord_details_computes_c_major_from_the_oracle() {
    let details = chord_details(chord("C", "major")).expect("C major is implemented in P1");

    assert_eq!(details.root, "C", "identity: root");
    assert_eq!(details.quality, "major", "identity: quality");
    assert_eq!(details.label, oracle_chord_label("C", "major"));
    assert_eq!(details.notes, oracle_chord_notes("C", "major"));
    assert_eq!(details.interval_labels, oracle_interval_labels("major"));
    assert_eq!(
        details.notes.len(),
        details.interval_labels.len(),
        "the oracle zips the score order of notes and labels"
    );
}

#[test]
fn chord_details_computes_a_second_root_from_the_oracle() {
    // The UI requests C and a second root to show that the answer is computed,
    // not stored. D major is a different chord with its own oracle record.
    let c_major = chord_details(chord("C", "major")).expect("C major");
    let d_major = chord_details(chord("D", "major")).expect("D major");

    assert_eq!(d_major.label, oracle_chord_label("D", "major"));
    assert_eq!(d_major.notes, oracle_chord_notes("D", "major"));
    assert_eq!(d_major.interval_labels, oracle_interval_labels("major"));

    assert_ne!(
        c_major.notes, d_major.notes,
        "a different root must produce a different chord"
    );
    assert_ne!(c_major.label, d_major.label);
}

#[test]
fn chord_details_rejects_an_unknown_quality_string() {
    for unknown in ["bogus", "m11b6", "MAJOR", "", "major "] {
        assert_error_code(
            chord_details(chord("C", unknown)).map(|_| ()),
            ErrorCode::UnknownIdentifier,
        );
    }
}

#[test]
fn chord_details_answers_every_catalog_quality_from_the_oracle() {
    // C06 completed the domain chord catalog, so the pending-quality path this
    // adapter test used to pin is gone: a real catalog identifier answers, and
    // its answer is the frozen oracle's, not a plausible wrong chord.
    for implemented in ["min7", "sus4", "dim", "7", "maj7", "13", "susb9", "m_add9"] {
        let details = chord_details(chord("C", implemented))
            .unwrap_or_else(|error| panic!("{implemented} is implemented since C06: {error:?}"));
        assert_eq!(
            details.notes,
            oracle_chord_notes("C", implemented),
            "the notes of C {implemented}"
        );
        assert_eq!(
            details.label,
            oracle_chord_label("C", implemented),
            "the label of C {implemented}"
        );
        assert_eq!(details.quality, implemented, "the quality identity");
    }
}

#[test]
fn a_catalog_quality_is_never_reported_as_pending() {
    // The `UnsupportedCapability` code stays in the frozen contract for the
    // capabilities later tasks add, but no catalog quality may reach the client
    // through it any more, and none of the codes it used to be confused with.
    for implemented in ["min7", "sus4", "dim", "7", "maj7"] {
        match chord_details(chord("C", implemented)) {
            Ok(_) => {}
            Err(error) => panic!("{implemented} must answer, got {error:?}"),
        }
    }

    // The code is still the one the domain raises for a capability this build
    // lacks, and the mapping still keeps it distinct: a pending capability is
    // neither a malformed request nor an inconsistent state.
    let error = AdapterError::from(CoreError::unsupported_capability("capability"));
    assert_eq!(error.code(), ErrorCode::UnsupportedCapability);
    assert_ne!(error.code(), ErrorCode::InvalidAction);
    assert_ne!(error.code(), ErrorCode::InvalidState);
    assert_ne!(error.code(), ErrorCode::UnknownIdentifier);
}

// ---------------------------------------------------------------------------
// Enum conversion in both directions
// ---------------------------------------------------------------------------

#[test]
fn instrument_ids_round_trip_through_the_frozen_wire_strings() {
    let oracle = oracle_instrument_ids();
    assert_eq!(
        oracle,
        vec!["guitar", "bass_4", "bass_5", "ukelele", "piano"],
        "the frozen catalog instrument ids, in catalog display order"
    );

    // String -> enum -> string must be the identity for every catalog id.
    for id in &oracle {
        let parsed = InstrumentDto::parse(id).unwrap_or_else(|error| panic!("{id}: {error:?}"));
        assert_eq!(
            parsed.as_str(),
            id,
            "an instrument id must keep its catalog wire string"
        );
    }

    // The five variants map to the five oracle strings, in order.
    let by_variant = [
        InstrumentDto::Guitar,
        InstrumentDto::Bass4,
        InstrumentDto::Bass5,
        InstrumentDto::Ukelele,
        InstrumentDto::Piano,
    ];
    let wire: Vec<String> = by_variant
        .iter()
        .map(|id| id.as_str().to_string())
        .collect();
    assert_eq!(wire, oracle, "the enum must equal the oracle ids in order");

    // Enum -> string -> enum must be the identity too.
    for id in by_variant {
        assert_eq!(
            InstrumentDto::parse(id.as_str())
                .unwrap_or_else(|error| panic!("{:?}: {error:?}", id.as_str())),
            id,
            "parse must invert as_str"
        );
    }

    assert_eq!(
        InstrumentDto::Ukelele.as_str(),
        "ukelele",
        "the frozen single-e spelling must survive"
    );
}

#[test]
fn instrument_id_parse_rejects_unknown_strings() {
    for unknown in ["violin", "Ukelele", "guitar_6", "piano ", "", "bass"] {
        assert_error_code(
            InstrumentDto::parse(unknown).map(|_| ()),
            ErrorCode::UnknownIdentifier,
        );
    }
}

#[test]
fn tab_ids_round_trip_through_the_frozen_wire_strings() {
    // The two tab wire strings are the frozen section-7 values; both occur in the
    // page-params oracle.
    let oracle = read_repo_file("fixtures/oracle/page-params.jsonl");
    for wire in ["visualizer", "analyzer"] {
        assert!(
            oracle.contains(&format!("\"tab\":\"{wire}\"")),
            "the frozen page-params oracle must contain tab {wire}"
        );
    }

    assert_eq!(TabDto::Visualizer.as_str(), "visualizer");
    assert_eq!(TabDto::Analyzer.as_str(), "analyzer");

    for tab in [TabDto::Visualizer, TabDto::Analyzer] {
        assert_eq!(
            TabDto::parse(tab.as_str()).unwrap_or_else(|error| panic!("{error:?}")),
            tab,
            "parse must invert as_str for {tab:?}"
        );
    }

    for unknown in ["nope", "Visualizer", "ANALYZER", ""] {
        assert_error_code(
            TabDto::parse(unknown).map(|_| ()),
            ErrorCode::UnknownIdentifier,
        );
    }
}

// ---------------------------------------------------------------------------
// Ordered lists, null highlight, typed invalid input
// ---------------------------------------------------------------------------

#[test]
fn validate_state_keeps_duplicate_chord_occurrences_in_order() {
    // Order and repetitions are the contract (section 3, 6): the list is
    // occurrences, not a set. The boundary conversion must not sort or dedupe.
    let occurrences = vec![
        chord("C", "major"),
        chord("D", "major"),
        chord("C", "major"),
    ];
    let state = guitar_state(occurrences.clone(), Some(chord("C", "major")));

    let validated = validate_state(state).expect("ordered occurrences are valid");
    assert_eq!(
        validated.chords, occurrences,
        "duplicate occurrences must keep their order"
    );
    assert_eq!(validated.chords.len(), 3, "both copies of C major remain");
    assert_eq!(
        validated.chords.first(),
        validated.chords.get(2),
        "the repeated chord keeps its identity at both positions"
    );
}

#[test]
fn validate_state_keeps_a_none_highlight_as_none() {
    let state = guitar_state(vec![chord("C", "major")], None);
    let validated = validate_state(state).expect("a chordless-highlight state is valid");
    assert!(
        validated.highlight.is_none(),
        "a None highlight must stay None, not become an empty value"
    );
}

#[test]
fn validate_state_rejects_a_highlight_absent_from_the_chords() {
    let state = guitar_state(vec![chord("C", "major")], Some(chord("G", "major")));
    assert_error_code(validate_state(state).map(|_| ()), ErrorCode::InvalidState);
}

#[test]
fn validate_state_rejects_an_out_of_range_piano_pitch() {
    let state = PageStateDto {
        instrument: InstrumentStateDto::Piano { selected: vec![47] },
        chords: Vec::new(),
        highlight: None,
        tab: TabDto::Analyzer,
    };
    let error = assert_error_code(validate_state(state).map(|_| ()), ErrorCode::OutOfRange);
    assert!(
        !error.message().is_empty(),
        "an out-of-range piano pitch must carry a message"
    );

    let in_range = PageStateDto {
        instrument: InstrumentStateDto::Piano {
            selected: vec![48, 83],
        },
        chords: Vec::new(),
        highlight: None,
        tab: TabDto::Analyzer,
    };
    assert!(
        validate_state(in_range).is_ok(),
        "the piano bounds 48..=83 must stay valid"
    );
}

#[test]
fn validate_state_rejects_an_out_of_range_fret() {
    let state = PageStateDto {
        instrument: InstrumentStateDto::Fretted {
            instrument: InstrumentDto::Guitar,
            tuning: standard_guitar_tuning(),
            selected: vec![PositionDto {
                string: 0,
                fret: 25,
            }],
        },
        chords: Vec::new(),
        highlight: None,
        tab: TabDto::Visualizer,
    };
    assert_error_code(validate_state(state).map(|_| ()), ErrorCode::OutOfRange);

    let valid = PageStateDto {
        instrument: InstrumentStateDto::Fretted {
            instrument: InstrumentDto::Guitar,
            tuning: standard_guitar_tuning(),
            selected: vec![PositionDto {
                string: 0,
                fret: 24,
            }],
        },
        chords: Vec::new(),
        highlight: None,
        tab: TabDto::Visualizer,
    };
    assert!(
        validate_state(valid).is_ok(),
        "fret 24 is the highest legal fret"
    );
}

// ---------------------------------------------------------------------------
// The stable error codes
// ---------------------------------------------------------------------------

#[test]
fn error_code_as_str_is_pinned_for_every_variant() {
    // The adapter maps the domain's `CoreError::code()` onto its own typed
    // `ErrorCode`; the wire string of each variant is the frozen code, so a
    // client can branch on it without parsing a message.
    let cases = [
        (ErrorCode::InvalidState, "InvalidState"),
        (ErrorCode::UnknownIdentifier, "UnknownIdentifier"),
        (ErrorCode::OutOfRange, "OutOfRange"),
        (ErrorCode::InvalidAction, "InvalidAction"),
        (ErrorCode::InvalidUrl, "InvalidUrl"),
        (ErrorCode::UnsupportedOrigin, "UnsupportedOrigin"),
        (ErrorCode::InputTooLarge, "InputTooLarge"),
        (ErrorCode::InvalidSnapshot, "InvalidSnapshot"),
        (
            ErrorCode::UnsupportedSchemaVersion,
            "UnsupportedSchemaVersion",
        ),
        (ErrorCode::UnsupportedCapability, "UnsupportedCapability"),
    ];
    for (code, expected) in cases {
        assert_eq!(code.as_str(), expected, "wrong wire code for {code:?}");
    }
}

#[test]
fn every_adapter_error_variant_reports_its_matching_code() {
    // Pin each enum variant to its code directly: a variant whose `code()` was
    // mis-mapped to a neighbour would otherwise pass unnoticed, because the
    // entry points only ever exercise the six codes the P1 slice can raise.
    // `InvalidUrl`, `UnsupportedOrigin`, `InputTooLarge`, `InvalidSnapshot` and
    // `UnsupportedSchemaVersion` have no P1 failure path at all, so the ten
    // variants are built explicitly here; the codes the entry points do reach
    // are also exercised through real failures in the tests above.
    let cases = [
        (
            AdapterError::InvalidState {
                message: "state is inconsistent".to_string(),
                field: Some("chords".to_string()),
            },
            ErrorCode::InvalidState,
        ),
        (
            AdapterError::UnknownIdentifier {
                message: "unknown catalog identifier".to_string(),
                field: Some("instrument".to_string()),
            },
            ErrorCode::UnknownIdentifier,
        ),
        (
            AdapterError::OutOfRange {
                message: "value is outside its range".to_string(),
                field: Some("fret".to_string()),
            },
            ErrorCode::OutOfRange,
        ),
        (
            AdapterError::InvalidAction {
                message: "the action does not apply".to_string(),
                field: None,
            },
            ErrorCode::InvalidAction,
        ),
        (
            AdapterError::InvalidUrl {
                message: "not a syntactically valid url".to_string(),
                field: None,
            },
            ErrorCode::InvalidUrl,
        ),
        (
            AdapterError::UnsupportedOrigin {
                message: "the origin is outside the share policy".to_string(),
                field: None,
            },
            ErrorCode::UnsupportedOrigin,
        ),
        (
            AdapterError::InputTooLarge {
                message: "the input is larger than the import limit".to_string(),
                field: None,
            },
            ErrorCode::InputTooLarge,
        ),
        (
            AdapterError::InvalidSnapshot {
                message: "the snapshot is corrupt".to_string(),
                field: Some("snapshot".to_string()),
            },
            ErrorCode::InvalidSnapshot,
        ),
        (
            AdapterError::UnsupportedSchemaVersion {
                message: "the snapshot schema version is not readable".to_string(),
                field: Some("schema_version".to_string()),
            },
            ErrorCode::UnsupportedSchemaVersion,
        ),
        (
            AdapterError::UnsupportedCapability {
                message: "the capability is not implemented yet".to_string(),
                field: Some("quality".to_string()),
            },
            ErrorCode::UnsupportedCapability,
        ),
    ];

    assert_eq!(cases.len(), 10, "every adapter error variant is pinned");
    for (error, expected) in cases {
        assert_eq!(error.code(), expected, "wrong code for {error:?}");
        assert_eq!(
            error.code().as_str(),
            expected.as_str(),
            "the wire code must agree with the typed code for {error:?}"
        );
        assert!(
            !error.message().is_empty(),
            "an adapter error must carry an English message: {error:?}"
        );
    }
}

#[test]
fn every_domain_error_variant_maps_to_its_own_adapter_code() {
    // The entry points can only raise a few of the ten codes, so the five with
    // no reachable path (`InvalidUrl`, `UnsupportedOrigin`, `InputTooLarge`,
    // `InvalidSnapshot`, `UnsupportedSchemaVersion`) are pinned here through the
    // mapping itself. A mis-mapped arm — `InvalidUrl` surfacing as
    // `UnsupportedOrigin`, say — passes every entry-point test otherwise.
    let cases = [
        (CoreError::InvalidState(None), ErrorCode::InvalidState),
        (
            CoreError::UnknownIdentifier(None),
            ErrorCode::UnknownIdentifier,
        ),
        (CoreError::OutOfRange(None), ErrorCode::OutOfRange),
        (CoreError::InvalidAction(None), ErrorCode::InvalidAction),
        (CoreError::InvalidUrl(None), ErrorCode::InvalidUrl),
        (
            CoreError::UnsupportedOrigin(None),
            ErrorCode::UnsupportedOrigin,
        ),
        (CoreError::InputTooLarge(None), ErrorCode::InputTooLarge),
        (CoreError::InvalidSnapshot(None), ErrorCode::InvalidSnapshot),
        (
            CoreError::UnsupportedSchemaVersion(None),
            ErrorCode::UnsupportedSchemaVersion,
        ),
        (
            CoreError::UnsupportedCapability(None),
            ErrorCode::UnsupportedCapability,
        ),
    ];
    assert_eq!(cases.len(), 10, "every domain error variant is covered");

    for (domain_error, expected) in cases {
        let error = AdapterError::from(domain_error);
        assert_eq!(
            error.code(),
            expected,
            "the mapped code of a domain error must be its own"
        );
        assert!(
            !error.message().is_empty(),
            "a mapped error must carry an English message"
        );
    }
}

#[test]
fn a_domain_error_keeps_its_field_name_as_diagnostic_detail() {
    let error = AdapterError::from(CoreError::invalid_state("highlight"));
    assert_eq!(error.code(), ErrorCode::InvalidState);
    assert_eq!(
        error.field(),
        Some("highlight"),
        "the domain's field name must survive the mapping"
    );
    assert!(
        error.message().contains("highlight"),
        "the domain's own sentence must survive the mapping: {error}"
    );
}

// ---------------------------------------------------------------------------
// A guard helper used only to keep the import surface pinned.
// ---------------------------------------------------------------------------

/// The adapter's details DTO must be usable as a value: owned, cloneable and
/// comparable, exactly like the state DTOs.
#[test]
fn adapter_dtos_are_owned_cloneable_and_comparable() {
    let details: ChordDetailsDto =
        chord_details(chord("C", "major")).expect("C major is implemented");
    let cloned = details.clone();
    assert_eq!(cloned, details, "the details DTO must be Clone + PartialEq");

    let state = default_state();
    let cloned_state = state.clone();
    assert_eq!(
        cloned_state, state,
        "the state DTO must be Clone + PartialEq"
    );
}
