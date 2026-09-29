//! Greedy multi-key suggestions (task `C17`; the pinned
//! `Fretboard.Music.Scale.suggest_multi_keys/1`).
//!
//! When a chord list shares no single key, the panel can offer several keys
//! that together explain it. The baseline builds the same 168 candidate keys
//! [`crate::suggest_keys`] builds, then runs a greedy set cover of at most three
//! groups:
//!
//! * each step takes the candidate covering the most **still uncovered**
//!   occurrences;
//! * ties go to the candidate whose *diatonic score* is higher — and that score
//!   counts **every** occurrence the candidate covers, not only the newly
//!   covered ones;
//! * remaining ties go to the earlier candidate in the frozen enumeration order,
//!   which is the tonic order and then the catalog scale order (the baseline's
//!   `Enum.max_by` keeps the first maximum over `{coverage, score, -priority}`);
//! * the cover stops after three groups, when nothing is left uncovered, or
//!   when no candidate covers a remaining chord.
//!
//! ## Exclusive assignment versus displayed membership
//!
//! The greedy's own assignment is *exclusive*: each step records the
//! occurrences it newly covered. That assignment decides exactly two things —
//! the all-singleton rule (a selection whose every group owns one occurrence
//! answers nothing at all) and which occurrences are left **unmatched** (they
//! are appended as a final group without a key). What a group *displays* is the
//! key's **full** membership: every input occurrence whose notes fit that key,
//! in input order, so one chord can appear in two groups. The plan's named case
//! (`D min, G, E, F`) shows both halves: C major displays `D min, G, F` and A
//! harmonic minor displays `D min, E, F`, although its exclusive share is the
//! single `E`.
//!
//! Groups with a key are ordered by the size of that displayed membership,
//! descending (a stable sort, so the greedy's own order decides equal sizes);
//! the keyless group, when there is one, is last.
//!
//! ## Spelling
//!
//! The baseline normalizes every input root through `@flat_to_sharp` before
//! covering and scoring, so the multi-key surface disagrees with the single-key
//! one about a flat-rooted input (`docs/p5-decision-evidence.md` measures both).
//! A native root is a [`crate::PitchClass`] with no spelling, so that
//! normalization is the identity here and the two surfaces agree.
//!
//! ## The page gate
//!
//! The raw operation answers nothing below three chords, and the page adds a
//! second half: it computes multi-key suggestions only when the *single-key*
//! operation answered nothing ([`page_multi_key_suggestions`]). Both live here
//! so no client re-implements them.

use std::cmp::Reverse;
use std::collections::BTreeSet;

use crate::chord::ChordMode;
use crate::keys::{KeySuggestion, candidate_keys, chord_notes, explains};
use crate::scale::{DiatonicChord, diatonic_chords, scale_notes};
use crate::state::ChordSpec;
use crate::types::{PitchClass, ScaleId};

/// The most groups a suggestion answers with.
pub const MULTI_KEY_MAX_GROUPS: usize = 3;

/// The number of active chords below which the multi-key operation answers
/// nothing, and the page computes nothing.
pub const MULTI_KEY_PAGE_MIN_CHORDS: usize = 3;

/// One displayed group: the key that explains the group's chords, and the chords
/// themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultiKeyGroup {
    /// The group's key, or `None` for the final group of unmatched chords — the
    /// occurrences no candidate key contains.
    pub key: Option<KeySuggestion>,
    /// The key's full membership: every input occurrence whose notes fit the
    /// key, in input order, with repeats.
    pub chords: Vec<ChordSpec>,
}

/// One candidate key with the input occurrences it covers and its score.
struct Candidate {
    tonic: PitchClass,
    scale_type: ScaleId,
    covered: BTreeSet<usize>,
    diatonic_score: usize,
    diatonic_chords: Vec<DiatonicChord>,
}

/// The greedy's rank of one candidate: how much of the still-uncovered set it
/// covers, its diatonic score, and its frozen scale priority — reversed, so a
/// larger rank means a better candidate.
type Rank = (usize, usize, Reverse<usize>);

/// The multi-key suggestion of a chord list.
///
/// Fewer than [`MULTI_KEY_PAGE_MIN_CHORDS`] chords answer an empty list, and so
/// does a selection whose every group owns exactly one occurrence (a list whose
/// chords share no key at all). Otherwise the answer carries up to
/// [`MULTI_KEY_MAX_GROUPS`] groups in display order, plus a final keyless group
/// when some occurrence is contained by no candidate key.
pub fn suggest_multi_keys(chords: &[ChordSpec]) -> Vec<MultiKeyGroup> {
    if chords.len() < MULTI_KEY_PAGE_MIN_CHORDS {
        return Vec::new();
    }

    let candidates = build_candidates(chords);
    let mut selected: Vec<(usize, BTreeSet<usize>)> = Vec::new();
    let mut covered: BTreeSet<usize> = BTreeSet::new();

    while selected.len() < MULTI_KEY_MAX_GROUPS {
        let remaining: BTreeSet<usize> = (0..chords.len())
            .filter(|index| !covered.contains(index))
            .collect();
        if remaining.is_empty() {
            break;
        }
        let Some(index) = best_candidate(&candidates, &remaining) else {
            break;
        };
        let Some(candidate) = candidates.get(index) else {
            break;
        };
        let newly: BTreeSet<usize> = candidate
            .covered
            .intersection(&remaining)
            .copied()
            .collect();
        covered.extend(newly.iter().copied());
        selected.push((index, newly));
    }

    // The singleton rule: if every selected group owned exactly one occurrence,
    // the keys share nothing and the answer is empty (the baseline's
    // `all_groups_are_singletons?`, which also answers empty for no selection).
    if selected.iter().all(|(_index, newly)| newly.len() == 1) {
        return Vec::new();
    }

    let mut groups: Vec<MultiKeyGroup> = selected
        .iter()
        .filter_map(|(index, _newly)| candidates.get(*index))
        .map(|candidate| MultiKeyGroup {
            key: Some(KeySuggestion {
                tonic: candidate.tonic,
                scale_type: candidate.scale_type,
                score: candidate.diatonic_score,
                total: chords.len(),
                diatonic_chords: candidate.diatonic_chords.clone(),
            }),
            chords: candidate
                .covered
                .iter()
                .filter_map(|index| chords.get(*index).copied())
                .collect(),
        })
        .collect();
    groups.sort_by_key(|group| Reverse(group.chords.len()));

    let matched: BTreeSet<usize> = selected
        .iter()
        .flat_map(|(_index, newly)| newly.iter().copied())
        .collect();
    let unmatched: Vec<ChordSpec> = chords
        .iter()
        .enumerate()
        .filter(|(index, _chord)| !matched.contains(index))
        .map(|(_index, chord)| *chord)
        .collect();
    if !unmatched.is_empty() {
        groups.push(MultiKeyGroup {
            key: None,
            chords: unmatched,
        });
    }

    groups
}

/// The page's multi-key suggestion: the raw operation, gated on the single-key
/// operation having answered nothing and on at least
/// [`MULTI_KEY_PAGE_MIN_CHORDS`] active chords.
///
/// This is the pinned LiveView's own rule (`suggestions == [] and
/// length(chords) >= 3`), kept in the domain so that a client shows the same
/// panel the baseline shows without re-implementing the gate. `single_keys` is
/// the answer of [`suggest_keys`] (or of [`page_key_suggestions`], which is
/// empty below two chords and therefore also empty here).
pub fn page_multi_key_suggestions(
    single_keys: &[KeySuggestion],
    chords: &[ChordSpec],
) -> Vec<MultiKeyGroup> {
    if single_keys.is_empty() && chords.len() >= MULTI_KEY_PAGE_MIN_CHORDS {
        suggest_multi_keys(chords)
    } else {
        Vec::new()
    }
}

/// Every candidate key with the occurrences it covers and its score.
///
/// The score counts the candidate's whole covered set, which is the rule the
/// greedy's tie-break depends on.
fn build_candidates(chords: &[ChordSpec]) -> Vec<Candidate> {
    let note_sets: Vec<Vec<PitchClass>> = chords.iter().map(chord_notes).collect();

    candidate_keys()
        .into_iter()
        .map(|(tonic, scale_type)| {
            let scale: BTreeSet<PitchClass> = scale_notes(tonic, scale_type).into_iter().collect();
            let covered: BTreeSet<usize> = note_sets
                .iter()
                .enumerate()
                .filter(|(_index, notes)| notes.iter().all(|note| scale.contains(note)))
                .map(|(index, _notes)| index)
                .collect();
            let diatonic_chords = diatonic_chords(tonic, scale_type, ChordMode::Triad);
            let diatonic_score = covered
                .iter()
                .filter(|index| {
                    chords
                        .get(**index)
                        .is_some_and(|chord| explains(&diatonic_chords, chord))
                })
                .count();

            Candidate {
                tonic,
                scale_type,
                covered,
                diatonic_score,
                diatonic_chords,
            }
        })
        .collect()
}

/// The candidate the greedy selects for the still-uncovered occurrences.
///
/// The rank is `(covered occurrences, diatonic score, -scale priority)` and the
/// **first** maximum wins, which is the baseline's `Enum.max_by` and is what
/// makes a tie deterministic: the candidates are enumerated in tonic order and
/// then in catalog scale order.
fn best_candidate(candidates: &[Candidate], remaining: &BTreeSet<usize>) -> Option<usize> {
    let mut best: Option<(usize, Rank)> = None;
    for (index, candidate) in candidates.iter().enumerate() {
        let coverage = candidate.covered.intersection(remaining).count();
        if coverage == 0 {
            continue;
        }
        let rank = (
            coverage,
            candidate.diatonic_score,
            Reverse(scale_priority(candidate.scale_type)),
        );
        match &best {
            Some((_index, best_rank)) if *best_rank >= rank => {}
            _ => best = Some((index, rank)),
        }
    }
    best.map(|(index, _rank)| index)
}

/// The frozen scale order of a scale type: its position in [`ScaleId::ALL`].
///
/// An unknown scale type (impossible for a validated identifier) ranks last.
fn scale_priority(scale_type: ScaleId) -> usize {
    ScaleId::ALL
        .iter()
        .position(|known| *known == scale_type)
        .unwrap_or(usize::MAX)
}
