use std::io;
use ratatui::{
        DefaultTerminal,
        Frame,
        symbols::Marker,
        layout::{
            Layout,
            Constraint,
            Spacing}, 
        widgets::{
            canvas::{
                Canvas,
                Line},
            Paragraph,
            Borders,
            Block
        },
        style::{
            Color,
            Style,
        },
};

mod model;

/// App entry point
fn main() -> io::Result<()> {
    ratatui::run(|terminal| app(terminal))
}

/// Main loop, to redraw the UI
fn app(terminal: &mut DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(test_canvas)?;
        if crossterm::event::read()?.is_key_press() {
            break Ok(());
        }
    }
}

fn test_canvas(frame: &mut Frame) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("")
        .title_style(Style::default().fg(Color::Yellow));

    let canvas = Canvas::default()
        .block(block)
        .x_bounds([0.0, 109.0])
        .y_bounds([0.0, 109.0])
        .marker(Marker::Braille)
        .paint(|ctx| {
            ctx.draw(&Line {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 50.0,
                    y2: 0.0,
                    color: Color::Red,
                });
                ctx.draw(&Line {
                    x1: 0.0,
                    y1: 0.5,
                    x2: 50.0,
                    y2: 0.5,
                    color: Color::Red,
                });
        });

    frame.render_widget(canvas, frame.area());
}

/// Renders the UI
fn render_ui(frame: &mut Frame) {
    let [left, right] = Layout::horizontal([
        Constraint::Percentage(80),
        Constraint::Percentage(20),
    ])
    .spacing(Spacing::Overlap(1))
    .areas(frame.area());

    let [strokes, jlpt_lvl, radical] = Layout::vertical( [
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Fill(1),
    ])
    .spacing(Spacing::Overlap(1))
    .areas(right);

    let [canvas, right_canv] = Layout::horizontal( [
        Constraint::Percentage(20),
        Constraint::Percentage(80),
    ])
    .spacing(Spacing::Overlap(1))
    .areas(left);

    let [on_yomi, kun_yomi, nanori, meaning] = Layout::vertical( [
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
        Constraint::Fill(1),
    ])
    .spacing(Spacing::Overlap(1))
    .areas(right_canv);
    
    frame.render_widget(Paragraph::new("Kanji")
        .block(Block::new().borders(Borders::ALL)), canvas);
    frame.render_widget(Paragraph::new("on_yomi")
        .block(Block::new().borders(Borders::ALL)), on_yomi);
    frame.render_widget(Paragraph::new("kun_yomi")
        .block(Block::new().borders(Borders::ALL)), kun_yomi);
    frame.render_widget(Paragraph::new("nanori")
        .block(Block::new().borders(Borders::ALL)), nanori);
    frame.render_widget(Paragraph::new("meaning")
        .block(Block::new().borders(Borders::ALL)), meaning);
    frame.render_widget(Paragraph::new("strokes")
        .block(Block::new().borders(Borders::ALL)), strokes);
    frame.render_widget(Paragraph::new("jlpt_lvl")
        .block(Block::new().borders(Borders::ALL)), jlpt_lvl);
    frame.render_widget(Paragraph::new("radical")
        .block(Block::new().borders(Borders::ALL)), radical);
}