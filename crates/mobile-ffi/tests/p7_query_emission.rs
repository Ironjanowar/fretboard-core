//! The page→query emission over the adapter (task `A20`'s core half).
//!
//! The pages below come *from the engine*, by importing a link — hand-written input, never
//! output-derived — and what is pinned is the boundary: the query crosses as text, it
//! carries no base and no leading `?` (the approved origin does not exist yet, `DEC-07`),
//! and the assertion that matters is that **the link a client builds from it imports back
//! to the same page**, through the very reader `A19` uses. The domain's spelling rules are
//! pinned against the frozen transport fixture in `crates/domain/tests/query_emission.rs`;
//! this file pins that the adapter neither adds nor loses anything on the way out.
//!
//! No production hostname appears: the bases below are reserved names, because no origin is
//! compared on either side.

// Test target: the same relaxations as the other contract tests.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use fretboard_mobile_ffi::{PageStateDto, default_state, encode_page_query, import_legacy_url};

/// The page one link names, as the engine reads it.
fn page_of(query: &str) -> PageStateDto {
    import_legacy_url(format!("https://any.test/?{query}"))
        .unwrap_or_else(|error| panic!("{query} did not import: {error:?}"))
}

/// The page one emitted query names when a client appends it to its base.
fn round_trip(page: &PageStateDto) -> PageStateDto {
    let query = encode_page_query(page.clone()).expect("the engine encodes the page it read");
    page_of(&query)
}

#[test]
fn the_default_page_emits_an_empty_query() {
    assert_eq!(
        "",
        encode_page_query(default_state()).expect("the default page encodes")
    );
}

#[test]
fn the_query_carries_no_base_and_no_question_mark() {
    let query = encode_page_query(page_of("chords=Cmaj")).expect("a page with one chord encodes");

    assert!(
        !query.contains('/'),
        "a query must not carry a path: {query}"
    );
    assert!(
        !query.contains("https://"),
        "a query must not carry an origin: {query}"
    );
    assert_eq!("chords=Cmaj", query);
}

#[test]
fn a_chord_page_comes_back_unchanged() {
    let original = page_of("chords=Cmaj&chords=Amin");

    assert_eq!(original, round_trip(&original));
}

#[test]
fn a_marked_position_comes_back_unchanged() {
    let original = page_of("chords=Cmaj&marked=5-2");

    assert_eq!(original, round_trip(&original));
}

#[test]
fn a_sharp_root_survives_the_trip_through_the_query() {
    // The frozen transport ends a query at a raw `#`, so this only works because the
    // emitter escaped it: `query_transport/raw-fragment-not-a-sharp` reads a raw one as `C`.
    let original = page_of("chords=C%23maj");

    assert_eq!(
        "chords=C%23maj",
        encode_page_query(original.clone()).expect("encodes")
    );
    assert_eq!(original, round_trip(&original));
}

#[test]
fn a_non_default_tuning_comes_back_unchanged() {
    // A reference with a space is what forces the `+` spelling on the way out.
    let original = page_of("pitches=38,45,50,55,59,64&reference=Drop+D");

    assert_eq!(original, round_trip(&original));
}

#[test]
fn the_piano_comes_back_unchanged() {
    let original = page_of("instrument=piano&keys=60,64&tab=analyzer");

    assert_eq!(original, round_trip(&original));
}
