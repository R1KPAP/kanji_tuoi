use crate::repo::json::InMemoryJson;
use crate::repo::{JlptLevel, KanjiGrade, KanjiRepo, RangeFilter, SearchCriteria};
use crossterm::cursor::SetCursorStyle;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use kanji_tui::model::{KanjiData, MyPoint};
use ratatui::layout::Alignment;
use ratatui::widgets::Padding;
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line as TLine, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph,
        canvas::{Canvas, Context, Line},
    },
};
use std::collections::HashSet;
use std::io;
use std::io::stdout;
use std::time::{Duration, Instant};
use textwrap::Options;
mod model;
mod repo;

const KANGXI_RADICALS: [char; 214] = [
    '一', '丨', '丶', '丿', '乙', '亅', '二', '亠', '人', '儿', '入', '八', '冂', '冖', '冫', '几',
    '凵', '刀', '力', '勹', '匕', '匚', '匸', '十', '卜', '卩', '厂', '厶', '又', '口', '囗', '土',
    '士', '夂', '夊', '夕', '大', '女', '子', '宀', '寸', '小', '尢', '尸', '屮', '山', '川', '工',
    '己', '巾', '干', '幺', '广', '廴', '廾', '弋', '弓', '彐', '彡', '彳', '心', '戈', '戸', '手',
    '支', '攴', '文', '斗', '斤', '方', '无', '日', '曰', '月', '木', '欠', '止', '歹', '殳', '毋',
    '比', '毛', '氏', '气', '水', '火', '爪', '父', '爻', '爿', '片', '牙', '牛', '犬', '玄', '玉',
    '瓜', '瓦', '甘', '生', '用', '田', '疋', '疒', '癶', '白', '皮', '皿', '目', '矛', '矢', '石',
    '示', '禸', '禾', '穴', '立', '竹', '米', '糸', '缶', '网', '羊', '羽', '老', '而', '耒', '耳',
    '聿', '肉', '臣', '自', '至', '臼', '舌', '舛', '舟', '艮', '色', '艸', '虍', '虫', '血', '行',
    '衣', '襾', '見', '角', '言', '谷', '豆', '豕', '豸', '貝', '赤', '走', '足', '身', '車', '辛',
    '辰', '辵', '邑', '酉', '釆', '里', '金', '長', '門', '阜', '隷', '隹', '雨', '靑', '非', '面',
    '革', '韋', '韭', '音', '頁', '風', '飛', '食', '首', '香', '馬', '骨', '高', '髟', '鬥', '鬯',
    '鬲', '鬼', '魚', '鳥', '鹵', '鹿', '麥', '麻', '黄', '黍', '黒', '黹', '黽', '鼎', '鼓', '鼠',
    '鼻', '齊', '齒', '龍', '龜', '龠',
];

struct MainLayout {
    canvas: Rect,
    radical: Rect,
    stats: Rect,
    details: Rect,
    used_in: Rect,
}

#[derive(PartialEq)]
enum ActivePopup {
    SearchRad,
    Search,
    Nope,
    Help,
    History,
}
#[derive(PartialEq, Eq)]
pub enum RangeSubfield {
    Min,
    Max,
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
    active_field: SearchField,
    is_editing: bool,
    cursor_offset: usize,
    max_colums: usize,

    draft: SearchCriteria,
}

impl SearchForm {
    fn new(filters: &SearchCriteria) -> Self {
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
            SearchField::Jlpt(idx) => SearchField::Grade(idx.min(8)),
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
            SearchField::Strokes(sub) => extract_range_len(self.draft.strokes.as_ref(), sub),
            SearchField::Rank(sub) => extract_range_len(self.draft.rank.as_ref(), sub),
            SearchField::Query => self.draft.query_text.as_deref().unwrap_or("").len(),
            _ => 0,
        }
    }
}

struct KanjiAnimState {
    current_stroke: usize,
    current_point: usize,
    is_finished: bool,
    is_paused: bool,
    last_update: Instant,
    step_duration: Duration,
    stroke_delay_ticks: usize,
    single_stroke_mode: bool,
}

impl KanjiAnimState {
    fn new(step_ms: u64) -> Self {
        Self {
            current_stroke: 0,
            current_point: 0,
            is_finished: false,
            is_paused: false,
            last_update: Instant::now(),
            step_duration: Duration::from_millis(step_ms),
            stroke_delay_ticks: 0,
            single_stroke_mode: false,
        }
    }

    fn reset(&mut self) {
        self.single_stroke_mode = false;
        self.is_finished = false;
        self.is_paused = false;
        self.current_stroke = 0;
        self.current_point = 0;
        self.last_update = Instant::now();
    }

    pub fn play_stroke(&mut self, stroke_idx: usize) {
        self.current_stroke = stroke_idx;
        self.current_point = 0;
        self.is_paused = false;
        self.is_finished = false;
        self.single_stroke_mode = true;
        self.stroke_delay_ticks = 0;
        self.last_update = Instant::now();
    }

    fn tick(&mut self, kanji: &KanjiData) {
        if self.is_paused || self.is_finished {
            return;
        }

        if self.last_update.elapsed() < self.step_duration {
            return;
        }

        self.last_update = Instant::now();

        if let Some(stroke) = kanji.strokes.get(self.current_stroke) {
            if self.current_point + 1 < stroke.strokes.len() {
                self.current_point += 1;
            } else {
                if self.single_stroke_mode {
                    self.is_paused = true;
                } else {
                    if self.stroke_delay_ticks == 0 {
                        self.stroke_delay_ticks = 10;
                    } else if self.stroke_delay_ticks > 1 {
                        self.stroke_delay_ticks -= 1;
                    } else {
                        self.stroke_delay_ticks = 0;
                        if self.current_stroke + 1 < kanji.strokes.len() {
                            self.current_stroke += 1;
                            self.current_point = 0;
                        } else {
                            self.is_finished = true;
                        }
                    }
                }
            }
        } else {
            self.is_finished = true;
        }
    }
}

pub struct History {
    items: Vec<char>,
    cursor: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            cursor: 0,
        }
    }

    pub fn current(&self) -> char {
        self.items[self.cursor]
    }

    pub fn push(&mut self, next: char) -> bool {
        if !self.items.is_empty() && self.current() == next {
            return false;
        }
        self.items.truncate(self.cursor + 1);
        self.items.push(next);
        self.cursor = self.items.len() - 1;
        return true;
    }

    pub fn back(&mut self) -> bool {
        if self.cursor > 0 {
            self.cursor -= 1;
            true
        } else if !self.items.is_empty() {
            self.items = Vec::new();
            true
        } else {
            false
        }
    }

    pub fn forward(&mut self) -> bool {
        if self.cursor + 1 < self.items.len() {
            self.cursor += 1;
            true
        } else {
            false
        }
    }
}

#[derive(PartialEq)]
enum JumpElement {
    Compounds(usize),
    UsedIn(usize),
    None,
}

struct App<'a> {
    repo: &'a dyn KanjiRepo,
    should_quit: bool,
    search_results: Vec<&'a KanjiData>,
    current_kanji: usize,
    active_search: SearchCriteria,
    jump_selected: JumpElement,

    compounds_state: ListState,
    anim_state: KanjiAnimState,
    active_popup: ActivePopup,
    search_form: Option<SearchForm>,
    jump_history: History,
}

impl<'a> App<'a> {
    fn new(repo: &'a dyn KanjiRepo) -> Self {
        let default_search = SearchCriteria::default();
        Self {
            repo,
            should_quit: false,
            search_results: repo.search(&default_search),
            current_kanji: 0,
            active_search: SearchCriteria::default(),
            compounds_state: ListState::default(),
            anim_state: KanjiAnimState::new(60),
            active_popup: ActivePopup::Nope,
            search_form: None,
            jump_history: History::new(),
            jump_selected: JumpElement::None,
        }
    }

    fn perform_search(&mut self) {
        self.active_popup = ActivePopup::Nope;
        self.search_form = None;
        self.search_results = self.repo.search(&self.active_search);
        self.current_kanji = 0;
    }

    fn reset_jump(&mut self) {
        self.compounds_state.select(None);
        self.jump_selected = JumpElement::None
    }

    fn current_kanji(&self) -> Option<&KanjiData> {
        if self.jump_history.items.is_empty() {
            self.search_results.get(self.current_kanji).copied()
        } else {
            self.repo.get_by_kanji(self.jump_history.current())
        }
    }

    fn current_radical(&self) -> Option<&'a KanjiData> {
        let kanji = self.current_kanji()?;
        let rad_idx = kanji.rad_index?;

        let radical_char = KANGXI_RADICALS[(rad_idx - 1) as usize];

        self.repo.get_by_kanji(radical_char)
    }

    fn show_current_radical(&mut self) {
        if let Some(kanji) = self.current_kanji()
            && let Some(rad_index) = kanji.rad_index
        {
            let radical_char = KANGXI_RADICALS[(rad_index - 1) as usize];
            self.display_jump(radical_char)
        }
    }

    pub fn move_left(&mut self) {
        match &mut self.jump_selected {
            JumpElement::Compounds(idx) => *idx = idx.saturating_sub(1),
            JumpElement::UsedIn(idx) => {
                let new_idx = idx.saturating_sub(1);
                self.compounds_state.select(Some(new_idx));
                *idx = new_idx
            },
            JumpElement::None => {}
        }
    }

    pub fn move_right(&mut self) {
        let curr_kanji = if self.jump_history.items.is_empty() {
            self.search_results.get(self.current_kanji).copied()
        } else {
            self.repo.get_by_kanji(self.jump_history.current())
        };
        if let Some(kanji) = curr_kanji {
            self.jump_selected = match &self.jump_selected {
                JumpElement::Compounds(idx) => {
                    JumpElement::Compounds((*idx + 1).min(kanji.decomposition.len() - 1))
                }
                JumpElement::UsedIn(idx) => {
                    let new_idx = (*idx + 1).min(kanji.used_in.len() - 1);
                    self.compounds_state.select(Some(new_idx));
                    JumpElement::UsedIn(new_idx)
                }
                JumpElement::None => JumpElement::None,
            }
        }
    }

    fn show_current_jump(&mut self) {
        if let Some(kanji) = self.current_kanji() {
            let target = match self.jump_selected {
                JumpElement::Compounds(idx) => kanji.decomposition.get(idx),
                JumpElement::UsedIn(idx) => kanji.used_in.get(idx),
                _ => None,
            };
            if let Some(&target_kanji) = target {
                self.display_jump(target_kanji);
            }
        }
    }

    fn display_jump(&mut self, kanji: char) {
        if self.jump_history.push(kanji) {
            self.anim_state.reset();

            if let Some(new_kanji) = self.current_kanji() {
                if new_kanji.strokes.is_empty() {
                    self.anim_state.is_finished = true;
                }
            }
        }
    }

    pub fn undo_jump(&mut self) {
        if self.jump_history.back() {
            self.anim_state.reset();
            self.anim_state.is_finished = self
                .repo
                .get_by_index(self.current_kanji)
                .is_some_and(|k| k.strokes.is_empty());
        }
    }

    pub fn redo_jump(&mut self) {
        if self.jump_history.forward() {
            self.anim_state.reset();
            self.anim_state.is_finished = self
                .repo
                .get_by_index(self.current_kanji)
                .is_some_and(|k| k.strokes.is_empty());
        }
    }

    fn next_kanji(&mut self) {
        if !self.search_results.is_empty() {
            if self.current_kanji == self.search_results.len() - 1 {
                self.current_kanji = 0;
            } else {
                self.current_kanji = self.current_kanji + 1;
            }
        }
        self.anim_state.is_finished = self
            .search_results
            .get(self.current_kanji)
            .is_some_and(|k| k.strokes.is_empty());
        self.anim_state.reset();
    }

    fn previous_kanji(&mut self) {
        if !self.search_results.is_empty() {
            if self.current_kanji == 0 {
                self.current_kanji = self.search_results.len() - 1;
            } else {
                self.current_kanji -= 1;
            }
            self.anim_state.is_finished = self
                .search_results
                .get(self.current_kanji)
                .is_some_and(|k| k.strokes.is_empty());
            self.anim_state.reset();
        }
    }

    fn to_next_stroke(&mut self) {
        if let Some(kanji) = self.current_kanji() {
            let total_strokes = kanji.strokes.len();
            if total_strokes == 0 || self.anim_state.current_stroke == total_strokes {
                return;
            }
            let next_idx = if self.anim_state.current_stroke + 1 <= total_strokes {
                self.anim_state.current_stroke + 1
            } else {
                total_strokes
            };
            self.anim_state.play_stroke(next_idx);
        }
    }

    fn to_prev_stroke(&mut self) {
        if let Some(kanji) = self.current_kanji() {
            if kanji.strokes.is_empty() || self.anim_state.current_stroke == 0 {
                return;
            }
            let prev_idx = self.anim_state.current_stroke.saturating_sub(1);
            self.anim_state.play_stroke(prev_idx);
        }
    }

    //Draws the UI
    fn render(&mut self, frame: &mut Frame) {
        let layout = define_layout(frame.area(), self.current_kanji());
        let current_kanji = self.current_kanji();
        let current_radical = self.current_radical();

        if let Some(kanji) = current_kanji {
            render_kanji_canvas(frame, layout.canvas, kanji, &self);
            render_radical_section(frame, layout.radical, current_radical);
            render_stats_section(frame, layout.stats, kanji);
            render_details_section(frame, layout.details, kanji, &self);

            let compounds: Vec<&KanjiData> = kanji
                .used_in
                .iter()
                .filter_map(|kanji_symbol| self.repo.get_by_kanji(*kanji_symbol))
                .collect();

            render_used_in_section(frame, layout.used_in, compounds, &mut self.compounds_state);
        } else {
            render_no_kanji(frame)
        }
        match self.active_popup {
            ActivePopup::Help => {
                render_help_popup(frame, layout.canvas.height + layout.details.height)
            }
            ActivePopup::Search => {
                if let Some(search_form) = &self.search_form {
                    render_search_form(
                        frame,
                        layout.canvas.height + layout.details.height,
                        search_form,
                    );
                }
            }
            ActivePopup::SearchRad => {
                if let Some(search_form) = &mut self.search_form {
                    render_search_rad_form(
                        frame,
                        layout.canvas.height + layout.details.height,
                        search_form,
                    );
                }
            }
            ActivePopup::History => {
                render_history_popup(frame,
                                    &self.jump_history,
                            layout.canvas.height + layout.details.height,
                    );
            }
            ActivePopup::Nope => {}
        }
    }

    /// Main loop, to redraw the UI
    fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let tick_rate = Duration::from_millis(30);
        while !self.should_quit {
            terminal.draw(|frame| self.render(frame))?;
            if event::poll(tick_rate)? {
                if let Event::Key(key) = event::read()? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key(key);
                    }
                }
            } else {
                let current_kanji = if self.jump_history.items.is_empty() {
                    self.search_results.get(self.current_kanji).copied()
                } else {
                    self.repo.get_by_kanji(self.jump_history.current())
                };
                if let Some(kanji) = current_kanji {
                    self.anim_state.tick(kanji);
                }
            }
        }
        Ok(())
    }

    //Handle user interaction
    fn handle_key(&mut self, key: KeyEvent) {
        if let Some(search_form) = &mut self.search_form {
            if search_form.is_editing {
                match key.code {
                    KeyCode::Right => search_form.cursor_right(),
                    KeyCode::Left => search_form.cursor_left(),
                    KeyCode::Enter => {
                        self.compounds_state.select(None);
                        self.jump_selected = JumpElement::None;
                        self.active_search = search_form.draft.clone();
                        self.perform_search();
                    }
                    KeyCode::Esc => {
                        search_form.cursor_offset = 0;
                        search_form.is_editing = false
                    }
                    KeyCode::Tab => search_form.move_right_tab(),
                    KeyCode::Delete => search_form.handle_delete(),
                    KeyCode::Backspace => search_form.handle_backspace(),
                    KeyCode::Char(c) => search_form.handle_input(c),
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Esc => {
                        self.active_popup = ActivePopup::Nope;
                        self.search_form = None;
                    }
                    KeyCode::Char('>') => {
                        self.active_popup = ActivePopup::SearchRad;
                        if let Some(form) = &mut self.search_form {
                            form.cursor_offset = 0;
                            form.is_editing = false;
                            form.active_field = SearchField::Radical(0);
                        }
                    }
                    KeyCode::Char('<') => {
                        self.active_popup = ActivePopup::Search;
                        if let Some(form) = &mut self.search_form {
                            form.cursor_offset = 0;
                            form.is_editing = false;
                            form.active_field = SearchField::Jlpt(0);
                        }
                    }
                    KeyCode::Char(' ') => search_form.toggle_current(),
                    KeyCode::Up => search_form.move_up(),
                    KeyCode::Right => search_form.move_right(),
                    KeyCode::Down => search_form.move_down(),
                    KeyCode::Left => search_form.move_left(),
                    KeyCode::Enter => {
                        self.compounds_state.select(None);
                        self.jump_selected = JumpElement::None;
                        self.active_search = search_form.draft.clone();
                        self.perform_search();
                    }
                    _ => {}
                }
            }
        } else {
            match key.code {
                KeyCode::Tab => {
                    self.jump_selected = match self.jump_selected {
                        JumpElement::Compounds(_) => {
                            self.compounds_state.select(Some(0));
                            JumpElement::UsedIn(0)
                        },
                        _ => {
                            self.compounds_state.select(None);
                            JumpElement::Compounds(0)
                        },
                    }
                }
                KeyCode::Char('h') => {
                    self.active_popup = match self.active_popup {
                        ActivePopup::History => ActivePopup::Nope,
                        _ => ActivePopup::History,
                    };
                },
                KeyCode::Esc => self.reset_jump(),
                KeyCode::Char('[') => self.move_left(),
                KeyCode::Char(']') => self.move_right(),
                KeyCode::Enter => {
                    self.show_current_jump();
                    self.reset_jump();
                }
                KeyCode::Char('s') => {
                    self.reset_jump();
                    self.active_popup = ActivePopup::Search;
                    self.search_form = Some(SearchForm::new(&self.active_search));
                }
                KeyCode::Char('d') => {
                    self.active_popup = match self.active_popup {
                        ActivePopup::Help => ActivePopup::Nope,
                        _ => ActivePopup::Help,
                    };
                }
                KeyCode::Char('k') => self.anim_state.is_finished = true,
                KeyCode::Char('r') => {
                    self.reset_jump();
                    self.show_current_radical();
                }
                KeyCode::Char('u') => {
                    self.reset_jump();
                    self.undo_jump();
                }
                KeyCode::Char('y') => {
                    self.reset_jump();
                    self.redo_jump();
                }
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('p') => {
                    self.anim_state.single_stroke_mode = false;
                    self.anim_state.is_paused = !self.anim_state.is_paused
                }
                KeyCode::Char('o') => self.anim_state.reset(),
                KeyCode::Up => self.to_next_stroke(),
                KeyCode::Down => self.to_prev_stroke(),
                KeyCode::Left => {
                    self.reset_jump();
                    self.previous_kanji();
                },
                KeyCode::Right => {
                    self.reset_jump();
                    self.next_kanji();
                }
                _ => {}
            }
        }
    }
}

/// App entry point
fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let repo = &InMemoryJson::new();
    let mut app = App::new(repo);
    let app_result = app.run(&mut terminal);
    ratatui::restore();

    app_result
}

fn define_layout(area: Rect, cur_kanji: Option<&KanjiData>) -> MainLayout {
    let details_height = cur_kanji
        .map(|k| calc_details_height(k, (area.width as f32 * 0.65) as u16))
        .unwrap_or(12);

    let [kanji_section, details_section] =
        Layout::vertical([Constraint::Length(10), Constraint::Length(details_height)])
            .spacing(0)
            .areas(area);

    let top_height = kanji_section.height;
    let kanji_width = (top_height.saturating_sub(2) * 2) + 2;

    let [canvas, stats, radical] = Layout::horizontal([
        Constraint::Length(kanji_width),
        Constraint::Length(29),
        Constraint::Fill(1),
    ])
    .spacing(0)
    .areas(kanji_section);

    let [details, used_in] = Layout::horizontal([Constraint::Percentage(65), Constraint::Fill(35)])
        .spacing(0)
        .areas(details_section);

    MainLayout {
        canvas,
        radical,
        stats,
        details,
        used_in,
    }
}

fn base_layout_block<'a>(title: &'a str) -> Block<'a> {
    let offset_x = 0;
    let title_text = format!("{}{}", "═".repeat(offset_x), title);

    let block = Block::default()
        .padding(Padding::vertical(1))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .title(TLine::from(vec![Span::styled(
            title_text,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]));

    block
}

fn render_kanji_canvas(frame: &mut Frame, area: Rect, kanji: &KanjiData, app: &App) {
    let tot_kanji = app.search_results.len();
    let block = base_layout_block("Kanji").padding(Padding::ZERO);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if kanji.strokes.is_empty() {
        let vertical_center: [Rect; 3] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(inner_area);

        let paragraph = Paragraph::new(kanji.kanji.to_string())
            .alignment(Alignment::Center)
            .style(
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            );

        frame.render_widget(paragraph, vertical_center[1]);
        return;
    }

    let [canvas_area, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner_area);

    let canvas_widget = Canvas::default()
        .x_bounds([8.0, 101.0])
        .y_bounds([8.0, 101.0])
        .marker(Marker::Braille)
        .paint(|ctx| {
            if app.anim_state.is_finished {
                for stroke in &kanji.strokes {
                    draw_stroke_points(ctx, &stroke.strokes, stroke.strokes.len(), Color::Red);
                }
            } else {
                let max_stroke = app.anim_state.current_stroke.min(kanji.strokes.len());

                for s_idx in 0..max_stroke {
                    let stroke = &kanji.strokes[s_idx];
                    draw_stroke_points(ctx, &stroke.strokes, stroke.strokes.len(), Color::Gray);
                }

                if app.anim_state.current_stroke < kanji.strokes.len() {
                    let current_stroke = &kanji.strokes[app.anim_state.current_stroke];

                    let points_to_draw = app.anim_state
                        .current_point
                        .saturating_add(1)
                        .min(current_stroke.strokes.len());

                    draw_stroke_points(ctx, &current_stroke.strokes, points_to_draw, Color::Red);
                }
            }
        });

    frame.render_widget(canvas_widget, canvas_area);

    let display_stroke = if app.anim_state.current_point == 0 && app.anim_state.current_stroke > 0 {
        app.anim_state.current_stroke
    } else {
        (app.anim_state.current_stroke + 1).min(kanji.strokes.len())
    };
    let title = format!("{}/{}",app.current_kanji + 1, tot_kanji);
    let footer_content = if app.anim_state.is_finished {
        format!("{} {}",kanji.kanji,title)
    } else {
        format!("Stroke: {}/{}", display_stroke, kanji.strokes.len())
    };

    let paragraph = Paragraph::new(footer_content).alignment(Alignment::Center);
    frame.render_widget(paragraph, footer_area);
}

fn render_radical_section(frame: &mut Frame, area: Rect, radical: Option<&KanjiData>) {
    if let Some(rad) = radical {
        let indent = 10 as usize;
        let max_length = (area.width as usize).saturating_sub(2 + 1 + indent);
        let options = Options::new(max_length).break_words(false);

        let symbol = rad.kanji.to_string();
        let names = if rad.names.is_empty() {
            "-".to_string()
        } else {
            textwrap::fill(rad.names.join(", ").as_ref(), max_length)
        };
        let variants = if rad.variants.is_empty() {
            "-".to_string()
        } else {
            let decomp: String = rad
                .variants
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let variants: &str = &decomp;
            textwrap::fill(variants, &options)
        };
        let meanings = if rad.meanings.is_empty() {
            "-".to_string()
        } else {
            textwrap::fill(rad.meanings.join(", ").as_ref(), max_length)
        };

        let mut lines = Vec::new();
        lines.push(TLine::from(vec![
            Span::styled(
                format!(" {:<indent$}", "Symbol:"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(symbol),
        ]));
        lines.push(TLine::from(vec![
            Span::styled(
                format!(" {:<indent$}", "Name:"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(names),
        ]));
        append_list_item(&mut lines, "Meanings:", &meanings, Some(indent));
        append_list_item(&mut lines, "Variants:", &variants, Some(indent));

        let paragraph = Paragraph::new(lines).block(base_layout_block("Radical"));
        frame.render_widget(paragraph, area);
    } else {
        let text_lines = vec![
            TLine::from(""),
            TLine::styled("No radical info", Style::default().fg(Color::DarkGray)),
        ];

        let paragraph = Paragraph::new(text_lines).block(base_layout_block("Radical"));
        frame.render_widget(paragraph, area);
    }
}

fn render_stats_section(frame: &mut Frame, area: Rect, kanji: &KanjiData) {
    let jlpt_span = match kanji.jlpt_level {
        Some(level) => Span::raw(format!("N{level}")),
        None => Span::styled("not available", Style::default().fg(Color::DarkGray)),
    };

    let grade_span = Span::raw(kanji.grade_detail());

    let stokes_span = match kanji.strokes_count {
        Some(strokes) => Span::raw(strokes.to_string()),
        None => Span::styled("not available", Style::default().fg(Color::DarkGray)),
    };

    let freq_span = match kanji.freq_rank {
        Some(freq_rank) => Span::raw(format!("#{freq_rank}/2501")),
        None => Span::styled("not available", Style::default().fg(Color::DarkGray)),
    };

    let lines = vec![
        TLine::from(vec![
            Span::styled(
                format!("{:<10}", " JLPT:"),
                Style::default().fg(Color::DarkGray),
            ),
            jlpt_span,
        ]),
        TLine::from(vec![
            Span::styled(
                format!("{:<10}", " Grade:"),
                Style::default().fg(Color::DarkGray),
            ),
            grade_span,
        ]),
        TLine::from(vec![
            Span::styled(
                format!("{:<10}", " Strokes:"),
                Style::default().fg(Color::DarkGray),
            ),
            stokes_span,
        ]),
        TLine::from(vec![
            Span::styled(
                format!("{:<10}", " Ranking:"),
                Style::default().fg(Color::DarkGray),
            ),
            freq_span,
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(base_layout_block("Statistics"));
    frame.render_widget(paragraph, area);
}

fn render_details_section(frame: &mut Frame, area: Rect, kanji: &KanjiData, app: &App) {
    let dec_title = "Compounds:";
    let on_title = "Onyomi:";
    let kun_title = "Kunyomi:";
    let nano_title = "Nanori:";
    let mean_title = "Meanings:";

    let decomp_lines: Vec<TLine> = if kanji.decomposition.is_empty() {
        vec![TLine::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(dec_title, Style::default().fg(Color::DarkGray)),
            Span::styled(" not available", Style::default().fg(Color::DarkGray)),
        ])]
    } else {
        let max_width = (area.width as usize).saturating_sub(2);

        let indent_str = " ".repeat(dec_title.len() + 2);
        let indent_width = indent_str.len();

        let mut lines = Vec::new();
        let mut current_spans = Vec::new();

        let title_text = format!(" {} ", dec_title);
        let mut current_width = title_text.len();
        current_spans.push(Span::styled(
            title_text,
            Style::default().fg(Color::DarkGray),
        ));

        let last_idx = kanji.decomposition.len() - 1;

        for (idx, n) in kanji.decomposition.iter().enumerate() {
            let is_selected = app.jump_selected == JumpElement::Compounds(idx);
            let style = if is_selected {
                Style::default().fg(Color::Black).bg(Color::Yellow)
            } else {
                Style::default().fg(Color::Reset)
            };

            let elem_text = format!(" {} ", n);
            let sep_text = if idx < last_idx { "+" } else { "" };

            let item_width = elem_text.len() + sep_text.len();

            if current_width + item_width > max_width && !current_spans.is_empty() {
                lines.push(TLine::from(current_spans));
                current_spans = vec![Span::raw(indent_str.clone())];
                current_width = indent_width;
            }

            current_spans.push(Span::styled(elem_text, style));

            if !sep_text.is_empty() {
                current_spans.push(Span::styled(sep_text, Style::default().fg(Color::DarkGray)));
            }

            current_width += item_width;
        }

        if !current_spans.is_empty() {
            lines.push(TLine::from(current_spans));
        }

        lines
    };

    let onyomi = if kanji.onyomi.is_empty() {
        "-".to_string()
    } else {
        let max_length = (area.width as usize).saturating_sub(2 + 2 + on_title.len());
        let options = Options::new(max_length).break_words(false);
        textwrap::fill(&kanji.onyomi.join(", "), &options)
    };

    let kunyomi = if kanji.kunyomi.is_empty() {
        "-".to_string()
    } else {
        let max_length = (area.width as usize).saturating_sub(2 + 2 + kun_title.len());
        let options = Options::new(max_length).break_words(false);
        textwrap::fill(&kanji.kunyomi.join(", "), &options)
    };

    let title_len = nano_title.len();
    let indent_size = title_len + 1;
    let prefix_cols = 1 + indent_size;

    let total_non_text_cols = prefix_cols + 2;
    let text_width = (area.width as usize).saturating_sub(total_non_text_cols);

    let nanori = if kanji.nanori.is_empty() {
        "-".to_string()
    } else if text_width > 0 {
        let options = textwrap::Options::new(text_width).break_words(false);
        textwrap::fill(&kanji.nanori.join(", "), &options)
    } else {
        kanji.nanori.join(", ")
    };

    let meanings = if kanji.meanings.is_empty() {
        "-".to_string()
    } else {
        let max_length = (area.width as usize).saturating_sub(2 + 2 + mean_title.len());
        let options = Options::new(max_length).break_words(false);
        textwrap::fill(&kanji.meanings.join(", "), &options)
    };

    let mut lines = Vec::new();
    lines.extend(decomp_lines);
    lines.push(TLine::raw(""));
    append_list_item(&mut lines, on_title, &onyomi, None);
    append_list_item(&mut lines, kun_title, &kunyomi, None);
    append_list_item(&mut lines, nano_title, &nanori, None);
    lines.push(TLine::raw(""));
    append_list_item(&mut lines, mean_title, &meanings, None);

    let paragraph = Paragraph::new(lines).block(base_layout_block("Reading & Meanings"));
    frame.render_widget(paragraph, area);
}

fn render_used_in_section(
    frame: &mut Frame,
    area: Rect,
    compounds: Vec<&KanjiData>,
    compounds_state: &mut ListState,
) {
    if compounds.is_empty() {
        let empty_item = vec![ListItem::new("No data available")];
        let list = List::new(empty_item).block(base_layout_block("Used in"));
        frame.render_widget(list, area);
        return;
    }

    let max_length = (area.width as usize).saturating_sub(4);
    let options = Options::new(max_length).break_words(false);

    let items: Vec<ListItem> = compounds
        .iter()
        .map(|c| {
            let mut lines = Vec::new();
            let line = format!(
                "{} ({}) - {}",
                c.kanji,
                c.onyomi.join(", "),
                c.meanings.join(", ")
            );
            let content = textwrap::fill(&line, &options);
            append_list_item(&mut lines, "", &content, Some(0));
            ListItem::new(Text::from(lines))
        })
        .collect();

    let list = List::new(items)
        .block(base_layout_block("Used in"))
        .highlight_style(Style::default().bg(Color::Indexed(237)).fg(Color::Yellow))
        .highlight_symbol("❯");

    frame.render_stateful_widget(list, area, compounds_state);
}

fn append_list_item(lines: &mut Vec<TLine>, title: &str, text: &str, indent: Option<usize>) {
    let mut lines_iter = text.lines();

    let indent = if let Some(indent) = indent {
        indent
    } else {
        title.len() + 1
    };

    if let Some(first_line) = lines_iter.next() {
        lines.push(TLine::from(vec![
            Span::styled(
                format!(" {:<indent$}", title),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(first_line.to_string()),
        ]));

        let padding = " ".repeat(1 + indent);

        for next_line in lines_iter {
            let content = format!("{}{}", padding, next_line);
            lines.push(TLine::from(Span::raw(content)));
        }
    }
}

fn draw_stroke_points(ctx: &mut Context, points: &[MyPoint], max_points: usize, color: Color) {
    if points.len() < 2 {
        return;
    }

    let limit = max_points.min(points.len());
    for i in 0..limit.saturating_sub(1) {
        let p1 = &points[i];
        let p2 = &points[i + 1];
        ctx.draw(&Line {
            x1: p1.x as f64,
            y1: 109.0 - p1.y as f64,
            x2: p2.x as f64,
            y2: 109.0 - p2.y as f64,
            color: color,
        });
    }
}

fn calc_field_lines(text: &str, target_width: usize) -> usize {
    if text.is_empty() || target_width == 0 {
        return 1;
    }
    let options = Options::new(target_width).break_words(false);
    textwrap::wrap(text, &options).len().max(1)
}

fn calc_details_height(curr_kanji: &KanjiData, area_width: u16) -> u16 {
    let inner_width = (area_width as usize).saturating_sub(4);

    let dec_text = curr_kanji
        .decomposition
        .iter()
        .map(|c| c.to_string())
        .collect::<Vec<_>>()
        .join(" + ");

    let dec_lines = calc_field_lines(&dec_text, inner_width.saturating_sub("Compounds: ".len()));
    let on_lines = calc_field_lines(
        &curr_kanji.onyomi.join(", "),
        inner_width.saturating_sub("Onyomi: ".len()),
    );
    let kun_lines = calc_field_lines(
        &curr_kanji.kunyomi.join(", "),
        inner_width.saturating_sub("Kunyomi: ".len()),
    );
    let nano_lines = calc_field_lines(
        &curr_kanji.nanori.join(", "),
        inner_width.saturating_sub("Nanori: ".len()),
    );
    let mean_lines = calc_field_lines(
        &curr_kanji.meanings.join(", "),
        inner_width.saturating_sub("Meanings: ".len()),
    );

    let total_text_lines = dec_lines + on_lines + kun_lines + nano_lines + mean_lines;

    (total_text_lines + 6) as u16
}

fn render_help_popup(frame: &mut Frame, used_area: u16) {
    let popup_area = centered_rect(91, 70, frame.area(), used_area);

    frame.render_widget(Clear, popup_area);

    let explanation = Text::from(vec![
        TLine::from("Grade:"),
        TLine::from(" Kyōiku: kanji taught during the six years of elementary school"),
        TLine::from(
            " Jōyō: Kyōiku kanji plus the remaining (total 2136), to be learned by the end of high school",
        ),
        TLine::from(
            " Jinmeiyō: kanji approved to be used in family name, registers and other official documents",
        ),
        TLine::from(
            " Hyōgai: kanji that fall outside of the list Jōyō + Jinmeiyō, are very rare or old",
        ),
        TLine::from(""),
        TLine::from(
            "The radical is the core component of a kanji, it shares its meaning and it used to classify it",
        ),
        TLine::from(
            "The rank expresses the relative frequency of occurrence of a kanji in modern Japanese",
        ),
        TLine::from("Kanji variants (異体字), are alternative graphical forms of the same kanji"),
        TLine::from("Compounds section is a list of kanji used inside the current kanji"),
        TLine::from(
            "Onyomi is the Chinese reading of a kanji, often used for words made of multiple kanji",
        ),
        TLine::from("Kunyomi is the native Japanese reading, primarly used for kanji used alone"),
        TLine::from("Nanori is the reading used almost exclusively in Japanese proper names"),
        TLine::from("Used in section is a list of kanji containg the current kanji"),
    ]);

    let help_widget =
        Paragraph::new(explanation).block(Block::default().title("Details").borders(Borders::ALL));

    frame.render_widget(help_widget, popup_area);
}

fn render_search_form(frame: &mut Frame, used_area: u16, form: &SearchForm) {
    let popup_area = centered_rect(91, 70, frame.area(), used_area);
    frame.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .title("Filters")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner_area = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let chunks = Layout::vertical([
        Constraint::Length(2), // JLPT
        Constraint::Length(2), // Grade
        Constraint::Length(2), // Strokes
        Constraint::Length(2), // Rank
        Constraint::Length(1), // Query
    ])
    .split(inner_area);

    //JLPT
    let mut jlpt_spans = vec![Span::styled(
        " JLPT Level: ",
        Style::default().fg(Color::DarkGray),
    )];

    let jlpt_items: Vec<Span> = (1..=6)
        .enumerate()
        .flat_map(|(idx, n)| {
            let is_selected = form
                .draft
                .jlpt
                .as_ref()
                .map_or(false, |set| set.contains(&JlptLevel::N(n)));

            let is_focused = form.active_field == SearchField::Jlpt(idx);

            let style = match (is_focused, is_selected) {
                (true, true) => Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                (true, false) => Style::default().fg(Color::Black).bg(Color::Yellow),
                (false, true) => Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                (false, false) => Style::default().fg(Color::Reset),
            };

            let label = if n == 6 {
                "Not available"
            } else {
                &format!("N{n}")
            };
            let text = if is_selected {
                format!("[{label}]")
            } else {
                format!("{label}")
            };

            let span = Span::styled(text, style);
            if n < 6 {
                vec![
                    span,
                    Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                ]
            } else {
                vec![span]
            }
        })
        .collect();

    jlpt_spans.extend(jlpt_items);

    let jlpt_widget = Paragraph::new(TLine::from(jlpt_spans));
    frame.render_widget(jlpt_widget, chunks[0]);

    //Grade
    let mut grade_spans = vec![Span::styled(
        " Grade: ",
        Style::default().fg(Color::DarkGray),
    )];
    let grade_items: Vec<Span> = KanjiGrade::descs()
        .into_iter()
        .flat_map(|(r, d, idx)| {
            let is_focused = form.active_field == SearchField::Grade(*idx);
            let is_selected = form
                .draft
                .grade
                .as_ref()
                .map_or(false, |set| set.contains(r));

            let style = match (is_focused, is_selected) {
                (true, true) => Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
                (true, false) => Style::default().fg(Color::Black).bg(Color::Yellow),
                (false, true) => Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
                (false, false) => Style::default().fg(Color::Reset),
            };

            let text = if is_selected {
                format!("[{d}]")
            } else {
                format!("{d}")
            };

            let span = Span::styled(text, style);
            if (*idx as u8) < 8 {
                vec![
                    span,
                    Span::styled(" | ", Style::default().fg(Color::DarkGray)),
                ]
            } else {
                vec![span]
            }
        })
        .collect();

    grade_spans.extend(grade_items);

    let grade_widget = Paragraph::new(TLine::from(grade_spans));
    frame.render_widget(grade_widget, chunks[1]);

    //Strokes
    let (min_stroke, max_stroke) = format_range(form.draft.strokes.as_ref());
    let stroke_prefix = " Number of strokes: ";
    let stroke_widget = Paragraph::new(TLine::from(vec![
        Span::styled(stroke_prefix, Style::default().fg(Color::DarkGray)),
        Span::raw(format!("min {} max {}", min_stroke, max_stroke)),
    ]));
    frame.render_widget(stroke_widget, chunks[2]);

    //Rank
    let (min_rank, max_rank) = format_range(form.draft.rank.as_ref());
    let rank_prefix = " Rank: ";
    let rank_widget = Paragraph::new(TLine::from(vec![
        Span::styled(rank_prefix, Style::default().fg(Color::DarkGray)),
        Span::raw(format!("min {} max {}", min_rank, max_rank)),
    ]));
    frame.render_widget(rank_widget, chunks[3]);

    //Text
    let query_prefix = " Textual search: ";
    let query_text = form.draft.query_text.as_deref().unwrap_or("_");
    let text_widget = Paragraph::new(TLine::from(vec![
        Span::styled(query_prefix, Style::default().fg(Color::DarkGray)),
        Span::raw(query_text),
    ]));
    frame.render_widget(text_widget, chunks[4]);

    let label_width = 4;

    let (prefix_len, chunk) = match form.active_field {
        SearchField::Strokes(RangeSubfield::Min) => {
            (stroke_prefix.len() + label_width, Some(chunks[2]))
        }
        SearchField::Strokes(RangeSubfield::Max) => (
            stroke_prefix.len() + 2 * label_width + 1 + min_stroke.to_string().len(),
            Some(chunks[2]),
        ),
        SearchField::Rank(RangeSubfield::Min) => (rank_prefix.len() + label_width, Some(chunks[3])),
        SearchField::Rank(RangeSubfield::Max) => (
            rank_prefix.len() + 2 * label_width + 1 + min_rank.to_string().len(),
            Some(chunks[3]),
        ),
        SearchField::Query => (query_prefix.len(), Some(chunks[4])),
        _ => (0, None),
    };

    if let Some(target_chunk) = chunk {
        let cursor_x = target_chunk.x + prefix_len as u16 + form.cursor_offset as u16;
        let cursor_y = target_chunk.y;
        frame.set_cursor_position((cursor_x, cursor_y));
        if form.is_editing {
            execute!(stdout(), SetCursorStyle::BlinkingBar).ok();
        } else {
            execute!(stdout(), SetCursorStyle::BlinkingBlock).ok();
        }
    }
}

fn render_search_rad_form(frame: &mut Frame, used_area: u16, form: &mut SearchForm) {
    let popup_area = centered_rect(91, 57, frame.area(), used_area);
    frame.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .title("Radical filter")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner_area = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let mut lines: Vec<TLine> = Vec::new();
    let mut current_spans: Vec<Span> = Vec::new();

    let cell_width: u16 = 4;
    let max_columns = ((inner_area.width / cell_width) as usize).max(1);
    let grid_width = (max_columns as u16) * cell_width;

    form.max_colums = max_columns;

    let total_items = KANGXI_RADICALS.len();
    let num_rows = total_items.div_ceil(max_columns);
    let grid_height = num_rows as u16;

    for (idx, radical) in KANGXI_RADICALS.iter().enumerate() {
        let is_selected = form
            .draft
            .rad_index
            .as_ref()
            .map_or(false, |set| set.contains(&(idx as u8)));

        let is_focused = form.active_field == SearchField::Radical(idx);

        let style = match (is_focused, is_selected) {
            (true, true) => Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            (true, false) => Style::default().fg(Color::Black).bg(Color::Yellow),
            (false, true) => Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            (false, false) => Style::default().fg(Color::Reset),
        };

        let text = format!(" {} ", radical);

        current_spans.push(Span::styled(text, style));

        if (idx + 1) % max_columns as usize == 0 || idx + 1 == KANGXI_RADICALS.len() {
            lines.push(TLine::from(current_spans.clone()));
            current_spans.clear();
        }
    }

    let grid_paragraph = Paragraph::new(lines);

    let vertical_chunks = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(grid_height.min(inner_area.height)),
        Constraint::Fill(1),
    ])
    .split(inner_area);

    let grid_area = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(grid_width.min(inner_area.width)),
        Constraint::Fill(1),
    ])
    .split(vertical_chunks[1])[1];

    frame.render_widget(grid_paragraph, grid_area);
}

fn render_no_kanji(frame: &mut Frame) {
    let [view] = Layout::vertical([Constraint::Length(22)])
        .spacing(0)
        .areas(frame.area());
    let block = base_layout_block("").padding(Padding::ZERO);

    frame.render_widget(block, view);
}

fn render_history_popup(frame: &mut Frame, history: &History, used_height: u16) {

    let target_area: Rect = Rect {
        x: frame.area().x,
        y: frame.area().y,
        width: frame.area().width,
        height: frame.area().height.min(used_height),
    };

    let [_, history_area, title_area] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Percentage(4),
        Constraint::Length(1),
    ])
    .areas(target_area);

    for (i, char) in " Jump History ".chars().enumerate() {
        let char_y = title_area.y +(used_height/2 - 7) + i as u16;
        if char_y >= title_area.height { break; }

        let char_widget = Paragraph::new(char.to_string());
        frame.render_widget(char_widget, Rect::new(title_area.x, char_y, 1, 1));
    }

    frame.render_widget(Clear, history_area);

    let items: Vec<ListItem> = history.items
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if i == history.cursor {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(format!("{}", entry))
                .style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .title("")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(list, history_area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect, used_height: u16) -> Rect {
    let effective_r = Rect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height.min(used_height),
    };
    let popup_layout = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(effective_r);

    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(popup_layout[1])[1]
}

fn format_range<T: std::fmt::Display>(range: Option<&RangeFilter<T>>) -> (String, String) {
    let min = range
        .and_then(|r| r.min.as_ref())
        .map_or("_".to_string(), |v| v.to_string());
    let max = range
        .and_then(|r| r.max.as_ref())
        .map_or("_".to_string(), |v| v.to_string());
    (min, max)
}

fn extract_range_len<T: std::fmt::Display>(
    filter: Option<&RangeFilter<T>>,
    subfield: &RangeSubfield,
) -> usize {
    let val = filter.and_then(|f| match subfield {
        RangeSubfield::Min => f.min.as_ref(),
        RangeSubfield::Max => f.max.as_ref(),
    });

    val.map_or(0, |v| v.to_string().len())
}

fn backspace_numeric<T: std::fmt::Display + std::str::FromStr>(
    val: &mut Option<T>,
    cursor_pos: usize,
) -> bool {
    if cursor_pos == 0 {
        return false;
    }

    if let Some(current) = val {
        let mut s = current.to_string();
        let char_idx = cursor_pos - 1;

        if let Some((byte_idx, _)) = s.char_indices().nth(char_idx) {
            s.remove(byte_idx);

            if s.is_empty() {
                *val = None;
            } else if let Ok(new_val) = s.parse::<T>() {
                *val = Some(new_val);
            }
            return true;
        }
    }
    false
}

fn delete_numeric<T: std::fmt::Display + std::str::FromStr>(
    val: &mut Option<T>,
    cursor_pos: usize,
) {
    if let Some(current) = val {
        let mut s = current.to_string();

        if let Some((byte_idx, _)) = s.char_indices().nth(cursor_pos) {
            s.remove(byte_idx);

            if s.is_empty() {
                *val = None;
            } else if let Ok(new_val) = s.parse::<T>() {
                *val = Some(new_val);
            }
        }
    }
}