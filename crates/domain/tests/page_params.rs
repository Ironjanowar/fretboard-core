//! Task `C08`: the page-params codec against the frozen oracle.
//!
//! Every case of `fixtures/oracle/page-params.jsonl` — the pinned
//! `Fretboard.Music.PageCodec.decode_page_params/1` and `encode_page_params/1` —
//! drives the codec in all three directions the baseline defines:
//!
//! * the decoded page, compared against the baseline's own decoded map,
//! * the encoded parameters, compared against the baseline's parameters,
//! * the baseline's encoded parameters decoded again, compared against the
//!   baseline's re-decoded page, so a decode→encode→decode cycle is pinned too.
//!
//! Nothing here retypes a value from the plan: the expected pages and parameters
//! are read record by record. The parameters are *given* to the codec as the
//! baseline received them, including the wrong-kind values (an integer
//! `chords`, a list `instrument`, a nested map `marked`), because tolerating
//! those is part of the contract.

// Test target: the same relaxations as the other contract tests. Arithmetic on
// small integers and direct indexing of fixture values are clearer here than the
// checked alternatives, and a panic in a test is a failure report.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod common;

use common::oracle_records;
use fretboard_core::{
    PageState, QualityId, chord_quality_label, decode_page_params, decoded_page, default_state,
    encode_page_params, quality_from_label, validate_state,
};
use serde_json::{Map, Value};

/// The frozen fixture and the number of cases it must carry.
const FIXTURE: &str = "fixtures/oracle/page-params.jsonl";
const CASES: usize = 104;

/// Every case of the fixture, with its identifier for failure messages.
fn cases() -> Vec<(String, Map<String, Value>, Value)> {
    oracle_records(FIXTURE)
        .into_iter()
        .map(|record| {
            let case = record["case_id"]
                .as_str()
                .expect("every case has an identifier")
                .to_owned();
            let input = record["input"]
                .as_object()
                .expect("every case has a parameter map")
                .clone();
            (case, input, record["output"].clone())
        })
        .collect()
}

/// The fixture is the frozen case list, not whatever happens to be on disk.
#[test]
fn the_fixture_carries_the_frozen_case_count() {
    assert_eq!(
        cases().len(),
        CASES,
        "the frozen fixture carries {CASES} cases"
    );
}

/// Decoding produces the baseline's decoded page, field for field.
#[test]
fn every_case_decodes_to_the_frozen_page() {
    for (case, params, output) in cases() {
        let state = decode_page_params(&params);
        assert_eq!(
            decoded_page(&state),
            output["decoded"],
            "{case}: the decoded page differs from the baseline"
        );
    }
}

/// Decoding never builds a state the public contract would reject.
#[test]
fn every_decoded_page_is_a_valid_state() {
    for (case, params, _) in cases() {
        let state = decode_page_params(&params);
        assert!(
            validate_state(&state).is_ok(),
            "{case}: the decoded state is not valid: {:?}",
            validate_state(&state)
        );
    }
}

/// Encoding produces the baseline's own parameters, omissions included.
#[test]
fn every_case_encodes_to_the_frozen_parameters() {
    for (case, params, output) in cases() {
        let state = decode_page_params(&params);
        assert_eq!(
            Value::Object(encode_page_params(&state)),
            output["encoded"],
            "{case}: the encoded parameters differ from the baseline"
        );
    }
}

/// The baseline's encoded parameters decode back to the baseline's re-decoded
/// page: the cycle is idempotent in the same direction the source is.
#[test]
fn the_frozen_parameters_decode_to_the_frozen_re_decoded_page() {
    for (case, _, output) in cases() {
        let encoded = output["encoded"]
            .as_object()
            .expect("every case encodes to a parameter map")
            .clone();
        let state = decode_page_params(&encoded);
        assert_eq!(
            decoded_page(&state),
            output["re_decoded"],
            "{case}: decoding the frozen parameters differs from the baseline"
        );
    }
}

/// A second cycle changes nothing: the codec's own output is a fixed point.
#[test]
fn encoding_a_decoded_page_is_idempotent() {
    for (case, params, _) in cases() {
        let once = decode_page_params(&params);
        let twice = decode_page_params(&encode_page_params(&once));
        assert_eq!(
            decoded_page(&once),
            decoded_page(&twice),
            "{case}: a second decode of the encoded parameters differs"
        );
        assert_eq!(
            encode_page_params(&once),
            encode_page_params(&twice),
            "{case}: a second encode differs"
        );
    }
}

/// The empty parameter map is the default state, in both directions.
#[test]
fn the_empty_parameters_are_the_default_state() {
    let state: PageState = decode_page_params(&Map::new());
    assert_eq!(
        state,
        default_state(),
        "no parameters must mean the default page"
    );
    assert!(
        encode_page_params(&default_state()).is_empty(),
        "the default page must encode to no parameters at all"
    );
}

/// The catalog's display labels are unique, so the wire label is a total reverse
/// lookup — which is what makes the chord tokens of this codec unambiguous.
#[test]
fn every_quality_label_is_a_unique_reverse_lookup() {
    let mut labels = std::collections::BTreeSet::new();
    for quality in QualityId::ALL {
        let label = chord_quality_label(quality);
        assert!(
            labels.insert(label),
            "the label {label} names more than one quality"
        );
        assert_eq!(
            quality_from_label(label),
            Some(quality),
            "the label {label} must name {quality:?} again"
        );
    }
    assert_eq!(
        labels.len(),
        QualityId::ALL.len(),
        "every quality has its own label"
    );
}

/// A label that is no quality at all is not a chord, and neither is a root on
/// its own or a root that is not a note.
#[test]
fn unknown_chord_tokens_are_not_chords() {
    for params in [
        vec![("chords", "C")],
        vec![("chords", "Xmaj")],
        vec![("chords", "Cmaj7b999")],
        vec![("chords", "")],
    ] {
        let map = params
            .into_iter()
            .map(|(key, value)| (key.to_owned(), Value::from(value)))
            .collect::<Map<String, Value>>();
        assert!(
            decode_page_params(&map).chords.is_empty(),
            "the token must not become a chord: {map:?}"
        );
    }
}
