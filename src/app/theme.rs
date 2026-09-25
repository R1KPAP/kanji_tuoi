use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Original,
    Cyberpunk,
}

impl ThemeMode {
    pub fn toggle(self) -> Self {
        match self {
            Self::Original => Self::Cyberpunk,
            Self::Cyberpunk => Self::Original,
        }
    }
}

pub struct Theme {
    pub bg: Color,
    pub rtext: Color,
    pub utext: Color,
    pub vtext: Color,
    pub htext: Color,
    pub btitle: Color,
    pub border: Color,
    pub kanji_stroke_new: Color,
    pub kanji_stroke_old: Color,
    pub text_muted: Color,
}

impl Theme {
    pub fn original() -> Self {
        Self {
            bg: Color::Rgb(238, 230, 217),
            rtext: Color::Rgb(107, 147, 98),
            vtext: Color::Rgb(245, 127, 79),
            utext: Color::Rgb(202, 105, 36),
            htext: Color::Rgb(252, 164, 116),
            btitle: Color::Rgb(155, 187, 95),
            border: Color::Rgb(191, 144, 0),
            kanji_stroke_new: Color::Rgb(40, 40, 40),
            kanji_stroke_old: Color::Rgb(224, 62, 0),
            text_muted: Color::Rgb(135, 100, 90),
        }
    }

    pub fn cyberpunk() -> Self {
        Self {
            bg: Color::Rgb(10, 10, 18),
            rtext: Color::Rgb(255, 130, 30),
            vtext: Color::Rgb(50, 255, 120),
            utext: Color::Rgb(255, 0, 127),
            htext: Color::Rgb(255, 255, 0),
            btitle: Color::Rgb(255, 255, 0),
            border: Color::Rgb(120, 50, 255),
            kanji_stroke_new: Color::Rgb(0, 255, 204),
            kanji_stroke_old: Color::Rgb(255, 0, 127),
            text_muted: Color::Rgb(80, 85, 110),
        }
    }

    pub fn from_mode(mode: ThemeMode) -> Self {
        match mode {
            ThemeMode::Original => Self::original(),
            ThemeMode::Cyberpunk => Self::cyberpunk(),
        }
    }
}
