use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct KanjiData {
    pub kanji: char,
    pub name: Option<String>,
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
#[derive(Debug, Deserialize, Serialize)]
pub struct KanjiStrokes     {
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