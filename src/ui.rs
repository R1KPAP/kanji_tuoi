use crate::app::{App, JumpElement, ScrollStates};
use crate::app::{History, Theme};
use crate::app::{ScrollElem, search::*};
use crate::constants::KANGXI_RADICALS;
use crate::model::{JlptLevel, KanjiGrade};
use crate::model::{KanjiData, MyPoint};
use crate::utilities::format_range;
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

const NO_RESULT: &str = r"⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⠟⢦⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
⢷⡄⠈⡓⠢⢄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣠⠤⠂⢹
⠈⡷⡄⠈⠲⢤⣈⠻⠉⠛⠉⠉⠁⠒⠖⠉⠉⠉⠒⠶⢦⣤⠴⠒⢉⣡⠴⠀⢀⠏
⠀⢸⡿⡂⠀⠀⠉⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣀⣠⣴⡞⠉⠀⢀⣠⡞⠀
⠀⠀⢙⣿⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠉⠀⠀⢠⡼⡟⠀⠀
⠀⠀⡼⠋⠀⣤⣀⠀⠀⠀⠀⠀⠈⠐⣂⣄⠀⠀⠀⠀⠀⠀⠀⢀⠀⣰⡟⠁⠀⠀
⠀⢠⡇⠀⠀⠘⠛⠃⠀⠀⠀⠀⠾⣿⠿⠟⠉⠀⠀⠀⠀⠀⠀⠀⠀⢻⠀⠀⠀⠀
⠀⢸⡇⢺⡀⠀⢠⡒⠠⣄⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠠⡀⠀⠀⠸⡇⠀⠀⠀
⠀⢸⡇⣘⠑⡀⠀⠙⢏⣁⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠈⠂⠀⣔⣇⠀⠀⠀
⠀⢸⡇⡁⠀⢳⣶⣾⣷⣦⣄⣀⡀⣀⣀⡀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢸⣿⠀⠀⠀
⠀⠸⡇⠁⠀⠀⢏⠉⠀⠀⠙⠛⠛⠛⠉⠁⠀⠀⠀⠀⠀⠀⠀⠀⠀⢈⡏⠀⠀⠀
⠀⠀⠯⣀⣈⣀⣈⣐⣲⣄⣄⣤⣴⣆⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣈⣛⡧⠀⠀⠀";

const TOO_SMALL: &str = r"
------===+=----=---------=----
-----=-------=-=---=-==-==-=--
----------------:---------=-==
-------------:..... :=--=-==--
-----------. .-===:.. -=======
--=**++*---.+:==-==.  ========
--+=::------*=+++==:   ++===+=
-+-:-------.+-+:-----  -=-----
==:-..:::.. .=---:::  .:::=---
-:+...:....  .-:::::......:::-
::-.... ....  .:::.. .....:..:
-  .. ........ .... ..........
--.----..  ...................
:-======..   . ...............
:::::::::..... . ..........:::";

#[derive(PartialEq)]
pub enum LayoutType {
    Horizontal,
    Vertical,
    Minimal,
    None,
}
pub struct MainLayout {
    pub canvas: Rect,
    pub radical: Rect,
    pub stats: Rect,
    pub details: Rect,
    pub used_in: Rect,
    pub layout_type: LayoutType,
}

// Normal, Vertical
pub fn define_layout(area: Rect, cur_kanji: Option<&KanjiData>) -> Option<MainLayout> {
    let top_height: u16 = 10;
    let kanji_width = (top_height.saturating_sub(2) * 2) + 2;
    let stats_width = 25;

    if area.height < 8 || area.width < kanji_width + stats_width {
        None
    } else if area.width >= kanji_width + (stats_width * 2) {
        define_hlayout(area, cur_kanji)
    } else {
        define_vlayout(area, cur_kanji)
    }
}

pub fn define_hlayout(area: Rect, cur_kanji: Option<&KanjiData>) -> Option<MainLayout> {
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
        Constraint::Max(29),
        Constraint::Fill(1),
    ])
    .spacing(0)
    .areas(kanji_section);

    let [details, used_in] = Layout::horizontal([Constraint::Percentage(65), Constraint::Fill(35)])
        .spacing(0)
        .areas(details_section);

    Some(MainLayout {
        canvas,
        radical,
        stats,
        details,
        used_in,
        layout_type: LayoutType::Horizontal,
    })
}

pub fn define_vlayout(area: Rect, cur_kanji: Option<&KanjiData>) -> Option<MainLayout> {
    let details_height = cur_kanji
        .map(|k| calc_details_height(k, (area.width as f32 * 0.55) as u16))
        .unwrap_or(12);

    let [kanji_section, details_section, used_in, _spacer] = Layout::vertical([
        Constraint::Length(8),
        Constraint::Length(details_height), // Usa direttamente l'altezza calcolata senza tagliarla
        Constraint::Length(8),
        Constraint::Min(0), // Lo spazio vuoto in eccesso va qui in fondo
    ])
    .spacing(0)
    .areas(area);

    let top_height = kanji_section.height;
    let kanji_width = (top_height.saturating_sub(2) * 2) + 2;

    let [canvas, stats] =
        Layout::horizontal([Constraint::Length(kanji_width), Constraint::Fill(1)])
            .spacing(0)
            .areas(kanji_section);

    let [radical, details] = Layout::horizontal([Constraint::Percentage(45), Constraint::Fill(55)])
        .spacing(0)
        .areas(details_section);

    Some(MainLayout {
        canvas,
        radical,
        stats,
        details,
        used_in,
        layout_type: LayoutType::Vertical,
    })
}

pub fn base_layout_block<'a>(title: &'a str, theme: &Theme) -> Block<'a> {
    let offset_x = 0;
    let title_text = format!("{}{}", "═".repeat(offset_x), title);

    let block = Block::default()
        .style(Style::default().bg(theme.bg))
        .padding(Padding::vertical(1))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(theme.border))
        .title(TLine::from(vec![Span::styled(
            title_text,
            Style::default().fg(theme.btitle).bold(),
        )]));

    block
}

pub fn render_kanji_canvas(frame: &mut Frame, area: Rect, kanji: &KanjiData, app: &App) {
    let current_theme = &app.current_theme();
    let tot_kanji = app.search_results.len();
    let block = base_layout_block("Kanji", current_theme).padding(Padding::ZERO);
    let inner_area = block.inner(area);
    frame.render_widget(block, area);

    if kanji.strokes.is_empty() {
        let vertical_center: [Rect; 4] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(inner_area);

        let paragraph = Paragraph::new(kanji.kanji.to_string())
            .alignment(Alignment::Center)
            .style(Style::default().fg(current_theme.rtext).bold());

        let title = format!("{}/{}", app.current_kanji + 1, tot_kanji);
        let footer = Paragraph::new(title)
            .style(Style::default().fg(current_theme.rtext))
            .alignment(Alignment::Center);
        frame.render_widget(footer, vertical_center[3]);

        frame.render_widget(paragraph, vertical_center[1]);
        return;
    }

    let [canvas_area, footer_area] =
        Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(inner_area);

    let canvas_widget = Canvas::default()
        .background_color(current_theme.bg)
        .x_bounds([8.0, 101.0])
        .y_bounds([8.0, 101.0])
        .marker(Marker::Braille)
        .paint(|ctx| {
            if app.anim_state.is_finished {
                for stroke in &kanji.strokes {
                    draw_stroke_points(
                        ctx,
                        &stroke.strokes,
                        stroke.strokes.len(),
                        current_theme.kanji_stroke_old,
                    );
                }
            } else {
                let max_stroke = app.anim_state.current_stroke.min(kanji.strokes.len());

                for s_idx in 0..max_stroke {
                    let stroke = &kanji.strokes[s_idx];
                    draw_stroke_points(
                        ctx,
                        &stroke.strokes,
                        stroke.strokes.len(),
                        current_theme.kanji_stroke_old,
                    );
                }

                if app.anim_state.current_stroke < kanji.strokes.len() {
                    let current_stroke = &kanji.strokes[app.anim_state.current_stroke];

                    let points_to_draw = app
                        .anim_state
                        .current_point
                        .saturating_add(1)
                        .min(current_stroke.strokes.len());

                    draw_stroke_points(
                        ctx,
                        &current_stroke.strokes,
                        points_to_draw,
                        current_theme.kanji_stroke_new,
                    );
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

    let paragraph = Paragraph::new(footer_content)
        .style(Style::default().fg(current_theme.rtext))
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, footer_area);
}

pub fn render_radical_section(
    frame: &mut Frame,
    area: Rect,
    radical: Option<&KanjiData>,
    theme: &Theme,
    scroll_state: &mut ScrollStates,
) {
    let inner_height = area.height.saturating_sub(2);

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
                Style::default().fg(theme.rtext).bold(),
            ),
            Span::raw(symbol).style(Style::default().fg(theme.vtext)),
        ]));
        lines.push(TLine::from(vec![
            Span::styled(
                format!(" {:<indent$}", "Name:"),
                Style::default().fg(theme.rtext).bold(),
            ),
            Span::raw(names).style(Style::default().fg(theme.vtext)),
        ]));
        append_list_item(
            &mut lines,
            "Meanings:",
            &meanings,
            Some(indent),
            theme.rtext,
            theme.vtext,
        );
        append_list_item(
            &mut lines,
            "Variants:",
            &variants,
            Some(indent),
            theme.rtext,
            theme.vtext,
        );

        let total_lines = lines.len() as u16;
        let max_scroll = total_lines.saturating_sub(inner_height);
        let clamped_offset = scroll_state.rad_offset.min(max_scroll);
        scroll_state.rad_offset = clamped_offset;
        let border_color = if scroll_state.curr_selected == ScrollElem::Radical {
            theme.kanji_stroke_old
        } else {
            theme.border
        };

        let paragraph = Paragraph::new(lines)
            .block(base_layout_block("Radical", theme)
        .border_style(Style::default().fg(border_color)))
            .scroll((clamped_offset, 0));
        frame.render_widget(paragraph, area);
    } else {
        let text_lines = vec![
            TLine::from(""),
            TLine::styled("No radical info", Style::default().fg(theme.text_muted)),
        ];

        let total_lines = text_lines.len() as u16;
        let max_scroll = total_lines.saturating_sub(inner_height);
        let clamped_offset = scroll_state.rad_offset.min(max_scroll);
        scroll_state.rad_offset = clamped_offset;
        let border_color = if scroll_state.curr_selected == ScrollElem::Radical {
            theme.kanji_stroke_old
        } else {
            theme.border
        };

        let paragraph = Paragraph::new(text_lines)
            .block(
                base_layout_block("Radical", theme)
                    .border_style(Style::default().fg(border_color)))
            .scroll((clamped_offset, 0));
        frame.render_widget(paragraph, area);
    }
}

pub fn render_stats_section(frame: &mut Frame, area: Rect, kanji: &KanjiData, theme: &Theme) {
    let jlpt_span = match kanji.jlpt_level {
        Some(level) => Span::styled(format!("N{level}"), Style::default().fg(theme.vtext)),
        None => Span::styled("not available", Style::default().fg(theme.text_muted)),
    };

    let grade_span = Span::styled(kanji.grade_detail(), Style::default().fg(theme.vtext));

    let stokes_span = match kanji.strokes_count {
        Some(strokes) => Span::styled(strokes.to_string(), Style::default().fg(theme.vtext)),
        None => Span::styled("not available", Style::default().fg(theme.text_muted)),
    };

    let freq_span = match kanji.freq_rank {
        Some(freq_rank) => Span::styled(
            format!("#{freq_rank}/2501"),
            Style::default().fg(theme.vtext),
        ),
        None => Span::styled("not available", Style::default().fg(theme.text_muted)),
    };

    let lines = vec![
        TLine::from(vec![
            Span::styled(
                format!("{:<10}", " JLPT:"),
                Style::default().fg(theme.rtext).bold(),
            ),
            jlpt_span,
        ]),
        TLine::from(vec![
            Span::styled(
                format!("{:<10}", " Grade:"),
                Style::default().fg(theme.rtext).bold(),
            ),
            grade_span,
        ]),
        TLine::from(vec![
            Span::styled(
                format!("{:<10}", " Strokes:"),
                Style::default().fg(theme.rtext).bold(),
            ),
            stokes_span,
        ]),
        TLine::from(vec![
            Span::styled(
                format!("{:<10}", " Ranking:"),
                Style::default().fg(theme.rtext).bold(),
            ),
            freq_span,
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(base_layout_block("Statistics", theme));
    frame.render_widget(paragraph, area);
}

pub fn render_details_section(frame: &mut Frame, area: Rect, kanji: &KanjiData, app: &mut App) {
    let current_theme = &app.current_theme();
    let dec_title = "Compounds:";
    let on_title = "Onyomi:";
    let kun_title = "Kunyomi:";
    let nano_title = "Nanori:";
    let mean_title = "Meanings:";

    let decomp_lines: Vec<TLine> = if kanji.decomposition.is_empty() {
        vec![TLine::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(dec_title, Style::default().fg(current_theme.rtext).bold()),
            Span::styled(
                " not available",
                Style::default().fg(current_theme.text_muted),
            ),
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
            Style::default().fg(current_theme.rtext),
        ));

        let last_idx = kanji.decomposition.len() - 1;

        for (idx, n) in kanji.decomposition.iter().enumerate() {
            let is_selected = app.jump_selected == JumpElement::Compounds(idx);
            let style = if is_selected {
                Style::default()
                    .fg(current_theme.htext)
                    .bg(current_theme.utext)
            } else {
                Style::default().fg(current_theme.utext)
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
                current_spans.push(Span::styled(
                    sep_text,
                    Style::default().fg(current_theme.rtext),
                ));
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
    append_list_item(
        &mut lines,
        on_title,
        &onyomi,
        None,
        current_theme.rtext,
        current_theme.vtext,
    );
    append_list_item(
        &mut lines,
        kun_title,
        &kunyomi,
        None,
        current_theme.rtext,
        current_theme.vtext,
    );
    append_list_item(
        &mut lines,
        nano_title,
        &nanori,
        None,
        current_theme.rtext,
        current_theme.vtext,
    );
    lines.push(TLine::raw(""));
    append_list_item(
        &mut lines,
        mean_title,
        &meanings,
        None,
        current_theme.rtext,
        current_theme.vtext,
    );

    let total_lines = lines.len() as u16;
    let inner_height = area.height.saturating_sub(2);
    let max_scroll = total_lines.saturating_sub(inner_height);
    let clamped_offset = app.scroll_states.details_offset.min(max_scroll);
    app.scroll_states.details_offset = clamped_offset;
    let border_color = if app.scroll_states.curr_selected == ScrollElem::Details {
        current_theme.kanji_stroke_old
    } else {
        current_theme.border
    };

    let paragraph = Paragraph::new(lines)
        .block(base_layout_block("Reading & Meanings", current_theme)
            .border_style(Style::default().fg(border_color)))
        .scroll((clamped_offset, 0));

    frame.render_widget(paragraph, area);
}

pub fn render_used_in_section(
    frame: &mut Frame,
    area: Rect,
    compounds: Vec<&KanjiData>,
    compounds_state: &mut ListState,
    theme: &Theme,
) {
    if compounds.is_empty() {
        let empty_item =
            vec![ListItem::new("No data available").style(Style::default().fg(theme.text_muted))];
        let list = List::new(empty_item).block(base_layout_block("Used in", theme));
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
            append_list_item(&mut lines, "", &content, Some(0), theme.rtext, theme.utext);
            ListItem::new(Text::from(lines))
        })
        .collect();

    let list = List::new(items)
        .block(base_layout_block("Used in", theme))
        .highlight_style(Style::default().bg(theme.bg).fg(theme.htext))
        .highlight_symbol("❯");

    frame.render_stateful_widget(list, area, compounds_state);
}

pub fn append_list_item(
    lines: &mut Vec<TLine>,
    title: &str,
    text: &str,
    indent: Option<usize>,
    rcolor: Color,
    vcolor: Color,
) {
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
                Style::default().fg(rcolor).bold(),
            ),
            Span::raw(first_line.to_string()).style(Style::default().fg(vcolor)),
        ]));

        let padding = " ".repeat(1 + indent);

        for next_line in lines_iter {
            let content = format!("{}{}", padding, next_line);
            lines.push(TLine::from(
                Span::raw(content).style(Style::default().fg(vcolor)),
            ));
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

pub fn render_help_popup(frame: &mut Frame, used_height: u16, theme: &Theme, layout: LayoutType, scroll_state: &mut ScrollStates) {
    let (x_percent, y_percent) = if layout == LayoutType::Horizontal {
        (92,86)
    } else {
        (94,92)
    };

    let popup_area = centered_rect(x_percent, y_percent, frame.area(), used_height);
    let max_width = (popup_area.width as usize).saturating_sub(4);

    frame.render_widget(Clear, popup_area);

    let mut lines = Vec::new();

    let help_sections = vec![
        ("Grade:", ""),
        (
            " Kyōiku:",
            "kanji taught during the six years of elementary school",
        ),
        (
            " Jōyō:",
            "Kyōiku kanji plus the remaining (total 2136), to be learned by the end of high school",
        ),
        (
            " Jinmeiyō:",
            "kanji approved to be used in family name, registers and other official documents",
        ),
        (
            " Hyōgai:",
            "kanji that fall outside of the list Jōyō + Jinmeiyō, are very rare or old",
        ),
        ("", ""),
        (
            "",
            "The radical is the core component of a kanji, it shares its meaning and it used to classify it",
        ),
        (
            "",
            "The rank expresses the relative frequency of occurrence of a kanji in modern Japanese",
        ),
        (
            "",
            "Kanji variants (異体字), are alternative graphical forms of the same kanji",
        ),
        (
            "",
            "Compounds section is a list of kanji used inside the current kanji",
        ),
        (
            "",
            "Onyomi is the Chinese reading of a kanji, often used for words made of multiple kanji",
        ),
        (
            "",
            "Kunyomi is the native Japanese reading, primarly used for kanji used alone",
        ),
        (
            "",
            "Nanori is the reading used almost exclusively in Japanese proper names",
        ),
        (
            "",
            "Used in section is a list of kanji containg the current kanji",
        ),
    ];

    for (prefix, text) in help_sections {
        if text.is_empty() {
            lines.push(TLine::from(prefix.to_string()));
        } else if text.is_empty() && prefix.is_empty() {
            lines.push(TLine::raw(""));
        } else {
            let options =
                textwrap::Options::new(max_width.saturating_sub(prefix.len())).break_words(false);

            let wrapped = textwrap::fill(text, &options);
            let padding = " ".repeat(prefix.len());

            for (idx, sub_line) in wrapped.lines().enumerate() {
                let line_prefix = if idx == 0 { prefix } else { &padding };
                lines.push(TLine::from(format!("{}{}", line_prefix, sub_line)));
            }
        }
    }

    let inner_height = popup_area.height.saturating_sub(2);
    let total_lines = lines.len() as u16;
    let max_scroll = total_lines.saturating_sub(inner_height);
    let clamped_offset = scroll_state.popup_offset.min(max_scroll);
    scroll_state.popup_offset = clamped_offset;

    let help_widget = Paragraph::new(lines)
        .block(base_layout_block("Details", theme)
            .padding(Padding::ZERO))
        .scroll((clamped_offset, 0))
        .style(Style::default().fg(theme.rtext));

    frame.render_widget(help_widget, popup_area);
}

pub fn render_keys_popup(frame: &mut Frame, used_height: u16, theme: &Theme, layout: LayoutType,scroll_state: &mut ScrollStates) {
    let (x_percent, y_percent) = if layout == LayoutType::Horizontal {
        (96, 92)
    } else {
        (95, 96)
    };
    
    let popup_area = centered_rect(x_percent, y_percent, frame.area(), used_height);

    let [left, right] = Layout::horizontal([Constraint::Percentage(53), Constraint::Fill(47)])
        .spacing(0)
        .areas(popup_area);

    frame.render_widget(Clear, popup_area);

    let wrap_lines = |raw_lines: &[&str], width: u16| -> Vec<TLine> {
        let max_width = (width as usize).saturating_sub(2);
        if max_width == 0 {
            return vec![];
        }
        let options = textwrap::Options::new(max_width).break_words(false);

        let mut lines = Vec::new();
        for text in raw_lines {
            let wrapped = textwrap::fill(text, &options);
            for sub_line in wrapped.lines() {
                lines.push(TLine::from(sub_line.to_string()));
            }
        }
        lines
    };

    let kanji_keys_raw = vec![
        "d: open/close description popup",
        "s: open/close search popup",
        "k: open/close key mapping popup",
        "h: open/close jumps history popup",
        "right arrow: next kanji in current search",
        "left arrow: previous kanji in current search",
        "up arrow: current kanji next stroke",
        "down arrow: current kanji previous stroke",
        "o: play strokes animation from the beginning",
        "e: end strokes animation",
        "p: pause/resume strokes animation",
        "t: switch theme Original/Cyberpunk",
        "q: quit the app",
        "r: jump to current kanji's radical",
        "Tab: switch between compunds and used in selection",
        "Enter: jump to the selected compunds or used in",
        "Esc: close any pop up",
        "] : move to the next element of the selection",
        "[ : move to the previous element of the selection",
    ];

    let search_keys_raw = vec![
        "Space: [un]toggle selection",
        "Enter: submit filters and start the search",
        "Esc: close search popup",
        ">: switch to radicals search",
        "<: switch to normal search",
        "While editing a custom search field:",
        "  - any alphanumeric key is used for editing",
        "  - Esc: stop the editing",
        "  - Tab: switch to the next editable field",
        "  - right arrow: move the cursor to the left",
        "  - left arrow: move the cursor to the right",
        "  - Backspace: delete on the left of the cursor",
        "  - Delete: delete on the right of the cursor",
        "  - Enter: submit filters and start the search",
    ];

    let kanji_lines = wrap_lines(&kanji_keys_raw, left.width);
    let search_lines = wrap_lines(&search_keys_raw, right.width);

    let left_inner_height = left.height.saturating_sub(2);
    let left_max_scroll = (kanji_lines.len() as u16).saturating_sub(left_inner_height);

    let right_inner_height = right.height.saturating_sub(2);
    let right_max_scroll = (search_lines.len() as u16).saturating_sub(right_inner_height);

    let max_scroll = left_max_scroll.max(right_max_scroll);

    scroll_state.popup_offset = scroll_state.popup_offset.min(max_scroll);

    let kanji_keys_widget = Paragraph::new(kanji_lines)
        .block(base_layout_block("Kanji view", theme)
            .padding(Padding::ZERO)
            .border_type(BorderType::Plain))
        .scroll((scroll_state.popup_offset, 0))
        .style(Style::default().fg(theme.rtext));

    frame.render_widget(kanji_keys_widget, left);

    let search_keys_widget = Paragraph::new(search_lines)
        .block(base_layout_block("Search view", theme)
            .padding(Padding::ZERO)
            .border_type(BorderType::Plain))
        .scroll((scroll_state.popup_offset, 0))
        .style(Style::default().fg(theme.rtext));

    frame.render_widget(search_keys_widget, right);
}

pub fn render_search_form(
    frame: &mut Frame,
    used_height: u16,
    form: &SearchForm,
    theme: &Theme,
    layout: LayoutType,
) {
    let (x_percent, y_percent, chunk_size) = if layout == LayoutType::Horizontal {
        (62, 48, 2)
    } else {
        (79, 48, 3)
    };
    let popup_area = centered_rect(x_percent, y_percent, frame.area(), used_height);
    frame.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .style(Style::default().bg(theme.bg))
        .title("Filters")
        .title_style(Style::default().fg(theme.btitle))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));

    let inner_area = popup_block.inner(popup_area);
    frame.render_widget(popup_block, popup_area);

    let chunks = Layout::vertical([
        Constraint::Length(chunk_size), // JLPT
        Constraint::Length(chunk_size), // Grade
        Constraint::Length(2),          // Strokes
        Constraint::Length(2),          // Rank
        Constraint::Length(1),          // Query
    ])
    .split(inner_area);

    //JLPT
    let mut jlpt_spans = vec![Span::styled(
        " JLPT Level: ",
        Style::default().fg(theme.rtext),
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
                    .fg(theme.htext)
                    .bg(theme.utext)
                    .add_modifier(Modifier::BOLD),
                (true, false) => Style::default().fg(theme.htext).bg(theme.utext),
                (false, true) => Style::default()
                    .fg(theme.kanji_stroke_old)
                    .add_modifier(Modifier::BOLD),
                (false, false) => Style::default().fg(theme.htext),
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
                vec![span, Span::styled(" | ", Style::default().fg(theme.rtext))]
            } else {
                vec![span]
            }
        })
        .collect();

    if layout == LayoutType::Horizontal {
        jlpt_spans.extend(jlpt_items);
        let jlpt_widget = Paragraph::new(TLine::from(jlpt_spans));
        frame.render_widget(jlpt_widget, chunks[0]);
    } else {
        let split_idx = 9;
        let mut jlpt_line1_spans = jlpt_spans.clone();
        jlpt_line1_spans.extend(jlpt_items[..split_idx].iter().cloned());

        let mut jlpt_line2_spans = vec![Span::styled("             ", Style::default())];
        jlpt_line2_spans.extend(jlpt_items[split_idx..].iter().cloned());

        let jlpt_widget = Paragraph::new(vec![
            TLine::from(jlpt_line1_spans),
            TLine::from(jlpt_line2_spans),
        ]);
        frame.render_widget(jlpt_widget, chunks[0]);
    };

    //Grade
    let mut grade_spans = vec![Span::styled(" Grade: ", Style::default().fg(theme.rtext))];

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
                    .fg(theme.htext)
                    .bg(theme.utext)
                    .add_modifier(Modifier::BOLD),
                (true, false) => Style::default().fg(theme.htext).bg(theme.utext),
                (false, true) => Style::default()
                    .fg(theme.kanji_stroke_old)
                    .add_modifier(Modifier::BOLD),
                (false, false) => Style::default().fg(theme.htext),
            };

            let text = if is_selected {
                format!("[{d}]")
            } else {
                format!("{d}")
            };

            let span = Span::styled(text, style);
            if (*idx as u8) < 8 {
                vec![span, Span::styled(" | ", Style::default().fg(theme.rtext))]
            } else {
                vec![span]
            }
        })
        .collect();

    if layout == LayoutType::Horizontal {
        grade_spans.extend(grade_items);

        let grade_widget = Paragraph::new(TLine::from(grade_spans));
        frame.render_widget(grade_widget, chunks[1]);
    } else {
        let split_idx = 11;
        let mut grade_line1_spans = grade_spans.clone();
        grade_line1_spans.extend(grade_items[..split_idx].iter().cloned());

        let mut grade_line2_spans = vec![Span::styled("        ", Style::default())];
        grade_line2_spans.extend(grade_items[split_idx..].iter().cloned());

        let grade_widget = Paragraph::new(vec![
            TLine::from(grade_line1_spans),
            TLine::from(grade_line2_spans),
        ]);
        frame.render_widget(grade_widget, chunks[1]);
    };

    //Strokes
    let (min_stroke, max_stroke) = format_range(form.draft.strokes.as_ref());
    let stroke_prefix = " Number of strokes: ";
    let stroke_widget = Paragraph::new(TLine::from(vec![
        Span::styled(stroke_prefix, Style::default().fg(theme.rtext)),
        Span::styled("min ", Style::default().fg(theme.rtext)),
        Span::styled(
            min_stroke.to_string(),
            Style::default().fg(theme.kanji_stroke_old),
        ),
        Span::styled(" max ", Style::default().fg(theme.rtext)),
        Span::styled(
            max_stroke.to_string(),
            Style::default().fg(theme.kanji_stroke_old),
        ),
    ]));
    frame.render_widget(stroke_widget, chunks[2]);

    //Rank
    let (min_rank, max_rank) = format_range(form.draft.rank.as_ref());
    let rank_prefix = " Rank: ";
    let rank_widget = Paragraph::new(TLine::from(vec![
        Span::styled(rank_prefix, Style::default().fg(theme.rtext)),
        Span::styled("min ", Style::default().fg(theme.rtext)),
        Span::styled(
            min_rank.to_string(),
            Style::default().fg(theme.kanji_stroke_old),
        ),
        Span::styled(" max ", Style::default().fg(theme.rtext)),
        Span::styled(
            max_rank.to_string(),
            Style::default().fg(theme.kanji_stroke_old),
        ),
    ]));
    frame.render_widget(rank_widget, chunks[3]);

    //Text
    let query_prefix = " Textual search: ";
    let query_text = form.draft.query_text.as_deref().unwrap_or("_");
    let text_widget = Paragraph::new(TLine::from(vec![
        Span::styled(query_prefix, Style::default().fg(theme.rtext)),
        Span::styled(query_text, Style::default().fg(theme.kanji_stroke_old)),
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

pub fn render_search_rad_form(
    frame: &mut Frame,
    used_height: u16,
    form: &mut SearchForm,
    theme: &Theme,
    layout: LayoutType,
) {
    let y_percent = if layout == LayoutType::Horizontal {
        58
    } else {
        79
    };
    let popup_area = centered_rect(91, y_percent, frame.area(), used_height);
    frame.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .padding(Padding::ZERO)
        .style(Style::default().bg(theme.bg))
        .title_style(Style::default().fg(theme.btitle))
        .title("Radical filter")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border));

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
                .fg(theme.htext)
                .bg(theme.utext)
                .add_modifier(Modifier::BOLD),
            (true, false) => Style::default().fg(theme.htext).bg(theme.utext),
            (false, true) => Style::default()
                .fg(theme.kanji_stroke_old)
                .add_modifier(Modifier::BOLD),
            (false, false) => Style::default().fg(theme.htext),
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
        Constraint::Length(grid_height.min(inner_area.height)),
    ])
    .split(inner_area);

    let grid_area = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(grid_width.min(inner_area.width)),
        Constraint::Fill(1),
    ])
    .split(vertical_chunks[0])[1];

    frame.render_widget(grid_paragraph, grid_area);
}

pub fn render_no_kanji(frame: &mut Frame, theme: &Theme) {
    let paragraph = Paragraph::new(NO_RESULT)
        .style(Style::default().fg(theme.kanji_stroke_old))
        .alignment(Alignment::Center)
        .block(base_layout_block("", theme).padding(Padding::uniform(0)));

    frame.render_widget(paragraph, frame.area());
}

pub fn render_history_popup(frame: &mut Frame, history: &History, used_height: u16, theme: &Theme, layout: &LayoutType) {
    let target_area: Rect = Rect {
        x: frame.area().x,
        y: frame.area().y,
        width: frame.area().width,
        height: frame.area().height.min(used_height),
    };

    let [_, history_area, title_area] = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(4),
        Constraint::Length(2),
    ])
    .areas(target_area);

    for (i, char) in " Jump History ".chars().enumerate() {
        let char_y = title_area.y + (used_height / 2 - 7) + i as u16;
        if char_y >= title_area.height {
            break;
        }

        let char_widget =
            Paragraph::new(char.to_string()).style(Style::default().fg(theme.kanji_stroke_old));
        frame.render_widget(char_widget, Rect::new(title_area.x, char_y, 1, 1));
    }

    frame.render_widget(Clear, history_area);

    let items: Vec<ListItem> = history
        .items
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let style = if i == history.cursor {
                Style::default().fg(theme.kanji_stroke_old)
            } else {
                Style::default().fg(theme.vtext)
            };
            ListItem::new(format!("{}", entry)).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .style(Style::default().bg(theme.bg))
            .title("")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border)),
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
        Constraint::Fill(1),
        Constraint::Length((used_height * percent_y)/100),
        Constraint::Fill(1),
    ])
    .split(effective_r);

    Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length((r.width * percent_x)/100),
        Constraint::Fill(1),
    ])
    .split(popup_layout[1])[1]
}

pub fn calc_details_height(kanji: &KanjiData, area_width: u16) -> u16 {
    let area_width = area_width as usize;
    if area_width <= 2 {
        return 12;
    }

    let dec_title = "Compounds:";
    let on_title = "Onyomi:";
    let kun_title = "Kunyomi:";
    let nano_title = "Nanori:";
    let mean_title = "Meanings:";

    let decomp_line_count = if kanji.decomposition.is_empty() {
        1
    } else {
        let max_width = area_width.saturating_sub(2);
        let indent_width = dec_title.len() + 2;
        let title_text_len = dec_title.len() + 2; // " Compounds: "

        let mut current_width = title_text_len;
        let mut lines_count = 1;
        let last_idx = kanji.decomposition.len().saturating_sub(1);

        for (idx, n) in kanji.decomposition.iter().enumerate() {
            let elem_text = format!(" {} ", n);
            let sep_text = if idx < last_idx { "+" } else { "" };
            let item_width = elem_text.len() + sep_text.len();

            if current_width + item_width > max_width {
                lines_count += 1;
                current_width = indent_width;
            }
            current_width += item_width;
        }
        lines_count
    };

    let calc_field_lines = |items: &[String], title_len: usize| -> usize {
        if items.is_empty() {
            1
        } else {
            let max_length = area_width.saturating_sub(2 + 2 + title_len);
            let options = Options::new(max_length).break_words(false);
            textwrap::fill(&items.join(", "), &options).lines().count()
        }
    };

    let onyomi_lines = calc_field_lines(&kanji.onyomi, on_title.len());
    let kunyomi_lines = calc_field_lines(&kanji.kunyomi, kun_title.len());
    let meanings_lines = calc_field_lines(&kanji.meanings, mean_title.len());

    let title_len = nano_title.len();
    let prefix_cols = 1 + (title_len + 1) + 1;
    let text_width = area_width.saturating_sub(prefix_cols + 2);

    let nanori_lines = if kanji.nanori.is_empty() {
        1
    } else if text_width > 0 {
        let options = textwrap::Options::new(text_width).break_words(false);
        textwrap::fill(&kanji.nanori.join(", "), &options)
            .lines()
            .count()
    } else {
        1
    };

    let total_text_lines =
        decomp_line_count + 1 + onyomi_lines + kunyomi_lines + nanori_lines + 1 + meanings_lines;

    (total_text_lines + 4) as u16
}
