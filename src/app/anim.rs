use std::time::{Duration, Instant};
use crate::model::KanjiData;

pub struct KanjiAnimState {
    pub current_stroke: usize,
    pub current_point: usize,
    pub is_finished: bool,
    pub is_paused: bool,
    pub last_update: Instant,
    pub step_duration: Duration,
    pub stroke_delay_ticks: usize,
    pub single_stroke_mode: bool,
}

impl KanjiAnimState {
    pub fn new(step_ms: u64) -> Self {
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

    pub fn reset(&mut self) {
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

    pub fn tick(&mut self, kanji: &KanjiData) {
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