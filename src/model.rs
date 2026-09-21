use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum JlptLevel {
    N(u8),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum KanjiGrade {
    G1,
    G2,
    G3,
    G4,
    G5,
    G6,
    Joyo,
    Jinmeiyo,
    Hyogai,
}

impl KanjiGrade {
    pub fn from_num(num: u8)
    -> Self {
        match num {
            1 => KanjiGrade::G1,
            2 => KanjiGrade::G2,
            3 => KanjiGrade::G3,
            4 => KanjiGrade::G4,
            5 => KanjiGrade::G5,
            6 => KanjiGrade::G6,
            7 => KanjiGrade::Joyo,
            8 => KanjiGrade::Jinmeiyo,
            _ => KanjiGrade::Hyogai,
        }
    }

    pub fn to_num(&self)
    -> u8 {
        match self {
            KanjiGrade::G1 => 1,
            KanjiGrade::G2 => 2,
            KanjiGrade::G3 => 3,
            KanjiGrade::G4 => 4,
            KanjiGrade::G5 => 5,
            KanjiGrade::G6 => 6,
            KanjiGrade::Joyo => 8,
            KanjiGrade::Jinmeiyo => 9,
            KanjiGrade::Hyogai => 11,
        }
    }

    pub fn descs()
    -> &'static [(KanjiGrade, &'static str, usize)] {
        &[
            (KanjiGrade::G1, "G1", 0),
            (KanjiGrade::G2, "G2", 1),
            (KanjiGrade::G3, "G3", 2),
            (KanjiGrade::G4, "G4", 3),
            (KanjiGrade::G5, "G5", 4),
            (KanjiGrade::G6, "G6", 5),
            (KanjiGrade::Joyo, "Jōyō", 6),
            (KanjiGrade::Jinmeiyo, "Jinmeiyō", 7),
            (KanjiGrade::Hyogai, "Hyōgai", 8),
        ]
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct KanjiData {
    pub kanji: char,
    pub names: Vec<String>,
    pub jlpt_level: Option<u8>,
    pub freq_rank: Option<u16>,
    pub grade: Option<u8>,
    pub strokes_count: Option<u8>,
    pub rad_index: Option<u8>,
    #[serde(default)]
    pub strokes: Vec<KanjiStrokes>,
    pub onyomi: Vec<String>,
    pub kunyomi: Vec<String>,
    pub nanori: Vec<String>,
    pub romaji: Vec<String>,
    pub meanings: Vec<String>,
    #[serde(default)]
    pub variants: Vec<char>,
    #[serde(default)]
    pub decomposition: Vec<char>,
    #[serde(default)]
    pub used_in: Vec<char>,
}

impl KanjiData {
    pub fn matches_text(&self, query: Option<&str>) -> bool {
        let Some(q) = query.filter(|s| !s.trim().is_empty()) else {
            return true;
        };

        let tokens: Vec<String> = q
            .to_lowercase()
            .split_whitespace()
            .map(String::from)
            .collect();

        tokens.iter().all(|token| {
            self.meanings.iter().any(|m| {
                m.to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .any(|word| word.starts_with(token))
            }) || self.romaji.iter().any(|r| r.to_lowercase().contains(token))
                || self.onyomi.iter().any(|r| r.contains(token))
                || self.kunyomi.iter().any(|r| r.contains(token))
                || self.nanori.iter().any(|r| r.contains(token))
        })
    }

    pub fn grade_detail(&self) -> String {
        match self.grade {
            Some(g @ 1..=6) => format!("Kyōiku (Grade {g})"),
            Some(8) => "Jōyō (Secondary)".to_string(),
            Some(9 | 10) => "Jinmeiyō (Name)".to_string(),
            _ => "Hyōgai (Extra)".to_string(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct KanjiStrokes {
    pub strokes: Vec<MyPoint>,
    pub order: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyPoint {
    pub x: f32,
    pub y: f32,
}

impl MyPoint {
    pub fn from_xy(x: f32, y: f32) -> Self {
        MyPoint { x, y }
    }
}