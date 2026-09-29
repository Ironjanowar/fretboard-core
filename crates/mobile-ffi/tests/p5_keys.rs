//! The P5 adapter boundary: the key panel, the multi-key panel and the
//! progression catalog, driven through the **exported** functions.
//!
//! The domain tests pin the musical rules; these pin the crossing — that the
//! recorded inputs arrive, become the domain's values, and come back as the DTOs
//! the frozen oracle records. Every expectation is read from a frozen fixture
//! (`keys.jsonl`, `key-groups.jsonl`, `multi-keys.jsonl`, `progressions.jsonl`,
//! `scales.jsonl`, `catalogs.json`, `page-events.jsonl`) and never retyped from
//! the plan.
//!
//! ## What the page gate means at the boundary
//!
//! The two key entry points respect the page's own gates, so three things the
//! raw fixtures record are deliberately *not* reproduced through them, and are
//! asserted instead:
//!
//! * `key_suggestions` answers nothing below two active chords
//!   (`KEY_PAGE_MIN_CHORDS`), so the `keys.jsonl` records with fewer chords cross
//!   as an empty row list while their raw answer is not empty;
//! * `multi_key_suggestions` answers nothing below three chords *and* nothing
//!   when the single-key operation found a key, so the records of
//!   `multi-keys.jsonl` whose chords share a key cross as an empty group list
//!   while the raw answer has groups;
//! * the wire carries sharp chord roots only (`CORE-D06`), so the two
//!   `keys.jsonl` records spelled with flats — `suggest_keys/flat-root-eb-major`
//!   (`Contract.D05`) and `suggest_keys/flat-root-Bb-with-g` — cannot be driven
//!   through a page at all; the flat page is rejected and the `Contract.D05`
//!   record is asserted through its sharp spelling.
//!
//! The countable facts (16/17 keys records, the four `Contract.D06` case ids,
//! the 15 raw multi-key records) are pinned by the domain tests; here the
//! adapter's own counts are pinned as constants so a change is a visible
//! failure.

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

use std::collections::BTreeSet;

use fretboard_mobile_ffi::{
    AdapterError, ChordDto, ChordModeDto, InstrumentDto, InstrumentStateDto, KeyRowDto,
    KeySuggestionDto, MultiKeyGroupDto, PageEventDto, PageStateDto, ProgressionGroupDto, TabDto,
    TuningDto, apply_page_event, default_state, diatonic_chords, group_key_suggestions,
    key_suggestions, multi_key_suggestions, progression_chords, progressions,
};
use serde_json::Value;

const KEYS: &str = "fixtures/oracle/keys.jsonl";
const KEY_GROUPS: &str = "fixtures/oracle/key-groups.jsonl";
const MULTI_KEYS: &str = "fixtures/oracle/multi-keys.jsonl";
const PROGRESSIONS: &str = "fixtures/oracle/progressions.jsonl";
const SCALES: &str = "fixtures/oracle/scales.jsonl";
const CATALOGS: &str = "fixtures/oracle/catalogs.json";
const PAGE_EVENTS: &str = "fixtures/oracle/page-events.jsonl";

/// The frozen fixture counts.
const KEYS_RECORDS: usize = 17;
const KEYS_GATED_RECORDS: usize = 3;
const KEYS_FLAT_ROOTED_RECORDS: usize = 2;
const KEYS_IDENTICAL_RECORDS: usize = 12;
const KEY_GROUPS_RECORDS: usize = 16;
const KEY_GROUPS_IDENTICAL_RECORDS: usize = 12;
const MULTI_KEYS_RECORDS: usize = 15;
const MULTI_KEYS_REACHABLE_RECORDS: usize = 5;
const MULTI_KEYS_SUPPRESSED_RECORDS: usize = 7;
const DIATONIC_CHORD_RECORDS: usize = 360;
const PROGRESSION_DEFINITION_RECORDS: usize = 59;
const PROGRESSION_LABEL_RECORDS: usize = 59;
const PROGRESSION_CHORD_RECORDS: usize = 1062;

/// The one `Contract.D05` record: the flat-rooted input the wire cannot carry.
const DEVIATING_KEYS_CASE: &str = "suggest_keys/flat-root-eb-major";

/// The second flat-rooted record, which answers nothing either way.
const EMPTY_FLAT_CASE: &str = "suggest_keys/flat-root-Bb-with-g";

/// The four `Contract.D06` case ids, by name (`approved-deviations.json`).
const DEVIATING_GROUP_CASES: [&str; 4] = [
    "key_groups/single-modal-suggestion-incomplete-group",
    "key_groups/incomplete-modal-pair-only",
    "key_groups/incomplete-modal-pair-with-non-modal-survivor",
    "key_groups/incomplete-sibling-group-dropped",
];

/// The seven keys/progressions `page-event` cases (task `C18`).
const OWNED_EVENTS: [&str; 7] = [
    "page_event/apply-key-modal-resets-preview-on-open",
    "page_event/apply-key-sevenths",
    "page_event/apply-key-triads-replaces-chords",
    "page_event/apply-progression-after-piano-instrument",
    "page_event/apply-progression-replaces-chords",
    "page_event/apply-suggested-key-inherits-seventh-mode",
    "page_event/apply-suggested-key-triads-from-existing-mode",
];

// ---------------------------------------------------------------------------
// Frozen fixtures
// ---------------------------------------------------------------------------

/// Every record of a frozen JSONL fixture.
fn records(path: &str) -> Vec<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()))
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every record is JSON"))
        .collect()
}

/// A frozen JSON document.
fn document(path: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(path);
    serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("{} must be JSON: {error}", path.display()))
}

/// The identifier of a record.
fn case_id(record: &Value) -> String {
    record["case_id"]
        .as_str()
        .expect("every record has a case id")
        .to_owned()
}

/// One record of a fixture by case id, or a loud failure naming the missing case.
fn fixture_case(fixture: &str, wanted: &str) -> Value {
    records(fixture)
        .into_iter()
        .find(|record| record["case_id"].as_str() == Some(wanted))
        .unwrap_or_else(|| panic!("the fixture has no case {wanted}"))
}

// ---------------------------------------------------------------------------
// Adapter DTO builders
// ---------------------------------------------------------------------------

/// A chord DTO from a wire root and quality.
fn chord(root: &str, quality: &str) -> ChordDto {
    ChordDto {
        root: root.to_owned(),
        quality: quality.to_owned(),
    }
}

/// The chord DTOs of a fixture chord array, keeping the recorded root spelling.
fn dto_chords(chords: &Value) -> Vec<ChordDto> {
    chords
        .as_array()
        .expect("a chord array")
        .iter()
        .map(|entry| {
            chord(
                entry["root"].as_str().expect("a root"),
                entry["quality"].as_str().expect("a quality"),
            )
        })
        .collect()
}

/// A guitar page carrying these chords, with the oracle Standard tuning and no
/// selection — the shape a client holds when it asks for key suggestions.
fn page_with_chords(chords: Vec<ChordDto>) -> PageStateDto {
    let mut page = default_state();
    page.chords = chords;
    page
}

/// One suggestion DTO from a fixture suggestion object.
fn dto_suggestion(item: &Value) -> KeySuggestionDto {
    KeySuggestionDto {
        tonic: item["tonic"].as_str().expect("a tonic").to_owned(),
        scale: item["scale_type"]
            .as_str()
            .expect("a scale type")
            .to_owned(),
        score: item["score"].as_u64().expect("a score"),
        total: item["total"].as_u64().expect("a total"),
    }
}

/// The suggestion DTOs of a fixture list.
fn dto_suggestions(items: &Value) -> Vec<KeySuggestionDto> {
    items
        .as_array()
        .expect("a suggestion array")
        .iter()
        .map(dto_suggestion)
        .collect()
}

// ---------------------------------------------------------------------------
// Renderers (the fixture's own text form)
// ---------------------------------------------------------------------------

/// One suggestion as `tonic:scale:score:total`.
fn render_suggestion(item: &KeySuggestionDto) -> String {
    format!(
        "{}:{}:{}:{}",
        item.tonic, item.scale, item.score, item.total
    )
}

/// The rows of a row list in the canonical text form.
fn our_rows(rows: &[KeyRowDto]) -> Vec<String> {
    rows.iter()
        .map(|row| match row {
            KeyRowDto::Single { item } => format!("S[{}]", render_suggestion(item)),
            KeyRowDto::Group { prominent, others } => format!(
                "G[{}|{}]",
                prominent
                    .iter()
                    .map(render_suggestion)
                    .collect::<Vec<String>>()
                    .join(";"),
                others
                    .iter()
                    .map(render_suggestion)
                    .collect::<Vec<String>>()
                    .join(";")
            ),
        })
        .collect()
}

/// Every suggestion a row list shows, in display order.
fn shown_suggestions(rows: &[KeyRowDto]) -> Vec<KeySuggestionDto> {
    let mut shown = Vec::new();
    for row in rows {
        match row {
            KeyRowDto::Single { item } => shown.push(item.clone()),
            KeyRowDto::Group { prominent, others } => {
                shown.extend(prominent.iter().cloned());
                shown.extend(others.iter().cloned());
            }
        }
    }
    shown
}

/// The frozen key-groups rows of a record, in the canonical text form.
fn frozen_rows(record: &Value) -> Vec<String> {
    record["output"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no rows", case_id(record)))
        .iter()
        .map(|row| {
            if row["collapsed?"].as_bool() == Some(true) {
                let prominent = row["prominent"].as_array().expect("prominent");
                let others = row["others"].as_array().expect("others");
                format!(
                    "G[{}|{}]",
                    prominent
                        .iter()
                        .map(|item| render_suggestion(&dto_suggestion(item)))
                        .collect::<Vec<String>>()
                        .join(";"),
                    others
                        .iter()
                        .map(|item| render_suggestion(&dto_suggestion(item)))
                        .collect::<Vec<String>>()
                        .join(";")
                )
            } else {
                format!("S[{}]", render_suggestion(&dto_suggestion(&row["item"])))
            }
        })
        .collect()
}

/// One multi-key group in the canonical text form.
fn render_group(group: &MultiKeyGroupDto) -> String {
    let key = group.key.as_ref().map_or_else(
        || "nil".to_owned(),
        |key| format!("{}:{}:{}:{}", key.tonic, key.scale, key.score, key.total),
    );
    format!(
        "[{key}|{}]",
        group
            .chords
            .iter()
            .map(|chord| format!("{}:{}", chord.root, chord.quality))
            .collect::<Vec<String>>()
            .join(",")
    )
}

/// The frozen multi-key groups of a record, in the same form.
fn frozen_groups(record: &Value) -> Vec<String> {
    record["output"]["groups"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no groups", case_id(record)))
        .iter()
        .map(|group| {
            let key = if group["key"].is_null() {
                "nil".to_owned()
            } else {
                format!(
                    "{}:{}:{}:{}",
                    group["key"]["tonic"].as_str().expect("a tonic"),
                    group["key"]["scale_type"].as_str().expect("a scale type"),
                    group["key"]["score"].as_u64().expect("a score"),
                    group["key"]["total"].as_u64().expect("a total")
                )
            };
            format!(
                "[{key}|{}]",
                group["chords"]
                    .as_array()
                    .expect("a group carries its chords")
                    .iter()
                    .map(|chord| format!(
                        "{}:{}",
                        chord["root"].as_str().expect("a root"),
                        chord["quality"].as_str().expect("a quality")
                    ))
                    .collect::<Vec<String>>()
                    .join(",")
            )
        })
        .collect()
}

/// One progression definition as the frozen export writes it.
fn frozen_definition(definition: &Value) -> Vec<String> {
    let degrees: Vec<String> = definition["degrees"]
        .as_array()
        .expect("degrees")
        .iter()
        .map(|degree| {
            let quality = degree["quality"]
                .as_str()
                .map_or_else(|| "nil".to_owned(), str::to_owned);
            format!(
                "{}:{}:{}",
                degree["degree"].as_u64().expect("a degree number"),
                degree["accidental"].as_i64().expect("an accidental"),
                quality
            )
        })
        .collect();
    let songs: Vec<String> = definition["notable_songs"]
        .as_array()
        .expect("notable_songs")
        .iter()
        .map(|song| song.as_str().expect("a song").to_owned())
        .collect();
    vec![
        definition["id"].as_str().expect("an id").to_owned(),
        definition["name"].as_str().expect("a name").to_owned(),
        definition["category"]
            .as_str()
            .expect("a category")
            .to_owned(),
        definition["genre"].as_str().expect("a genre").to_owned(),
        definition["description"]
            .as_str()
            .expect("a description")
            .to_owned(),
        definition["example_key"]
            .as_str()
            .expect("an example key")
            .to_owned(),
        definition["scale_type"]
            .as_str()
            .expect("a scale type")
            .to_owned(),
        degrees.join(","),
        songs.join("|"),
    ]
}

/// The same shape, rendered from an adapter progression DTO.
fn dto_definition(progression: &fretboard_mobile_ffi::ProgressionDto) -> Vec<String> {
    let degrees: Vec<String> = progression
        .degrees
        .iter()
        .map(|degree| {
            let quality = degree.quality.clone().unwrap_or_else(|| "nil".to_owned());
            format!("{}:{}:{}", degree.degree, degree.accidental, quality)
        })
        .collect();
    vec![
        progression.id.clone(),
        progression.name.clone(),
        progression.category.clone(),
        progression.genre.clone(),
        progression.description.clone(),
        progression.example_key.clone(),
        progression.scale.clone(),
        degrees.join(","),
        progression.notable_songs.join("|"),
    ]
}

/// One chord as `root:quality`.
fn render_chord(chord: &ChordDto) -> String {
    format!("{}:{}", chord.root, chord.quality)
}

// ---------------------------------------------------------------------------
// Error helper
// ---------------------------------------------------------------------------

/// The failure of one call, with its code checked.
fn code_of<T: std::fmt::Debug>(result: Result<T, AdapterError>) -> String {
    match result {
        Ok(value) => panic!("expected a failure, got Ok({value:?})"),
        Err(error) => error.code().as_str().to_owned(),
    }
}

// ---------------------------------------------------------------------------
// keys.jsonl through key_suggestions
// ---------------------------------------------------------------------------

#[test]
fn every_wire_legal_key_case_answers_the_frozen_rows() {
    let all = records(KEYS);
    assert_eq!(
        all.len(),
        KEYS_RECORDS,
        "the frozen fixture carries 17 cases"
    );

    let mut gated = 0usize;
    let mut flat = 0usize;
    let mut identical = 0usize;

    for record in &all {
        assert_eq!(
            record["operation"].as_str(),
            Some("suggest_keys"),
            "an unexpected operation"
        );
        let case = case_id(record);
        let chords = dto_chords(&record["input"]["chords"]);
        let page = page_with_chords(chords.clone());

        match key_suggestions(page) {
            Err(error) => {
                assert_eq!(
                    error.code().as_str(),
                    "UnknownIdentifier",
                    "{case}: a flat root is not a wire value"
                );
                assert!(
                    chords
                        .iter()
                        .any(|chord| fretboard_core::note_index(&chord.root).is_ok()
                            && chord.root
                                != fretboard_core::note_index(&chord.root)
                                    .expect("a note")
                                    .name()),
                    "{case}: only a flat-rooted record is rejected here"
                );
                flat += 1;
            }
            Ok(rows) => {
                if chords.len() < 2 {
                    assert!(
                        rows.is_empty(),
                        "{case}: the page gate answers nothing below two chords"
                    );
                    gated += 1;
                    continue;
                }
                let expected =
                    group_key_suggestions(dto_suggestions(&record["output"]["suggestions"]))
                        .expect("the frozen suggestions group");
                assert_eq!(
                    our_rows(&rows),
                    our_rows(&expected),
                    "{case}: the rows differ from the frozen answer grouped"
                );
                identical += 1;
            }
        }
    }

    assert_eq!(
        gated, KEYS_GATED_RECORDS,
        "records below the two-chord gate"
    );
    assert_eq!(flat, KEYS_FLAT_ROOTED_RECORDS, "flat-rooted records");
    assert_eq!(
        identical, KEYS_IDENTICAL_RECORDS,
        "records the wire can carry"
    );
}

#[test]
fn the_recorded_flat_root_deviation_is_unreachable_and_score_only() {
    // `Contract.D05`: the baseline scores a flat-rooted record 0:2 while the
    // sharp spelling scores 2:2. The wire carries sharp roots only (`CORE-D06`),
    // so the record is driven through its sharp spelling here; membership is
    // identical and only the score moves.
    let record = fixture_case(KEYS, DEVIATING_KEYS_CASE);
    let flat_page = page_with_chords(dto_chords(&record["input"]["chords"]));
    assert_eq!(
        code_of(key_suggestions(flat_page)),
        "UnknownIdentifier",
        "the flat-rooted page is rejected: the record is unreachable on the wire"
    );

    let sharp_chords: Vec<ChordDto> = record["input"]["chords"]
        .as_array()
        .expect("chords")
        .iter()
        .map(|entry| {
            let root = entry["root"].as_str().expect("a root");
            chord(
                fretboard_core::note_index(root)
                    .expect("a note name")
                    .name(),
                entry["quality"].as_str().expect("a quality"),
            )
        })
        .collect();
    let rows = key_suggestions(page_with_chords(sharp_chords)).expect("the sharp page answers");
    let shown = shown_suggestions(&rows);

    let frozen = dto_suggestions(&record["output"]["suggestions"]);
    assert_eq!(shown.len(), frozen.len(), "the same suggestions");
    assert!(
        frozen.iter().all(|item| item.score == 0 && item.total == 2),
        "the baseline answers 0:2 for every suggestion"
    );
    assert!(
        shown.iter().all(|item| item.score == 2 && item.total == 2),
        "the native pitch class answers 2:2 for every suggestion"
    );

    // Membership and order, compared as sets because the panel groups the rows.
    let identity =
        |item: &KeySuggestionDto| format!("{}:{}:{}", item.tonic, item.scale, item.total);
    assert_eq!(
        shown.iter().map(identity).collect::<BTreeSet<String>>(),
        frozen.iter().map(identity).collect::<BTreeSet<String>>(),
        "only the score moved: membership and order are identical"
    );

    // The other flat record answers nothing either way, so the deviation is one
    // record and not a class of them.
    let other = fixture_case(KEYS, EMPTY_FLAT_CASE);
    assert!(
        dto_suggestions(&other["output"]["suggestions"]).is_empty(),
        "the second flat record answers nothing in the baseline"
    );
    assert_eq!(
        code_of(key_suggestions(page_with_chords(dto_chords(
            &other["input"]["chords"]
        )))),
        "UnknownIdentifier",
        "and it is unreachable on the wire too"
    );
}

// ---------------------------------------------------------------------------
// key-groups.jsonl through group_key_suggestions
// ---------------------------------------------------------------------------

#[test]
fn every_frozen_group_case_answers_its_rows_except_the_four_recorded_deviations() {
    let all = records(KEY_GROUPS);
    assert_eq!(all.len(), KEY_GROUPS_RECORDS);

    let mut identical = 0usize;
    let mut deviating: BTreeSet<String> = BTreeSet::new();

    for record in &all {
        let case = case_id(record);
        let rows = group_key_suggestions(dto_suggestions(&record["input"]))
            .unwrap_or_else(|error| panic!("{case}: the frozen input groups: {error:?}"));
        let ours = our_rows(&rows);
        let frozen = frozen_rows(record);
        if ours == frozen {
            identical += 1;
            continue;
        }
        assert!(
            DEVIATING_GROUP_CASES.contains(&case.as_str()),
            "{case}: the rows differ outside the recorded Contract.D06 deviation"
        );
        deviating.insert(case);
    }

    assert_eq!(identical, KEY_GROUPS_IDENTICAL_RECORDS);
    assert_eq!(
        deviating,
        DEVIATING_GROUP_CASES
            .map(str::to_owned)
            .into_iter()
            .collect(),
        "the deviating records are exactly the four the ledger names"
    );
}

#[test]
fn the_deviating_group_records_show_the_incomplete_groups_the_baseline_drops() {
    // `Contract.D06`, option B: a perfect suggestion is never dropped. The two
    // records whose baseline answer is empty show their incomplete group, and the
    // sibling record keeps the group the baseline hides.
    for case in [DEVIATING_GROUP_CASES[0], DEVIATING_GROUP_CASES[1]] {
        let record = fixture_case(KEY_GROUPS, case);
        assert!(
            frozen_rows(&record).is_empty(),
            "{case}: the baseline shows nothing"
        );
        let rows = group_key_suggestions(dto_suggestions(&record["input"]))
            .unwrap_or_else(|error| panic!("{case}: {error:?}"));
        assert!(
            !our_rows(&rows).is_empty(),
            "{case}: the option-B rule shows the incomplete group"
        );
    }

    let sibling = fixture_case(KEY_GROUPS, DEVIATING_GROUP_CASES[3]);
    let rows = group_key_suggestions(dto_suggestions(&sibling["input"]))
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(frozen_rows(&sibling).len(), 1, "the baseline shows one row");
    assert_eq!(
        rows.len(),
        2,
        "the option-B rule shows the second group too"
    );
}

#[test]
fn a_perfect_seven_mode_group_collapses_its_prominent_pair() {
    let record = fixture_case(KEY_GROUPS, "key_groups/complete-seven-mode-shared-note-set");
    let rows = group_key_suggestions(dto_suggestions(&record["input"]))
        .unwrap_or_else(|error| panic!("{error:?}"));
    assert_eq!(our_rows(&rows), frozen_rows(&record), "the frozen row");

    let group = rows.iter().find_map(|row| match row {
        KeyRowDto::Group { prominent, others } => Some((prominent, others)),
        KeyRowDto::Single { .. } => None,
    });
    let (prominent, others) = group.expect("the record carries a collapsed row");
    assert_eq!(prominent.len(), 2, "major and minor are prominent");
    assert_eq!(prominent[0].scale, "major");
    assert_eq!(prominent[1].scale, "minor");
    assert_eq!(others.len(), 5, "the other five modes");
}

// ---------------------------------------------------------------------------
// multi-keys.jsonl through multi_key_suggestions
// ---------------------------------------------------------------------------

#[test]
fn every_reachable_multi_key_case_answers_the_frozen_groups() {
    let all = records(MULTI_KEYS);
    assert_eq!(all.len(), MULTI_KEYS_RECORDS);

    let mut reachable = 0usize;
    let mut suppressed = 0usize;

    for record in &all {
        let case = case_id(record);
        let chords = dto_chords(&record["input"]["chords"]);
        let page = page_with_chords(chords.clone());
        let groups = multi_key_suggestions(page)
            .unwrap_or_else(|error| panic!("{case}: the page answers: {error:?}"));
        let ours = groups.iter().map(render_group).collect::<Vec<String>>();
        let frozen = frozen_groups(record);

        // The gate the adapter respects: three active chords and no single key.
        let domain_chords: Vec<fretboard_core::ChordSpec> = chords
            .iter()
            .map(|chord| fretboard_core::ChordSpec {
                root: fretboard_core::note_index(&chord.root).expect("a note name"),
                quality: chord.quality.parse().expect("a catalog quality"),
            })
            .collect();
        let single = fretboard_core::page_key_suggestions(&domain_chords);
        if single.is_empty() && domain_chords.len() >= 3 {
            assert_eq!(
                ours, frozen,
                "{case}: the groups differ from the frozen answer"
            );
            reachable += 1;
        } else {
            assert!(
                ours.is_empty(),
                "{case}: the page gate suppresses the multi-key panel"
            );
            if !frozen.is_empty() {
                suppressed += 1;
            }
        }
    }

    assert_eq!(reachable, MULTI_KEYS_REACHABLE_RECORDS);
    assert_eq!(suppressed, MULTI_KEYS_SUPPRESSED_RECORDS);
}

#[test]
fn an_overlapping_group_carries_its_full_displayed_membership() {
    // The plan's named case: `Dmin, Gmaj, Emaj, Fmaj`. Two groups, and `D min`
    // appears in both because a group displays every chord the key contains.
    let record = fixture_case(MULTI_KEYS, "suggest_multi_keys/dmin-gmaj-emaj-fmaj");
    let groups = multi_key_suggestions(page_with_chords(dto_chords(&record["input"]["chords"])))
        .expect("the page answers");
    assert_eq!(
        groups.iter().map(render_group).collect::<Vec<String>>(),
        frozen_groups(&record)
    );
    assert_eq!(groups.len(), 2, "two groups");
    assert!(groups.iter().all(|group| group.key.is_some()));
}

// ---------------------------------------------------------------------------
// diatonic_chords against scales.jsonl
// ---------------------------------------------------------------------------

#[test]
fn every_frozen_diatonic_chord_record_is_the_adapter_answer() {
    let all: Vec<Value> = records(SCALES)
        .into_iter()
        .filter(|record| record["operation"].as_str() == Some("diatonic_chords"))
        .collect();
    assert_eq!(all.len(), DIATONIC_CHORD_RECORDS);

    for record in &all {
        let case = case_id(record);
        let input = &record["input"];
        let mode = ChordModeDto::parse(input["mode"].as_str().expect("a mode"))
            .unwrap_or_else(|error| panic!("{case}: {error:?}"));
        let chords = diatonic_chords(
            input["tonic"].as_str().expect("a tonic").to_owned(),
            input["scale_type"].as_str().expect("a scale").to_owned(),
            mode,
        )
        .unwrap_or_else(|error| panic!("{case}: {error:?}"));
        let ours: Vec<String> = chords.iter().map(render_chord).collect();
        let frozen: Vec<String> = record["output"]["chords"]
            .as_array()
            .expect("chords")
            .iter()
            .map(|chord| {
                format!(
                    "{}:{}",
                    chord["root"].as_str().expect("a root"),
                    chord["quality"].as_str().expect("a quality")
                )
            })
            .collect();
        assert_eq!(ours, frozen, "{case}: the diatonic chords differ");
    }
}

#[test]
fn the_two_modes_and_their_rejections_cross_the_boundary() {
    // The two wire spellings are the modal's own, and a mode it does not offer is
    // refused rather than read as a neighbour.
    assert_eq!(ChordModeDto::Triad.as_str(), "triad");
    assert_eq!(ChordModeDto::Seventh.as_str(), "seventh");
    for (wire, mode) in [
        ("triad", ChordModeDto::Triad),
        ("seventh", ChordModeDto::Seventh),
    ] {
        assert_eq!(ChordModeDto::parse(wire).expect("a mode"), mode);
    }
    assert_eq!(
        code_of(ChordModeDto::parse("ninth").map(|_| ())),
        "UnknownIdentifier"
    );

    assert_eq!(
        code_of(diatonic_chords(
            "H".to_owned(),
            "major".to_owned(),
            ChordModeDto::Triad
        )),
        "UnknownIdentifier",
        "an unknown tonic"
    );
    assert_eq!(
        code_of(diatonic_chords(
            "C".to_owned(),
            "no_such_scale".to_owned(),
            ChordModeDto::Triad
        )),
        "UnknownIdentifier",
        "an unknown scale"
    );
}

// ---------------------------------------------------------------------------
// The progression catalog and its chords
// ---------------------------------------------------------------------------

#[test]
fn the_progression_catalog_crosses_grouped_and_whole() {
    let catalogs = document(CATALOGS);
    let grouped = catalogs["grouped_progressions"]
        .as_array()
        .expect("grouped_progressions");
    let definitions = catalogs["progression_definitions"]
        .as_array()
        .expect("progression_definitions");
    assert_eq!(definitions.len(), PROGRESSION_DEFINITION_RECORDS);

    let ours: Vec<ProgressionGroupDto> = progressions();
    assert_eq!(ours.len(), grouped.len(), "the catalog's own grouping");

    for (group, frozen) in ours.iter().zip(grouped) {
        assert_eq!(
            group.category,
            frozen["category"].as_str().expect("a category")
        );
        let frozen_ids: Vec<String> = frozen["progressions"]
            .as_array()
            .expect("a group carries its progressions")
            .iter()
            .map(|entry| entry["id"].as_str().expect("an id").to_owned())
            .collect();
        let our_ids: Vec<String> = group
            .progressions
            .iter()
            .map(|progression| progression.id.clone())
            .collect();
        assert_eq!(our_ids, frozen_ids, "the group's order is the frozen order");
    }

    // Every definition crosses whole: find each DTO by id and compare every field
    // with the frozen export.
    let mut by_id: Vec<(&str, &fretboard_mobile_ffi::ProgressionDto)> = Vec::new();
    for group in &ours {
        for progression in &group.progressions {
            by_id.push((progression.id.as_str(), progression));
        }
    }
    assert_eq!(by_id.len(), PROGRESSION_DEFINITION_RECORDS);
    for definition in definitions {
        let id = definition["id"].as_str().expect("an id");
        let found = by_id
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .unwrap_or_else(|| panic!("{id}: the catalog is missing the progression"));
        assert_eq!(
            dto_definition(found.1),
            frozen_definition(definition),
            "{id}: the definition differs from the frozen export"
        );
    }
}

#[test]
fn every_frozen_progression_record_is_the_adapter_answer() {
    let all = records(PROGRESSIONS);
    assert_eq!(
        all.len(),
        PROGRESSION_DEFINITION_RECORDS + PROGRESSION_LABEL_RECORDS + PROGRESSION_CHORD_RECORDS
    );

    let mut chords = 0usize;
    let mut labels = 0usize;
    let mut definitions = 0usize;
    for record in &all {
        let case = case_id(record);
        let id = record["input"]["id"].as_str().expect("an id").to_owned();
        match record["operation"].as_str().expect("an operation") {
            "progression_chords" => {
                let tonic = record["input"]["tonic"]
                    .as_str()
                    .expect("a tonic")
                    .to_owned();
                let ours: Vec<String> = progression_chords(tonic, id.clone())
                    .unwrap_or_else(|error| panic!("{case}: {error:?}"))
                    .iter()
                    .map(render_chord)
                    .collect();
                let frozen: Vec<String> = record["output"]["chords"]
                    .as_array()
                    .expect("chords")
                    .iter()
                    .map(|chord| {
                        format!(
                            "{}:{}",
                            chord["root"].as_str().expect("a root"),
                            chord["quality"].as_str().expect("a quality")
                        )
                    })
                    .collect();
                assert_eq!(ours, frozen, "{case}: the chords differ");
                chords += 1;
            }
            "progression_label" => {
                let name = catalog_progressions()
                    .into_iter()
                    .find(|progression| progression.id == id)
                    .unwrap_or_else(|| panic!("{case}: the catalog is missing the progression"))
                    .name;
                assert_eq!(
                    name,
                    record["output"]["label"].as_str().expect("a label"),
                    "{case}: the label differs"
                );
                labels += 1;
            }
            "progression" => {
                let dto = catalog_progressions()
                    .into_iter()
                    .find(|progression| progression.id == id)
                    .unwrap_or_else(|| panic!("{case}: the catalog is missing the progression"));
                assert_eq!(
                    dto_definition(&dto),
                    frozen_definition(&record["output"]["definition"]),
                    "{case}: the definition differs"
                );
                definitions += 1;
            }
            other => panic!("{case}: unexpected operation {other}"),
        }
    }
    assert_eq!(definitions, PROGRESSION_DEFINITION_RECORDS);
    assert_eq!(labels, PROGRESSION_LABEL_RECORDS);
    assert_eq!(chords, PROGRESSION_CHORD_RECORDS);
}

/// The catalog definitions the picker reads, flattened out of the groups.
fn catalog_progressions() -> Vec<fretboard_mobile_ffi::ProgressionDto> {
    progressions()
        .into_iter()
        .flat_map(|group| group.progressions)
        .collect()
}

#[test]
fn an_unknown_progression_or_note_crosses_as_a_typed_failure() {
    assert_eq!(
        code_of(progression_chords(
            "C".to_owned(),
            "no_such_progression".to_owned()
        )),
        "UnknownIdentifier"
    );
    assert_eq!(
        code_of(progression_chords(
            "H".to_owned(),
            "pop_i_v_vi_iv".to_owned()
        )),
        "UnknownIdentifier"
    );
    // A flat note name is a note through the domain's lookup, unlike a chord root
    // (`CORE-D06`): the modal's tonic field accepts what the catalog stores as
    // `example_key`.
    assert!(
        progression_chords("Bb".to_owned(), "pop_i_v_vi_iv".to_owned()).is_ok(),
        "a flat tonic resolves through the domain's note lookup"
    );
}

// ---------------------------------------------------------------------------
// The three new page events
// ---------------------------------------------------------------------------

/// The page state DTO of one fixture state, in the shape `page-events.jsonl`
/// records it.
fn page_dto_of(state: &Value) -> PageStateDto {
    let instrument = InstrumentDto::parse(state["instrument"].as_str().expect("an instrument"))
        .expect("a catalog instrument");
    let instrument_state = if instrument == InstrumentDto::Piano {
        InstrumentStateDto::Piano {
            selected: state["selection"]
                .as_array()
                .expect("piano keys are a list")
                .iter()
                .map(|pitch| u8::try_from(pitch.as_u64().expect("a key")).expect("fits"))
                .collect(),
        }
    } else {
        let tuning = &state["tuning_state"];
        let mut strings: Vec<u8> = state["selection"]
            .as_object()
            .expect("fretted marks are an object")
            .keys()
            .map(|string| string.parse::<u8>().expect("a string index"))
            .collect();
        strings.sort_unstable();
        InstrumentStateDto::Fretted {
            instrument,
            tuning: TuningDto {
                pitches: tuning["pitches"]
                    .as_array()
                    .expect("pitches")
                    .iter()
                    .map(|pitch| u8::try_from(pitch.as_u64().expect("a pitch")).expect("fits"))
                    .collect(),
                reference: tuning["reference"]
                    .as_str()
                    .expect("a reference")
                    .to_owned(),
            },
            selected: strings
                .into_iter()
                .map(|string| fretboard_mobile_ffi::PositionDto {
                    string,
                    fret: u8::try_from(
                        state["selection"][string.to_string()]
                            .as_u64()
                            .expect("a fret"),
                    )
                    .expect("fits"),
                })
                .collect(),
        }
    };

    let chords: Vec<ChordDto> = state["active_chords"]
        .as_array()
        .expect("a chord list")
        .iter()
        .map(|entry| {
            chord(
                entry["root"].as_str().expect("a root"),
                entry["quality"].as_str().expect("a quality"),
            )
        })
        .collect();
    let highlight = match &state["highlighted_chord"] {
        Value::Null => None,
        Value::Number(index) => chords
            .get(usize::try_from(index.as_u64().expect("an index")).expect("fits"))
            .cloned(),
        Value::String(root) => chords.iter().find(|chord| &chord.root == root).cloned(),
        _ => panic!("an unexpected highlight"),
    };

    PageStateDto {
        instrument: instrument_state,
        chords,
        highlight,
        tab: TabDto::parse(state["tab"].as_str().expect("a tab")).expect("a tab"),
    }
}

/// A key draft a client holds while the modal is open.
struct KeyDraft {
    tonic: String,
    scale: String,
    mode: ChordModeDto,
}

/// A progression draft a client holds while the modal is open.
struct ProgressionDraft {
    tonic: String,
    progression: String,
}

/// Drive one recorded keys/progression case through the exported functions.
///
/// The modal's own steps are UI-only: they update the draft the client holds and
/// push nothing. Only the Apply steps call `apply_page_event`, with the draft the
/// client was holding — exactly the way the pinned panel works.
#[allow(clippy::too_many_lines)]
fn drive_event_case(record: &Value) -> (PageStateDto, u64, Vec<bool>) {
    let case = case_id(record);
    let mut state = page_dto_of(&record["output"]["initial_state"]);
    let mut keys: Option<KeyDraft> = None;
    let mut progression: Option<ProgressionDraft> = None;
    let mut push_flags = Vec::new();
    let mut patch_count = 0u64;

    for step in record["input"]["steps"].as_array().expect("steps") {
        let event = step["event"].as_str();
        let pushed = match event {
            Some("open_key_modal") => {
                keys = Some(KeyDraft {
                    tonic: "C".to_owned(),
                    scale: "major".to_owned(),
                    mode: ChordModeDto::Triad,
                });
                false
            }
            Some("close_key_modal") => {
                keys = None;
                false
            }
            Some("apply_key") => {
                let Some(draft) = keys.take() else {
                    push_flags.push(false);
                    continue;
                };
                state = apply_page_event(
                    state.clone(),
                    PageEventDto::CommitKeys {
                        tonic: draft.tonic,
                        scale: draft.scale,
                        mode: draft.mode,
                    },
                )
                .unwrap_or_else(|error| panic!("{case}: the key apply fails: {error:?}"));
                true
            }
            Some("open_progression_modal") => {
                progression = Some(ProgressionDraft {
                    tonic: "C".to_owned(),
                    progression: "pop_i_v_vi_iv".to_owned(),
                });
                false
            }
            Some("close_progression_modal") => {
                progression = None;
                false
            }
            Some("apply_progression") => {
                let Some(draft) = progression.take() else {
                    push_flags.push(false);
                    continue;
                };
                state = apply_page_event(
                    state.clone(),
                    PageEventDto::CommitProgression {
                        tonic: draft.tonic,
                        progression: draft.progression,
                    },
                )
                .unwrap_or_else(|error| panic!("{case}: the progression apply fails: {error:?}"));
                true
            }
            Some("apply_suggested_key") => {
                let value = &step["value"];
                state = apply_page_event(
                    state.clone(),
                    PageEventDto::CommitSuggestedKeys {
                        tonic: value["tonic"].as_str().expect("a tonic").to_owned(),
                        scale: value["scale_type"].as_str().expect("a scale").to_owned(),
                    },
                )
                .unwrap_or_else(|error| panic!("{case}: the suggested-key apply fails: {error:?}"));
                true
            }
            Some(other) => panic!("{case}: an unexpected step {other}"),
            None => {
                // A recorded `change` of one of the two modal forms: UI-only.
                match step["selector"].as_str().expect("a selector") {
                    "#key-form" => {
                        let key = &step["value"]["key"];
                        keys = Some(KeyDraft {
                            tonic: key["tonic"].as_str().expect("a tonic").to_owned(),
                            scale: key["scale_type"].as_str().expect("a scale").to_owned(),
                            mode: ChordModeDto::parse(key["chord_mode"].as_str().expect("a mode"))
                                .expect("a mode"),
                        });
                    }
                    "#progression-form" => {
                        let draft = &step["value"]["progression"];
                        progression = Some(ProgressionDraft {
                            tonic: draft["tonic"].as_str().expect("a tonic").to_owned(),
                            progression: draft["id"].as_str().expect("an id").to_owned(),
                        });
                    }
                    other => panic!("{case}: an unexpected form {other}"),
                }
                false
            }
        };
        push_flags.push(pushed);
        patch_count += u64::from(pushed);
    }

    (state, patch_count, push_flags)
}

#[test]
fn every_keys_and_progression_case_reaches_the_frozen_page() {
    let all = records(PAGE_EVENTS);
    for case in OWNED_EVENTS {
        let record = all
            .iter()
            .find(|record| record["case_id"].as_str() == Some(case))
            .unwrap_or_else(|| panic!("the fixture has no case {case}"));
        let output = &record["output"];
        let frozen_patches = output["patch_count"].as_u64().expect("a patch count");

        let (state, patches, flags) = drive_event_case(record);
        assert_eq!(patches, frozen_patches, "{case}: the patch count differs");
        for (index, recorded) in output["steps"]
            .as_array()
            .expect("step outputs")
            .iter()
            .enumerate()
        {
            assert_eq!(
                flags[index],
                recorded["patched"].as_bool().unwrap_or(false),
                "{case}: step {index} pushes differently"
            );
        }
        assert_eq!(
            state,
            page_dto_of(&output["final_state"]),
            "{case}: the final page differs"
        );
    }
}

#[test]
fn an_apply_replaces_the_chords_and_keeps_everything_else() {
    // The page of `apply-key-triads-replaces-chords` carries a marked position,
    // and the progression case carries a highlight: both survive or clear exactly
    // as the fixture records.
    let record = fixture_case(PAGE_EVENTS, "page_event/apply-key-triads-replaces-chords");
    let initial = page_dto_of(&record["output"]["initial_state"]);
    let (final_state, _, _) = drive_event_case(&record);

    let InstrumentStateDto::Fretted {
        instrument,
        tuning,
        selected,
    } = &final_state.instrument
    else {
        panic!("the key apply must not change the instrument");
    };
    assert_eq!(*instrument, InstrumentDto::Guitar, "the instrument is kept");
    assert_eq!(tuning.reference, "Standard", "the committed tuning is kept");
    assert!(!selected.is_empty(), "the marked positions are kept");
    assert_eq!(final_state.tab, TabDto::Visualizer, "the tab is kept");
    assert!(
        final_state.highlight.is_none(),
        "the key apply clears the highlight"
    );
    assert_eq!(
        final_state.chords,
        page_dto_of(&record["output"]["final_state"]).chords
    );
    assert!(
        initial.highlight.is_none(),
        "the fixture page starts without a highlight here"
    );

    let record = fixture_case(PAGE_EVENTS, "page_event/apply-progression-replaces-chords");
    let initial = page_dto_of(&record["output"]["initial_state"]);
    assert!(
        initial.highlight.is_some(),
        "the progression case starts highlighted"
    );
    let (final_state, _, _) = drive_event_case(&record);
    assert!(
        final_state.highlight.is_none(),
        "the progression apply clears the highlight"
    );
    assert_eq!(
        final_state.chords.len(),
        4,
        "the progression's own chord list"
    );
}

#[test]
fn the_suggested_key_inherits_the_mode_of_the_current_chords() {
    // The suggested-key step names the key and the scale and nothing else; the
    // reducer infers triads or sevenths from the chords on the page.
    for case in [
        "page_event/apply-suggested-key-triads-from-existing-mode",
        "page_event/apply-suggested-key-inherits-seventh-mode",
    ] {
        let record = fixture_case(PAGE_EVENTS, case);
        let (final_state, _, _) = drive_event_case(&record);
        let expected = page_dto_of(&record["output"]["final_state"]);
        assert_eq!(
            final_state.chords, expected.chords,
            "{case}: the chord list"
        );
        // The same key in the other mode is a different list, so the recorded
        // answer really is mode-dependent.
        let step = record["input"]["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .find(|step| step["event"].as_str() == Some("apply_suggested_key"))
            .expect("the suggested-key step");
        let tonic = step["value"]["tonic"].as_str().expect("a tonic").to_owned();
        let scale = step["value"]["scale_type"]
            .as_str()
            .expect("a scale")
            .to_owned();
        let other = if final_state
            .chords
            .iter()
            .any(|chord| chord.quality.contains('7'))
        {
            ChordModeDto::Triad
        } else {
            ChordModeDto::Seventh
        };
        assert_ne!(
            diatonic_chords(tonic, scale, other).expect("the other mode"),
            expected.chords,
            "{case}: the two modes differ for this key"
        );
    }
}

#[test]
fn the_key_apply_uses_the_modal_draft_in_both_modes() {
    // The two key cases drive the whole draft through the DTO event: the preview
    // of the same key and mode is the committed chord list.
    for (case, mode) in [
        (
            "page_event/apply-key-triads-replaces-chords",
            ChordModeDto::Triad,
        ),
        ("page_event/apply-key-sevenths", ChordModeDto::Seventh),
    ] {
        let record = fixture_case(PAGE_EVENTS, case);
        let (final_state, _, _) = drive_event_case(&record);
        let expected = page_dto_of(&record["output"]["final_state"]);
        assert_eq!(
            final_state.chords, expected.chords,
            "{case}: the chord list"
        );

        let step = record["input"]["steps"]
            .as_array()
            .expect("steps")
            .iter()
            .find(|step| step["selector"].as_str() == Some("#key-form"))
            .expect("the key-form change");
        let key = &step["value"]["key"];
        let preview = diatonic_chords(
            key["tonic"].as_str().expect("a tonic").to_owned(),
            key["scale_type"].as_str().expect("a scale").to_owned(),
            mode,
        )
        .expect("the preview");
        assert_eq!(preview, expected.chords, "{case}: the preview is the apply");
    }
}

#[test]
fn an_unreadable_draft_never_becomes_a_page_event() {
    // A tonic the wire cannot resolve, a scale outside the catalog and a
    // progression the catalog does not carry are typed failures, never a wrong
    // apply.
    assert_eq!(
        code_of(apply_page_event(
            default_state(),
            PageEventDto::CommitKeys {
                tonic: "H".to_owned(),
                scale: "major".to_owned(),
                mode: ChordModeDto::Triad,
            }
        )),
        "UnknownIdentifier"
    );
    assert_eq!(
        code_of(apply_page_event(
            default_state(),
            PageEventDto::CommitSuggestedKeys {
                tonic: "C".to_owned(),
                scale: "no_such_scale".to_owned(),
            }
        )),
        "UnknownIdentifier"
    );
    assert_eq!(
        code_of(apply_page_event(
            default_state(),
            PageEventDto::CommitProgression {
                tonic: "C".to_owned(),
                progression: "no_such_progression".to_owned(),
            }
        )),
        "UnknownIdentifier"
    );
}
