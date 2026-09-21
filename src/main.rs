use kanji_tui::app::App;
use std::io;
use kanji_tui::repo::InMemoryJson;

/// App entry point
fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let repo = &InMemoryJson::new();
    let mut app = App::new(repo);
    let app_result = app.run(&mut terminal);
    ratatui::restore();

    app_result
}