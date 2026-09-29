//! The P6 adapter boundary: the URL import transport (`C19`) and the snapshot
//! envelope (`C20`), driven through the **exported** functions.
//!
//! The domain tests pin the transport rules and the envelope; these pin the
//! crossing — that a URL a caller supplies arrives, becomes the domain's value,
//! and comes back as the page DTO, and that a snapshot string crosses whole. The
//! expectations here are hand-written: the pages the tests compare against are
//! built from DTOs the way the other phase tests build them, and the frozen
//! contract examples (`snapshot-v1.json`, `snapshot-future.json`,
//! `snapshot-corrupt.txt`) are the only files read.
//!
//! ## What the transport refuses and why
//!
//! The policy is supplied by the caller, never by the adapter, so no production
//! hostname appears here. `fretboard.example` is the reserved example domain
//! (`RFC 2606`): a lookalike suffix host, a credential-carrying authority,
//! another scheme, another path or an input over the cap is refused by the
//! domain's own transport rule, and the adapter reports the code it produced.
//!
//! A URL that names a path other than the policy's own is
//! `UnsupportedOrigin`, because `import_absolute_url` compares the path exactly
//! and answers the origin code when it differs. `ImportedPage::NotFound` — a
//! target the transport does not serve — is therefore unreachable through this
//! entry point; the adapter still maps it to `InvalidUrl`, so a bare route (a
//! target that is not an absolute URL at all) and the mapping agree on the one
//! code the frozen taxonomy gives a refused route.
//!
//! ## What the snapshot refuses and why
//!
//! The envelope is the domain's: a version this build cannot read is
//! `UnsupportedSchemaVersion` (read before the page, so a future snapshot is
//! reported as a version problem and not as a corrupt one) and everything else
//! the strict reader refuses — malformed or truncated JSON, an absent key, an
//! unknown field, a structurally inconsistent page — is `InvalidSnapshot`.

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::print_stdout,
    clippy::unwrap_used,
    variant_size_differences
)]

use fretboard_mobile_ffi::{
    AdapterError, ChordDto, ErrorCode, InstrumentDto, InstrumentStateDto, PageStateDto,
    PositionDto, TabDto, TuningDto, UrlPolicyDto, decode_snapshot, encode_snapshot, import_url,
    snapshot_schema_version,
};

/// The frozen snapshot examples of the contract.
const SNAPSHOT_V1: &str = "fixtures/contract/snapshot-v1.json";
const SNAPSHOT_FUTURE: &str = "fixtures/contract/snapshot-future.json";
const SNAPSHOT_CORRUPT: &str = "fixtures/contract/snapshot-corrupt.txt";

/// The canonical URL every accepted case uses, and the cap it needs.
const CANONICAL_URL: &str =
    "https://fretboard.example/?chords=Cmaj,Amin7&highlight=Cmaj&marked=0-3&tab=analyzer";
const GENEROUS_CAP: u64 = 4096;

/// A frozen contract file, read from the repository root.
fn frozen(path: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()))
}

// ---------------------------------------------------------------------------
// Adapter DTO builders (hand-written expectations)
// ---------------------------------------------------------------------------

/// A chord DTO from a wire root and quality.
fn chord(root: &str, quality: &str) -> ChordDto {
    ChordDto {
        root: root.to_owned(),
        quality: quality.to_owned(),
    }
}

/// The frozen guitar Standard tuning, in physical string order.
fn standard_tuning() -> TuningDto {
    TuningDto {
        pitches: vec![40, 45, 50, 55, 59, 64],
        reference: "Standard".to_owned(),
    }
}

/// A guitar page built from its parts, with the frozen Standard tuning.
fn guitar_page(
    chords: Vec<ChordDto>,
    highlight: Option<ChordDto>,
    selected: Vec<PositionDto>,
    tab: TabDto,
) -> PageStateDto {
    PageStateDto {
        instrument: InstrumentStateDto::Fretted {
            instrument: InstrumentDto::Guitar,
            tuning: standard_tuning(),
            selected,
        },
        chords,
        highlight,
        tab,
    }
}

/// The share policy the tests supply: the reserved example host, the page route
/// and a cap generous enough for every canonical input.
fn policy() -> UrlPolicyDto {
    UrlPolicyDto {
        scheme: "https".to_owned(),
        host: "fretboard.example".to_owned(),
        path: "/".to_owned(),
        max_bytes: GENEROUS_CAP,
    }
}

/// The failure of one call, with its code checked.
fn code_of<T: std::fmt::Debug>(result: Result<T, AdapterError>) -> String {
    match result {
        Ok(value) => panic!("expected a failure, got Ok({value:?})"),
        Err(error) => error.code().as_str().to_owned(),
    }
}

// ---------------------------------------------------------------------------
// The URL import transport
// ---------------------------------------------------------------------------

#[test]
fn a_canonical_url_imports_the_page_its_query_names() {
    // The URL the frozen transport serves: the policy's own scheme, authority
    // and path, and a query that names chords, the highlight, a marked position
    // and the analyzer tab.
    let imported = import_url(CANONICAL_URL.to_owned(), policy())
        .unwrap_or_else(|error| panic!("the canonical URL imports: {error:?}"));
    let expected = guitar_page(
        vec![chord("C", "major"), chord("A", "min7")],
        Some(chord("C", "major")),
        vec![PositionDto { string: 0, fret: 3 }],
        TabDto::Analyzer,
    );
    assert_eq!(
        imported, expected,
        "the imported page is the one the query names"
    );

    // An empty query is the default page.
    let bare = import_url("https://fretboard.example/".to_owned(), policy())
        .unwrap_or_else(|error| panic!("the bare route imports: {error:?}"));
    assert_eq!(
        bare,
        guitar_page(Vec::new(), None, Vec::new(), TabDto::Visualizer)
    );

    // One chord, nothing else.
    let single = import_url(
        "https://fretboard.example/?chords=Cmaj".to_owned(),
        policy(),
    )
    .unwrap_or_else(|error| panic!("the single chord imports: {error:?}"));
    assert_eq!(
        single,
        guitar_page(
            vec![chord("C", "major")],
            None,
            Vec::new(),
            TabDto::Visualizer
        )
    );

    // The fragment is not part of the query a browser sends.
    let with_fragment = import_url(
        "https://fretboard.example/?chords=Cmaj#play".to_owned(),
        policy(),
    )
    .unwrap_or_else(|error| panic!("a fragment does not refuse the URL: {error:?}"));
    assert_eq!(with_fragment, single);
}

#[test]
fn every_refusal_the_transport_promises_crosses_as_its_code() {
    let cases: [(&str, &str); 8] = [
        // A lookalike suffix host is not the policy's authority.
        (
            "https://fretboard.example.evil/?chords=Cmaj",
            "UnsupportedOrigin",
        ),
        // Credentials in the authority are refused whatever the host is.
        (
            "https://user:pass@fretboard.example/?chords=Cmaj",
            "UnsupportedOrigin",
        ),
        // Another scheme is not the policy's scheme.
        ("http://fretboard.example/?chords=Cmaj", "UnsupportedOrigin"),
        (
            "javascript://fretboard.example/?chords=Cmaj",
            "UnsupportedOrigin",
        ),
        ("file://fretboard.example/?chords=Cmaj", "UnsupportedOrigin"),
        // Another path is not the policy's path.
        (
            "https://fretboard.example/nope?chords=Cmaj",
            "UnsupportedOrigin",
        ),
        // A non-URL carries no scheme at all.
        ("not a url", "InvalidUrl"),
        // A bare route is not an absolute URL.
        ("/nope", "InvalidUrl"),
    ];

    for (url, expected) in cases {
        assert_eq!(
            code_of(import_url(url.to_owned(), policy())),
            expected,
            "{url}: the transport's own code"
        );
    }
}

#[test]
fn a_url_the_transport_does_not_serve_has_no_code_of_its_own() {
    // The frozen taxonomy has no `NotFound`, so a bare route — the one shape a
    // caller can hand over that the absolute-URL reader cannot parse — crosses
    // as `InvalidUrl`, and the adapter's own `ImportedPage::NotFound` mapping
    // answers the same code rather than inventing one.
    for target in ["/nope", "nope", "", "?chords=Cmaj"] {
        assert_eq!(
            code_of(import_url(target.to_owned(), policy())),
            ErrorCode::InvalidUrl.as_str(),
            "{target:?}: a target that is not an absolute URL"
        );
    }
}

#[test]
fn an_input_over_the_supplied_cap_is_refused_by_the_cap() {
    // The cap is the only difference: the canonical URL is accepted under the
    // generous policy and refused under a policy smaller than it.
    assert!(
        import_url(CANONICAL_URL.to_owned(), policy()).is_ok(),
        "the canonical URL fits the generous cap"
    );

    let small = UrlPolicyDto {
        max_bytes: 24,
        ..policy()
    };
    assert!(
        CANONICAL_URL.len() > 24,
        "the test's URL is longer than the small cap"
    );
    assert_eq!(
        code_of(import_url(CANONICAL_URL.to_owned(), small)),
        ErrorCode::InputTooLarge.as_str(),
        "the input exceeds the supplied cap"
    );
}

// ---------------------------------------------------------------------------
// The snapshot envelope
// ---------------------------------------------------------------------------

#[test]
fn the_snapshot_round_trips_a_page() {
    let page = guitar_page(
        vec![chord("C", "major"), chord("A", "min7"), chord("C", "major")],
        Some(chord("C", "major")),
        vec![
            PositionDto { string: 0, fret: 3 },
            PositionDto { string: 1, fret: 2 },
        ],
        TabDto::Visualizer,
    );

    let encoded = encode_snapshot(page.clone()).unwrap_or_else(|error| panic!("{error:?}"));
    assert!(
        encoded.starts_with("{\"schema_version\":1,\"page\":{"),
        "the frozen envelope shape: {encoded}"
    );

    let decoded = decode_snapshot(encoded).unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(decoded, page, "the page survives the round trip");
}

#[test]
fn the_frozen_snapshot_example_decodes_to_its_page() {
    let document: serde_json::Value =
        serde_json::from_str(&frozen(SNAPSHOT_V1)).expect("the frozen example is JSON");
    assert_eq!(
        document["schema_version"].as_u64(),
        Some(u64::from(snapshot_schema_version())),
        "the frozen example carries the schema version this build reads"
    );

    let decoded = decode_snapshot(frozen(SNAPSHOT_V1)).unwrap_or_else(|error| panic!("{error:?}"));
    let expected = guitar_page(
        vec![chord("C", "major"), chord("A", "min7"), chord("C", "major")],
        Some(chord("C", "major")),
        vec![
            PositionDto { string: 0, fret: 3 },
            PositionDto { string: 1, fret: 2 },
        ],
        TabDto::Visualizer,
    );
    assert_eq!(decoded, expected, "the frozen example's own page");
}

#[test]
fn the_snapshot_schema_version_is_the_frozen_one() {
    assert_eq!(snapshot_schema_version(), 1);
}

#[test]
fn a_future_schema_version_is_refused_before_the_page_is_read() {
    // The version is read first, so a snapshot from a later build is a version
    // problem whatever its page looks like.
    assert_eq!(
        code_of(decode_snapshot(frozen(SNAPSHOT_FUTURE))),
        ErrorCode::UnsupportedSchemaVersion.as_str()
    );
}

#[test]
fn a_corrupt_or_truncated_snapshot_is_refused() {
    let cases: [String; 4] = [
        // The frozen truncated example of the contract.
        frozen(SNAPSHOT_CORRUPT),
        // A hand-truncated envelope.
        "{\"schema_version\":1,\"page\":{\"instrument\"".to_owned(),
        // Not JSON at all.
        "not a snapshot".to_owned(),
        // The envelope without its page.
        "{\"schema_version\":1}".to_owned(),
    ];

    for bytes in cases {
        assert_eq!(
            code_of(decode_snapshot(bytes.clone())),
            ErrorCode::InvalidSnapshot.as_str(),
            "{bytes:?}: the strict reader refuses it"
        );
    }
}

#[test]
fn a_snapshot_with_an_unknown_field_is_refused() {
    // A page field this build does not know: the durable format may not be
    // extended by another build without a version change.
    let unknown_page_field = "{\"schema_version\":1,\"page\":{\"instrument\":\
        {\"kind\":\"fretted\",\"id\":\"guitar\",\"tuning\":\
        {\"pitches\":[40,45,50,55,59,64],\"reference\":\"Standard\"},\"selection\":[]},\
        \"chords\":[],\"highlight\":null,\"tab\":\"visualizer\",\"colour\":\"red\"}}";
    assert_eq!(
        code_of(decode_snapshot(unknown_page_field.to_owned())),
        ErrorCode::InvalidSnapshot.as_str()
    );

    // An envelope field this build does not know.
    let unknown_envelope_field = "{\"schema_version\":1,\"page\":{\"instrument\":\
        {\"kind\":\"fretted\",\"id\":\"guitar\",\"tuning\":\
        {\"pitches\":[40,45,50,55,59,64],\"reference\":\"Standard\"},\"selection\":[]},\
        \"chords\":[],\"highlight\":null,\"tab\":\"visualizer\"},\"written_by\":\"other\"}";
    assert_eq!(
        code_of(decode_snapshot(unknown_envelope_field.to_owned())),
        ErrorCode::InvalidSnapshot.as_str()
    );

    // A structurally inconsistent page is refused by the page's own validator.
    let inconsistent = "{\"schema_version\":1,\"page\":{\"instrument\":\
        {\"kind\":\"fretted\",\"id\":\"guitar\",\"tuning\":\
        {\"pitches\":[40,45,50,55,59,64],\"reference\":\"Standard\"},\"selection\":[]},\
        \"chords\":[],\"highlight\":{\"root\":\"C\",\"quality\":\"major\"},\
        \"tab\":\"visualizer\"}}";
    assert_eq!(
        code_of(decode_snapshot(inconsistent.to_owned())),
        ErrorCode::InvalidSnapshot.as_str(),
        "a highlight absent from the chords is not persisted"
    );
}

#[test]
fn an_inconsistent_page_is_never_encoded() {
    // The page's own validation runs before the envelope is written.
    let inconsistent = guitar_page(
        Vec::new(),
        Some(chord("C", "major")),
        Vec::new(),
        TabDto::Visualizer,
    );
    assert_eq!(
        code_of(encode_snapshot(inconsistent)),
        ErrorCode::InvalidState.as_str()
    );

    // A chord outside the frozen catalog is refused by the conversion, never
    // written into a snapshot.
    let unknown_quality = guitar_page(
        vec![chord("C", "no_such_quality")],
        None,
        Vec::new(),
        TabDto::Visualizer,
    );
    assert_eq!(
        code_of(encode_snapshot(unknown_quality)),
        ErrorCode::UnknownIdentifier.as_str()
    );
}
