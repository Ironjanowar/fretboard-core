//! Task `C20`: the snapshot envelope against the frozen schema.
//!
//! `fixtures/contract/snapshot-v1.json` is the schema example C02 froze, and the
//! contract fixes the envelope as `{"schema_version": 1, "page": …}` where the
//! `page` object is exactly the typed page state's own strict serialised shape.
//! The tests below drive the envelope in both directions: the frozen example
//! decodes to a valid state and re-encodes to the same schema, every kind and
//! tab round-trips with its own shape, the output is byte-stable, a future
//! schema version is refused **with its bytes left intact for recovery**, and
//! every malformed shape is a rejection rather than a silently altered state.
//!
//! Baselines come from the frozen example, never from the implementation: the
//! schema key sets asserted here are the ones `state_contract.rs` pins for the
//! same file. A `v0` snapshot format does not exist, so no test claims a
//! migration succeeded — the registry's emptiness is asserted instead.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod common;

use common::{assert_valid, name_set, object_keys, read_repo_file};
use fretboard_core::{
    CURRENT_SNAPSHOT_SCHEMA, ChordSpec, CoreError, Fret, InstrumentId, InstrumentState, OpenPitch,
    PageState, PitchClass, Position, PresetName, QualityId, SNAPSHOT_MIGRATIONS, StringIndex, Tab,
    TuningState, decode_snapshot, encode_snapshot, validate_state,
};
use serde_json::Value;

const EXAMPLE: &str = "fixtures/contract/snapshot-v1.json";
const FUTURE: &str = "fixtures/contract/snapshot-future.json";
const CORRUPT: &str = "fixtures/contract/snapshot-corrupt.txt";

fn pitch_class(name: &str) -> PitchClass {
    name.parse().expect("a sharp pitch-class name")
}

fn quality(id: &str) -> QualityId {
    id.parse().expect("a catalog quality identifier")
}

fn preset(name: &str) -> PresetName {
    name.parse().expect("a catalog preset name")
}

fn open_pitch(value: u8) -> OpenPitch {
    OpenPitch::try_from(value).expect("an absolute open pitch")
}

fn string_index(value: u8) -> StringIndex {
    StringIndex::try_from(value).expect("a string index")
}

fn fret(value: u8) -> Fret {
    Fret::try_from(value).expect("a fret")
}

fn chord(root: &str, quality_id: &str) -> ChordSpec {
    ChordSpec {
        root: pitch_class(root),
        quality: quality(quality_id),
    }
}

fn standard_guitar() -> TuningState {
    TuningState {
        pitches: [40, 45, 50, 55, 59, 64].map(open_pitch).to_vec(),
        reference: preset("Standard"),
    }
}

/// The guitar page of the frozen example: ordered duplicate chords and a
/// highlight that occurs in them.
fn example_state() -> PageState {
    PageState {
        instrument: InstrumentState::Fretted {
            instrument: InstrumentId::Guitar,
            tuning: standard_guitar(),
            selected: vec![
                Position {
                    string: string_index(0),
                    fret: fret(3),
                },
                Position {
                    string: string_index(1),
                    fret: fret(2),
                },
            ],
        },
        chords: vec![chord("C", "major"), chord("A", "min7"), chord("C", "major")],
        highlight: Some(chord("C", "major")),
        tab: Tab::Visualizer,
    }
}

/// A piano page: absolute keys, no tuning, analyzer tab, no highlight.
fn piano_state() -> PageState {
    PageState {
        instrument: InstrumentState::Piano {
            selected: vec![open_pitch(48), open_pitch(60), open_pitch(83)],
        },
        chords: vec![chord("C", "major")],
        highlight: None,
        tab: Tab::Analyzer,
    }
}

fn encode_to_value(state: &PageState) -> Value {
    let text = encode_snapshot(state).expect("a valid state encodes");
    serde_json::from_str(&text).expect("the encoding is JSON")
}

fn code_of(result: Result<PageState, CoreError>) -> String {
    match result {
        Ok(state) => panic!("expected a refusal, got {state:?}"),
        Err(error) => error.code().to_owned(),
    }
}

fn keys_of_object(value: &Value) -> std::collections::BTreeSet<String> {
    value
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect()
}

/// A fresh parse of the frozen example, so a patched case never mutates it.
fn example_value() -> Value {
    serde_json::from_str(&read_repo_file(EXAMPLE)).expect("the example is JSON")
}

/// The frozen example decodes to a valid state, and re-encoding it reproduces
/// the frozen schema exactly.
#[test]
fn the_frozen_example_decodes_and_re_encodes_to_the_frozen_schema() {
    let text = read_repo_file(EXAMPLE);

    let state = decode_snapshot(text.as_bytes()).expect("the frozen example must decode");
    assert_valid(validate_state(&state));

    let re_encoded = encode_to_value(&state);
    let example: Value = serde_json::from_str(&text).expect("the example is JSON");
    assert_eq!(
        re_encoded, example,
        "re-encoding the decoded example must reproduce the frozen schema"
    );
}

/// Every kind and tab round-trips, each with its own frozen shape: a fretted
/// page carries `id`, `tuning` and `selection`, and the piano carries `id` and
/// `selection` with no `tuning` at all.
#[test]
fn every_kind_and_tab_round_trips_with_its_own_shape() {
    let states = [
        example_state(),
        piano_state(),
        PageState {
            instrument: InstrumentState::Fretted {
                instrument: InstrumentId::Bass5,
                tuning: TuningState {
                    pitches: [23, 28, 33, 38, 43].map(open_pitch).to_vec(),
                    reference: preset("Standard"),
                },
                selected: Vec::new(),
            },
            chords: Vec::new(),
            highlight: None,
            tab: Tab::Analyzer,
        },
    ];

    for state in states {
        let encoded = encode_to_value(&state);

        assert_eq!(
            keys_of_object(&encoded),
            name_set(&["schema_version", "page"]),
            "the envelope is exactly the frozen pair"
        );
        assert_eq!(
            encoded["schema_version"],
            Value::from(CURRENT_SNAPSHOT_SCHEMA)
        );

        let page = encoded["page"].as_object().expect("page is an object");
        assert_eq!(
            keys_of_object(&encoded["page"]),
            name_set(&["instrument", "chords", "highlight", "tab"]),
            "the page is exactly the frozen four"
        );

        let instrument = page["instrument"].as_object().expect("instrument object");
        match &state.instrument {
            InstrumentState::Fretted { .. } => {
                assert_eq!(
                    keys_of_object(&encoded["page"]["instrument"]),
                    name_set(&["kind", "id", "tuning", "selection"]),
                    "a fretted instrument carries its tuning"
                );
            }
            InstrumentState::Piano { .. } => {
                assert_eq!(
                    keys_of_object(&encoded["page"]["instrument"]),
                    name_set(&["kind", "id", "selection"]),
                    "the piano must not carry a tuning"
                );
                assert!(
                    !instrument.contains_key("tuning"),
                    "the piano must not carry a tuning"
                );
            }
        }

        let expected_tab = match state.tab {
            Tab::Visualizer => "visualizer",
            Tab::Analyzer => "analyzer",
        };
        assert_eq!(page["tab"], Value::from(expected_tab));

        // Ordered occurrences, duplicates included, and the highlight identity.
        let chords = page["chords"].as_array().expect("chords is an array");
        assert_eq!(chords.len(), state.chords.len(), "ordered occurrences kept");
        assert_eq!(
            page["highlight"].is_null(),
            state.highlight.is_none(),
            "the highlight is null exactly when there is none"
        );

        // The whole state survives the round trip.
        let text = encode_snapshot(&state).expect("a valid state encodes");
        assert_eq!(
            decode_snapshot(text.as_bytes()).expect("its own encoding decodes"),
            state,
            "the encoded state must decode back to the same page"
        );
    }
}

/// The encoding is byte-stable: encoding a decoded snapshot reproduces the same
/// bytes, so a restored session writes back exactly what it read.
#[test]
fn the_encoding_is_byte_stable() {
    for state in [example_state(), piano_state()] {
        let first = encode_snapshot(&state).expect("a valid state encodes");
        let decoded = decode_snapshot(first.as_bytes()).expect("its own encoding decodes");
        let second = encode_snapshot(&decoded).expect("the decoded state encodes");

        assert_eq!(first, second, "encode(decode(encode(s))) is encode(s)");
    }
}

/// A future schema version is refused, and the bytes survive the refusal: the
/// same bytes still decode once only the version is corrected, so a caller keeps
/// the original snapshot for recovery.
#[test]
fn a_future_schema_version_is_refused_without_touching_the_bytes() {
    let text = read_repo_file(FUTURE);
    let bytes = text.clone().into_bytes();

    match decode_snapshot(&bytes) {
        Err(error) => assert_eq!(error.code(), "UnsupportedSchemaVersion", "got {error:?}"),
        Ok(state) => panic!("a future schema version decoded as {state:?}"),
    }

    // The original bytes were neither consumed nor rewritten: the very same
    // payload decodes once the version alone is corrected.
    let corrected = text.replace("\"schema_version\": 2", "\"schema_version\": 1");
    let recovered = decode_snapshot(corrected.as_bytes())
        .expect("the frozen payload behind the future version is a valid schema 1 snapshot");
    assert_valid(validate_state(&recovered));
    assert_eq!(
        recovered,
        example_state(),
        "the retained bytes are the caller's recovery path"
    );
}

/// Schema 0 does not exist as a native format, so it is refused rather than
/// migrated: the registry claims no upgrade it cannot perform.
#[test]
fn an_unknown_older_schema_version_is_refused_not_migrated() {
    assert_eq!(
        CURRENT_SNAPSHOT_SCHEMA, 1,
        "this build writes and reads schema 1"
    );
    assert!(
        SNAPSHOT_MIGRATIONS.is_empty(),
        "no native v0 exists, so the migration registry is empty"
    );

    let text = read_repo_file(EXAMPLE).replace("\"schema_version\": 1", "\"schema_version\": 0");
    assert_eq!(
        code_of(decode_snapshot(text.as_bytes())),
        "UnsupportedSchemaVersion",
        "an older version without a migration step must not be silently upgraded"
    );
}

/// Corrupt, truncated and mistyped snapshots are rejected, never reinterpreted.
#[test]
fn corrupt_truncated_and_mistyped_snapshots_are_refused() {
    let example = read_repo_file(EXAMPLE);
    let corrupt = read_repo_file(CORRUPT);

    // Truncated mid-document, exactly what a torn write would leave behind.
    let truncated = example
        .as_bytes()
        .get(..example.len() / 2)
        .expect("a prefix")
        .to_vec();

    let mut wrong_type = example_value();
    wrong_type["page"]["chords"] = serde_json::json!("nope");

    let mut unknown_page_field = example_value();
    unknown_page_field["page"]["colors"] = serde_json::json!([]);

    let mut unknown_envelope_field = example_value();
    unknown_envelope_field["platform"] = serde_json::json!("android");

    let mut piano_tuning = example_value();
    piano_tuning["page"]["instrument"] =
        serde_json::json!({"kind": "piano", "id": "piano", "tuning": null, "selection": [60]});

    // A snapshot truncated after the chords key must not silently lose its
    // highlight: a present-but-null key is legal, an absent one is an error.
    let mut missing_highlight = example_value();
    missing_highlight["page"]
        .as_object_mut()
        .expect("page is an object")
        .remove("highlight");

    let cases: [(&str, Vec<u8>); 7] = [
        ("the corrupt fixture", corrupt.into_bytes()),
        ("a truncated snapshot", truncated),
        (
            "a wrong type for chords",
            serde_json::to_vec(&wrong_type).expect("a patched example serialises"),
        ),
        (
            "an unknown field in the page",
            serde_json::to_vec(&unknown_page_field).expect("a patched example serialises"),
        ),
        (
            "an unknown field in the envelope",
            serde_json::to_vec(&unknown_envelope_field).expect("a patched example serialises"),
        ),
        (
            "a tuning on the piano",
            serde_json::to_vec(&piano_tuning).expect("a patched example serialises"),
        ),
        (
            "a snapshot truncated before its highlight",
            serde_json::to_vec(&missing_highlight).expect("a patched example serialises"),
        ),
    ];

    for (description, text) in cases {
        assert_eq!(
            code_of(decode_snapshot(&text)),
            "InvalidSnapshot",
            "{description} must be refused as an invalid snapshot"
        );
    }
}

/// An unknown catalog identifier inside a snapshot is a rejection: the typed
/// reader is strict, so no misspelled id becomes a default.
#[test]
fn an_unknown_identifier_in_a_snapshot_is_refused() {
    let mut unknown_instrument = example_value();
    unknown_instrument["page"]["instrument"]["id"] = serde_json::json!("violin");

    let mut unknown_quality = example_value();
    unknown_quality["page"]["chords"][1]["quality"] = serde_json::json!("nope");

    let mut unknown_preset = example_value();
    unknown_preset["page"]["instrument"]["tuning"]["reference"] = serde_json::json!("Nope");

    let cases: [(&str, &Value); 3] = [
        ("an unknown instrument", &unknown_instrument),
        ("an unknown quality", &unknown_quality),
        ("an unknown preset", &unknown_preset),
    ];

    for (description, value) in cases {
        let text = serde_json::to_vec(value).expect("a patched example serialises");
        assert_eq!(
            code_of(decode_snapshot(&text)),
            "InvalidSnapshot",
            "{description} must be refused"
        );
    }
}

/// A page that is well-typed but structurally inconsistent is refused in both
/// directions: it is never written, and a hand-written snapshot claiming it is
/// never restored.
#[test]
fn a_structurally_inconsistent_page_is_never_persisted_and_never_restored() {
    // Encoding refuses: a page the engine would reject is never written.
    let mut state = example_state();
    state.highlight = Some(chord("G", "major"));
    let error = encode_snapshot(&state).expect_err("an inconsistent page must not be persisted");
    assert_eq!(error.code(), "InvalidState", "got {error:?}");

    // A hand-written snapshot claiming a highlight that occurs in no chord.
    let mut strays: Value =
        serde_json::from_str(&read_repo_file(EXAMPLE)).expect("the example is JSON");
    strays["page"]["highlight"] = serde_json::json!({"root": "G", "quality": "major"});
    let strays = serde_json::to_vec(&strays).expect("a patched example serialises");
    assert_eq!(
        code_of(decode_snapshot(&strays)),
        "InvalidSnapshot",
        "a highlight absent from the chords must not be restored"
    );

    // A piano key below the keyboard's own range.
    let mut piano: Value = serde_json::from_str(
        &encode_snapshot(&piano_state()).expect("a valid piano state encodes"),
    )
    .expect("the encoding is JSON");
    piano["page"]["instrument"]["selection"] = serde_json::json!([40]);
    let below_range = serde_json::to_vec(&piano).expect("a patched piano serialises");
    assert_eq!(
        code_of(decode_snapshot(&below_range)),
        "InvalidSnapshot",
        "a piano key outside 48..=83 must not be restored"
    );
}

/// Nothing but the committed page is persisted: no colours, results, drafts,
/// timestamps or platform values can appear in the encoding.
#[test]
fn no_derived_or_draft_value_is_persisted() {
    for state in [example_state(), piano_state()] {
        let encoded = encode_to_value(&state);

        assert_eq!(
            object_keys(&encoded),
            name_set(&["schema_version", "page"]),
            "the envelope carries nothing else"
        );
        assert_eq!(
            keys_of_object(&encoded["page"]),
            name_set(&["instrument", "chords", "highlight", "tab"]),
            "the page carries nothing but the committed state"
        );

        let page = encoded["page"].as_object().expect("page is an object");
        for derived in [
            "colors",
            "results",
            "draft",
            "modal",
            "timestamp",
            "analysis",
        ] {
            assert!(
                !page.contains_key(derived),
                "{derived} must never be persisted"
            );
        }
    }
}
