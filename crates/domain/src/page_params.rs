//! The page-params codec: the query parameters of a Fretboard page
//! (`02-core-contract.md` section 6, and the pinned
//! `lib/fretboard/music/{url_codec,page_codec}.ex`).
//!
//! This file belongs to task `C08`. The codec is the one place where the wire
//! spelling of a page is interpreted, so the URL layer (`C19`) and the session
//! schema (`P6`) do not grow a second, drifting copy of it.
//!
//! Two pure directions:
//!
//! * [`decode_page_params`] turns parameters into the canonical [`PageState`].
//!   Every field is decoded on its own, and a field that cannot be understood is
//!   left at its default instead of failing the whole page: the pinned source
//!   ignores an invalid sibling rather than discarding a valid one, and so does
//!   this codec. The rules are the frozen ones, including their deliberate
//!   asymmetries: `pitches` is authoritative and never falls back to legacy
//!   notes; an invalid `pitches` resets the reference too; `marked` overwrites
//!   before it filters; `keys` are canonicalised while `chords` are not.
//! * [`encode_page_params`] turns a state into the parameters that reproduce it,
//!   omitting every field whose value is the default of its instrument.
//!
//! [`decoded_page`] exposes the decoded form the pinned source returns as a plain
//! map (`PageCodec.decode_page_params/1`). The canonical state carries the same
//! information projected onto the typed contract, so a consumer that needs the
//! baseline's decoded shape — the parity work and the adapter's tests in `P6` —
//! asks for it here instead of re-deriving it.
//!
//! Fixture: every record of `fixtures/oracle/page-params.jsonl` (104 cases of
//! `PageCodec.decode_page_params/1` and `encode_page_params/1`) is pinned by
//! `tests/page_params.rs`, including the re-decoded form, so a
//! decode→encode→decode cycle cannot drift from the baseline.

use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;

use serde_json::{Map, Value};

use crate::chord::{chord_quality_label, quality_from_label};
use crate::instrument_catalog::{
    instrument_frets, instrument_strings, keyboard_pitch_range, preset_pitches, standard_pitches,
    standard_tuning_notes,
};
use crate::state::{ChordSpec, InstrumentState, PageState, Position, TuningState};
use crate::types::{
    Fret, InstrumentId, OpenPitch, PITCH_CLASS_NAMES, PitchClass, PresetName, StringIndex, Tab,
};

/// The reference every instrument's standard preset is named after.
const STANDARD: &str = "Standard";

/// Decode one page's query parameters into the canonical page state.
///
/// The rules are the frozen `PageCodec.decode_page_params/1`:
///
/// * `instrument` selects the shape; an unknown or non-string value is the
///   guitar, and `piano` is the only value that takes the piano branch.
/// * `chords` and `highlight` are chord labels; an invalid token is skipped
///   individually and a repeated chord is kept, because an occurrence is not an
///   identity. The highlight is the *first* occurrence of its label.
/// * For a fretted instrument, `pitches` (exact MIDI numbers, one per string) is
///   authoritative over `tuning`, and an invalid `pitches` resets both the
///   pitches and the `reference` to the instrument's standard. Without
///   `pitches`, `tuning` is a list of note names resolved to the pitch nearest
///   the standard pitch of that string. A `reference` is accepted only when it
///   names a preset of that instrument, and only alongside a valid `pitches`.
/// * `marked` is `string-fret` pairs: a later pair overwrites an earlier one for
///   the same string, and only then are out-of-range strings and frets dropped.
/// * For the piano, `keys` are absolute pitches, canonicalised (deduplicated and
///   ascending); every fretted field is ignored.
/// * `tab` is `visualizer` unless it is exactly `analyzer`.
pub fn decode_page_params(params: &Map<String, Value>) -> PageState {
    let instrument = text(params, "instrument")
        .and_then(|value| InstrumentId::from_str(value).ok())
        .unwrap_or(InstrumentId::Guitar);

    let chords = decode_chords(text(params, "chords"));
    let highlight = decode_highlight(text(params, "highlight"), &chords);

    let instrument_state = if instrument == InstrumentId::Piano {
        InstrumentState::Piano {
            selected: decode_keys(text(params, "keys")),
        }
    } else {
        InstrumentState::Fretted {
            instrument,
            tuning: decode_fretted_tuning(params, instrument),
            selected: decode_marked(
                text(params, "marked"),
                instrument_strings(instrument).unwrap_or_default(),
                instrument_frets(instrument).unwrap_or_default(),
            ),
        }
    };

    PageState {
        instrument: instrument_state,
        chords,
        highlight,
        tab: decode_tab(text(params, "tab")),
    }
}

/// Encode a page state into the query parameters that reproduce it.
///
/// The rules are the frozen `PageCodec.encode_page_params/1`, which omits every
/// field whose value is the default of its instrument:
///
/// * `chords` and `highlight` are written as chord labels; an empty chord list
///   and a highlight that is none of the occurrences are omitted.
/// * `instrument` is omitted for the guitar.
/// * For a fretted instrument, `tuning` is written when the pitches' note names
///   differ from the instrument's standard names, and `pitches` (with
///   `reference`) whenever the pitches or the reference differ from the
///   standard. A non-standard reference always travels with its `pitches`,
///   because the decoder ignores a reference on its own.
/// * `marked` and `keys` are written when the selection is not empty, ascending.
/// * `tab` is omitted unless it is the analyzer.
pub fn encode_page_params(state: &PageState) -> Map<String, Value> {
    let mut params = Map::new();

    if !state.chords.is_empty() {
        let labels = state
            .chords
            .iter()
            .map(chord_label)
            .collect::<Vec<String>>();
        put(&mut params, "chords", &labels.join(","));
    }

    if let Some(index) = highlighted_index_of(state.highlight.as_ref(), &state.chords)
        && let Some(spec) = state.chords.get(index)
    {
        put(&mut params, "highlight", &chord_label(spec));
    }

    match &state.instrument {
        InstrumentState::Fretted {
            instrument,
            tuning,
            selected,
        } => {
            if *instrument != InstrumentId::Guitar {
                put(&mut params, "instrument", instrument.as_str());
            }

            let standard = standard_pitches(*instrument).unwrap_or(&[]);
            let notes = tuning
                .pitches
                .iter()
                .copied()
                .map(note_name)
                .collect::<Vec<&str>>();
            let standard_notes = standard
                .iter()
                .copied()
                .map(note_name)
                .collect::<Vec<&str>>();

            if notes != standard_notes {
                put(&mut params, "tuning", &notes.join(","));
            }
            if tuning.pitches != standard || tuning.reference.as_str() != STANDARD {
                let pitches = tuning
                    .pitches
                    .iter()
                    .copied()
                    .map(u8::from)
                    .map(|pitch| pitch.to_string())
                    .collect::<Vec<String>>();
                put(&mut params, "pitches", &pitches.join(","));
                if tuning.reference.as_str() != STANDARD {
                    put(&mut params, "reference", tuning.reference.as_str());
                }
            }
            if !selected.is_empty() {
                put(&mut params, "marked", &encode_marked(selected));
            }
        }
        InstrumentState::Piano { selected } => {
            put(&mut params, "instrument", InstrumentId::Piano.as_str());
            if !selected.is_empty() {
                let keys = selected
                    .iter()
                    .copied()
                    .map(u8::from)
                    .map(|pitch| pitch.to_string())
                    .collect::<Vec<String>>();
                put(&mut params, "keys", &keys.join(","));
            }
        }
    }

    if state.tab == Tab::Analyzer {
        put(&mut params, "tab", "analyzer");
    }

    params
}

/// The decoded form of a page state: the map shape the pinned
/// `PageCodec.decode_page_params/1` returns.
///
/// The canonical state and this form carry the same information; the difference
/// is spelling. `selection` is keyed by string index for a fretted instrument and
/// is a list of absolute pitches for the piano, `tuning_state` is absent for the
/// piano, and the highlight is the *index* of its first occurrence rather than
/// the identity it points at — which is exactly how the baseline stores it, and
/// why [`encode_page_params`] can write the label back.
pub fn decoded_page(state: &PageState) -> Value {
    let mut decoded = Map::new();

    let chords = state
        .chords
        .iter()
        .map(|spec| {
            let mut chord = Map::new();
            put(&mut chord, "root", spec.root.name());
            put(&mut chord, "quality", spec.quality.as_str());
            Value::Object(chord)
        })
        .collect::<Vec<Value>>();
    decoded.insert("active_chords".to_owned(), Value::Array(chords));

    let highlight = highlighted_index_of(state.highlight.as_ref(), &state.chords)
        .map_or(Value::Null, |index| {
            Value::from(u64::try_from(index).unwrap_or_default())
        });
    decoded.insert("highlighted_chord".to_owned(), highlight);

    decoded.insert(
        "tab".to_owned(),
        Value::from(match state.tab {
            Tab::Visualizer => "visualizer",
            Tab::Analyzer => "analyzer",
        }),
    );

    match &state.instrument {
        InstrumentState::Fretted {
            instrument,
            tuning,
            selected,
        } => {
            decoded.insert("instrument".to_owned(), Value::from(instrument.as_str()));

            let mut selection = Map::new();
            for position in selected {
                selection.insert(
                    u8::from(position.string).to_string(),
                    Value::from(u8::from(position.fret)),
                );
            }
            decoded.insert("selection".to_owned(), Value::Object(selection));

            let mut tuning_state = Map::new();
            let pitches = tuning
                .pitches
                .iter()
                .copied()
                .map(u8::from)
                .map(Value::from)
                .collect::<Vec<Value>>();
            tuning_state.insert("pitches".to_owned(), Value::Array(pitches));
            tuning_state.insert(
                "reference".to_owned(),
                Value::from(tuning.reference.as_str()),
            );
            decoded.insert("tuning_state".to_owned(), Value::Object(tuning_state));
        }
        InstrumentState::Piano { selected } => {
            decoded.insert(
                "instrument".to_owned(),
                Value::from(InstrumentId::Piano.as_str()),
            );
            let keys = selected
                .iter()
                .copied()
                .map(u8::from)
                .map(Value::from)
                .collect::<Vec<Value>>();
            decoded.insert("selection".to_owned(), Value::Array(keys));
            decoded.insert("tuning_state".to_owned(), Value::Null);
        }
    }

    Value::Object(decoded)
}

/// The string value of one parameter, when the parameter is a string at all.
fn text<'a>(params: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    params.get(key).and_then(Value::as_str)
}

/// Insert one string field.
fn put(params: &mut Map<String, Value>, key: &str, value: &str) {
    params.insert(key.to_owned(), Value::from(value));
}

/// Parse one token the way the pinned source's `Integer.parse/1` does: an
/// optional sign, then one or more digits, and nothing else.
///
/// The whole token has to be consumed, so surrounding whitespace, a fractional
/// part and trailing text all fail, while a sign and leading zeros are accepted.
fn integer_token(token: &str) -> Option<i64> {
    let (sign, digits) = token.strip_prefix('-').map_or_else(
        || (1_i64, token.strip_prefix('+').unwrap_or(token)),
        |rest| (-1_i64, rest),
    );

    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }

    digits
        .parse::<i64>()
        .ok()
        .and_then(|value| value.checked_mul(sign))
}

/// Split a parameter the way `String.split(",", trim: true)` does: empty tokens
/// are dropped wherever they appear.
fn tokens(value: &str) -> Vec<&str> {
    value.split(',').filter(|token| !token.is_empty()).collect()
}

/// Split a parameter the way `String.split(",", trim: false)` does: an empty
/// token is kept, and an empty string yields the single empty token.
fn tokens_keeping_empty(value: &str) -> Vec<&str> {
    value.split(',').collect()
}

/// The note name of one pitch: its pitch class, without an octave.
///
/// The fallback is the first name and is unreachable: a remainder below twelve is
/// always a valid pitch class.
#[allow(clippy::integer_division)]
fn note_name(pitch: OpenPitch) -> &'static str {
    let class = usize::from(u8::from(pitch) % 12);
    PITCH_CLASS_NAMES.get(class).copied().unwrap_or("C")
}

/// The wire label of one chord: its root followed by its quality's label.
fn chord_label(spec: &ChordSpec) -> String {
    format!("{}{}", spec.root.name(), chord_quality_label(spec.quality))
}

/// Parse one chord token into an identity, or nothing when it is not a chord.
///
/// The token is a root (one character, or two when the second is `#`) followed by
/// a quality label, which has to be one of the frozen catalog's labels.
fn chord_spec(token: &str) -> Option<ChordSpec> {
    let (root, rest) = if token.as_bytes().get(1) == Some(&b'#') {
        (token.get(..2)?, token.get(2..)?)
    } else {
        (token.get(..1)?, token.get(1..)?)
    };

    Some(ChordSpec {
        root: PitchClass::from_str(root).ok()?,
        quality: quality_from_label(rest)?,
    })
}

/// Decode the `chords` parameter: valid tokens in order, repeats kept.
fn decode_chords(value: Option<&str>) -> Vec<ChordSpec> {
    value.map_or_else(Vec::new, |parameter| {
        tokens(parameter)
            .into_iter()
            .filter_map(chord_spec)
            .collect()
    })
}

/// Decode the `highlight` parameter: a chord label, kept only when it names one
/// of the page's occurrences. The state stores the identity it points at; which
/// occurrence it was is recoverable, and the baseline stores the index.
fn decode_highlight(value: Option<&str>, chords: &[ChordSpec]) -> Option<ChordSpec> {
    let spec = chord_spec(value?)?;
    chords.contains(&spec).then_some(spec)
}

/// The index of the first occurrence of a highlighted identity.
fn highlighted_index_of(highlight: Option<&ChordSpec>, chords: &[ChordSpec]) -> Option<usize> {
    let highlight = highlight?;
    chords.iter().position(|spec| spec == highlight)
}

/// Decode the `tab` parameter: everything but `analyzer` is the visualizer.
fn decode_tab(value: Option<&str>) -> Tab {
    match value {
        Some("analyzer") => Tab::Analyzer,
        _ => Tab::Visualizer,
    }
}

/// The state's standard tuning: the instrument's standard preset pitches, under
/// the standard reference.
fn standard_tuning_state(instrument: InstrumentId) -> TuningState {
    TuningState {
        pitches: standard_pitches(instrument)
            .map_or_else(|_error| Vec::new(), <[OpenPitch]>::to_vec),
        reference: PresetName::from_catalog(STANDARD),
    }
}

/// Decode the `pitches` parameter: exactly one MIDI number per string, each of
/// them a token the frozen parser accepts and a value in `0..=127`.
fn decode_pitches(value: &str, count: u8) -> Option<Vec<OpenPitch>> {
    let raw = tokens_keeping_empty(value);
    if raw.len() != usize::from(count) {
        return None;
    }

    let mut pitches = Vec::with_capacity(raw.len());
    for token in raw {
        let number = integer_token(token)?;
        let byte = u8::try_from(number).ok()?;
        pitches.push(OpenPitch::try_from(byte).ok()?);
    }
    Some(pitches)
}

/// Decode the legacy `tuning` parameter into note names, or the instrument's
/// standard notes when it is not exactly one valid name per string.
fn decode_tuning_notes(value: &str, instrument: InstrumentId) -> Vec<PitchClass> {
    let standard = standard_tuning_notes(instrument).unwrap_or_default();
    let parsed = tokens(value)
        .into_iter()
        .map(|token| PitchClass::from_str(token).ok())
        .collect::<Option<Vec<PitchClass>>>();

    match parsed {
        Some(notes) if notes.len() == standard.len() => notes,
        _ => standard,
    }
}

/// The pitch of one pitch class that is closest to a reference pitch.
///
/// The candidate octaves are the reference's own octave and its neighbours; the
/// pinned source lists them in ascending order and takes the first minimum, so a
/// tie of six semitones deterministically resolves downwards.
fn closest_pitch(note: PitchClass, reference: OpenPitch) -> Option<OpenPitch> {
    let reference = i16::from(u8::from(reference));
    #[allow(clippy::integer_division)]
    let remainder = reference % 12;
    let anchor = reference.saturating_sub(remainder);
    let base = anchor.saturating_add(i16::from(u8::from(note)));
    let candidates = [base.saturating_sub(12), base, base.saturating_add(12)];

    let chosen = candidates
        .into_iter()
        .min_by_key(|candidate| candidate.saturating_sub(reference).unsigned_abs())?;
    let byte = u8::try_from(chosen).ok()?;
    OpenPitch::try_from(byte).ok()
}

/// Resolve note names against the standard pitches of the instrument: every name
/// becomes the pitch of its class nearest that string's standard pitch.
fn anchored_pitches(notes: &[PitchClass], instrument: InstrumentId) -> Vec<OpenPitch> {
    let standard = standard_pitches(instrument).unwrap_or(&[]);
    notes
        .iter()
        .zip(standard.iter())
        .filter_map(|(note, reference)| closest_pitch(*note, *reference))
        .collect()
}

/// Decode the fretted tuning of a page: `pitches` when it is present, the legacy
/// notes otherwise.
fn decode_fretted_tuning(params: &Map<String, Value>, instrument: InstrumentId) -> TuningState {
    let standard = standard_tuning_state(instrument);

    let Some(value) = text(params, "pitches") else {
        let notes = decode_tuning_notes(text(params, "tuning").unwrap_or_default(), instrument);
        return TuningState {
            pitches: anchored_pitches(&notes, instrument),
            reference: standard.reference,
        };
    };

    let count = instrument_strings(instrument).unwrap_or_default();
    decode_pitches(value, count).map_or(standard, |pitches| TuningState {
        pitches,
        reference: decode_reference(params, instrument),
    })
}

/// Decode the `reference` parameter: a preset name of this instrument, or the
/// standard reference.
fn decode_reference(params: &Map<String, Value>, instrument: InstrumentId) -> PresetName {
    let standard = PresetName::from_catalog(STANDARD);
    text(params, "reference")
        .and_then(|value| PresetName::parse(value).ok())
        .filter(|name| preset_pitches(instrument, *name).is_some())
        .unwrap_or(standard)
}

/// Decode the `marked` parameter into an ascending selection.
///
/// A pair whose string exists is recorded even when its fret does not, so a later
/// invalid pair erases an earlier valid one for the same string; the frets are
/// filtered only afterwards, like the pinned source's map-then-filter.
fn decode_marked(value: Option<&str>, strings: u8, frets: u8) -> Vec<Position> {
    let mut recorded: BTreeMap<i64, i64> = BTreeMap::new();

    if let Some(parameter) = value {
        for token in tokens(parameter) {
            let Some((string, fret)) = token.split_once('-') else {
                continue;
            };
            let (Some(string), Some(fret)) = (integer_token(string), integer_token(fret)) else {
                continue;
            };
            if (0..i64::from(strings)).contains(&string) {
                recorded.insert(string, fret);
            }
        }
    }

    recorded
        .into_iter()
        .filter(|(_, fret)| (0..=i64::from(frets)).contains(fret))
        .filter_map(|(string, fret)| {
            Some(Position {
                string: StringIndex::try_from(u8::try_from(string).ok()?).ok()?,
                fret: Fret::try_from(u8::try_from(fret).ok()?).ok()?,
            })
        })
        .collect()
}

/// Encode a selection as `string-fret` pairs, ascending by string.
fn encode_marked(selected: &[Position]) -> String {
    let mut pairs = selected
        .iter()
        .map(|position| (u8::from(position.string), u8::from(position.fret)))
        .collect::<Vec<(u8, u8)>>();
    pairs.sort_unstable();

    pairs
        .iter()
        .map(|(string, fret)| format!("{string}-{fret}"))
        .collect::<Vec<String>>()
        .join(",")
}

/// Decode the piano's `keys`: the absolute pitches inside the keyboard range,
/// deduplicated and ascending.
fn decode_keys(value: Option<&str>) -> Vec<OpenPitch> {
    let (low, high) = keyboard_pitch_range();
    let range = u8::from(low)..=u8::from(high);
    let mut keys = BTreeSet::new();

    if let Some(parameter) = value {
        for token in tokens(parameter) {
            if let Some(number) = integer_token(token)
                && let Ok(byte) = u8::try_from(number)
                && range.contains(&byte)
            {
                keys.insert(byte);
            }
        }
    }

    keys.into_iter()
        .filter_map(|byte| OpenPitch::try_from(byte).ok())
        .collect()
}
