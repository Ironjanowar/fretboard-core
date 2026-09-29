//! The page's grouped suggestion rows (task `C16`; the pinned
//! `FretboardWeb.FretboardLive.group_key_suggestions/1`).
//!
//! The page does not show the flat suggestion list: it shows **rows**. A row is
//! either a single suggestion or a collapsed group of relative modes, and the
//! rule that builds them is the baseline's own:
//!
//! * only the suggestions at the **maximum score** are grouped, and only when
//!   that maximum is perfect (`score == total`);
//! * within that branch the seven modal modes are partitioned by note set, each
//!   note set becomes one collapsed row whose major and relative minor are
//!   prominent and whose other five modes are listed beside them, and the
//!   non-modal suggestions follow, then the lower-scoring ones;
//! * when the maximum is not perfect, the first three suggestions are shown as
//!   single rows and nothing else — the lower-scoring rest is *not* appended in
//!   that branch.
//!
//! Two independent things decide what a client sees, and they were measured
//! separately (`docs/p5-decision-evidence.md`):
//!
//! * **The row order is deterministic.** It is the enumeration order of the
//!   grouping map, whose key is the note set — the term order of a small map of
//!   note names, stable across runs and across atom-table states — so it is
//!   ported verbatim; see [`note_set_key`].
//! * **The drop is not, and is not ported.** The baseline keeps only the note
//!   sets that carry **all seven** modes and silently drops every other modal
//!   group, so a *perfect* C major can leave the panel empty and an equally
//!   perfect G major can vanish next to a complete group. `Contract.D06` records
//!   the user's choice (option B): **a perfect suggestion is never dropped.**
//!   This module therefore builds a row for every note set a perfect maximum
//!   carries — the incomplete group is shown with the members it has — and
//!   changes nothing else. `tests/key_groups.rs` pins the four affected records
//!   by name and asserts that no suggestion at the maximum score is hidden.
//!
//! Fixture: every record of `fixtures/oracle/key-groups.jsonl` is pinned by
//! `tests/key_groups.rs`.

use std::collections::BTreeMap;

use crate::keys::KeySuggestion;
use crate::scale::scale_notes;
use crate::types::ScaleId;

/// The seven modal modes that share a note set: the only scale types this
/// grouping collapses.
const MODAL_MODES: [&str; 7] = [
    "major",
    "minor",
    "dorian",
    "phrygian",
    "lydian",
    "mixolydian",
    "locrian",
];

/// The two modes of a group shown prominently, in the baseline's own priority
/// order (`major`, then its relative `minor`).
const PROMINENT_MODES: [&str; 2] = ["major", "minor"];

/// The imperfect branch shows this many single rows and nothing else.
pub const IMPERFECT_ROWS: usize = 3;

/// One display row of the key panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyRow {
    /// One suggestion, shown on its own.
    Single(KeySuggestion),
    /// A collapsed group of relative modes: the prominent pair and the rest.
    ///
    /// The baseline renders this row as "the major and the relative minor, plus
    /// the other modes"; how many modes it carries is not always seven, because
    /// `Contract.D06` shows an incomplete group instead of dropping it.
    Group {
        /// The group's prominent entries: the major and the relative minor the
        /// group carries, in that order.
        prominent: Vec<KeySuggestion>,
        /// Every other member of the group, in the input's order.
        others: Vec<KeySuggestion>,
    },
}

/// Groups flat key suggestions into the rows the panel renders.
///
/// The input is the suggestion list [`crate::suggest_keys`] answers, already
/// ordered by score: the first entry's score is the maximum, exactly as the
/// baseline reads `hd(suggestions).score`.
pub fn group_key_suggestions(suggestions: &[KeySuggestion]) -> Vec<KeyRow> {
    let Some(first) = suggestions.first() else {
        return Vec::new();
    };
    let max_score = first.score;
    let mut rows: Vec<KeyRow> = Vec::new();

    if max_score == first.total {
        rows.extend(modal_rows(suggestions, max_score));
        rows.extend(
            suggestions
                .iter()
                .filter(|suggestion| {
                    suggestion.score == max_score && !is_modal(suggestion.scale_type)
                })
                .cloned()
                .map(KeyRow::Single),
        );
        rows.extend(
            suggestions
                .iter()
                .filter(|suggestion| suggestion.score < max_score)
                .cloned()
                .map(KeyRow::Single),
        );
    } else {
        rows.extend(
            suggestions
                .iter()
                .take(IMPERFECT_ROWS)
                .cloned()
                .map(KeyRow::Single),
        );
    }

    rows
}

/// Whether a scale type is one of the seven modal modes.
fn is_modal(scale_type: ScaleId) -> bool {
    MODAL_MODES.contains(&scale_type.as_str())
}

/// The collapsed rows of the perfect branch: one per note set, in note-set term
/// order.
///
/// The baseline keeps only the note sets with all seven modes; `Contract.D06`
/// shows every one of them, so the filter that produced the drop is deliberately
/// absent here.
fn modal_rows(suggestions: &[KeySuggestion], max_score: usize) -> Vec<KeyRow> {
    let mut groups: BTreeMap<(usize, Vec<String>), Vec<&KeySuggestion>> = BTreeMap::new();
    for suggestion in suggestions
        .iter()
        .filter(|suggestion| suggestion.score == max_score && is_modal(suggestion.scale_type))
    {
        groups
            .entry(note_set_key(suggestion))
            .or_default()
            .push(suggestion);
    }

    groups
        .into_values()
        .map(|members| {
            let prominent = prominent(&members);
            let others = others(&members, &prominent);
            KeyRow::Group { prominent, others }
        })
        .collect()
}

/// The grouping key of one suggestion: its note set, ordered the way the
/// baseline's grouping map enumerates its keys.
///
/// The baseline groups by a `MapSet` of note names and returns the rows in that
/// map's enumeration order; for a map this small, that order is the term order
/// of its keys — the number of notes first, then the sorted note names. Every
/// modal mode is heptatonic, so the size term never decides in practice; it is
/// stated anyway, because that is the order the frozen rows were exported in
/// (`key_groups/tied-complete-seven-mode-row-order` spells `C major` first and
/// the fixture answers the `F major` group first).
fn note_set_key(suggestion: &KeySuggestion) -> (usize, Vec<String>) {
    let mut names: Vec<String> = scale_notes(suggestion.tonic, suggestion.scale_type)
        .iter()
        .map(|note| note.name().to_owned())
        .collect();
    names.sort_unstable();
    (names.len(), names)
}

/// The prominent entries of a group.
///
/// The baseline finds the group's `major` and `minor` members and drops the
/// missing ones, which is what lets an incomplete group be shown instead of
/// dropped (`Contract.D06`).
fn prominent(members: &[&KeySuggestion]) -> Vec<KeySuggestion> {
    PROMINENT_MODES
        .iter()
        .filter_map(|wanted| {
            members
                .iter()
                .find(|member| member.scale_type.as_str() == *wanted)
                .map(|member| (*member).clone())
        })
        .collect()
}

/// Every remaining member of a group, in the order the group was built in.
///
/// The baseline rejects by *scale type*, so a group that carries two majors
/// loses both from `others`; that cannot happen for a note set of a heptatonic
/// scale, and the rule is kept as it is.
fn others(members: &[&KeySuggestion], prominent: &[KeySuggestion]) -> Vec<KeySuggestion> {
    members
        .iter()
        .filter(|member| {
            !prominent
                .iter()
                .any(|shown| shown.scale_type == member.scale_type)
        })
        .map(|member| (*member).clone())
        .collect()
}
