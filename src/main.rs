mod app;
mod collect;
mod config;
mod theme;
mod ui;
mod util;
mod widgets;

use std::io;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event};
use ratatui::crossterm::execute;

fn main() -> io::Result<()> {
    let mut app = app::App::new()?;

    let mut terminal = ratatui::init();
    let mouse_enabled = execute!(io::stdout(), EnableMouseCapture).is_ok();

    let result = run(&mut terminal, &mut app);

    if mouse_enabled {
        let _ = execute!(io::stdout(), DisableMouseCapture);
    }
    ratatui::restore();
    app.save_config();

    result
}

fn run(terminal: &mut ratatui::DefaultTerminal, app: &mut app::App) -> io::Result<()> {
    let mut last_sample = Instant::now();
    let mut last_second = chrono::Local::now().timestamp();

    loop {
        if app.dirty {
            terminal.draw(|frame| ui::draw(frame, app))?;
            app.dirty = false;
        }

        if last_sample.elapsed() >= Duration::from_millis(app.config.update_ms) {
            app.tick();
            last_sample = Instant::now();
        }

        if event::poll(Duration::from_millis(25))? {
            match event::read()? {
                Event::Key(key) => app.on_key(key),
                Event::Mouse(mouse) => app.on_mouse(mouse),
                Event::Resize(_, _) => app.dirty = true,
                _ => {}
            }
        }

        let second = chrono::Local::now().timestamp();
        if second != last_second {
            last_second = second;
            app.dirty = true;
        }

        if app.quit {
            break;
        }
    }

    Ok(())
}
