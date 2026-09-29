//! The tolerant, origin-free URL import (task `C21`'s second half, needed by `A19`).
//!
//! [`import_absolute_url`] is the *share* reader: it compares the scheme, the
//! authority and the path against an approved policy, because it is what the app
//! accepts from its own links. This module pins the other one — the **legacy** reader
//! behind paste and `ACTION_SEND`, where the origin is deliberately not compared
//! (`DEC-07` is open, and nothing here emits a link: `share_url` is `D08`).
//!
//! What is pinned here is therefore the *transport* half: which inputs are read at
//! all, which route is served, and that the query reaches the legacy page codec
//! untouched. The page semantics themselves are the frozen ones from `C08` and are
//! pinned in `page_params.rs` against the oracle; the expectations below are written
//! in the oracle's own rendering (`decoded_page`), never generated from this
//! implementation.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

mod common;

use fretboard_core::{
    CoreError, ImportedPage, PageState, decode_query, decoded_page, import_legacy_url,
};
use serde_json::{Value, json};

/// The page one URL imported, or a failure that names the URL.
fn page_of(url: &str) -> PageState {
    match import_legacy_url(url) {
        Ok(ImportedPage::Route(state)) => state,
        other => panic!("{url} did not import: {other:?}"),
    }
}

/// The oracle's own rendering of one imported page.
fn rendered(url: &str) -> Value {
    decoded_page(&page_of(url))
}

/// The default page's rendering: what the legacy reader answers for a query that
/// names nothing.
fn defaults() -> Value {
    rendered("https://any.test/")
}

#[test]
fn any_scheme_and_any_authority_are_read() {
    // The origin is not this reader's business: `DEC-07` has no approved host yet,
    // and a pasted link is read whatever host it names.
    for url in [
        "https://fretboard.test/?chords=Cmaj",
        "http://localhost:4000/?chords=Cmaj",
        "https://someone.else.example/?chords=Cmaj",
        "custom-scheme://h/?chords=Cmaj",
    ] {
        assert_eq!(
            json!([{ "root": "C", "quality": "major" }]),
            rendered(url)["active_chords"],
            "{url} was not read"
        );
    }
}

#[test]
fn only_the_page_route_is_served() {
    assert_eq!(
        ImportedPage::NotFound,
        import_legacy_url("https://any.test/nope?chords=Cmaj").expect("a URL is read, not refused"),
    );
}

#[test]
fn a_url_with_no_path_is_the_page_route() {
    assert_eq!(
        json!([{ "root": "C", "quality": "major" }]),
        rendered("https://any.test?chords=Cmaj")["active_chords"],
    );
}

#[test]
fn the_fragment_is_dropped_and_the_query_kept() {
    assert_eq!(
        json!([{ "root": "C", "quality": "major" }]),
        rendered("https://any.test/?chords=Cmaj#section")["active_chords"],
    );
}

#[test]
fn an_unknown_field_is_ignored_and_its_sibling_kept() {
    // The legacy codec's own tolerance: the unknown key is not the import's problem.
    assert_eq!(
        json!([{ "root": "C", "quality": "major" }]),
        rendered("https://any.test/?chords=Cmaj&unknown=value")["active_chords"],
    );
}

#[test]
fn a_bad_field_is_defaulted_rather_than_failing_the_import() {
    // A field the codec cannot use is defaulted; the rest of the page survives.
    assert_eq!(
        defaults()["instrument"],
        rendered("https://any.test/?instrument=horn&chords=Cmaj")["instrument"],
    );
    assert_eq!(
        json!([{ "root": "C", "quality": "major" }]),
        rendered("https://any.test/?instrument=horn&chords=Cmaj")["active_chords"],
    );
}

#[test]
fn the_wire_id_of_the_ukulele_is_exact() {
    assert_eq!(
        json!("ukelele"),
        rendered("https://any.test/?instrument=ukelele")["instrument"],
    );
}

#[test]
fn an_input_without_a_scheme_is_refused() {
    assert!(matches!(
        import_legacy_url("fretboard.test/?chords=Cmaj"),
        Err(CoreError::InvalidUrl { .. })
    ));
    assert!(
        import_legacy_url("?chords=Cmaj").is_err(),
        "a bare query is not an absolute URL"
    );
}

#[test]
fn a_query_that_cannot_be_decoded_is_refused() {
    // A percent escape that is not valid UTF-8 is the one decoding failure the
    // pinned transport has; it must be a rejection here too.
    let escape = decode_query("%FF");
    assert!(escape.is_err(), "the pinned decoder refuses invalid UTF-8");

    assert!(
        import_legacy_url("https://any.test/?chords=%FF").is_err(),
        "the import carries that refusal through"
    );
}
