use crate::repo::{
    KanjiRepo,
    SearchCriteria,
};
use kanji_tui::model::KanjiData;
use std::collections::HashMap;
use std::{
    fs::File,
    io::BufReader,
};
use std::cmp::Ordering;

pub struct Indexes {
    jlpt_index: HashMap<u8, Vec<usize>>,
    grade_index: HashMap<u8, Vec<usize>>,
    stroke_index: HashMap<u8, Vec<usize>>,
    freq_index: HashMap<u16, Vec<usize>>,
    rad_index: HashMap<u8, Vec<usize>>,
}

enum MissingValue {
    Jlpt,
    Grade,
    Frequence,
}

impl Indexes {
    pub fn build(dataset: &[KanjiData]) -> Self {
        let mut jlpt_index: HashMap<u8, Vec<usize>> = HashMap::new();
        let mut grade_index: HashMap<u8, Vec<usize>> = HashMap::new();
        let mut stroke_index: HashMap<u8, Vec<usize>> = HashMap::new();
        let mut freq_index: HashMap<u16, Vec<usize>> = HashMap::new();
        let mut rad_index: HashMap<u8, Vec<usize>> = HashMap::new();

        for (idx, kanji) in dataset.iter().enumerate() {
            jlpt_index.entry(kanji.jlpt_level
                .unwrap_or(handle_missing_value(kanji, MissingValue::Jlpt) as u8))
                .or_default()
                .push(idx);
            grade_index.entry(kanji.grade
                .unwrap_or(handle_missing_value(kanji, MissingValue::Grade) as u8))
                .or_default()
                .push(idx);
            stroke_index.entry(kanji.strokes_count
                .unwrap())
                .or_default()
                .push(idx);
            freq_index.entry(kanji.freq_rank
                .unwrap_or(handle_missing_value(kanji, MissingValue::Frequence)))
                .or_default()
                .push(idx);
            rad_index.entry(kanji.rad_index
                .unwrap())
                .or_default()
                .push(idx);
        }

        Self { jlpt_index, grade_index, stroke_index, freq_index, rad_index }
    }
}

fn handle_missing_value(kanji: &KanjiData, value_type: MissingValue) -> u16 {
    return match value_type {
        MissingValue::Jlpt=> {
            if kanji.name.is_none() { 6 }
            else { 0 }
        },
        MissingValue::Grade => 11,
        MissingValue::Frequence => {
            if kanji.name.is_none() { 0 }
            else { u16::MAX }
        }
    };
}

struct InMemoryJson {
    kanji_dataset: Vec<KanjiData>,
    indexes: Indexes,
}

impl InMemoryJson {

    fn new() -> Self {
        let kanji_dataset = load_dataset().expect("Error loading kanji dataset");
        let indexes = Indexes::build(&kanji_dataset);
        Self { kanji_dataset, indexes: indexes }
    }

    fn get_candidates(&self, criteria: &SearchCriteria) -> Vec<usize> {
        let mut active_indices: Vec<&[usize]> = Vec::new();

        if let Some(jlpt) = criteria.jlpt {
            if let Some(list) = self.indexes.jlpt_index.get(&jlpt) {
                active_indices.push(list.as_slice());
            }
        }
        if let Some(grade) = criteria.grade {
            if let Some(list) = self.indexes.grade_index.get(&grade) {
                active_indices.push(list.as_slice());
            }
        }
        if let Some(strokes) = criteria.strokes {
            if let Some(list) = self.indexes.stroke_index.get(&strokes) {
                active_indices.push(list.as_slice());
            }
        }
        if let Some(rank) = criteria.rank {
            if let Some(list) = self.indexes.freq_index.get(&rank) {
                active_indices.push(list.as_slice());
            }
        }
        if let Some(rad_index) = criteria.rad_index {
            if let Some(list) = self.indexes.rad_index.get(&rad_index) {
                active_indices.push(list.as_slice());
            }
        }

        if active_indices.is_empty() {
            return (0..self.kanji_dataset.len()).collect();
        }

        active_indices.sort_by_key(|slice| slice.len());

        let mut result = active_indices[0].to_vec();
        for &next_slice in &active_indices[1..] {
            result = intersect_sorted(&result, next_slice);
            if result.is_empty() {
                break;
            }
        }
        result
    }
}

impl KanjiRepo for InMemoryJson {
    fn search(&self, search_criteria: SearchCriteria) -> Vec<&KanjiData> {
        let candidates = self.get_candidates(&search_criteria);

        candidates
            .iter()
            .map(|&idx| &self.kanji_dataset[idx])
            .filter(|kanji| kanji.matches_text(search_criteria.query_text.as_deref()))
            .collect()
    }
}

fn load_dataset() -> Result<Vec<KanjiData>, Box<dyn std::error::Error>> {
    let file = File::open("data/kanji_dataset.bin")?;
    let reader = BufReader::new(file);
    let dataset: Vec<KanjiData> = bincode::deserialize_from(reader)?;
    Ok(dataset)
}

fn intersect_sorted(a: &[usize], b: &[usize]) -> Vec<usize> {
    let mut iter_a = a.iter().peekable();
    let mut iter_b = b.iter().peekable();
    let mut result = Vec::new();
    while let (Some(&x), Some(&y)) = (iter_a.peek(), iter_b.peek()) {
        match x.cmp(&y) {
            Ordering::Equal => {
                result.push(x.to_owned());
                iter_a.next();
                iter_b.next();
            }
            Ordering::Less => { iter_a.next(); }
            Ordering::Greater => { iter_b.next(); }
        }
    }
    result
}