//! Task `C16`, half two: the page's grouped suggestion rows against the frozen
//! oracle.
//!
//! Every record of `fixtures/oracle/key-groups.jsonl` (16 records) is consumed
//! here, each failure naming its `case_id`. The records come from the pinned
//! `FretboardWeb.FretboardLive.group_key_suggestions/1`.
//!
//! ## The row rule, and the one recorded deviation (`Contract.D06`)
//!
//! The baseline groups only the suggestions at the **maximum score** and only
//! when that score is perfect (`max_score == total`); within that branch it
//! partitions the seven modal modes by note set, keeps only the note sets that
//! carry all seven modes, renders the major and minor of each such set
//! prominently with the other five modes beside them, appends the non-modal
//! suggestions and then the lower-scoring ones. When the maximum is not perfect
//! it shows the first three suggestions as single rows and nothing else.
//!
//! Two independent things were measured (`docs/p5-decision-evidence.md`):
//!
//! * **The row order is deterministic** — it is the term order of a small map
//!   keyed by the note set, stable across runs and across atom-table states —
//!   so it is ported verbatim;
//! * **the drop is not.** The baseline silently drops every modal group that is
//!   not a perfect seven-mode set, so `single-modal-suggestion-incomplete-group`
//!   and `incomplete-modal-pair-only` leave the panel empty on a perfect match
//!   and `incomplete-sibling-group-dropped` hides an equally perfect G major and
//!   E minor. The user chose option B (`Contract.D06`, recorded in
//!   `fixtures/contract/approved-deviations.json`): **a perfect suggestion is
//!   never dropped.** Only the drop changes; every other rule above is ported
//!   verbatim, so the partial group is shown as its own row with the members it
//!   has.
//!
//! This file keeps that deviation visible instead of absorbing it:
//!
//! * the counts are pinned (`IDENTICAL_RECORDS`, `DEVIATING_RECORDS`), the four
//!   case ids are asserted by name as the *only* differences, and the four
//!   expected row lists are written out from the rule above (not read back from
//!   the implementation);
//! * a second test asserts the invariant the decision states — every suggestion
//!   the input carries at the maximum score is shown — so the deviation cannot
//!   be replaced by a slightly different drop without failing.
//!
//! ## What the rest pins
//!
//! The perfect seven-mode collapse (prominent pair, others, one row per note
//! set), the pinned row order on the record whose input spells the *second*
//! note set first, the imperfect top-three truncation and nothing else, the
//! grouped rows before the lower-scoring rest, and the empty input.

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

mod common;

use std::collections::BTreeSet;

use common::oracle_records;
use fretboard_core::{
    ChordMode, KeyRow, KeySuggestion, diatonic_chords, group_key_suggestions, note_index,
    scale_notes,
};
use serde_json::Value;

/// The frozen fixture and the record count it must carry
/// (`fixtures/oracle/manifest.json`).
const FIXTURE: &str = "fixtures/oracle/key-groups.jsonl";
const RECORDS: usize = 16;

/// Frozen records the native rows reproduce exactly.
const IDENTICAL_RECORDS: usize = 12;

/// Frozen records that differ — the recorded `Contract.D06` deviation.
const DEVIATING_RECORDS: usize = 4;

/// The four deviating records, by case id
/// (`fixtures/contract/approved-deviations.json`).
const DEVIATING_CASES: [&str; 4] = [
    "key_groups/single-modal-suggestion-incomplete-group",
    "key_groups/incomplete-modal-pair-only",
    "key_groups/incomplete-modal-pair-with-non-modal-survivor",
    "key_groups/incomplete-sibling-group-dropped",
];

/// The records whose input carries no suggestion at all.
const EMPTY_INPUT_RECORDS: usize = 2;

/// The records whose first suggestion is perfect (`score == total`, the
/// grouped branch) and the records that stay in the single-row branch.
const PERFECT_BRANCH_RECORDS: usize = 12;
const IMPERFECT_BRANCH_RECORDS: usize = 2;

/// The imperfect branch shows three single rows and nothing else.
const IMPERFECT_ROWS: usize = 3;

/// Every record of the fixture, in file order.
fn records() -> Vec<Value> {
    let records = oracle_records(FIXTURE);
    assert_eq!(
        records.len(),
        RECORDS,
        "the frozen fixture carries {RECORDS} cases"
    );
    records
}

/// The identifier of a record.
fn case_id(record: &Value) -> String {
    record["case_id"]
        .as_str()
        .expect("every record has a case id")
        .to_owned()
}

/// One record by case id, or a loud failure naming the missing case.
fn fixture_case(case_id_wanted: &str) -> Value {
    records()
        .into_iter()
        .find(|record| record["case_id"].as_str() == Some(case_id_wanted))
        .unwrap_or_else(|| panic!("the key-groups fixture has no case {case_id_wanted}"))
}

/// One input row or one output item of the fixture, as a native suggestion.
///
/// The fixture's suggestions carry only the four fields the grouping reads; the
/// diatonic triads are the ones the key really has, so the value satisfies the
/// type's own invariant without a second source of truth.
fn native_suggestion(item: &Value) -> KeySuggestion {
    let tonic =
        note_index(item["tonic"].as_str().expect("a suggestion has a tonic")).expect("a note name");
    let scale_type = item["scale_type"]
        .as_str()
        .expect("a suggestion has a scale type")
        .parse()
        .expect("a catalog scale");
    KeySuggestion {
        tonic,
        scale_type,
        score: usize::try_from(item["score"].as_u64().expect("a score")).expect("a score in usize"),
        total: usize::try_from(item["total"].as_u64().expect("a total")).expect("a total in usize"),
        diatonic_chords: diatonic_chords(tonic, scale_type, ChordMode::Triad),
    }
}

/// The input suggestions of a record, in order.
fn input_suggestions(record: &Value) -> Vec<KeySuggestion> {
    record["input"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no input", case_id(record)))
        .iter()
        .map(native_suggestion)
        .collect()
}

/// One suggestion as `tonic:scale:score:total`.
fn render_item(suggestion: &KeySuggestion) -> String {
    format!(
        "{}:{}:{}:{}",
        suggestion.tonic.name(),
        suggestion.scale_type.as_str(),
        suggestion.score,
        suggestion.total
    )
}

/// The native rows of a record, in the canonical text form.
fn our_rows(record: &Value) -> Vec<String> {
    group_key_suggestions(&input_suggestions(record))
        .iter()
        .map(|row| match row {
            KeyRow::Group { prominent, others } => format!(
                "G[{}|{}]",
                prominent
                    .iter()
                    .map(render_item)
                    .collect::<Vec<String>>()
                    .join(";"),
                others
                    .iter()
                    .map(render_item)
                    .collect::<Vec<String>>()
                    .join(";")
            ),
            KeyRow::Single(item) => format!("S[{}]", render_item(item)),
        })
        .collect()
}

/// The frozen rows of a record, in the same form.
fn frozen_rows(record: &Value) -> Vec<String> {
    record["output"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no rows", case_id(record)))
        .iter()
        .map(|row| {
            if row["collapsed?"].as_bool() == Some(true) {
                let prominent = row["prominent"]
                    .as_array()
                    .expect("a collapsed row carries its prominent entries");
                let others = row["others"]
                    .as_array()
                    .expect("a collapsed row carries its others");
                format!(
                    "G[{}|{}]",
                    prominent
                        .iter()
                        .map(|item| render_item(&native_suggestion(item)))
                        .collect::<Vec<String>>()
                        .join(";"),
                    others
                        .iter()
                        .map(|item| render_item(&native_suggestion(item)))
                        .collect::<Vec<String>>()
                        .join(";")
                )
            } else {
                format!("S[{}]", render_item(&native_suggestion(&row["item"])))
            }
        })
        .collect()
}

/// Every suggestion a row list shows, as `(tonic, scale, score, total)`.
fn shown_suggestions(rows: &[KeyRow]) -> Vec<String> {
    let mut shown = Vec::new();
    for row in rows {
        match row {
            KeyRow::Group { prominent, others } => {
                shown.extend(prominent.iter().map(render_item));
                shown.extend(others.iter().map(render_item));
            }
            KeyRow::Single(item) => shown.push(render_item(item)),
        }
    }
    shown
}

/// Whether the record's first suggestion is perfect, which selects the grouped
/// branch.
fn is_perfect_branch(record: &Value) -> bool {
    input_suggestions(record)
        .first()
        .is_some_and(|first| first.score == first.total)
}

#[test]
fn the_frozen_fixture_carries_every_group_case() {
    let records = records();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for record in &records {
        assert_eq!(
            record["operation"].as_str(),
            Some("key_groups"),
            "an unexpected operation"
        );
        assert!(
            ids.insert(case_id(record)),
            "duplicate case id in the fixture"
        );
    }
    assert_eq!(ids.len(), RECORDS, "the case ids are unique");

    let empty_inputs = records
        .iter()
        .filter(|record| input_suggestions(record).is_empty())
        .count();
    assert_eq!(empty_inputs, EMPTY_INPUT_RECORDS);
    for record in &records {
        if input_suggestions(record).is_empty() {
            assert!(
                frozen_rows(record).is_empty(),
                "{}: no suggestion, no row",
                case_id(record)
            );
        }
    }

    let perfect = records
        .iter()
        .filter(|record| is_perfect_branch(record))
        .count();
    let imperfect = records
        .iter()
        .filter(|record| !input_suggestions(record).is_empty() && !is_perfect_branch(record))
        .count();
    assert_eq!(perfect, PERFECT_BRANCH_RECORDS);
    assert_eq!(imperfect, IMPERFECT_BRANCH_RECORDS);
    assert_eq!(perfect + imperfect + empty_inputs, RECORDS);
}

#[test]
fn every_frozen_group_case_answers_its_record_except_the_four_recorded_deviations() {
    let mut identical = 0usize;
    let mut deviating: BTreeSet<String> = BTreeSet::new();

    for record in records() {
        let case = case_id(&record);
        if our_rows(&record) == frozen_rows(&record) {
            identical += 1;
            continue;
        }

        assert!(
            DEVIATING_CASES.contains(&case.as_str()),
            "{case}: the rows differ outside the recorded Contract.D06 deviation"
        );
        deviating.insert(case);
    }

    assert_eq!(identical, IDENTICAL_RECORDS, "the identical record count");
    assert_eq!(deviating.len(), DEVIATING_RECORDS);
    assert_eq!(
        deviating,
        DEVIATING_CASES.map(str::to_owned).into_iter().collect(),
        "the deviating records are exactly the four the ledger names"
    );
}

#[test]
fn the_deviating_records_show_the_incomplete_groups_the_baseline_drops() {
    // The four row lists the option-B rule produces, written from the rule and
    // the decision (the drop removed, everything else ported verbatim): the
    // partial group becomes its own row, its members are shown, and the
    // non-modal and lower-scoring rows keep their places.
    let expected: [(&str, &[&str]); 4] = [
        (
            // One perfect `C major 1:1`: the baseline answers 0 rows.
            DEVIATING_CASES[0],
            &["G[C:major:1:1|]"],
        ),
        (
            // A perfect `C major 2:2` and its relative `A minor 2:2`, one note
            // set: the baseline answers 0 rows.
            DEVIATING_CASES[1],
            &["G[C:major:2:2;A:minor:2:2|]"],
        ),
        (
            // The same pair plus a perfect non-modal `C pentatonic major`: the
            // baseline answers only the pentatonic row.
            DEVIATING_CASES[2],
            &["G[C:major:2:2;A:minor:2:2|]", "S[C:pentatonic_major:2:2]"],
        ),
        (
            // The C major seven-mode group and a second, equally perfect group
            // carrying only `G major` and `E minor`: the baseline hides the
            // second group entirely.
            DEVIATING_CASES[3],
            &[
                "G[C:major:2:2;A:minor:2:2|D:dorian:2:2;E:phrygian:2:2;F:lydian:2:2;G:mixolydian:2:2;B:locrian:2:2]",
                "G[G:major:2:2;E:minor:2:2|]",
            ],
        ),
    ];

    for (case, rows) in expected {
        let record = fixture_case(case);
        assert_eq!(
            our_rows(&record),
            rows.to_vec(),
            "{case}: the option-B rows differ"
        );
        assert_ne!(
            frozen_rows(&record),
            our_rows(&record),
            "{case}: this record is one the deviation changes"
        );
    }

    // The two records whose baseline answer is empty are exactly the two the
    // ledger describes as "a perfect C major shows nothing".
    for case in [DEVIATING_CASES[0], DEVIATING_CASES[1]] {
        assert!(
            frozen_rows(&fixture_case(case)).is_empty(),
            "{case}: the baseline drops the whole panel"
        );
    }
    // ...and the two others lose a perfect suggestion instead.
    let sibling = fixture_case(DEVIATING_CASES[3]);
    let frozen = frozen_rows(&sibling);
    let ours = our_rows(&sibling);
    assert_eq!(frozen.len(), 1, "the baseline shows one collapsed row");
    assert_eq!(
        ours.len(),
        2,
        "the option-B rule shows the second group too"
    );
    assert!(
        !frozen.contains(&"G[G:major:2:2;E:minor:2:2|]".to_owned()),
        "which the baseline hides"
    );
}

#[test]
fn no_suggestion_at_the_maximum_score_is_ever_hidden_by_the_grouping() {
    // `Contract.D06`: a perfect suggestion is never dropped. Every row shows
    // input occurrences (no invented row, no row shown twice for one input), and
    // on a perfect maximum the rows account for the whole input list.
    for record in records() {
        let case = case_id(&record);
        let input = input_suggestions(&record);
        let rows = group_key_suggestions(&input);
        let shown = shown_suggestions(&rows);

        let mut remaining: Vec<String> = input.iter().map(render_item).collect();
        for item in &shown {
            let position = remaining
                .iter()
                .position(|candidate| candidate == item)
                .unwrap_or_else(|| panic!("{case}: the row {item} is not an input suggestion"));
            remaining.remove(position);
        }

        if is_perfect_branch(&record) {
            assert!(
                remaining.is_empty(),
                "{case}: a perfect input must be shown completely, hidden: {remaining:?}"
            );
        }
    }
}

#[test]
fn a_perfect_seven_mode_group_collapses_its_prominent_pair() {
    // The C major seven-mode set, spelled sorted in one record and scrambled in
    // the other: one collapsed row each, the major and the relative minor
    // prominent in that order, the other five modes in the input order. The
    // frozen record carries the same row, so the row itself is fixture-driven;
    // the two records are compared with each other to pin that the collapse
    // survives a reordered input.
    let mut member_sets: Vec<BTreeSet<String>> = Vec::new();
    for case in [
        "key_groups/complete-seven-mode-shared-note-set",
        "key_groups/scrambled-modal-input-key-keeps-prominent-pair",
    ] {
        let record = fixture_case(case);
        let rows = group_key_suggestions(&input_suggestions(&record));
        assert_eq!(our_rows(&record), frozen_rows(&record), "{case}: the row");
        let collapsed: Vec<&KeyRow> = rows
            .iter()
            .filter(|row| matches!(row, KeyRow::Group { .. }))
            .collect();
        let group = collapsed
            .last()
            .unwrap_or_else(|| panic!("{case}: the record carries a collapsed row"));
        let KeyRow::Group { prominent, others } = group else {
            panic!("{case}: the row is not a collapsed group");
        };
        assert_eq!(prominent.len(), 2, "{case}: major and minor are prominent");
        assert_eq!(prominent[0].scale_type.as_str(), "major");
        assert_eq!(prominent[1].scale_type.as_str(), "minor");
        assert_eq!(others.len(), 5, "{case}: the other five modes");
        assert_eq!(
            prominent.len() + others.len(),
            7,
            "{case}: every mode of the group is shown"
        );

        // One note set behind the seven modes: their scales are the same seven
        // note names.
        let notes: BTreeSet<String> = prominent
            .iter()
            .chain(others)
            .map(|member| {
                let mut names: Vec<String> = scale_notes(member.tonic, member.scale_type)
                    .iter()
                    .map(|note| note.name().to_owned())
                    .collect();
                names.sort_unstable();
                names.join(",")
            })
            .collect();
        assert_eq!(notes.len(), 1, "{case}: seven modes, one note set");

        member_sets.push(
            prominent
                .iter()
                .chain(others)
                .map(|member| format!("{}:{}", member.tonic.name(), member.scale_type.as_str()))
                .collect(),
        );
    }
    assert_eq!(
        member_sets[0], member_sets[1],
        "a reordered input still groups the same seven modes"
    );
    assert_eq!(member_sets[0].len(), 7);
}

#[test]
fn the_row_order_is_the_term_order_of_the_note_set() {
    // The record spells the C major set first and the F major set second; the
    // frozen rows answer the F major group first, because the baseline's row
    // order is the term order of the note set (`A#set < Cset`). Ported verbatim.
    let record = fixture_case("key_groups/tied-complete-seven-mode-row-order");
    let rows = group_key_suggestions(&input_suggestions(&record));
    assert_eq!(rows.len(), 2, "two complete seven-mode groups");
    assert_eq!(
        our_rows(&record),
        frozen_rows(&record),
        "the frozen row order"
    );

    // The key of each group, as the term order of a small map compares it: the
    // number of notes first, then the sorted note names.
    let mut keys: Vec<(usize, Vec<String>)> = Vec::new();
    for row in &rows {
        let KeyRow::Group { prominent, .. } = row else {
            panic!("the record's rows are collapsed groups");
        };
        let mut names: Vec<String> = scale_notes(prominent[0].tonic, prominent[0].scale_type)
            .iter()
            .map(|note| note.name().to_owned())
            .collect();
        names.sort_unstable();
        keys.push((names.len(), names));
    }
    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "the rows are not in note-set term order");

    // The first group's prominent pair is the F major one although the input
    // spells `C major` first: the order is the note set's, not the input's.
    let first = rows.first().expect("two rows");
    let KeyRow::Group { prominent, .. } = first else {
        panic!("a collapsed row");
    };
    assert_eq!(
        prominent[0].tonic.name(),
        "F",
        "the row order is not the input order"
    );
    assert_eq!(prominent[0].scale_type.as_str(), "major");
    assert_eq!(prominent[1].tonic.name(), "D");
    assert_eq!(prominent[1].scale_type.as_str(), "minor");
}

#[test]
fn the_imperfect_branch_shows_the_top_three_and_nothing_else() {
    // Seven candidates at 2:3 collapse to three single rows; two candidates at
    // 1:2 stay two single rows. The `rest` is not appended in this branch.
    let record = fixture_case("key_groups/imperfect-top3-truncates-seven-candidates");
    let input = input_suggestions(&record);
    assert!(input.len() > IMPERFECT_ROWS, "a real truncation");
    assert!(!is_perfect_branch(&record));
    let rows = group_key_suggestions(&input);
    assert_eq!(rows.len(), IMPERFECT_ROWS, "the top three and nothing else");
    assert_eq!(our_rows(&record), frozen_rows(&record));
    let shown = shown_suggestions(&rows);
    assert_eq!(
        shown.len(),
        input.len().min(IMPERFECT_ROWS),
        "the wrapper hides exactly the suggestions above the third"
    );
    assert!(
        rows.iter().all(|row| matches!(row, KeyRow::Single(_))),
        "no grouping in the imperfect branch"
    );

    let record = fixture_case("key_groups/imperfect-with-two-candidates");
    let rows = group_key_suggestions(&input_suggestions(&record));
    assert_eq!(rows.len(), 2, "two candidates, two rows");
    assert_eq!(our_rows(&record), frozen_rows(&record));
}

#[test]
fn the_grouped_rows_come_before_the_lower_scoring_rest() {
    // The perfect branch answers: every collapsed group, then the non-modal
    // suggestions at the maximum score, then the lower-scoring suggestions in
    // input order — the fixture's input puts one of them before a non-modal
    // perfect one, and the row order still follows the rule.
    let record = fixture_case("key_groups/grouped-rows-before-lower-scoring-rest");
    let input = input_suggestions(&record);
    assert!(is_perfect_branch(&record), "the grouped branch");
    let rows = group_key_suggestions(&input);
    assert_eq!(our_rows(&record), frozen_rows(&record));
    assert_eq!(rows.len(), 4, "one group, one non-modal, two lower rows");

    let max_score = input.first().expect("an input").score;
    let rest: Vec<String> = input
        .iter()
        .filter(|suggestion| suggestion.score < max_score)
        .map(render_item)
        .collect();
    let tail: Vec<String> = shown_suggestions(&rows)
        .into_iter()
        .rev()
        .take(rest.len())
        .collect::<Vec<String>>()
        .into_iter()
        .rev()
        .collect();
    assert_eq!(tail, rest, "the lower-scoring rest closes the list");

    let non_modal = input
        .iter()
        .find(|suggestion| {
            suggestion.score == max_score
                && ![
                    "major",
                    "minor",
                    "dorian",
                    "phrygian",
                    "lydian",
                    "mixolydian",
                    "locrian",
                ]
                .contains(&suggestion.scale_type.as_str())
        })
        .expect("the fixture carries a non-modal suggestion at the maximum score");
    let KeyRow::Single(item) = &rows[1] else {
        panic!("the second row is the non-modal suggestion");
    };
    assert_eq!(item, non_modal, "the non-modal row comes right after");
}
