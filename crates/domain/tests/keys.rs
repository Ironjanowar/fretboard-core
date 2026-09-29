//! Task `C16`, half one: single-key scoring against the frozen oracle.
//!
//! Every record of `fixtures/oracle/keys.jsonl` (17 records, plus the three
//! flat-root/spelled inputs they carry) is consumed here, each failure naming
//! its `case_id`. The records come from the pinned
//! `Fretboard.Music.Scale.suggest_keys/1`.
//!
//! ## The one recorded deviation (`Contract.D05`)
//!
//! The baseline scores a candidate key by comparing the input's **raw root
//! string** against the candidate's sharp note names, while containment resolves
//! a flat root through `Note.note_index/1` first. A flat-rooted chord therefore
//! passes containment and can never score: `suggest_keys/flat-root-eb-major`
//! answers 14 suggestions at `0:2` where the same chords spelled sharp answer
//! `2:2`. The native root is a `PitchClass` and has no spelling, so the case
//! cannot be reproduced; the user approved option A (`Contract.D05`, recorded in
//! `fixtures/contract/approved-deviations.json`) — the triad-base map and the
//! containment/scoring rule are ported unchanged and the one record differs in
//! the `score` field only, with membership and order identical.
//!
//! This file keeps that deviation visible instead of absorbing it:
//!
//! * the counts are pinned as constants (`IDENTICAL_RECORDS`,
//!   `DEVIATING_RECORDS`, `DEVIATING_SUGGESTIONS`, `DEVIATING_SCORE`), so a
//!   change in the deviation is a visible failure and not a silent absorption;
//! * the deviating record is asserted by its case id, and the assertion checks
//!   that *only* the score moved — the (tonic, scale type) sequence, the totals
//!   and the suggestion count equal the frozen ones;
//! * the native answer for the flat spelling must equal the native answer for
//!   the same chords spelled sharp, which is exactly the option-A rule (the
//!   native root is a spelling-free pitch class) and is not read from the
//!   implementation.
//!
//! ## What the rest pins
//!
//! Containment decides the candidate set; the triad-base map is complete over
//! the 47 frozen qualities (the named entries are the ones
//! `docs/p5-decision-evidence.md` measures); duplicate occurrences score once
//! each; equal scores are ordered by the lexical tie-break (tonic, then scale
//! type); and the raw empty operation answers every one of the 168 candidates
//! where the page gate answers nothing below two active chords.

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
    ChordSpec, KeySuggestion, PitchClass, QualityId, chord_details, note_index,
    page_key_suggestions, scale_notes, suggest_keys, triad_base,
};
use serde_json::Value;

/// The frozen fixture and the record count it must carry
/// (`fixtures/oracle/manifest.json`).
const FIXTURE: &str = "fixtures/oracle/keys.jsonl";
const RECORDS: usize = 17;

/// The candidate keys of the raw operation: twelve tonics times fourteen scale
/// types, the chromatic scale excluded.
const CANDIDATE_KEYS: usize = 168;

/// Frozen records the native rule reproduces exactly.
const IDENTICAL_RECORDS: usize = 16;

/// Frozen records that differ from the native answer — the recorded
/// `Contract.D05` deviation.
const DEVIATING_RECORDS: usize = 1;

/// The deviating record, by case id (`fixtures/contract/approved-deviations.json`).
const DEVIATING_CASE: &str = "suggest_keys/flat-root-eb-major";

/// The suggestions the deviating record carries, and the score every one of
/// them answers natively (`docs/p5-decision-evidence.md`: "the same 14
/// suggestions in the same order, scores 2:2").
const DEVIATING_SUGGESTIONS: usize = 14;
const DEVIATING_SCORE: usize = 2;

/// Frozen records whose answer is empty.
const EMPTY_ANSWER_RECORDS: usize = 3;

/// Frozen records with two or more chords whose answer is not empty: the page
/// gate only removes what it is asked to remove.
const ANSWERING_RECORDS: usize = 11;

/// The two records `docs/p5-decision-evidence.md` measures the flat-root
/// asymmetry with: one scores natively and does not in the baseline, the other
/// has no suggestion either way (`suggest_keys/flat-root-Bb-with-g`).
const FLAT_ROOT_CASES: [&str; 2] = [
    "suggest_keys/flat-root-eb-major",
    "suggest_keys/flat-root-Bb-with-g",
];

/// The six triad qualities the map answers with.
const TRIAD_QUALITIES: [&str; 6] = ["major", "minor", "dim", "aug", "sus2", "sus4"];

/// The named triad bases of `docs/p5-decision-evidence.md`, which reads them
/// from the pinned `@quality_to_triad` table: the qualities that do not map to
/// their own triad base.
const NAMED_TRIAD_BASES: [(&str, &str); 14] = [
    ("9b5", "dim"),
    ("7b5", "dim"),
    ("m11b5", "dim"),
    ("dim_maj7", "dim"),
    ("dim7b13", "dim"),
    ("9#5", "aug"),
    ("aug7", "aug"),
    ("aug_maj7", "aug"),
    ("7#9", "major"),
    ("7b9", "major"),
    ("13b9", "major"),
    ("7b13", "major"),
    ("m_add9", "minor"),
    ("7sus4", "sus4"),
];

/// The three chords of `suggest_keys/no-compatible-three-major-triad-roots`,
/// three major triads no single key contains together.
const INCOMPATIBLE_CASE: &str = "suggest_keys/no-compatible-three-major-triad-roots";

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
        .unwrap_or_else(|| panic!("the keys fixture has no case {case_id_wanted}"))
}

/// The chords of a record's input.
///
/// The roots are read with `note_index`, the domain lookup the baseline's
/// `Chord.notes/2` uses: two of the frozen records spell their roots with flats
/// (`Eb`, `Bb`) and those are not wire values (`CORE-D06`), but they are the
/// input the raw domain operation was measured with.
fn input_chords(record: &Value) -> Vec<ChordSpec> {
    record["input"]["chords"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no chords", case_id(record)))
        .iter()
        .map(|chord| {
            let root = chord["root"]
                .as_str()
                .unwrap_or_else(|| panic!("{}: a chord has no root", case_id(record)));
            let quality = chord["quality"]
                .as_str()
                .unwrap_or_else(|| panic!("{}: a chord has no quality", case_id(record)));
            ChordSpec {
                root: note_index(root)
                    .unwrap_or_else(|error| panic!("{}: {root}: {error:?}", case_id(record))),
                quality: quality
                    .parse()
                    .unwrap_or_else(|error| panic!("{}: {quality}: {error:?}", case_id(record))),
            }
        })
        .collect()
}

/// The native suggestions of a record, as `(tonic, scale type, score, total)`
/// tuples in order.
fn our_identities(suggestions: &[KeySuggestion]) -> Vec<(String, String, u64, u64)> {
    suggestions
        .iter()
        .map(|suggestion| {
            (
                suggestion.tonic.name().to_owned(),
                suggestion.scale_type.as_str().to_owned(),
                u64::try_from(suggestion.score).expect("a score fits in u64"),
                u64::try_from(suggestion.total).expect("a total fits in u64"),
            )
        })
        .collect()
}

/// The frozen suggestions of a record, as the same tuples in order.
fn frozen_identities(record: &Value) -> Vec<(String, String, u64, u64)> {
    record["output"]["suggestions"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no suggestions", case_id(record)))
        .iter()
        .map(|suggestion| {
            (
                suggestion["tonic"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{}: no tonic", case_id(record)))
                    .to_owned(),
                suggestion["scale_type"]
                    .as_str()
                    .unwrap_or_else(|| panic!("{}: no scale type", case_id(record)))
                    .to_owned(),
                suggestion["score"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{}: no score", case_id(record))),
                suggestion["total"]
                    .as_u64()
                    .unwrap_or_else(|| panic!("{}: no total", case_id(record))),
            )
        })
        .collect()
}

/// The frozen diatonic triads of a suggestion record, as `(root, quality)`
/// pairs in order.
fn frozen_triads(record: &Value, suggestion: &Value) -> Vec<(String, String)> {
    suggestion["diatonic_chords"]
        .as_array()
        .unwrap_or_else(|| panic!("{}: no diatonic chords", case_id(record)))
        .iter()
        .map(|chord| {
            (
                chord["root"].as_str().expect("a triad root").to_owned(),
                chord["quality"]
                    .as_str()
                    .expect("a triad quality")
                    .to_owned(),
            )
        })
        .collect()
}

/// The native diatonic triads of one suggestion, as the same pairs.
fn our_triads(suggestion: &KeySuggestion) -> Vec<(String, String)> {
    suggestion
        .diatonic_chords
        .iter()
        .map(|triad| {
            (
                triad.root.name().to_owned(),
                triad.quality.as_str().to_owned(),
            )
        })
        .collect()
}

/// The score key the baseline sorts with: score descending, then the tonic name
/// and the scale type lexically, both ascending.
fn score_key(suggestion: &KeySuggestion) -> (i64, String, String) {
    (
        -i64::try_from(suggestion.score).expect("a score fits in i64"),
        suggestion.tonic.name().to_owned(),
        suggestion.scale_type.as_str().to_owned(),
    )
}

#[test]
fn the_frozen_fixture_carries_every_key_case() {
    let records = records();
    let mut ids: BTreeSet<String> = BTreeSet::new();
    for record in &records {
        assert_eq!(
            record["operation"].as_str(),
            Some("suggest_keys"),
            "an unexpected operation"
        );
        assert!(
            ids.insert(case_id(record)),
            "duplicate case id in the fixture"
        );
    }
    assert_eq!(ids.len(), RECORDS, "the case ids are unique");

    // Three records answer nothing (one flat-rooted pair and the two
    // incompatible inputs) and one has no input at all and therefore answers
    // every candidate key.
    let empty = records
        .iter()
        .filter(|record| {
            record["output"]["suggestions"]
                .as_array()
                .is_some_and(Vec::is_empty)
        })
        .count();
    assert_eq!(empty, EMPTY_ANSWER_RECORDS);

    let without_input = records
        .iter()
        .filter(|record| input_chords(record).is_empty())
        .count();
    assert_eq!(without_input, 1, "one record has no active chord");
    let raw_empty = fixture_case("suggest_keys/empty");
    assert_eq!(
        frozen_identities(&raw_empty).len(),
        CANDIDATE_KEYS,
        "the raw empty operation answers every candidate key"
    );
}

#[test]
fn every_frozen_key_case_answers_its_record_except_the_recorded_deviation() {
    let mut identical = 0usize;
    let mut deviating: BTreeSet<String> = BTreeSet::new();

    for record in records() {
        let case = case_id(&record);
        let chords = input_chords(&record);
        let ours = suggest_keys(&chords);
        let frozen = frozen_identities(&record);

        if our_identities(&ours) == frozen {
            identical += 1;
            continue;
        }

        assert_eq!(
            case, DEVIATING_CASE,
            "{case}: the answer differs outside the recorded Contract.D05 deviation"
        );
        deviating.insert(case);
    }

    assert_eq!(identical, IDENTICAL_RECORDS, "the identical record count");
    assert_eq!(deviating.len(), DEVIATING_RECORDS);
    assert_eq!(
        deviating,
        BTreeSet::from([DEVIATING_CASE.to_owned()]),
        "the deviating record is the one the ledger names"
    );
}

#[test]
fn the_recorded_deviation_moves_the_score_of_one_flat_rooted_record_only() {
    // `Contract.D05`, option A: the triad-base map and the containment/scoring
    // rule are ported unchanged, and the one flat-rooted record differs in the
    // `score` field of its suggestions — membership and order identical.
    let record = fixture_case(DEVIATING_CASE);
    let chords = input_chords(&record);
    let ours = suggest_keys(&chords);
    let frozen = frozen_identities(&record);

    assert_eq!(
        ours.len(),
        DEVIATING_SUGGESTIONS,
        "{DEVIATING_CASE}: the suggestion count"
    );
    assert_eq!(frozen.len(), DEVIATING_SUGGESTIONS, "and the frozen count");

    // Membership and order: the (tonic, scale type) sequence is identical and
    // the totals are identical; only the score moved.
    let ours_identity: Vec<(String, String)> = ours
        .iter()
        .map(|suggestion| {
            (
                suggestion.tonic.name().to_owned(),
                suggestion.scale_type.as_str().to_owned(),
            )
        })
        .collect();
    let frozen_identity: Vec<(String, String)> = frozen
        .iter()
        .map(|(tonic, scale, _, _)| (tonic.clone(), scale.clone()))
        .collect();
    assert_eq!(
        ours_identity, frozen_identity,
        "{DEVIATING_CASE}: membership and order"
    );

    // The baseline scores nothing at all; the native pitch class scores every
    // suggestion perfectly, which is what the deviation records.
    assert!(
        frozen
            .iter()
            .all(|(_, _, score, total)| *score == 0 && *total == 2),
        "{DEVIATING_CASE}: the baseline answers 0:2 for every suggestion"
    );
    assert!(
        ours.iter()
            .all(|suggestion| suggestion.score == DEVIATING_SCORE && suggestion.total == 2),
        "{DEVIATING_CASE}: the native answer is {DEVIATING_SCORE}:2 for every suggestion"
    );

    // The option-A rule itself: a native root is a spelling-free pitch class, so
    // the flat spelling and its sharp enharmonic are *the same input* — the
    // record is not skipped, it is answered by the pitch-class rule the user
    // approved. This is not read from the implementation: it is the sentence the
    // decision rests on, asserted through the domain's own note lookup.
    for (flat, sharp) in [("Eb", "D#"), ("Bb", "A#")] {
        assert_eq!(
            note_index(flat).expect("a flat alias"),
            sharp.parse::<PitchClass>().expect("a sharp name"),
            "{flat} and {sharp} are one pitch class, so no spelling survives"
        );
    }
    assert!(
        chords
            .iter()
            .all(|chord| chord.root == note_index("D#").expect("a note")
                || chord.root == note_index("A#").expect("a note")),
        "{DEVIATING_CASE}: the input roots are the sharp pitch classes"
    );
}

#[test]
fn the_second_flat_rooted_record_answers_nothing_either_way() {
    // `suggest_keys/flat-root-Bb-with-g` has no suggestion in the baseline
    // *and* none natively, because no candidate key contains both chords: the
    // asymmetry needs a record the key actually contains, which the first flat
    // case has and this one does not.
    let record = fixture_case(FLAT_ROOT_CASES[1]);
    let chords = input_chords(&record);
    assert_eq!(chords.len(), 2, "the record carries two chords");
    assert!(
        frozen_identities(&record).is_empty(),
        "the baseline answers nothing"
    );
    assert!(
        suggest_keys(&chords).is_empty(),
        "and so does the native rule: no candidate key contains both chords"
    );
}

#[test]
fn containment_decides_which_candidates_are_suggested() {
    // Every suggestion must contain every input chord, and the incompatible
    // record's emptiness comes from the combination: each of its three chords is
    // contained in some candidate key on its own.
    for record in records() {
        let case = case_id(&record);
        let chords = input_chords(&record);
        for suggestion in suggest_keys(&chords) {
            let scale: BTreeSet<PitchClass> = scale_notes(suggestion.tonic, suggestion.scale_type)
                .into_iter()
                .collect();
            for chord in &chords {
                let notes = chord_details(chord).expect("a catalog chord").notes;
                assert!(
                    notes.iter().all(|note| scale.contains(note)),
                    "{case}: {} {} is suggested without containing {chord:?}",
                    suggestion.tonic.name(),
                    suggestion.scale_type.as_str()
                );
            }
        }
    }

    let record = fixture_case(INCOMPATIBLE_CASE);
    let chords = input_chords(&record);
    assert!(
        frozen_identities(&record).is_empty(),
        "{INCOMPATIBLE_CASE}: the baseline answers nothing"
    );
    assert!(
        suggest_keys(&chords).is_empty(),
        "{INCOMPATIBLE_CASE}: the native answer is empty"
    );
    for chord in &chords {
        assert!(
            !suggest_keys(std::slice::from_ref(chord)).is_empty(),
            "{INCOMPATIBLE_CASE}: {chord:?} alone does have candidates, so the \
             emptiness comes from the three chords together"
        );
    }
}

#[test]
fn the_triad_base_map_covers_the_catalog_and_answers_its_named_entries() {
    // The map is the pinned `@quality_to_triad`: every one of the 47 frozen
    // qualities has a triad base, and the base is one of the six triad
    // qualities. A map that misses an entry, or that falls back to the
    // quality's own label, cannot answer these.
    let triad_qualities: BTreeSet<&str> = TRIAD_QUALITIES.iter().copied().collect();
    for quality in QualityId::ALL {
        let base = triad_base(quality);
        assert!(
            triad_qualities.contains(base.as_str()),
            "{}: {} is not a triad quality",
            quality.as_str(),
            base.as_str()
        );
    }
    assert_eq!(
        QualityId::ALL.len(),
        47,
        "the frozen catalog carries 47 qualities"
    );

    // A triad maps to itself...
    for id in TRIAD_QUALITIES {
        let quality: QualityId = id.parse().expect("a catalog triad");
        assert_eq!(
            triad_base(quality).as_str(),
            id,
            "{id}: a triad maps to itself"
        );
    }

    // ...and these are the named entries the decision evidence reads from the
    // pinned table: the sevenths, sixths, added tones, ninths and suspensions
    // that do not carry their own triad base.
    for (id, base) in NAMED_TRIAD_BASES {
        let quality: QualityId = id.parse().expect("a catalog quality");
        assert_eq!(
            triad_base(quality).as_str(),
            base,
            "{id}: the triad base differs from the pinned table"
        );
    }
}

#[test]
fn the_triad_base_map_decides_the_scores_of_the_extended_records() {
    // The fixture's own records for the two halves the evidence measures: the
    // ninths (`maj9` and `13` both map to major, so a C maj9 / G 13 input
    // answers 2:2) and the mixed suspensions (a sus4, a sus2 and a dim never
    // reach a perfect score).
    let ninths = fixture_case("suggest_keys/extended-ninths");
    let chords = input_chords(&ninths);
    let ours = suggest_keys(&chords);
    assert_eq!(our_identities(&ours), frozen_identities(&ninths));

    let expected_score = chords
        .iter()
        .filter(|chord| triad_base(chord.quality).as_str() == "major")
        .count();
    assert_eq!(
        expected_score,
        chords.len(),
        "both input qualities map to the major triad base"
    );
    assert!(
        ours.iter()
            .all(|suggestion| suggestion.score == expected_score),
        "every suggestion of the ninths record scores every chord"
    );

    let mixed = fixture_case("suggest_keys/sus-and-dim-mixed");
    let ours = suggest_keys(&input_chords(&mixed));
    assert_eq!(our_identities(&ours), frozen_identities(&mixed));
    assert!(
        ours.iter()
            .all(|suggestion| suggestion.score < suggestion.total),
        "the suspended and diminished input never reaches a perfect score"
    );
}

#[test]
fn duplicate_occurrences_score_once_each() {
    // The input carries C twice and G once; every suggestion that contains both
    // roots scores all three occurrences, not the two distinct chords.
    let record = fixture_case("suggest_keys/duplicate-occurrences");
    let chords = input_chords(&record);
    assert_eq!(chords.len(), 3, "two C majors and one G major");
    let ours = suggest_keys(&chords);
    assert_eq!(our_identities(&ours), frozen_identities(&record));
    assert!(
        ours.iter().any(|suggestion| suggestion.score == 3),
        "a suggestion scores the repeated chord twice"
    );
}

#[test]
fn equal_scores_are_ordered_by_the_lexical_tonic_and_scale_tie_break() {
    // Every record: score descending, then the tonic name and the scale type
    // ascending. The tied records (`suggest_keys/lexical-order-tie`,
    // `suggest_keys/single-c-major`) and the ordering of their tie groups are
    // read from the fixture; the ordering rule itself is checked on all of them.
    for record in records() {
        let ours = suggest_keys(&input_chords(&record));
        let keys: Vec<(i64, String, String)> = ours.iter().map(score_key).collect();
        let mut sorted = keys.clone();
        sorted.sort();
        assert_eq!(
            keys,
            sorted,
            "{}: the suggestions are not ordered by score, tonic and scale type",
            case_id(&record)
        );
    }

    let record = fixture_case("suggest_keys/lexical-order-tie");
    let ours = suggest_keys(&input_chords(&record));
    let frozen = frozen_identities(&record);
    assert_eq!(our_identities(&ours), frozen);
    assert_eq!(
        ours.iter().filter(|s| s.score == 2).count(),
        frozen.iter().filter(|(_, _, score, _)| *score == 2).count(),
        "the tied group is the same size"
    );
    assert!(
        frozen.iter().filter(|(_, _, score, _)| *score == 2).count() > 3,
        "the record really carries a tie group worth ordering"
    );
}

#[test]
fn the_raw_empty_operation_answers_every_candidate_where_the_page_gate_answers_nothing() {
    // `suggest_keys/empty`: with no chord at all, containment is vacuous for
    // every candidate, so the raw operation answers all 168 of them at 0:0, in
    // the lexical tonic and scale order. The page's own gate (`length(chords) >=
    // 2` in the pinned LiveView) is the only thing that answers nothing, and it
    // lives in the domain so that no client re-implements it.
    let raw = suggest_keys(&[]);
    assert_eq!(raw.len(), CANDIDATE_KEYS);
    assert!(
        raw.iter()
            .all(|suggestion| suggestion.score == 0 && suggestion.total == 0),
        "an empty input scores nothing but answers every candidate"
    );
    let identities: Vec<(String, String)> = raw
        .iter()
        .map(|suggestion| {
            (
                suggestion.tonic.name().to_owned(),
                suggestion.scale_type.as_str().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        identities,
        frozen_identities(&fixture_case("suggest_keys/empty"))
            .into_iter()
            .map(|(tonic, scale, _, _)| (tonic, scale))
            .collect::<Vec<(String, String)>>(),
        "the raw empty answer is the frozen one"
    );

    assert!(
        page_key_suggestions(&[]).is_empty(),
        "the page gate answers nothing for no chord"
    );
    let single = input_chords(&fixture_case("suggest_keys/single-c-major"));
    assert_eq!(single.len(), 1);
    assert!(
        page_key_suggestions(&single).is_empty(),
        "the page gate answers nothing below two active chords"
    );
    let mut answering = 0usize;
    for record in records() {
        let chords = input_chords(&record);
        if chords.len() < 2 {
            continue;
        }
        assert_eq!(
            page_key_suggestions(&chords),
            suggest_keys(&chords),
            "{}: from two active chords the page gate is the raw operation",
            case_id(&record)
        );
        if !page_key_suggestions(&chords).is_empty() {
            answering += 1;
        }
    }
    // The gate changes nothing else: of the fourteen records with two or more
    // chords, the three whose chords no candidate contains still answer nothing
    // (the two `no-compatible` records and the second flat-rooted one).
    assert_eq!(answering, ANSWERING_RECORDS);
}

#[test]
fn a_suggestion_carries_the_frozen_diatonic_triads_of_its_key() {
    // The output field the clients render: every suggestion carries the triads
    // its score was computed on, in the frozen order, and they are the triads
    // the frozen record carries for the same tonic and scale type.
    let mut checked = 0usize;
    for record in records() {
        let case = case_id(&record);
        let ours = suggest_keys(&input_chords(&record));
        let frozen = record["output"]["suggestions"]
            .as_array()
            .expect("suggestions");
        assert_eq!(ours.len(), frozen.len(), "{case}: the suggestion count");
        for (suggestion, frozen_suggestion) in ours.iter().zip(frozen) {
            assert_eq!(
                suggestion.tonic.name(),
                frozen_suggestion["tonic"].as_str().expect("a tonic"),
                "{case}: the tonic order"
            );
            assert_eq!(
                suggestion.scale_type.as_str(),
                frozen_suggestion["scale_type"]
                    .as_str()
                    .expect("a scale type"),
                "{case}: the scale order"
            );
            assert_eq!(
                our_triads(suggestion),
                frozen_triads(&record, frozen_suggestion),
                "{case}: the diatonic triads of {} {}",
                suggestion.tonic.name(),
                suggestion.scale_type.as_str()
            );
            checked += 1;
        }
    }
    assert!(checked > 100, "the fixture carries {checked} suggestions");
}
