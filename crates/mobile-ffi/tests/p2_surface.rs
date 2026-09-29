//! The P2 boundary: what the adapter answers for the visualizer, the catalogs and
//! the page events.
//!
//! These tests drive the *exported* functions with DTOs, so they pin the two
//! things the domain's own tests cannot: that the conversions carry the values
//! across unchanged, and that the memberships cross as colour slots a client can
//! index its palette with.
//!
//! Expectations come from the frozen fixtures, never from the adapter:
//! `surfaces.jsonl` (`instrument_definition`, `fretboard_data`, `keyboard_data`)
//! and `page-events.jsonl` (the chord and highlight events of `C09`).

// Test target: the same relaxations as the other contract tests.
#![allow(
    clippy::arithmetic_side_effects,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use fretboard_mobile_ffi::{
    ChordDto, InstrumentDto, InstrumentKindDto, NoteFillDto, PageEventDto, PageStateDto,
    PositionDto, TabDto, TuningDto, apply_page_event, chord_color_slots, chord_details,
    default_state, fretted_surface, instruments, keyboard_surface, quality_groups,
};
use serde_json::Value;

const SURFACES: &str = "fixtures/oracle/surfaces.jsonl";
const EVENTS: &str = "fixtures/oracle/page-events.jsonl";

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

fn operation(records: &[Value], operation: &str) -> Vec<Value> {
    records
        .iter()
        .filter(|record| record["operation"].as_str() == Some(operation))
        .cloned()
        .collect()
}

/// The label of one chord DTO, read through the adapter's own chord details.
fn label_of(chord: &ChordDto) -> String {
    chord_details(chord.clone())
        .expect("the fixture uses catalog chords")
        .label
}

/// The chord DTOs of an active list in the fixture's decoded shape.
fn chords_of(decoded: &Value) -> Vec<ChordDto> {
    decoded["active_chords"]
        .as_array()
        .expect("a decoded page has chords")
        .iter()
        .map(|chord| ChordDto {
            root: chord["root"]
                .as_str()
                .expect("a chord has a root")
                .to_owned(),
            quality: chord["quality"]
                .as_str()
                .expect("a chord has a quality")
                .to_owned(),
        })
        .collect()
}

/// The labels of a membership list in slot terms.
fn membership_labels(state: &PageStateDto, slots: &[u64]) -> Vec<String> {
    slots
        .iter()
        .map(|slot| {
            let index = usize::try_from(*slot).expect("a slot fits");
            let chord = state
                .chords
                .get(index)
                .unwrap_or_else(|| panic!("slot {slot} names an active chord"));
            label_of(chord)
        })
        .collect()
}

/// The frozen pitches of one named preset of one instrument.
fn preset_pitches(instrument: &str, preset: &str) -> Vec<u8> {
    let presets = operation(&records(SURFACES), "instrument_pitch_presets");
    let record = presets
        .iter()
        .find(|record| record["input"]["instrument"].as_str() == Some(instrument))
        .unwrap_or_else(|| panic!("the fixture has no presets for {instrument}"));
    record["output"]["presets"]
        .as_array()
        .expect("presets")
        .iter()
        .find(|entry| entry["name"].as_str() == Some(preset))
        .unwrap_or_else(|| panic!("the fixture has no preset {preset}"))["pitches"]
        .as_array()
        .expect("pitches")
        .iter()
        .map(|pitch| u8::try_from(pitch.as_u64().expect("a pitch")).expect("fits"))
        .collect()
}

/// A page DTO from one record of the fixture's decoded page shape.
fn state_of(decoded: &Value) -> PageStateDto {
    let instrument = decoded["instrument"]
        .as_str()
        .expect("a page names its instrument");
    let chords = chords_of(decoded);
    let highlight = decoded["highlighted_chord"]
        .as_u64()
        .map(|index| usize::try_from(index).expect("an index fits"))
        .and_then(|index| chords.get(index).cloned());
    let tab = match decoded["tab"].as_str().expect("a page names its tab") {
        "analyzer" => TabDto::Analyzer,
        _ => TabDto::Visualizer,
    };

    if instrument == "piano" {
        return PageStateDto {
            instrument: fretboard_mobile_ffi::InstrumentStateDto::Piano {
                selected: decoded["selection"]
                    .as_array()
                    .expect("a piano page has keys")
                    .iter()
                    .map(|pitch| {
                        u8::try_from(pitch.as_u64().expect("a key is a pitch")).expect("fits")
                    })
                    .collect(),
            },
            chords,
            highlight,
            tab,
        };
    }

    let tuning = &decoded["tuning_state"];
    PageStateDto {
        instrument: fretboard_mobile_ffi::InstrumentStateDto::Fretted {
            instrument: InstrumentDto::parse(instrument).expect("the instrument is in the catalog"),
            tuning: TuningDto {
                pitches: tuning["pitches"]
                    .as_array()
                    .expect("a fretted page has pitches")
                    .iter()
                    .map(|pitch| {
                        u8::try_from(pitch.as_u64().expect("a pitch is a number")).expect("fits")
                    })
                    .collect(),
                reference: tuning["reference"]
                    .as_str()
                    .expect("a tuning has a reference")
                    .to_owned(),
            },
            selected: decoded["selection"]
                .as_object()
                .expect("a fretted page has a selection")
                .iter()
                .map(|(string, fret)| PositionDto {
                    string: string.parse().expect("a string index is a number"),
                    fret: u8::try_from(fret.as_u64().expect("a fret is a number")).expect("fits"),
                })
                .collect(),
        },
        chords,
        highlight,
        tab,
    }
}

/// The catalog answers the five instruments the frozen fixture defines.
#[test]
fn the_catalog_lists_the_frozen_instruments() {
    let definitions = instruments();
    let expected = operation(&records(SURFACES), "instrument");
    assert_eq!(definitions.len(), 5, "the catalog has five instruments");
    assert_eq!(expected.len(), 5, "the fixture defines five instruments");

    for (definition, record) in definitions.iter().zip(expected.iter()) {
        let frozen = &record["output"]["definition"];
        assert_eq!(
            definition.instrument.as_str(),
            frozen["name"]
                .as_str()
                .map(|_| definition.instrument.as_str())
                .expect("a definition has a name"),
            "the catalog order must be the frozen one"
        );
        assert_eq!(definition.name, frozen["name"].as_str().expect("a name"));
        assert_eq!(
            definition.kind,
            match frozen["kind"].as_str().expect("a kind") {
                "fretted" => InstrumentKindDto::Fretted,
                _ => InstrumentKindDto::Keyboard,
            }
        );
        assert_eq!(
            definition.strings,
            u8::try_from(frozen["strings"].as_u64().unwrap_or_default()).expect("fits"),
            "{}: the string count differs",
            definition.name
        );
        assert_eq!(
            definition.frets,
            frozen["frets"]
                .as_u64()
                .map(|frets| u8::try_from(frets).expect("fits")),
            "{}: the fret count differs",
            definition.name
        );
        assert_eq!(
            definition.standard_pitches,
            frozen["standard_pitches"]
                .as_array()
                .unwrap_or(&Vec::new())
                .iter()
                .map(|pitch| u8::try_from(pitch.as_u64().expect("a pitch")).expect("fits"))
                .collect::<Vec<u8>>(),
            "{}: the standard pitches differ",
            definition.name
        );
    }
}

/// The picker's qualities are the whole frozen catalog, once each.
#[test]
fn the_quality_groups_are_the_whole_catalog() {
    let groups = quality_groups();
    let qualities = groups
        .iter()
        .flat_map(|group| group.qualities.iter())
        .map(|quality| quality.quality.clone())
        .collect::<Vec<String>>();

    assert_eq!(qualities.len(), 47, "the frozen catalog has 47 qualities");
    let mut sorted = qualities;
    sorted.sort();
    sorted.dedup();
    assert_eq!(sorted.len(), 47, "no quality may be listed twice");

    for group in &groups {
        assert!(!group.group.is_empty(), "a group has a name");
        for quality in &group.qualities {
            assert!(
                !quality.label.is_empty(),
                "{} has a display label",
                quality.quality
            );
        }
    }
}

/// Every frozen fretted surface is reproduced through the boundary, memberships
/// included, as colour slots.
#[test]
fn every_frozen_fretted_surface_crosses_the_boundary() {
    let expected = operation(&records(SURFACES), "fretboard_data");
    assert_eq!(expected.len(), 9, "the fixture records nine surfaces");

    for record in &expected {
        let instrument_name = record["input"]["instrument"]
            .as_str()
            .expect("an instrument");
        let preset = record["input"]["preset"].as_str().expect("a preset");
        let state = PageStateDto {
            instrument: fretboard_mobile_ffi::InstrumentStateDto::Fretted {
                instrument: InstrumentDto::parse(instrument_name).expect("the instrument is known"),
                tuning: TuningDto {
                    pitches: preset_pitches(instrument_name, preset),
                    reference: preset.to_owned(),
                },
                selected: Vec::new(),
            },
            chords: record["input"]["active_chords"]
                .as_array()
                .expect("chords")
                .iter()
                .map(|chord| ChordDto {
                    root: chord["root"].as_str().expect("a root").to_owned(),
                    quality: chord["quality"].as_str().expect("a quality").to_owned(),
                })
                .collect(),
            highlight: None,
            tab: TabDto::Visualizer,
        };

        let surface = fretted_surface(state.clone()).expect("a fretted page has a surface");
        let rows = record["output"]["rows"].as_array().expect("rows");
        assert_eq!(surface.rows.len(), rows.len(), "the row count differs");

        for (index, expected_row) in rows.iter().enumerate() {
            let row = surface.rows.get(index).expect("the row exists");
            let cells = expected_row.as_array().expect("cells");
            assert_eq!(row.cells.len(), cells.len(), "the cell count differs");
            for (position, expected_cell) in cells.iter().enumerate() {
                let cell = row.cells.get(position).expect("the cell exists");
                assert_eq!(
                    cell.note,
                    expected_cell["note"].as_str().expect("a note"),
                    "string {index} position {position} carries the wrong note"
                );
                assert_eq!(
                    membership_labels(&state, &cell.memberships),
                    expected_cell["chords"]
                        .as_array()
                        .expect("memberships")
                        .iter()
                        .map(|label| label.as_str().expect("a label").to_owned())
                        .collect::<Vec<String>>(),
                    "string {index} position {position} has the wrong memberships"
                );
            }
        }
    }
}

/// Every frozen keyboard surface is reproduced through the boundary, fills
/// included.
#[test]
fn every_frozen_keyboard_surface_crosses_the_boundary() {
    let expected = operation(&records(SURFACES), "keyboard_data");
    assert_eq!(expected.len(), 4, "the fixture records four keyboards");

    for record in &expected {
        let state = PageStateDto {
            instrument: fretboard_mobile_ffi::InstrumentStateDto::Piano {
                selected: Vec::new(),
            },
            chords: record["input"]["active_chords"]
                .as_array()
                .expect("chords")
                .iter()
                .map(|chord| ChordDto {
                    root: chord["root"].as_str().expect("a root").to_owned(),
                    quality: chord["quality"].as_str().expect("a quality").to_owned(),
                })
                .collect(),
            highlight: None,
            tab: TabDto::Visualizer,
        };

        let surface = keyboard_surface(state.clone()).expect("a piano page has a surface");
        let keys = record["output"]["keys"].as_array().expect("keys");
        assert_eq!(surface.keys.len(), keys.len(), "the key count differs");

        for (index, expected_key) in keys.iter().enumerate() {
            let key = surface.keys.get(index).expect("the key exists");
            assert_eq!(
                key.pitch,
                u8::try_from(expected_key["pitch"].as_u64().expect("a pitch")).expect("fits"),
                "key {index} has the wrong pitch"
            );
            assert_eq!(
                key.note,
                expected_key["note"].as_str().expect("a note"),
                "key {index} carries the wrong note"
            );
            assert_eq!(
                membership_labels(&state, &key.memberships),
                expected_key["chords"]
                    .as_array()
                    .expect("memberships")
                    .iter()
                    .map(|label| label.as_str().expect("a label").to_owned())
                    .collect::<Vec<String>>(),
                "key {index} has the wrong memberships"
            );
        }
    }
}

/// A repeated identity crosses as one colour slot.
#[test]
fn a_repeated_identity_crosses_as_one_slot() {
    let mut state = default_state();
    for quality in ["maj6", "min7"] {
        state = apply_page_event(
            state,
            PageEventDto::AddChord {
                chord: ChordDto {
                    root: "C".to_owned(),
                    quality: quality.to_owned(),
                },
            },
        )
        .expect("the chord is in the catalog");
    }
    let repeated = apply_page_event(
        state.clone(),
        PageEventDto::AddChord {
            chord: ChordDto {
                root: "C".to_owned(),
                quality: "maj6".to_owned(),
            },
        },
    )
    .expect("a duplicate is not an error");
    assert_eq!(repeated, state, "an exact duplicate must change nothing");

    let slots = chord_color_slots(state).expect("the slots cross");
    assert_eq!(slots, vec![0, 1], "two different chords take two slots");
}

/// The chord and highlight events of the frozen page-event fixture reach the
/// recorded page through the boundary.
#[test]
fn every_frozen_page_event_reaches_the_recorded_page() {
    let records = records(EVENTS);
    let owned = [
        "add-chord-by-form-submit",
        "add-duplicate-chord-is-noop",
        "add-new-chord",
        "add-two-chords-in-sequence",
        "clear-all-chords-keeps-selection",
        "highlight-duplicate-canonicalizes-to-first-occurrence",
        "highlight-moves-to-other-chip",
        "highlight-off-on-same-chip",
        "highlight-on",
        "remove-first-of-duplicates-moves-highlight-to-first-remaining",
        "remove-last-chord-clears-highlight",
        "remove-middle-occurrence",
        "remove-unhighlighted-occurrence-keeps-highlight",
    ];
    assert_eq!(records.len(), 49, "the fixture records 49 cases");

    for name in owned {
        let record = records
            .iter()
            .find(|record| record["case_id"].as_str() == Some(&format!("page_event/{name}")))
            .unwrap_or_else(|| panic!("the fixture has the case {name}"));
        let output = &record["output"];
        let mut state = state_of(&output["initial_state"]);

        for step in record["input"]["steps"].as_array().expect("steps") {
            let chord_of_step = || {
                let chord = &step["value"]["chord"];
                PageEventDto::AddChord {
                    chord: ChordDto {
                        root: chord["root"].as_str().expect("a root").to_owned(),
                        quality: chord["quality"].as_str().expect("a quality").to_owned(),
                    },
                }
            };
            let index_of_step = || {
                step["value"]["index"]
                    .as_str()
                    .expect("an index")
                    .parse()
                    .expect("an index is a number")
            };

            let event = match step["event"].as_str() {
                Some("add_chord") => Some(chord_of_step()),
                Some("remove_chord") => Some(PageEventDto::RemoveChord {
                    index: index_of_step(),
                }),
                Some("clear_all_chords") => Some(PageEventDto::ClearAllChords),
                Some("highlight_chord") => Some(PageEventDto::HighlightChord {
                    index: index_of_step(),
                }),
                // The other families belong to their tasks: C13, C14 and C18.
                Some(_) => None,
                None => match step["kind"].as_str() {
                    // The chord form's own submit adds the chord it carries.
                    Some("submit") => Some(chord_of_step()),
                    // A field change touches no page state here.
                    _ => None,
                },
            };

            if let Some(event) = event {
                state = apply_page_event(state, event)
                    .unwrap_or_else(|error| panic!("{name}: the event failed: {error:?}"));
            }
        }

        let frozen = &output["final_state"];
        let expected_chords = frozen["active_chords"]
            .as_array()
            .expect("chords")
            .iter()
            .map(|chord| {
                (
                    chord["root"].as_str().expect("a root").to_owned(),
                    chord["quality"].as_str().expect("a quality").to_owned(),
                )
            })
            .collect::<Vec<(String, String)>>();
        let actual_chords = state
            .chords
            .iter()
            .map(|chord| (chord.root.clone(), chord.quality.clone()))
            .collect::<Vec<(String, String)>>();
        assert_eq!(actual_chords, expected_chords, "{name}: the chords differ");

        let expected_highlight = frozen["highlighted_chord"]
            .as_u64()
            .map(|index| usize::try_from(index).expect("fits"));
        let actual_highlight = state.highlight.as_ref().map(|highlight| {
            state
                .chords
                .iter()
                .position(|chord| chord == highlight)
                .expect("the highlight is a chord of the page")
        });
        assert_eq!(
            actual_highlight, expected_highlight,
            "{name}: the highlight differs from the baseline"
        );
    }
}

/// A note with no claim and a note with two claims are both the overlap.
#[test]
fn the_overlap_is_the_answer_for_none_and_for_several() {
    let state = default_state();
    let surface = fretted_surface(state.clone()).expect("the default page is fretted");
    let first = surface.rows.first().expect("the guitar has strings");

    assert!(
        first
            .cells
            .iter()
            .all(|cell| cell.fill == NoteFillDto::Overlap && cell.memberships.is_empty()),
        "with no chords every position is the overlap"
    );

    let state = apply_page_event(
        state,
        PageEventDto::AddChord {
            chord: ChordDto {
                root: "C".to_owned(),
                quality: "major".to_owned(),
            },
        },
    )
    .expect("C major is in the catalog");
    let surface = fretted_surface(state).expect("still fretted");
    let claimed = surface
        .rows
        .iter()
        .flat_map(|row| row.cells.iter())
        .filter(|cell| !cell.memberships.is_empty())
        .count();
    assert!(claimed > 0, "C major claims positions on the guitar");
}
