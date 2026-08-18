use std::io;
use ratatui::{DefaultTerminal, Frame, 
    layout::{Layout, Constraint, Spacing}, 
    widgets::{Paragraph, Borders, Block}
};

/// App entry point
fn main() -> io::Result<()> {
    ratatui::run(|terminal| app(terminal))
}

/// Main loop, to redraw the UI
pub fn app(terminal: &mut DefaultTerminal) -> io::Result<()> {
    loop {
        terminal.draw(render)?;
        if crossterm::event::read()?.is_key_press() {
            break Ok(());
        }
    }
}

/// Renders the UI
fn render(frame: &mut Frame) {

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
    frame.render_widget(Paragraph::new("kun_yomi")
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