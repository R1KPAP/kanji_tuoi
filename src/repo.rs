use crate::model::{KanjiData, JlptLevel};
use crate::app::SearchCriteria;
use std::collections::HashSet;

impl Default for SearchCriteria {
    fn default() -> Self {
        let jlpt_space = HashSet::from([JlptLevel::N(1)]);
        Self {
            jlpt: Some(jlpt_space),
            grade: None,
            strokes: None,
            rank: None,
            rad_index: None,
            query_text: None,
        }
    }
}

pub trait KanjiRepo {
    fn search(&self, search_criteria: &SearchCriteria) -> Vec<&KanjiData>;
    fn get_by_kanji(&self, kanji: char) -> Option<&KanjiData>;
    fn get_by_index(&self, index: usize) -> Option<&KanjiData>;
}

//Conditional compilation for local json
#[cfg(feature = "json-backend")]
pub mod json;

#[cfg(feature = "json-backend")]
pub use json::InMemoryJson;