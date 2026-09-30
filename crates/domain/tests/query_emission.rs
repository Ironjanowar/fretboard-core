//! Emitting a page's query, and reading it back with the frozen transport.
//!
//! `A20` needs the *emitting* direction: a page becomes the query that reproduces it, and
//! a link is that query under an approved base (which does not exist yet, so no host
//! appears anywhere in this file either).
//!
//! Two things are pinned. The spelling rules, hand-written, each with the frozen case that
//! forces it: `#` must be percent-encoded because the frozen transport reads a raw one as
//! the end of the query (`query_transport/raw-fragment-not-a-sharp` reads `/?chords=C#maj`
//! as `C`), a space must be a `+` because that is what the transport decodes as a space
//! (`query_transport/plus-sign-becomes-space`), and `,` stays literal because the legacy
//! spelling is `/?chords=Cmaj,Amin`.
//!
//! And the round trip over **every** decoded case of the frozen transport fixture: read the
//! case's own request target with the transport, emit the query back, read it again, and
//! require the same page the fixture pins. The expectations are the fixture's, never this
//! code's. The two cases the fixture itself refuses (a non-UTF-8 escape and a 404) are
//! counted, so a refusal cannot silently grow into a skip.

// Test target: the same relaxations as the other contract tests.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use serde_json::{Map, Value};

use fretboard_core::{
    ImportedPage, PageState, decode_page_params, decoded_page, encode_page_query, import_legacy_url,
};

/// The frozen transport fixture, one parsed record per line.
fn transport_cases() -> Vec<Value> {
    let raw = std::fs::read_to_string("../../fixtures/oracle/query-transport.jsonl")
        .expect("the frozen transport fixture is missing");
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("a fixture line is not JSON"))
        .collect()
}

/// The page one parameter map decodes to.
fn page_of(params: &Map<String, Value>) -> PageState {
    decode_page_params(params)
}

/// The page one request target decodes to, or `None` when the transport refuses it.
///
/// Read through the *transport*, not through the parameter decoder: the transport is what
/// ends the query at a raw `#` (`query_transport/raw-fragment-not-a-sharp` reads
/// `/?chords=C#maj` as `C`) and what serves only the page route. The base is deliberately a
/// reserved name: this reader does not compare it.
fn page_of_target(target: &str) -> Option<PageState> {
    match import_legacy_url(&format!("https://any.test{target}")) {
        Ok(ImportedPage::Route(page)) => Some(page),
        _ => None,
    }
}

#[test]
fn a_sharp_root_is_escaped_because_a_raw_one_would_truncate_the_query() {
    let page = page_of(&Map::from_iter([(
        "chords".to_owned(),
        Value::String("C#maj".to_owned()),
    )]));

    assert_eq!("chords=C%23maj", encode_page_query(&page));
}

#[test]
fn a_space_becomes_a_plus_because_that_is_what_the_transport_reads_as_a_space() {
    let page = page_of(&Map::from_iter([
        (
            "pitches".to_owned(),
            Value::String("38,45,50,55,59,64".to_owned()),
        ),
        (
            "reference".to_owned(),
            Value::String("Half Step Down".to_owned()),
        ),
    ]));

    let query = encode_page_query(&page);

    assert!(query.contains("reference=Half+Step+Down"), "{query}");
    assert_eq!(
        page_of_target(&format!("/?{query}"))
            .as_ref()
            .map(decoded_page),
        Some(decoded_page(&page)),
    );
}

#[test]
fn a_separator_stays_literal_because_that_is_the_legacy_spelling() {
    let page = page_of(&Map::from_iter([(
        "chords".to_owned(),
        Value::String("Cmaj,Amin".to_owned()),
    )]));

    assert_eq!("chords=Cmaj,Amin", encode_page_query(&page));
}

#[test]
fn the_frozen_transport_reads_back_what_this_emits() {
    let cases = transport_cases();
    let mut round_tripped = 0;
    let mut refused = 0;

    for case in &cases {
        let target = case["input"]["path"]
            .as_str()
            .expect("every transport case carries a path");

        let Some(imported) = page_of_target(target) else {
            // The transport itself refuses this target — a non-UTF-8 escape, or a path that
            // is not the page route — so there is nothing to reproduce. The case still
            // counts, so a refusal here cannot silently grow into a skip.
            assert!(
                case["output"]["page_state"].is_null(),
                "{} was refused here but the fixture decoded a page",
                case["case_id"],
            );
            refused += 1;
            continue;
        };

        let emitted = encode_page_query(&imported);
        let read_back = page_of_target(&format!("/?{emitted}")).unwrap_or_else(|| {
            panic!(
                "{} emitted a query the transport refuses: {emitted}",
                case["case_id"]
            )
        });

        assert_eq!(
            case["output"]["page_state"],
            decoded_page(&read_back),
            "{} did not round trip (emitted {emitted:?})",
            case["case_id"],
        );
        round_tripped += 1;
    }

    assert_eq!(36, cases.len(), "the fixture is not the frozen one");
    assert_eq!(
        2, refused,
        "the frozen fixture refuses exactly two targets: the non-UTF-8 escape and the 404"
    );
    assert_eq!(34, round_tripped, "every other frozen case must round trip");
}
