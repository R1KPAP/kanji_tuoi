use crate::repo::{JlptLevel, KanjiRepo, SearchCriteria};
use crate::model::KanjiData;
use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::io::{BufReader, Cursor};

//Indexes for fast searches
pub struct Indexes {
    jlpt_index: HashMap<u8, Vec<usize>>,
    grade_index: HashMap<u8, Vec<usize>>,
    stroke_index: HashMap<u8, Vec<usize>>,
    freq_index: HashMap<u16, Vec<usize>>,
    rad_index: HashMap<u8, Vec<usize>>,
    char_to_index: HashMap<char, usize>,
}

enum MissingValue {
    Jlpt,
    Grade,
    Frequence,
}

impl Indexes {
    //Create indexes from the binary dataset stored locally
    pub fn build(dataset: &[KanjiData]) -> Self {
        let mut jlpt_index: HashMap<u8, Vec<usize>> = HashMap::new();
        let mut grade_index: HashMap<u8, Vec<usize>> = HashMap::new();
        let mut stroke_index: HashMap<u8, Vec<usize>> = HashMap::new();
        let mut freq_index: HashMap<u16, Vec<usize>> = HashMap::new();
        let mut rad_index: HashMap<u8, Vec<usize>> = HashMap::new();
        let mut char_to_index: HashMap<char, usize> = HashMap::new();

        for (idx, kanji) in dataset.iter().enumerate() {
            jlpt_index
                .entry(
                    kanji
                        .jlpt_level
                        .unwrap_or(handle_missing_value(kanji, MissingValue::Jlpt) as u8),
                )
                .or_default()
                .push(idx);
            grade_index
                .entry(
                    kanji
                        .grade
                        .unwrap_or(handle_missing_value(kanji, MissingValue::Grade) as u8),
                )
                .or_default()
                .push(idx);
            stroke_index
                .entry(kanji.strokes_count.unwrap())
                .or_default()
                .push(idx);
            freq_index
                .entry(
                    kanji
                        .freq_rank
                        .unwrap_or(handle_missing_value(kanji, MissingValue::Frequence)),
                )
                .or_default()
                .push(idx);
            rad_index
                .entry(kanji.rad_index.unwrap())
                .or_default()
                .push(idx);
            char_to_index.insert(kanji.kanji, idx);
        }

        Self {
            jlpt_index,
            grade_index,
            stroke_index,
            freq_index,
            rad_index,
            char_to_index,
        }
    }
}

fn handle_missing_value(kanji: &KanjiData, value_type: MissingValue)
 -> u16 {
    return match value_type {
        MissingValue::Jlpt => {
            if kanji.names.is_empty() {
                6
            } else {
                0
            }
        }
        MissingValue::Grade => 11,
        MissingValue::Frequence => {
            if kanji.names.is_empty() {
                0
            } else {
                u16::MAX
            }
        }
    };
}

pub struct InMemoryJson {
    pub kanji_dataset: Vec<KanjiData>,
    pub indexes: Indexes,
}

impl InMemoryJson {
    pub fn new() -> Self {
        let kanji_dataset = load_dataset().expect("Error loading kanji dataset");
        let indexes = Indexes::build(&kanji_dataset);
        Self {
            kanji_dataset,
            indexes: indexes,
        }
    }

    /* In case of filtered search, first try to reduce the search space by using the smallest
    subset of kanji and finally in case of textual search, address it on the smallest subset*/
    fn get_candidates(&self, criteria: &SearchCriteria)
    -> Vec<usize> {
        let mut active_indices: Vec<Cow<[usize]>> = Vec::new();

        // JLPT
        if let Some(jlpt) = &criteria.jlpt {
            let mut combined: Vec<usize> = jlpt
                .iter()
                .filter_map(|JlptLevel::N(n)| self.indexes.jlpt_index.get(&n))
                .flatten()
                .copied()
                .collect();

            if !combined.is_empty() {
                combined.sort_unstable();
                active_indices.push(Cow::Owned(combined));
            }
        }

        // Grade
        if let Some(grade) = &criteria.grade {
            let mut combined: Vec<usize> = grade
                .iter()
                .filter_map(|g| self.indexes.grade_index.get(&g.to_num()))
                .flatten()
                .copied()
                .collect();

            if !combined.is_empty() {
                combined.sort_unstable();
                active_indices.push(Cow::Owned(combined));
            }
        }

        // Strokes
        if let Some(strokes) = &criteria.strokes {
            let mut combined: Vec<usize> = self
                .indexes
                .stroke_index
                .iter()
                .filter(|&(k, _)| strokes.matches(*k))
                .flat_map(|(_, v)| v)
                .copied()
                .collect();

            if !combined.is_empty() {
                combined.sort_unstable();
                active_indices.push(Cow::Owned(combined));
            }
        }

        // Rank
        if let Some(rank) = &criteria.rank {
            let mut combined: Vec<usize> = self
                .indexes
                .freq_index
                .iter()
                .filter(|&(k, _)| rank.matches(*k))
                .flat_map(|(_, v)| v)
                .copied()
                .collect();

            if !combined.is_empty() {
                combined.sort_unstable();
                active_indices.push(Cow::Owned(combined));
            }
        }

        // Radical Index
        if let Some(rad_index) = &criteria.rad_index {
            let mut combined: Vec<usize> = rad_index
                .iter()
                .filter_map(|idx| self.indexes.rad_index.get(&(idx + 1)))
                .flatten()
                .copied()
                .collect();

            if !combined.is_empty() {
                combined.sort_unstable();
                active_indices.push(Cow::Owned(combined));
            }
        }

        if active_indices.is_empty() {
            return (0..self.kanji_dataset.len()).collect();
        }

        active_indices.sort_by_key(|slice| slice.len());

        let mut result = active_indices[0].to_vec();
        for next_slice in &active_indices[1..] {
            result = intersect_sorted(&result, next_slice);
            if result.is_empty() {
                break;
            }
        }
        
        result
    }
}

impl KanjiRepo for InMemoryJson {
    fn search(&self, search_criteria: &SearchCriteria)
    -> Vec<&KanjiData> {
        let candidates = self.get_candidates(&search_criteria);

        let mut results: Vec<&KanjiData> = candidates
        .iter()
        .map(|&idx| &self.kanji_dataset[idx])
        .filter(|kanji| kanji.matches_text(search_criteria.query_text.as_deref()))
        .collect();

        results.sort_unstable_by_key(|kanji| kanji.freq_rank.unwrap_or(u16::MAX));

        results
    }

    fn get_by_index(&self, index: usize) -> Option<&KanjiData> {
        self.kanji_dataset.get(index)
    }

    fn get_by_kanji(&self, kanji: char) -> Option<&KanjiData> {
        if let Some(index) = self.indexes.char_to_index.get(&kanji) {
            self.kanji_dataset.get(*index)
        } else {
            None
        }
    }
}

fn load_dataset()
-> Result<Vec<KanjiData>, Box<dyn std::error::Error>> {
    static DATASET_BYTES: &[u8] = include_bytes!("../../data/kanji_dataset.bin");
    let cursor = Cursor::new(DATASET_BYTES);
    let reader = BufReader::new(cursor);
    let dataset: Vec<KanjiData> = bincode::deserialize_from(reader)?;
    Ok(dataset)
}

fn intersect_sorted(a: &[usize], b: &[usize])
-> Vec<usize> {
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
            Ordering::Less => {
                iter_a.next();
            }
            Ordering::Greater => {
                iter_b.next();
            }
        }
    }
    result
}
