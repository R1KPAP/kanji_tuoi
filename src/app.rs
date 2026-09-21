pub use search::{SearchCriteria, SearchField, SearchForm, RangeFilter};
pub use anim::KanjiAnimState;
pub use history::History;
use crate::model::KanjiData;
use crate::repo::KanjiRepo;
use crate::ui::*;
use crate::constants::KANGXI_RADICALS;
use ratatui::{DefaultTerminal, Frame, widgets::ListState};
use std::time::Duration;
use std::io;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};

pub mod anim;
pub mod history;
pub mod search;

#[derive(PartialEq)]
pub enum ActivePopup {
    SearchRad,
    Search,
    Nope,
    Help,
    History,
}

#[derive(PartialEq)]
pub enum JumpElement {
    Compounds(usize),
    UsedIn(usize),
    None,
}
pub struct App<'a> {
    pub repo: &'a dyn KanjiRepo,
    pub should_quit: bool,
    pub search_results: Vec<&'a KanjiData>,
    pub current_kanji: usize,
    pub active_search: SearchCriteria,
    pub jump_selected: JumpElement,

    pub compounds_state: ListState,
    pub anim_state: KanjiAnimState,
    pub active_popup: ActivePopup,
    pub search_form: Option<SearchForm>,
    pub jump_history: History,
}

impl<'a> App<'a> {
    pub fn new(repo: &'a dyn KanjiRepo) -> Self {
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
            }
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
                render_history_popup(
                    frame,
                    &self.jump_history,
                    layout.canvas.height + layout.details.height,
                );
            }
            ActivePopup::Nope => {}
        }
    }

    /// Main loop, to redraw the UI
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
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
                        }
                        _ => {
                            self.compounds_state.select(None);
                            JumpElement::Compounds(0)
                        }
                    }
                }
                KeyCode::Char('h') => {
                    self.active_popup = match self.active_popup {
                        ActivePopup::History => ActivePopup::Nope,
                        _ => ActivePopup::History,
                    };
                }
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
                }
                KeyCode::Right => {
                    self.reset_jump();
                    self.next_kanji();
                }
                _ => {}
            }
        }
    }
}