//! The legacy URL import over the adapter (task `C21`, needed by `A19`).
//!
//! `import_url` is the strict reader: it compares the scheme, the authority and the
//! path against a policy the caller supplies. `import_legacy_url` is the one behind
//! paste and `ACTION_SEND`, and it is pinned here at the boundary — what crosses as a
//! page, what refuses, and with which code — while the domain's transport rules are
//! pinned in `crates/domain/tests/legacy_import.rs` and the page semantics in
//! `page_params.rs`.
//!
//! Every expectation below is hand-written. No production hostname appears, which is
//! the point of this reader: the origin is deliberately not compared, so the tests
//! bring their own reserved and local names.

// Test target: the same relaxations as the other contract tests.
#![allow(clippy::expect_used, clippy::indexing_slicing, clippy::panic)]

use fretboard_mobile_ffi::{
    AdapterError, InstrumentDto, InstrumentStateDto, PageStateDto, TabDto, default_state,
    import_legacy_url,
};

/// The page one imported URL answers, or a failure that names the URL.
fn page_of(url: &str) -> PageStateDto {
    import_legacy_url(url.to_owned()).unwrap_or_else(|error| panic!("{url} was refused: {error:?}"))
}

/// The instrument one fretted page names.
fn fretted_of(page: &PageStateDto) -> InstrumentDto {
    match &page.instrument {
        InstrumentStateDto::Fretted { instrument, .. } => *instrument,
        InstrumentStateDto::Piano { .. } => {
            panic!("expected a fretted instrument, got the keyboard")
        }
    }
}

#[test]
fn a_legacy_url_of_any_origin_crosses_as_the_page_its_query_names() {
    let page = page_of("https://someone.else.example/?chords=Cmaj");

    assert_eq!(1, page.chords.len(), "one chord in {page:?}");
    assert_eq!("C", page.chords[0].root);
    assert_eq!("major", page.chords[0].quality);
    assert_eq!(InstrumentDto::Guitar, fretted_of(&page));
    assert_eq!(TabDto::Visualizer, page.tab);
    assert_eq!(None, page.highlight);
}

#[test]
fn a_query_that_names_nothing_is_the_default_page() {
    assert_eq!(default_state(), page_of("https://any.test/"));
}

#[test]
fn a_path_this_transport_does_not_serve_is_refused_as_an_invalid_url() {
    let refusal = import_legacy_url("https://any.test/nope?chords=Cmaj".to_owned())
        .expect_err("another path is not a route");

    assert!(
        matches!(refusal, AdapterError::InvalidUrl { .. }),
        "{refusal:?}"
    );
    assert_eq!(Some("url"), refusal.field());
}

#[test]
fn an_input_that_is_not_an_absolute_url_is_refused() {
    for input in ["fretboard.test/?chords=Cmaj", "?chords=Cmaj", ""] {
        assert!(
            matches!(
                import_legacy_url(input.to_owned()),
                Err(AdapterError::InvalidUrl { .. })
            ),
            "{input:?} was not refused"
        );
    }
}

#[test]
fn a_field_the_codec_cannot_use_is_defaulted_and_its_sibling_kept() {
    let page = page_of("https://any.test/?instrument=horn&chords=Cmaj");

    assert_eq!(
        InstrumentDto::Guitar,
        fretted_of(&page),
        "an unknown id is not the import's problem",
    );
    assert_eq!("C", page.chords[0].root);
}

#[test]
fn the_wire_id_of_the_ukulele_crosses_exactly() {
    assert_eq!(
        InstrumentDto::Ukelele,
        fretted_of(&page_of("https://any.test/?instrument=ukelele")),
    );
}

#[test]
fn a_query_the_pinned_decoder_cannot_read_is_refused() {
    // A percent escape that is not valid UTF-8 is the one decoding failure the
    // transport has, and it crosses as `InvalidUrl` rather than as a page.
    assert!(matches!(
        import_legacy_url("https://any.test/?chords=%FF".to_owned()),
        Err(AdapterError::InvalidUrl { .. })
    ));
}
