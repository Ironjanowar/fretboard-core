//! Task `C19`: the URL import transport against the frozen oracle.
//!
//! `fixtures/oracle/query-transport.jsonl` is the pinned capture of the web's
//! own request-target behaviour — `Plug.Conn.Query.decode/1` behind the
//! `FretboardWeb.Router` `GET /` route — and contract decision `D09` says the
//! transport is not defined by a `map()` codec alone, so it is captured rather
//! than assumed. Every one of its cases drives the transport in the three
//! directions the baseline defines:
//!
//! * the decoded query map, compared against the baseline's own `query_params`,
//! * the imported page, compared against the baseline's own `page_state`,
//! * the route outcome, compared against the baseline's own `status`.
//!
//! Nothing here retypes an expectation from the plan: the maps, the pages and
//! the routes are read record by record. The one case the baseline answers with
//! an exception (invalid UTF-8 in a percent escape) must be a rejection here
//! too, never a reinterpretation.
//!
//! The origin allowlist and the length/resource limits are *not* frozen: `D09`
//! says they must be approved in P0 and the contract supplies them from a
//! validated configuration rather than inventing a hostname in Rust. The
//! envelope tests below therefore construct their own policy — no production
//! host is guessed and no limit is invented.

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
    ImportedPage, UrlPolicy, decode_query, decoded_page, import_absolute_url,
    import_request_target, parse_request_target, query_params_to_json,
};
use serde_json::Value;

/// The frozen fixture and the number of cases it must carry.
const FIXTURE: &str = "fixtures/oracle/query-transport.jsonl";
const CASES: usize = 36;

/// One frozen case: its identifier, the request target and the baseline output.
struct Case {
    id: String,
    target: String,
    output: Value,
}

/// Every case of the fixture, in file order.
fn cases() -> Vec<Case> {
    oracle_records(FIXTURE)
        .into_iter()
        .map(|record| Case {
            id: record["case_id"]
                .as_str()
                .expect("every case has an identifier")
                .to_owned(),
            target: record["input"]["path"]
                .as_str()
                .expect("every case has a request target")
                .to_owned(),
            output: record["output"].clone(),
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

/// Every case decodes to the baseline's own query map, or is rejected exactly
/// where the baseline raised.
#[test]
fn every_pinned_case_decodes_to_the_baselines_query_map() {
    for case in cases() {
        let target = parse_request_target(&case.target);
        let decoded = decode_query(&target.query);
        let raised = !case.output["transport_error"].is_null();

        match (decoded, raised) {
            (Ok(params), false) => assert_eq!(
                query_params_to_json(&params),
                case.output["query_params"],
                "case {} decoded a different query map",
                case.id
            ),
            (Err(error), true) => assert_eq!(
                error.code(),
                "InvalidUrl",
                "case {} must be rejected as an invalid URL, got {error:?}",
                case.id
            ),
            (Ok(params), true) => panic!(
                "case {} raised in the baseline but decoded to {}",
                case.id,
                query_params_to_json(&params)
            ),
            (Err(error), false) => panic!(
                "case {} decoded in the baseline but was rejected with {error:?}",
                case.id
            ),
        }
    }
}

/// Every case imports the baseline's own page, or answers the baseline's own
/// route outcome.
#[test]
fn every_pinned_case_imports_the_baselines_page_or_route() {
    for case in cases() {
        let expected_status = case.output["status"].as_u64();
        match import_request_target(&case.target) {
            Ok(ImportedPage::Route(state)) => {
                assert_eq!(
                    expected_status,
                    Some(200),
                    "case {} answered a page where the baseline did not",
                    case.id
                );
                assert_eq!(
                    decoded_page(&state),
                    case.output["page_state"],
                    "case {} imported a different page",
                    case.id
                );
            }
            Ok(ImportedPage::NotFound) => assert_eq!(
                expected_status,
                Some(404),
                "case {} answered not-found where the baseline answered a page",
                case.id
            ),
            Err(error) => assert!(
                !case.output["transport_error"].is_null(),
                "case {} was rejected with {error:?} where the baseline answered",
                case.id
            ),
        }
    }
}

/// The invalid UTF-8 case is a rejection, never a reinterpretation: the
/// baseline raised `Plug.Conn.InvalidQueryError` and the native contract has no
/// page for it.
#[test]
fn an_invalid_utf8_escape_is_rejected_and_never_imported() {
    let case = cases()
        .into_iter()
        .find(|case| case.id == "query_transport/invalid-utf8-escape")
        .expect("the frozen fixture carries the invalid UTF-8 case");

    let error = decode_query(&parse_request_target(&case.target).query)
        .expect_err("an invalid UTF-8 escape must be rejected");
    assert_eq!(error.code(), "InvalidUrl");

    match import_request_target(&case.target) {
        Err(error) => assert_eq!(error.code(), "InvalidUrl"),
        Ok(outcome) => panic!("the invalid UTF-8 case imported as {outcome:?}"),
    }
}

/// A policy the test itself supplies: no production hostname is guessed.
fn test_policy() -> UrlPolicy {
    UrlPolicy {
        scheme: "https".to_owned(),
        host: "fretboard.example".to_owned(),
        path: "/".to_owned(),
        max_bytes: 2_048,
    }
}

/// An absolute URL inside the supplied policy imports the page its query names.
#[test]
fn an_absolute_url_inside_the_supplied_policy_imports_its_page() {
    let policy = test_policy();

    let page = match import_absolute_url("https://fretboard.example/?chords=Cmaj", &policy) {
        Ok(ImportedPage::Route(state)) => state,
        other => panic!("the canonical share URL must import a page, got {other:?}"),
    };

    assert_eq!(
        decoded_page(&page),
        serde_json::json!({
            "active_chords": [{"root": "C", "quality": "major"}],
            "highlighted_chord": Value::Null,
            "instrument": "guitar",
            "selection": {},
            "tab": "visualizer",
            "tuning_state": {"pitches": [40, 45, 50, 55, 59, 64], "reference": "Standard"},
        }),
        "the canonical share URL must import the page its query names"
    );
}

/// Everything outside the supplied policy is refused with the share policy's
/// own code, and nothing is imported.
#[test]
fn an_absolute_url_outside_the_supplied_policy_is_refused() {
    let policy = test_policy();
    let refused = [
        // A scheme the policy does not allow.
        "javascript://fretboard.example/?chords=Cmaj",
        "file://fretboard.example/?chords=Cmaj",
        "http://fretboard.example/?chords=Cmaj",
        // Credentials in the authority.
        "https://user:secret@fretboard.example/?chords=Cmaj",
        // A lookalike host that merely ends with the allowed one.
        "https://fretboard.example.evil.test/?chords=Cmaj",
        // A path the policy does not allow.
        "https://fretboard.example/nope?chords=Cmaj",
        // Not a URL at all.
        "not a url",
    ];

    for url in refused {
        match import_absolute_url(url, &policy) {
            Err(error) => assert!(
                matches!(error.code(), "UnsupportedOrigin" | "InvalidUrl"),
                "{url} must be refused as an origin or syntax failure, got {error:?}"
            ),
            Ok(outcome) => panic!("{url} was accepted as {outcome:?}"),
        }
    }
}

/// The supplied cap is the cap: a longer input is an explicit refusal, never a
/// truncated import.
#[test]
fn an_input_larger_than_the_supplied_cap_is_refused() {
    let policy = UrlPolicy {
        max_bytes: 32,
        ..test_policy()
    };
    let oversized = format!("https://fretboard.example/?chords={}", "Cmaj,".repeat(16));

    match import_absolute_url(&oversized, &policy) {
        Err(error) => assert_eq!(error.code(), "InputTooLarge", "got {error:?}"),
        Ok(outcome) => panic!("an oversized input imported as {outcome:?}"),
    }
}
