use kanji_tui::model::KanjiData;

#[derive(Debug)]
pub struct SearchCriteria {
    pub jlpt: Option<u8>,
    pub grade: Option<u8>,
    pub strokes: Option<u8>,
    pub rank: Option<u16>,
    pub rad_index: Option<u8>,

    pub query_text: Option<String>,
}
trait KanjiRepo {
    fn search(&self, search_criteria: SearchCriteria) -> Vec<&KanjiData>;
}

//Conditional compilation for local json
#[cfg(feature = "json-backend")]
pub mod json;