use std::collections::HashSet;
use crate::model::{KanjiGrade, JlptLevel};
use crate::utilities::{backspace_numeric, delete_numeric};
use crate::constants::KANGXI_RADICALS;


#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RangeFilter<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T: PartialOrd> RangeFilter<T> {
    pub fn matches(&self, value: T)
    -> bool {
        self.min.as_ref().is_none_or(|min| &value >= min)
            && self.max.as_ref().is_none_or(|max| &value <= max)
    }
}

#[derive(Debug, Clone)]
pub struct SearchCriteria {
    pub jlpt: Option<HashSet<JlptLevel>>,
    pub grade: Option<HashSet<KanjiGrade>>,
    pub strokes: Option<RangeFilter<u8>>,
    pub rank: Option<RangeFilter<u16>>,
    pub rad_index: Option<HashSet<u8>>,

    pub query_text: Option<String>,
}

#[derive(PartialEq, Eq)]
pub enum RangeSubfield {
    Min,
    Max,
}

impl RangeSubfield {
    pub fn extract_range_len<T: std::fmt::Display>(
        filter: Option<&RangeFilter<T>>,
        subfield: &RangeSubfield,
    ) -> usize {
        let val = filter.and_then(|f| match subfield {
            RangeSubfield::Min => f.min.as_ref(),
            RangeSubfield::Max => f.max.as_ref(),
        });

        val.map_or(0, |v| v.to_string().len())
    }
}

#[derive(PartialEq, Eq)]
pub enum SearchField {
    Jlpt(usize),
    Grade(usize),
    Strokes(RangeSubfield),
    Rank(RangeSubfield),
    Radical(usize),
    Query,
}

impl Default for SearchField {
    fn default() -> Self {
        Self::Jlpt(0)
    }
}

#[derive(Default)]
pub struct SearchForm {
    pub active_field: SearchField,
    pub is_editing: bool,
    pub cursor_offset: usize,
    pub max_colums: usize,

    pub draft: SearchCriteria,
}

impl SearchForm {
    pub fn new(filters: &SearchCriteria) -> Self {
        Self {
            draft: filters.clone(),
            is_editing: false,
            ..Default::default()
        }
    }

    pub fn handle_input(&mut self, c: char) {
        match &self.active_field {
            SearchField::Strokes(sub) => {
                let Some(digit) = c.to_digit(10) else { return };

                let filter = self.draft.strokes.get_or_insert_with(RangeFilter::default);
                let target = match sub {
                    RangeSubfield::Min => &mut filter.min,
                    RangeSubfield::Max => &mut filter.max,
                };

                let current = target.unwrap_or(0) as u32;
                let new_val = current * 10 + digit;

                if new_val <= u8::MAX as u32 {
                    *target = Some(new_val as u8);
                }
            }

            SearchField::Rank(sub) => {
                let Some(digit) = c.to_digit(10) else { return };

                let filter = self.draft.rank.get_or_insert_with(RangeFilter::default);
                let target = match sub {
                    RangeSubfield::Min => &mut filter.min,
                    RangeSubfield::Max => &mut filter.max,
                };

                let current = target.unwrap_or(0) as u32;
                let new_val = current * 10 + digit;

                if new_val <= u16::MAX as u32 {
                    *target = Some(new_val as u16);
                }
            }

            SearchField::Query => {
                let text = self.draft.query_text.get_or_insert_with(String::new);
                text.push(c);
            }

            _ => {}
        }
        self.cursor_right();
    }

    fn set_active_field(&mut self, field: SearchField) {
        self.active_field = field;
        match self.active_field {
            SearchField::Strokes(_) | SearchField::Rank(_) | SearchField::Query => {
                self.is_editing = true
            }
            _ => self.is_editing = false,
        }
        self.cursor_offset = 0;
    }

    pub fn toggle_current(&mut self) {
        match self.active_field {
            SearchField::Radical(idx) => {
                let set = self.draft.rad_index.get_or_insert_with(HashSet::new);
                if !set.remove(&(idx as u8)) {
                    set.insert(idx as u8);
                }
            }
            SearchField::Jlpt(idx) => {
                let level = JlptLevel::N((idx + 1) as u8);
                let set = self.draft.jlpt.get_or_insert_with(HashSet::new);
                if !set.remove(&level) {
                    set.insert(level);
                }
            }

            SearchField::Grade(idx) => {
                let grade = &KanjiGrade::from_num((idx + 1) as u8);
                let set = self.draft.grade.get_or_insert_with(HashSet::new);
                if !set.remove(&grade) {
                    set.insert(*grade);
                }
            }
            _ => {}
        }
    }

    pub fn handle_backspace(&mut self) {
        match &self.active_field {
            SearchField::Strokes(sub) => {
                if let Some(filter) = &mut self.draft.strokes {
                    let target = match sub {
                        RangeSubfield::Min => &mut filter.min,
                        RangeSubfield::Max => &mut filter.max,
                    };
                    if backspace_numeric(target, self.cursor_offset) {
                        self.cursor_offset -= 1;
                    }
                }
            }

            SearchField::Rank(sub) => {
                if let Some(filter) = &mut self.draft.rank {
                    let target = match sub {
                        RangeSubfield::Min => &mut filter.min,
                        RangeSubfield::Max => &mut filter.max,
                    };
                    if backspace_numeric(target, self.cursor_offset) {
                        self.cursor_offset -= 1;
                    }
                }
            }

            SearchField::Query => {
                if self.cursor_offset > 0 {
                    if let Some(text) = &mut self.draft.query_text {
                        if let Some((byte_idx, _)) = text.char_indices().nth(self.cursor_offset - 1)
                        {
                            text.remove(byte_idx);
                            self.cursor_offset -= 1;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn handle_delete(&mut self) {
        match &self.active_field {
            SearchField::Strokes(sub) => {
                if let Some(filter) = &mut self.draft.strokes {
                    let target = match sub {
                        RangeSubfield::Min => &mut filter.min,
                        RangeSubfield::Max => &mut filter.max,
                    };
                    delete_numeric(target, self.cursor_offset);
                }
            }

            SearchField::Rank(sub) => {
                if let Some(filter) = &mut self.draft.rank {
                    let target = match sub {
                        RangeSubfield::Min => &mut filter.min,
                        RangeSubfield::Max => &mut filter.max,
                    };
                    delete_numeric(target, self.cursor_offset);
                }
            }

            SearchField::Query => {
                if let Some(text) = &mut self.draft.query_text {
                    if let Some((byte_idx, _)) = text.char_indices().nth(self.cursor_offset) {
                        text.remove(byte_idx);
                        if text.is_empty() {
                            self.draft.query_text = None;
                        }
                    }
                }
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        self.set_active_field(match self.active_field {
            SearchField::Radical(idx) => {
                SearchField::Radical((idx + self.max_colums).min(KANGXI_RADICALS.len() - 1))
            }
            SearchField::Jlpt(_) => SearchField::Grade(0),
            SearchField::Grade(_) => SearchField::Strokes(RangeSubfield::Min),
            SearchField::Strokes(_) => SearchField::Rank(RangeSubfield::Min),
            SearchField::Rank(_) => SearchField::Query,
            SearchField::Query => SearchField::Jlpt(0),
        });
    }

    pub fn move_up(&mut self) {
        self.set_active_field(match self.active_field {
            SearchField::Radical(idx) => SearchField::Radical(idx.saturating_sub(self.max_colums)),
            SearchField::Jlpt(_) => SearchField::Query,
            SearchField::Grade(_) => SearchField::Jlpt(0),
            SearchField::Strokes(_) => SearchField::Grade(0),
            SearchField::Rank(_) => SearchField::Strokes(RangeSubfield::Min),
            SearchField::Query => SearchField::Rank(RangeSubfield::Min),
        });
    }

    pub fn move_left(&mut self) {
        match &mut self.active_field {
            SearchField::Radical(idx) => {
                *idx = idx.saturating_sub(1).min(KANGXI_RADICALS.len() - 1)
            }
            SearchField::Jlpt(idx) => *idx = idx.saturating_sub(1),
            SearchField::Grade(idx) => *idx = idx.saturating_sub(1),
            SearchField::Strokes(sub) => {
                *sub = RangeSubfield::Min;
                self.is_editing = true;
            }
            SearchField::Rank(sub) => {
                *sub = RangeSubfield::Min;
                self.is_editing = true;
            }
            SearchField::Query => {
                self.cursor_offset = self.cursor_offset.saturating_sub(1);
            }
        }
    }

    pub fn move_right(&mut self) {
        match &mut self.active_field {
            SearchField::Radical(idx) => *idx = (*idx + 1).min(KANGXI_RADICALS.len() - 1),
            SearchField::Jlpt(idx) => *idx = (*idx + 1).min(5),
            SearchField::Grade(idx) => *idx = (*idx + 1).min(8),
            SearchField::Strokes(sub) => {
                *sub = RangeSubfield::Max;
                self.is_editing = true;
            }
            SearchField::Rank(sub) => {
                *sub = RangeSubfield::Max;
                self.is_editing = true;
            }
            SearchField::Query => {
                let max_len = self.draft.query_text.as_ref().map_or(0, |s| s.len());
                if self.cursor_offset < max_len {
                    self.cursor_offset += 1;
                }
            }
        }
    }

    pub fn move_right_tab(&mut self) {
        match &self.active_field {
            SearchField::Strokes(sub) => {
                self.set_active_field(match sub {
                    RangeSubfield::Min => SearchField::Strokes(RangeSubfield::Max),
                    RangeSubfield::Max => SearchField::Rank(RangeSubfield::Min),
                });
            }
            SearchField::Rank(sub) => {
                self.set_active_field(match sub {
                    RangeSubfield::Min => SearchField::Rank(RangeSubfield::Max),
                    RangeSubfield::Max => SearchField::Query,
                });
            }
            SearchField::Query => self.set_active_field(SearchField::Jlpt(0)),
            _ => {}
        };
    }

    pub fn cursor_right(&mut self) {
        let max_offset = self.active_field_len();
        if self.cursor_offset < max_offset {
            self.cursor_offset += 1;
        }
    }

    pub fn cursor_left(&mut self) {
        self.cursor_offset = self.cursor_offset.saturating_sub(1);
    }

    fn active_field_len(&self) -> usize {
        match &self.active_field {
            SearchField::Strokes(sub) => {
                RangeSubfield::extract_range_len(self.draft.strokes.as_ref(), sub)
            }
            SearchField::Rank(sub) => {
                RangeSubfield::extract_range_len(self.draft.rank.as_ref(), sub)
            }
            SearchField::Query => self.draft.query_text.as_deref().unwrap_or("").len(),
            _ => 0,
        }
    }
}