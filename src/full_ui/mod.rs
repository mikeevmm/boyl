use std::io;
use std::io::Stdout;
use std::time::Duration;
use termion::event::Key;
use termion::raw::IntoRawMode;
use termion::raw::RawTerminal;
use tokio::time::sleep;
use tui::backend::TermionBackend;
use tui::Terminal;
use tui::widgets::Block;
use tui::widgets::Borders;
use std::thread::{self,JoinHandle};

enum Event {
    Tick,
    Key(Key),
}

pub struct UiConfig {
    tick_rate: Duration,
}

impl Default for UiConfig {
    fn default() -> Self {
        UiConfig {
            tick_rate: Duration::from_millis(200),
        }
    }
}

pub struct Ui {
    tokio_handle: JoinHandle<()>,
}

impl Ui {
    pub fn start() -> Result<Self, io::Error> {
        Self::from_config(UiConfig::default())
    }

    pub fn from_config(config: UiConfig) -> Result<Self, io::Error> {
        let stdout = io::stdout().into_raw_mode()?;
        let backend = TermionBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        let tokio_handle = thread::spawn(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let draw_handle = tokio::spawn(async move {
                        loop {
                            terminal.draw(|f| {
                                let size = f.size();
                                let block = Block::default().title("Block").borders(Borders::ALL);
                                f.render_widget(block, size);
                            }).unwrap();
                            sleep(config.tick_rate).await;
                        }
                    });
                });
        });
        Ok(Ui {
            tokio_handle,
        })
    }
}
