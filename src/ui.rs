use crate::model::{KanjiData, MyPoint};
use crate::app::History;
use crate::app::search::*;
use crate::utilities::format_range;
use crate::app::{App, JumpElement};
use crate::model::{JlptLevel, KanjiGrade};
use crate::constants::KANGXI_RADICALS;
use crossterm::cursor::SetCursorStyle;
use crossterm::execute;
use ratatui::layout::Alignment;
use ratatui::widgets::Padding;
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols::Marker,
    text::{Line as TLine, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph,
        canvas::{Canvas, Context, Line},
    },
};
use std::io::stdout;
use textwrap::Options;

pub struct MainLayout {
    pub canvas: Rect,
    pub radical: Rect,
    pub stats: Rect,
    pub details: Rect,
    pub used_in: Rect,
}

pub fn define_layout(area: Rect, cur_kanji: Option<&KanjiData>) -> MainLayout {
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

pub fn base_layout_block<'a>(title: &'a str) -> Block<'a> {
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

pub fn render_kanji_canvas(frame: &mut Frame, area: Rect, kanji: &KanjiData, app: &App) {
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

                    let points_to_draw = app
                        .anim_state
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
    let title = format!("{}/{}", app.current_kanji + 1, tot_kanji);
    let footer_content = if app.anim_state.is_finished {
        format!("{} {}", kanji.kanji, title)
    } else {
        format!("Stroke: {}/{}", display_stroke, kanji.strokes.len())
    };

    let paragraph = Paragraph::new(footer_content).alignment(Alignment::Center);
    frame.render_widget(paragraph, footer_area);
}

pub fn render_radical_section(frame: &mut Frame, area: Rect, radical: Option<&KanjiData>) {
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

pub fn render_stats_section(frame: &mut Frame, area: Rect, kanji: &KanjiData) {
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

pub fn render_details_section(frame: &mut Frame, area: Rect, kanji: &KanjiData, app: &App) {
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

pub fn render_used_in_section(
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

pub fn append_list_item(lines: &mut Vec<TLine>, title: &str, text: &str, indent: Option<usize>) {
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

pub fn draw_stroke_points(ctx: &mut Context, points: &[MyPoint], max_points: usize, color: Color) {
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

pub fn render_help_popup(frame: &mut Frame, used_area: u16) {
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

pub fn render_search_form(frame: &mut Frame, used_area: u16, form: &SearchForm) {
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

pub fn render_search_rad_form(frame: &mut Frame, used_area: u16, form: &mut SearchForm) {
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

pub fn render_no_kanji(frame: &mut Frame) {
    let [view] = Layout::vertical([Constraint::Length(22)])
        .spacing(0)
        .areas(frame.area());
    let block = base_layout_block("").padding(Padding::ZERO);

    frame.render_widget(block, view);
}

pub fn render_history_popup(frame: &mut Frame, history: &History, used_height: u16) {
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
        let char_y = title_area.y + (used_height / 2 - 7) + i as u16;
        if char_y >= title_area.height {
            break;
        }

        let char_widget = Paragraph::new(char.to_string());
        frame.render_widget(char_widget, Rect::new(title_area.x, char_y, 1, 1));
    }

    frame.render_widget(Clear, history_area);

    let items: Vec<ListItem> = history
        .items
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if i == history.cursor {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(format!("{}", entry)).style(style)
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