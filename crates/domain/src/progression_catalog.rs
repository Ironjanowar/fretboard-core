//! The frozen progression catalog: the fifty-nine definitions of the pinned
//! `lib/fretboard/music/progression.ex` `@progressions`, with their identifiers,
//! display names, categories, genres, descriptions, example keys, scale types,
//! degree specifications and notable songs.
//!
//! This file belongs to task `C18`. The definitions are the values the frozen
//! export carries (`fixtures/oracle/catalogs.json` `progression_definitions`, in
//! the source's own order, and the fifty-nine `progression/1` records of
//! `fixtures/oracle/progressions.jsonl`), and `tests/progressions.rs` pins every
//! field of every entry against those two exports. Nothing here is retyped from
//! the plan.
//!
//! `Contract.D07` is the approved decision behind this port: label and data are
//! ported **verbatim**, so the contradictions the decision measured — six labels
//! that contradict their own degrees, seven that are names rather than chord
//! lists, seven that claim a mode the data has not — are inherited deliberately.
//! A test that repaired one of them would be the defect.
//!
//! Nothing here resolves a degree into a chord: [`crate::progression_chords`]
//! does that with the scale's own diatonic chords.

use crate::chord::str_eq;
use crate::types::{ProgressionId, QualityId, ScaleId};

/// One degree specification of a progression definition.
///
/// `degree` is the scale degree (1-7), `accidental` the semitone offset from the
/// diatonic root of that degree (-1 flattens, +1 sharpens, 0 is the diatonic
/// note) and `quality` an explicit chord quality that overrides the diatonic one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Degree {
    /// The scale degree, 1-7.
    pub degree: u8,
    /// The semitone offset from the degree's diatonic root, in -2..=2.
    pub accidental: i8,
    /// The explicit chord quality, or `None` for the scale's own.
    pub quality: Option<QualityId>,
}

/// One progression of the frozen catalog.
///
/// Every field is the pinned source's own value. `example_key` stays the text the
/// catalog stores (the one flat name included), because it is a catalog value and
/// not a wire value: a client that needs the key itself parses it through the
/// domain's note lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progression {
    /// The stable catalog identifier.
    pub id: ProgressionId,
    /// The display name (also the label of `progression_label/1`).
    pub name: &'static str,
    /// The display category, one of [`ProgressionGroup::category`].
    pub category: &'static str,
    /// The genre note.
    pub genre: &'static str,
    /// The description note.
    pub description: &'static str,
    /// The example key the catalog stores for this progression.
    pub example_key: &'static str,
    /// The scale type the degrees are read in.
    pub scale_type: ScaleId,
    /// The degree specifications, in playing order.
    pub degrees: &'static [Degree],
    /// The notable songs note.
    pub notable_songs: &'static [&'static str],
}

/// The fifty-nine progressions, in the pinned catalog order.
const CATALOG: [Progression; 59] = [
    Progression {
        id: ProgressionId::from_catalog("pop_i_v_vi_iv"),
        name: "Pop: I-V-vi-IV",
        category: "Famous / Classic",
        genre: "Pop, pop-punk, rock",
        description: "The 'Axis of Awesome' progression — used in countless pop hits",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Don't Stop Believin'",
            "Let It Be",
            "No Woman No Cry",
            "I'm Yours",
            "Despacito",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("classic_i_iv_v"),
        name: "Classic: I-IV-V",
        category: "Famous / Classic",
        genre: "Rock, blues, country, folk",
        description: "The foundational three-chord rock/blues progression",
        example_key: "G",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Wild Thing",
            "La Bamba",
            "Twist and Shout",
            "Rock Around the Clock",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("blues_12_bar"),
        name: "Blues: 12-Bar (I-IV-V)",
        category: "Famous / Classic",
        genre: "Blues, rock, jazz",
        description: "The quintessential 12-bar blues form",
        example_key: "A",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Sweet Home Chicago",
            "Pride and Joy",
            "Johnny B. Goode",
            "Red House",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("jazz_ii_v_i"),
        name: "Jazz: ii-V-I",
        category: "Famous / Classic",
        genre: "Jazz",
        description: "The most important progression in jazz; appears in virtually every standard",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &["Autumn Leaves", "All The Things You Are", "Tune Up"],
    },
    Progression {
        id: ProgressionId::from_catalog("fifties_i_vi_iv_v"),
        name: "50s: I-vi-IV-V",
        category: "Famous / Classic",
        genre: "Doo-wop, 1950s pop, early rock & roll",
        description: "The '50s progression' or 'doo-wop progression'",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Earth Angel", "Stand By Me", "Everyday", "Duke of Earl"],
    },
    Progression {
        id: ProgressionId::from_catalog("pachelbel_canon"),
        name: "Classical: Pachelbel (I-V-vi-iii-IV-I-IV-V)",
        category: "Famous / Classic",
        genre: "Classical, baroque, pop",
        description: "Based on Pachelbel's Canon — descending fifth circular motion",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Canon in D", "Basket Case", "Graduation (Friends Forever)"],
    },
    Progression {
        id: ProgressionId::from_catalog("pop_vi_iv_i_v"),
        name: "Pop: vi-IV-I-V",
        category: "Famous / Classic",
        genre: "Pop, adult contemporary",
        description: "Same chords as I-V-vi-IV starting on vi — more melancholic, yearning quality",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Apologize", "Glycerine", "Save Tonight", "Zombie"],
    },
    Progression {
        id: ProgressionId::from_catalog("rock_i_bvii_iv"),
        name: "Rock: I-bVII-IV",
        category: "Famous / Classic",
        genre: "Rock, folk-rock (mixolydian)",
        description: "Uses the flattened 7th degree from mixolydian mode — quintessential rock sound",
        example_key: "D",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: -1,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Sweet Home Alabama",
            "A Hard Day's Night",
            "Sympathy for the Devil",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("folk_i_iv"),
        name: "Folk: I-IV",
        category: "Famous / Classic",
        genre: "Folk, rock, pop, drone",
        description: "Simplest common progression — creates an open, unresolved vamp",
        example_key: "E",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Born in the U.S.A.", "Mellowship Slinky in B Major"],
    },
    Progression {
        id: ProgressionId::from_catalog("minor_pop_i_vi_iii_vii"),
        name: "Pop: i-VI-III-VII",
        category: "Famous / Classic",
        genre: "Pop, EDM, dance",
        description: "Minor-key equivalent of vi-IV-I-V — extremely common in modern pop and EDM",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Wake Me Up",
            "Rolling in the Deep",
            "Don't You Worry Child",
            "Let Her Go",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("minor_pop_i_bvi_biii_bvii"),
        name: "Pop: i-bVI-bIII-bVII",
        category: "Famous / Classic",
        genre: "Pop, rock",
        description: "All-natural-minor diatonic chords — the most common minor-key four-chord loop",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Mr. Brightside", "Disturbia", "Stronger"],
    },
    Progression {
        id: ProgressionId::from_catalog("canon_rock"),
        name: "Rock: V-i-VI-IV (Canon Rock)",
        category: "Famous / Classic",
        genre: "Rock, instrumental guitar (harmonic minor)",
        description: "Fusion of Pachelbel's Canon with rock — uses harmonic minor raised 7th",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Canon Rock", "various Yngwie Malmsteen passages"],
    },
    Progression {
        id: ProgressionId::from_catalog("modal_interchange_i_iv"),
        name: "Modal Interchange: I-iv",
        category: "Curious / Interesting",
        genre: "Pop, rock, classical",
        description: "Borrowing iv from parallel minor — creates emotional, bittersweet color",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("minor")),
            },
        ],
        notable_songs: &["Creep", "In My Life", "Beethoven Sonata Op. 13"],
    },
    Progression {
        id: ProgressionId::from_catalog("creep_progression"),
        name: "Modal Interchange: I-III-IV-iv (Creep)",
        category: "Curious / Interesting",
        genre: "Alternative rock, art rock",
        description: "Chromatic mediant I→III plus modal interchange IV→iv — iconic Radiohead sound",
        example_key: "G",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("minor")),
            },
        ],
        notable_songs: &["Creep", "SexyBack", "Loser"],
    },
    Progression {
        id: ProgressionId::from_catalog("chromatic_mediant_i_biii"),
        name: "Chromatic Mediant: I-bIII",
        category: "Curious / Interesting",
        genre: "Rock, film music, progressive",
        description: "Roots a major third apart — creates a dramatic, cinematic quality",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 3,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &[
            "Where the Streets Have No Name",
            "Strawberry Fields Forever",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("chromatic_mediant_i_bvi"),
        name: "Chromatic Mediant: I-bVI",
        category: "Curious / Interesting",
        genre: "Rock, film music, classical",
        description: "bVI borrowed from parallel minor — creates a broad, heroic, expansive quality",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &["Yesterday", "Dream On", "various film scores"],
    },
    Progression {
        id: ProgressionId::from_catalog("chromatic_mediant_i_iii"),
        name: "Chromatic Mediant: I-III",
        category: "Curious / Interesting",
        genre: "Jazz, classical, Broadway",
        description: "Roots a major third apart, both major — bright, unexpected harmonic lift",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &["Have You Met Miss Jones?"],
    },
    Progression {
        id: ProgressionId::from_catalog("neapolitan_i_bii_v_i"),
        name: "Classical: Neapolitan (i-bII-V-i)",
        category: "Curious / Interesting",
        genre: "Classical, jazz",
        description: "The Neapolitan chord (bII major) — dramatic pre-dominant function; related to tritone substitution",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Beethoven Piano Sonatas", "Schubert songs"],
    },
    Progression {
        id: ProgressionId::from_catalog("descending_chromatic_bass"),
        name: "Chromatic: Descending Bass (I-i7-IV-iv6-I)",
        category: "Curious / Interesting",
        genre: "Pop, jazz, classical",
        description: "Chromatic descending bass line — sophisticated and emotive; common in jazz ballads",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Stairway to Heaven", "My Funny Valentine", "Chelsea Bridge"],
    },
    Progression {
        id: ProgressionId::from_catalog("line_cliche_i_bvii_bvi_v"),
        name: "Chromatic: Line Cliche (i-bVII-bVI-V)",
        category: "Curious / Interesting",
        genre: "Jazz, pop, film",
        description: "Chromatic descending bass — the 'James Bond' chord progression; cinematic and mysterious",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &["My Favorite Things", "Europa", "various James Bond themes"],
    },
    Progression {
        id: ProgressionId::from_catalog("omnipotent_progression"),
        name: "Classical: Omnipotent (I-VII-iv-iv°-III-II-I)",
        category: "Curious / Interesting",
        genre: "Classical, Baroque",
        description: "Baroque descending bass line with passing diminished chord — foundational in classical music",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("minor")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("dim")),
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Bach chorales", "Stairway to Heaven (partial descent)"],
    },
    Progression {
        id: ProgressionId::from_catalog("ascending_bass_i_ii_iii_iv"),
        name: "Pop: Ascending (I-ii-iii-IV)",
        category: "Curious / Interesting",
        genre: "Pop, jazz",
        description: "Ascending stepwise motion through diatonic chords — gentle, building, optimistic",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Lean on Me (partial)", "various jazz ballad intros"],
    },
    Progression {
        id: ProgressionId::from_catalog("chromatic_walkdown_i_bvii_vi_bvii_i"),
        name: "Rock: Chromatic Walkdown (I-bVII-VI-bVII-I)",
        category: "Curious / Interesting",
        genre: "Rock, pop",
        description: "Stepwise descent from I to bVI with return through bVII — dramatic harmonic gesture",
        example_key: "A",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: -1,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: -1,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Hey Jude (outro section)", "various rock ballads"],
    },
    Progression {
        id: ProgressionId::from_catalog("andalusian_cadence"),
        name: "Flamenco: Andalusian Cadence (i-bVII-bVI-V)",
        category: "Exotic / World",
        genre: "Flamenco, Spanish, classical, rock",
        description: "The most famous flamenco progression — descending bass from i to V; the raised 7th provides Spanish tension",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &[
            "Hit the Road Jack",
            "California Dreamin'",
            "Stray Cat Strut",
            "Sultans of Swing",
            "Runaway",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("flamenco_phrygian_dominant"),
        name: "Flamenco: Phrygian Dominant (I-bII-bIII-bII)",
        category: "Exotic / World",
        genre: "Flamenco, Middle Eastern",
        description: "Phrygian dominant scale (5th mode of harmonic minor) — the tonic is major and the bII creates the characteristic flamenco bite",
        example_key: "E",
        scale_type: ScaleId::from_catalog("phrygian_dominant"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 3,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &[
            "various flamenco palos",
            "Middle Eastern-influenced rock/metal",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("harmonic_minor_i_iv_v"),
        name: "Harmonic Minor: i-iv-V",
        category: "Exotic / World",
        genre: "Classical, Eastern European, metal",
        description: "Uses raised 7th (V instead of bVII) from harmonic minor for stronger dominant-tonal resolution",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("minor")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &["various classical pieces", "klezmer", "neo-classical metal"],
    },
    Progression {
        id: ProgressionId::from_catalog("byzantine_double_harmonic"),
        name: "Exotic: Byzantine / Double Harmonic (I-bII-I)",
        category: "Exotic / World",
        genre: "Byzantine, Greek, Middle Eastern, Indian",
        description: "Based on the double harmonic scale with augmented 2nd intervals — the I-bII is the signature sound",
        example_key: "D",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Miserlou",
            "Greek and Middle Eastern traditional music",
            "Bollywood",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("hungarian_minor"),
        name: "Exotic: Hungarian Minor (i-bII-iv)",
        category: "Exotic / World",
        genre: "Hungarian, Eastern European, klezmer, gypsy jazz",
        description: "Hungarian minor scale (harmonic minor with raised 4th) — distinctive Eastern European / gypsy flavor",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("minor")),
            },
        ],
        notable_songs: &[
            "Traditional Hungarian and Roma music",
            "Brahms Hungarian Dances",
            "Django Reinhardt gypsy jazz",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("japanese_hirajoshi"),
        name: "World: Japanese / Hirajoshi (I-bII-V-bVI)",
        category: "Exotic / World",
        genre: "Japanese traditional, ambient, world fusion",
        description: "Based on hirajoshi pentatonic scale — distinctly Japanese harmonic colors; bII and bVI give Asian-influenced sound",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: -1,
                quality: None,
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: -1,
                quality: None,
            },
        ],
        notable_songs: &[
            "Traditional Japanese koto/shamisen music",
            "Joe Hisaishi (partial influence)",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("middle_eastern_hijaz"),
        name: "World: Hijaz / Makam (I-bII-bIII-iv)",
        category: "Exotic / World",
        genre: "Middle Eastern, Arabic, Turkish makam",
        description: "The Hijaz mode/makam (phrygian dominant) — the tonic is major and the augmented 2nd between bII and bIII is the hallmark of Middle Eastern music",
        example_key: "D",
        scale_type: ScaleId::from_catalog("phrygian_dominant"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 3,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("minor")),
            },
        ],
        notable_songs: &[
            "Traditional Arabic/Turkish music",
            "Misirlou",
            "various Middle Eastern pop",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("klezmer_freygish"),
        name: "World: Klezmer / Freygish (I-bII-III-VII)",
        category: "Exotic / World",
        genre: "Klezmer, Jewish, Eastern European",
        description: "'Freygish' = Yiddish for phrygian dominant — the tonic is major and the I-bII-III movement is the core of klezmer harmony",
        example_key: "D",
        scale_type: ScaleId::from_catalog("phrygian_dominant"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 3,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 7,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &[
            "Hava Nagila (partial)",
            "Bei Mir Bist Du Schön",
            "various Eastern European folk",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("dorian_vamp_i_iv"),
        name: "Modal: Dorian Vamp (i-IV)",
        category: "Exotic / World",
        genre: "Jazz, rock, folk, funk (dorian mode)",
        description: "The raised 6th in dorian gives a brighter quality than natural minor — common in modal jazz",
        example_key: "D",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &[
            "So What",
            "Oye Como Va",
            "Eleanor Rigby",
            "Scarborough Fair",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("dorian_aeolian_i_bvii_iv"),
        name: "Modal: Dorian-Aeolian (i-bVII-IV)",
        category: "Exotic / World",
        genre: "Rock, funk, soul",
        description: "Combines dorian brightness with aeolian — the IV chord is the key dorian characteristic",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &["Roxanne", "Billie Jean (partial)", "Thriller (partial)"],
    },
    Progression {
        id: ProgressionId::from_catalog("lydian_i_ii"),
        name: "Modal: Lydian (I-II)",
        category: "Exotic / World",
        genre: "Jazz, film, progressive rock (lydian mode)",
        description: "The II chord (major, not diminished) comes from the lydian raised 4th — floating, ethereal, dreamlike",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &["Dreams", "Flying in a Blue Dream", "various film scores"],
    },
    Progression {
        id: ProgressionId::from_catalog("whole_tone"),
        name: "Modal: Whole Tone (I-II-III)",
        category: "Exotic / World",
        genre: "Jazz, impressionist, film",
        description: "Based on the whole tone scale — augmented chords create a floating, ambiguous, otherworldly quality",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("aug")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("aug")),
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: Some(QualityId::from_catalog("aug")),
            },
        ],
        notable_songs: &[
            "Debussy impressionist pieces",
            "Bemsha Swing (partial)",
            "various film dream sequences",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("mixolydian_bvi_i_bvii_bvi_bvii"),
        name: "Modal: Mixolydian bVI (I-bVII-bVI-bVII)",
        category: "Exotic / World",
        genre: "Rock, pop (mixolydian with modal interchange)",
        description: "Combines mixolydian bVII with modal interchange bVI — rock anthem quality with dramatic lift",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: -1,
                quality: None,
            },
            Degree {
                degree: 6,
                accidental: -1,
                quality: None,
            },
            Degree {
                degree: 7,
                accidental: -1,
                quality: None,
            },
        ],
        notable_songs: &["Hey Jude (partial)", "various rock anthems"],
    },
    Progression {
        id: ProgressionId::from_catalog("phrygian_vamp_i_bii_i"),
        name: "Modal: Phrygian (i-bII-i)",
        category: "Exotic / World",
        genre: "Flamenco, metal, progressive rock (phrygian mode)",
        description: "The simplest phrygian vamp — the bII major chord creates the characteristic dark, tense phrygian sound",
        example_key: "E",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Between the Wheels",
            "various metal and prog rock",
            "traditional flamenco",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("spanish_phrygian_i_bii_iii"),
        name: "Modal: Spanish Phrygian (i-bII-III)",
        category: "Exotic / World",
        genre: "Flamenco, Spanish",
        description: "Adds the III chord to the phrygian vamp — common in Spanish guitar music",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 2,
                accidental: -1,
                quality: Some(QualityId::from_catalog("major")),
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: Some(QualityId::from_catalog("major")),
            },
        ],
        notable_songs: &["various flamenco forms", "Malagueña"],
    },
    Progression {
        id: ProgressionId::from_catalog("blues_dominant_i7_iv7_v7"),
        name: "Blues: Dominant 7th (I7-IV7-V7)",
        category: "Exotic / World",
        genre: "Blues, rock",
        description: "All dominant 7th chords — the defining blues characteristic (dominant 7th on I doesn't fit diatonic major)",
        example_key: "A",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &["Pride and Joy", "Red House", "Sweet Home Chicago"],
    },
    Progression {
        id: ProgressionId::from_catalog("minor_blues_i7_iv7_v7"),
        name: "Blues: Minor Blues (i7-iv7-V7)",
        category: "Exotic / World",
        genre: "Blues, jazz",
        description: "Minor-key blues — the V7 uses the harmonic minor raised 7th for stronger resolution",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &["The Thrill is Gone", "As The Years Go Passing By"],
    },
    Progression {
        id: ProgressionId::from_catalog("rhythm_changes_a"),
        name: "Jazz: Rhythm Changes A (I-vi-ii-V)",
        category: "Jazz / Sophisticated",
        genre: "Jazz, bebop",
        description: "The A section of rhythm changes — one of the two most important progressions in jazz (with blues)",
        example_key: "Bb",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &[
            "I Got Rhythm",
            "Oleo",
            "Anthropology",
            "The Flintstones Theme",
            "Rhythm-a-Ning",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("rhythm_changes_b"),
        name: "Jazz: Rhythm Changes B (III7-VI7-II7-V7)",
        category: "Jazz / Sophisticated",
        genre: "Jazz, bebop",
        description: "The B section (bridge) of rhythm changes — circle of fifths through secondary dominants",
        example_key: "Bb",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 3,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &["I Got Rhythm bridge", "Oleo bridge", "Anthropology bridge"],
    },
    Progression {
        id: ProgressionId::from_catalog("coltrane_changes"),
        name: "Jazz: Coltrane Changes (Giant Steps)",
        category: "Jazz / Sophisticated",
        genre: "Jazz, post-bop",
        description: "Root movement by major thirds — three key centers an augmented triad apart; Coltrane's harmonic innovation",
        example_key: "Bb",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 5,
                accidental: -1,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 3,
                accidental: 1,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &["Giant Steps", "Countdown", "Lazy Bird", "Satellite"],
    },
    Progression {
        id: ProgressionId::from_catalog("coltrane_sub_ii_v_i"),
        name: "Jazz: Coltrane Sub (ii-V-I with major third substitution)",
        category: "Jazz / Sophisticated",
        genre: "Jazz",
        description: "Inserts a ii-V-I a major third away before resolving — creates rapid harmonic motion through distant keys",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &["Countdown", "Tune Up reharmonization"],
    },
    Progression {
        id: ProgressionId::from_catalog("backdoor_progression"),
        name: "Jazz: Backdoor (iv-bVII7-I)",
        category: "Jazz / Sophisticated",
        genre: "Jazz, soul",
        description: "Resolves to I via the 'back door' using iv and its dominant bVII7 — very common in modern jazz",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 7,
                accidental: -1,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &["Lady Bird", "various jazz standards"],
    },
    Progression {
        id: ProgressionId::from_catalog("jazz_turnaround"),
        name: "Jazz: Turnaround (I-vi-ii-V)",
        category: "Jazz / Sophisticated",
        genre: "Jazz, pop",
        description: "The standard jazz turnaround — leads back to the top of the form; supports many substitutions",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &[
            "end of nearly every jazz standard",
            "Rhythm Changes A section",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("minor_ii_v_i"),
        name: "Jazz: Minor ii-V-i",
        category: "Jazz / Sophisticated",
        genre: "Jazz",
        description: "The minor key version of ii-V-I — ii is half-diminished (m7b5), V7 uses harmonic minor raised 7th",
        example_key: "A",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("m7b5")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
        ],
        notable_songs: &[
            "Autumn Leaves (sections)",
            "Black Orpheus",
            "Blue Bossa (sections)",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("tritone_sub_ii_bii_i"),
        name: "Jazz: Tritone Substitution (ii-bII7-I)",
        category: "Jazz / Sophisticated",
        genre: "Jazz",
        description: "Replaces V7 with bII7 — shares the same tritone, creating a chromatic bass descent; essential jazz substitution",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: -1,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &["Misty", "Round Midnight", "most bebop standards"],
    },
    Progression {
        id: ProgressionId::from_catalog("secondary_dominant"),
        name: "Jazz: Secondary Dominant (I-V/ii-ii-V-I)",
        category: "Jazz / Sophisticated",
        genre: "Jazz, Broadway",
        description: "Uses a secondary dominant (V/ii) to approach ii chromatically — creates forward harmonic motion",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 5,
                accidental: 1,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &["I've Got Rhythm", "various Broadway tunes"],
    },
    Progression {
        id: ProgressionId::from_catalog("bird_blues"),
        name: "Jazz: Bird Blues",
        category: "Jazz / Sophisticated",
        genre: "Jazz, bebop",
        description: "Charlie Parker's reharmonization of the 12-bar blues with ii-V chains and substitutions",
        example_key: "Bb",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &["Blues for Alice", "An Oscar for Oscar"],
    },
    Progression {
        id: ProgressionId::from_catalog("modal_jazz_vamp"),
        name: "Jazz: Modal Vamp (i-iv)",
        category: "Jazz / Sophisticated",
        genre: "Modal jazz (dorian)",
        description: "Modal jazz uses very few chords — often just a vamp, allowing extended improvisation on a single mode",
        example_key: "D",
        scale_type: ScaleId::from_catalog("minor"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &["So What", "Impressions", "Maiden Voyage"],
    },
    Progression {
        id: ProgressionId::from_catalog("extended_ii_v_chain"),
        name: "Jazz: Extended Chain (iii-vi-ii-V-I)",
        category: "Jazz / Sophisticated",
        genre: "Jazz",
        description: "Extended circle-of-fifths chain — each chord resolves down a fifth to the next; smooth continuous motion",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 3,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &["many jazz standards, especially intros and turnarounds"],
    },
    Progression {
        id: ProgressionId::from_catalog("iv_minor_substitution"),
        name: "Jazz: IV-iv-I (Minor Subdominant)",
        category: "Jazz / Sophisticated",
        genre: "Jazz, pop, soul",
        description: "The IV to iv movement — one of the most expressive modal interchange devices; also called 'minor plagal cadence'",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &["Night and Day", "Charleston", "many jazz standards"],
    },
    Progression {
        id: ProgressionId::from_catalog("plagal_cadence"),
        name: "Classical: Plagal (IV-I)",
        category: "Jazz / Sophisticated",
        genre: "Classical, hymns, rock",
        description: "The 'Amen' cadence — less final than V-I but with a warm, resolved quality",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 4,
                accidental: 0,
                quality: None,
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Amen cadence in hymns",
            "Hey Jude (ending)",
            "many rock endings",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("deceptive_cadence"),
        name: "Classical: Deceptive (V-vi)",
        category: "Jazz / Sophisticated",
        genre: "Classical, pop, jazz",
        description: "The V resolves deceptively to vi instead of I — creates surprise and extends the phrase",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("minor")),
            },
        ],
        notable_songs: &[
            "Beethoven symphonies",
            "various classical pieces",
            "pop songs for unexpected resolution",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("minor_plagal"),
        name: "Classical: Minor Plagal (iv-I)",
        category: "Jazz / Sophisticated",
        genre: "Pop, jazz, soul",
        description: "The minor subdominant resolving to major tonic — bittersweet, nostalgic quality; very expressive modal interchange",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("minor")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: None,
            },
        ],
        notable_songs: &[
            "Creep (ending)",
            "various soul and R&B songs",
            "Beatles songs",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("jazz_blues_form"),
        name: "Jazz: Jazz Blues",
        category: "Jazz / Sophisticated",
        genre: "Jazz, blues",
        description: "The jazz blues adds ii-V chains and turnarounds to the basic blues form",
        example_key: "Bb",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 4,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &[
            "Tenor Madness",
            "Blue Monk",
            "Freddie Freeloader",
            "Straight, No Chaser",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("bossa_nova_ii_v_i"),
        name: "Jazz: Bossa Nova (ii-V-I)",
        category: "Jazz / Sophisticated",
        genre: "Bossa nova, Brazilian jazz",
        description: "Brazilian jazz typically uses ii-V-I with extended chord voicings (9ths, 11ths, 13ths)",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
        ],
        notable_songs: &[
            "The Girl from Ipanema",
            "Desafinado",
            "Blue Bossa",
            "Corcovado",
        ],
    },
    Progression {
        id: ProgressionId::from_catalog("aaba_form"),
        name: "Jazz: AABA Form",
        category: "Jazz / Sophisticated",
        genre: "Jazz, Broadway, Tin Pan Alley",
        description: "The most important song form in jazz besides blues — 32 bars in AABA structure",
        example_key: "C",
        scale_type: ScaleId::from_catalog("major"),
        degrees: &[
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 3,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
            Degree {
                degree: 1,
                accidental: 0,
                quality: Some(QualityId::from_catalog("maj7")),
            },
            Degree {
                degree: 6,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 2,
                accidental: 0,
                quality: Some(QualityId::from_catalog("min7")),
            },
            Degree {
                degree: 5,
                accidental: 0,
                quality: Some(QualityId::from_catalog("7")),
            },
        ],
        notable_songs: &["I Got Rhythm", "Blue Moon", "Satin Doll", "A Foggy Day"],
    },
];

/// The identifiers of [`CATALOG`], in the same order: the accepted set of
/// [`ProgressionId`].
///
/// [`ProgressionId`]'s parser searches exactly this table, so the enumerated
/// accepted set and the parsed accepted set cannot drift.
pub(crate) const PROGRESSION_IDS: [&str; 59] = [
    "pop_i_v_vi_iv",
    "classic_i_iv_v",
    "blues_12_bar",
    "jazz_ii_v_i",
    "fifties_i_vi_iv_v",
    "pachelbel_canon",
    "pop_vi_iv_i_v",
    "rock_i_bvii_iv",
    "folk_i_iv",
    "minor_pop_i_vi_iii_vii",
    "minor_pop_i_bvi_biii_bvii",
    "canon_rock",
    "modal_interchange_i_iv",
    "creep_progression",
    "chromatic_mediant_i_biii",
    "chromatic_mediant_i_bvi",
    "chromatic_mediant_i_iii",
    "neapolitan_i_bii_v_i",
    "descending_chromatic_bass",
    "line_cliche_i_bvii_bvi_v",
    "omnipotent_progression",
    "ascending_bass_i_ii_iii_iv",
    "chromatic_walkdown_i_bvii_vi_bvii_i",
    "andalusian_cadence",
    "flamenco_phrygian_dominant",
    "harmonic_minor_i_iv_v",
    "byzantine_double_harmonic",
    "hungarian_minor",
    "japanese_hirajoshi",
    "middle_eastern_hijaz",
    "klezmer_freygish",
    "dorian_vamp_i_iv",
    "dorian_aeolian_i_bvii_iv",
    "lydian_i_ii",
    "whole_tone",
    "mixolydian_bvi_i_bvii_bvi_bvii",
    "phrygian_vamp_i_bii_i",
    "spanish_phrygian_i_bii_iii",
    "blues_dominant_i7_iv7_v7",
    "minor_blues_i7_iv7_v7",
    "rhythm_changes_a",
    "rhythm_changes_b",
    "coltrane_changes",
    "coltrane_sub_ii_v_i",
    "backdoor_progression",
    "jazz_turnaround",
    "minor_ii_v_i",
    "tritone_sub_ii_bii_i",
    "secondary_dominant",
    "bird_blues",
    "modal_jazz_vamp",
    "extended_ii_v_chain",
    "iv_minor_substitution",
    "plagal_cadence",
    "deceptive_cadence",
    "minor_plagal",
    "jazz_blues_form",
    "bossa_nova_ii_v_i",
    "aaba_form",
];

/// Lookup used by [`ProgressionId::parse`](crate::ProgressionId::parse): the
/// catalog's own identifier table.
pub(crate) fn canonical_progression_id(name: &str) -> Option<&'static str> {
    PROGRESSION_IDS
        .iter()
        .find(|known| **known == name)
        .copied()
}

/// The invariant [`catalog_entry`] relies on: the identifier table and the
/// catalog carry the same fifty-nine entries in the same order, and no
/// identifier is duplicated.
///
/// A mistranscribed catalog entry fails the build instead of silently answering
/// with another progression's degrees. The strings are compared with the crate's
/// one const string comparison, because the `PartialEq` operator is not available
/// in const evaluation.
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects)]
const fn assert_catalog_matches_ids() {
    let ids = PROGRESSION_IDS;
    assert!(
        CATALOG.len() == ids.len(),
        "the catalog and the identifier list differ in length"
    );
    let mut index = 0;
    while index < ids.len() {
        assert!(
            str_eq(CATALOG[index].id.as_str(), ids[index]),
            "the catalog is not in the frozen identifier order"
        );
        let mut other = index + 1;
        while other < ids.len() {
            assert!(
                !str_eq(ids[index], ids[other]),
                "the identifier list repeats an identifier"
            );
            other += 1;
        }
        index += 1;
    }
}

/// Compile-time proof of the invariant [`catalog_entry`] relies on.
const _: () = assert_catalog_matches_ids();

/// Every progression of the frozen catalog, in catalog order.
pub const fn all_progressions() -> &'static [Progression] {
    &CATALOG
}

/// The catalog entry of one identifier.
///
/// [`ProgressionId`] accepts exactly the frozen catalog identifiers and
/// [`CATALOG`] carries exactly one entry per identifier in the same order —
/// proved at compile time by [`assert_catalog_matches_ids`] — so the lookup
/// cannot miss; the first entry is returned only to keep the function total.
pub fn progression(id: ProgressionId) -> &'static Progression {
    let index = ProgressionId::ALL
        .iter()
        .position(|known| *known == id)
        .unwrap_or(0);
    // The position indexes the identifier table exactly, and that table is proved
    // to match the catalog at compile time, so the index is in bounds.
    #[allow(clippy::indexing_slicing)]
    {
        &CATALOG[index]
    }
}

/// The catalog entry of one identifier as text, or `None` when the catalog does
/// not carry it.
pub fn progression_by_str(id: &str) -> Option<&'static Progression> {
    CATALOG.iter().find(|entry| entry.id.as_str() == id)
}

/// One display group of progressions, in catalog display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressionGroup {
    /// The group's display (and catalog) category name.
    pub category: &'static str,
    /// The group's progressions, in display order.
    pub ids: &'static [ProgressionId],
}

/// The catalog's own grouping, in the frozen order.
const GROUPS: [ProgressionGroup; 4] = [
    ProgressionGroup {
        category: "Famous / Classic",
        ids: &[
            ProgressionId::from_catalog("pop_i_v_vi_iv"),
            ProgressionId::from_catalog("classic_i_iv_v"),
            ProgressionId::from_catalog("blues_12_bar"),
            ProgressionId::from_catalog("jazz_ii_v_i"),
            ProgressionId::from_catalog("fifties_i_vi_iv_v"),
            ProgressionId::from_catalog("pachelbel_canon"),
            ProgressionId::from_catalog("pop_vi_iv_i_v"),
            ProgressionId::from_catalog("rock_i_bvii_iv"),
            ProgressionId::from_catalog("folk_i_iv"),
            ProgressionId::from_catalog("minor_pop_i_vi_iii_vii"),
            ProgressionId::from_catalog("minor_pop_i_bvi_biii_bvii"),
            ProgressionId::from_catalog("canon_rock"),
        ],
    },
    ProgressionGroup {
        category: "Curious / Interesting",
        ids: &[
            ProgressionId::from_catalog("modal_interchange_i_iv"),
            ProgressionId::from_catalog("creep_progression"),
            ProgressionId::from_catalog("chromatic_mediant_i_biii"),
            ProgressionId::from_catalog("chromatic_mediant_i_bvi"),
            ProgressionId::from_catalog("chromatic_mediant_i_iii"),
            ProgressionId::from_catalog("neapolitan_i_bii_v_i"),
            ProgressionId::from_catalog("descending_chromatic_bass"),
            ProgressionId::from_catalog("line_cliche_i_bvii_bvi_v"),
            ProgressionId::from_catalog("omnipotent_progression"),
            ProgressionId::from_catalog("ascending_bass_i_ii_iii_iv"),
            ProgressionId::from_catalog("chromatic_walkdown_i_bvii_vi_bvii_i"),
        ],
    },
    ProgressionGroup {
        category: "Exotic / World",
        ids: &[
            ProgressionId::from_catalog("andalusian_cadence"),
            ProgressionId::from_catalog("flamenco_phrygian_dominant"),
            ProgressionId::from_catalog("harmonic_minor_i_iv_v"),
            ProgressionId::from_catalog("byzantine_double_harmonic"),
            ProgressionId::from_catalog("hungarian_minor"),
            ProgressionId::from_catalog("japanese_hirajoshi"),
            ProgressionId::from_catalog("middle_eastern_hijaz"),
            ProgressionId::from_catalog("klezmer_freygish"),
            ProgressionId::from_catalog("dorian_vamp_i_iv"),
            ProgressionId::from_catalog("dorian_aeolian_i_bvii_iv"),
            ProgressionId::from_catalog("lydian_i_ii"),
            ProgressionId::from_catalog("whole_tone"),
            ProgressionId::from_catalog("mixolydian_bvi_i_bvii_bvi_bvii"),
            ProgressionId::from_catalog("phrygian_vamp_i_bii_i"),
            ProgressionId::from_catalog("spanish_phrygian_i_bii_iii"),
            ProgressionId::from_catalog("blues_dominant_i7_iv7_v7"),
            ProgressionId::from_catalog("minor_blues_i7_iv7_v7"),
        ],
    },
    ProgressionGroup {
        category: "Jazz / Sophisticated",
        ids: &[
            ProgressionId::from_catalog("rhythm_changes_a"),
            ProgressionId::from_catalog("rhythm_changes_b"),
            ProgressionId::from_catalog("coltrane_changes"),
            ProgressionId::from_catalog("coltrane_sub_ii_v_i"),
            ProgressionId::from_catalog("backdoor_progression"),
            ProgressionId::from_catalog("jazz_turnaround"),
            ProgressionId::from_catalog("minor_ii_v_i"),
            ProgressionId::from_catalog("tritone_sub_ii_bii_i"),
            ProgressionId::from_catalog("secondary_dominant"),
            ProgressionId::from_catalog("bird_blues"),
            ProgressionId::from_catalog("modal_jazz_vamp"),
            ProgressionId::from_catalog("extended_ii_v_chain"),
            ProgressionId::from_catalog("iv_minor_substitution"),
            ProgressionId::from_catalog("plagal_cadence"),
            ProgressionId::from_catalog("deceptive_cadence"),
            ProgressionId::from_catalog("minor_plagal"),
            ProgressionId::from_catalog("jazz_blues_form"),
            ProgressionId::from_catalog("bossa_nova_ii_v_i"),
            ProgressionId::from_catalog("aaba_form"),
        ],
    },
];

/// The progression groups of the frozen catalog, in catalog display order.
pub const fn grouped_progressions() -> &'static [ProgressionGroup] {
    &GROUPS
}

/// The category names, in the frozen display order.
pub const fn progression_categories() -> &'static [&'static str] {
    &CATEGORIES
}

/// The category names of [`GROUPS`], in the same order.
const CATEGORIES: [&str; 4] = [
    "Famous / Classic",
    "Curious / Interesting",
    "Exotic / World",
    "Jazz / Sophisticated",
];
