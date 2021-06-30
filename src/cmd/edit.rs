use crate::{
    config::LoadedConfig,
    ui::{help, input::InputField, layout::VisualBox, list::List, UiState, UiStateReaction},
};
use colored::Colorize;
use termion::event::Key;
use tui::{
    backend::Backend,
    style::{Color, Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders},
};

enum EditUiMode {
    List,
    Delete,
    Rename,
}

struct EditUi<'conf> {
    list: List<'conf, Spans<'conf>>,
    mode: EditUiMode,
}

impl<'conf> EditUi<'conf> {
    fn new(config: &'conf LoadedConfig) -> Self {
        let elements = config
            .config
            .templates
            .values()
            .map(|f| {
                Spans::from(vec![
                    Span::raw(f.name.clone()),
                    Span::raw(" "),
                    Span::styled(
                        f.description
                            .as_deref()
                            .unwrap_or("(No description.)")
                            .to_string(),
                        Style::default().fg(Color::DarkGray),
                    ),
                ])
            })
            .collect();

        EditUi {
            list: List::new(elements),
            mode: EditUiMode::List,
        }
    }

    fn list_input(&mut self, key: Key) -> Option<crate::ui::UiStateReaction> {
        match key {
            Key::Up | Key::Char('k') => {
                self.list.go_up();
            }
            Key::Down | Key::Char('j') => {
                self.list.go_down();
            }
            Key::Ctrl('c') | Key::Char('\n') | Key::Char('\r') => {
                return Some(UiStateReaction::Exit);
            }
            Key::Char('x') => todo!(),
            Key::Char('e') => todo!(),
            _ => {}
        }

        None
    }

    fn list_draw(&mut self, f: &mut tui::Frame<impl Backend>) {
        let (help_texts, help_boxes): (Vec<String>, Vec<VisualBox>) = vec![
            help::make_help_box("Up/K", "Move up in list"),
            help::make_help_box("Down/J", "Move down in list"),
            help::make_help_box("X", "Delete template"),
            help::make_help_box("E", "Edit description"),
            help::make_help_box("Enter", "Exit"),
        ]
        .into_iter()
        .unzip();
        let remaining = help::draw_help(help_texts, help_boxes, f, f.size());
        let block = Block::default().borders(Borders::ALL).title("Templates:");
        let block_inner = block.inner(remaining);
        f.render_widget(block, remaining);
        self.list.draw(f, block_inner);
    }
}

impl<'conf, B: Backend> UiState<B> for EditUi<'conf> {
    fn require_ticking(&self) -> Option<std::time::Duration> {
        None
    }

    fn on_key(&mut self, key: Key) -> Option<crate::ui::UiStateReaction> {
        match self.mode {
            EditUiMode::List => self.list_input(key),
            EditUiMode::Delete => todo!(),
            EditUiMode::Rename => todo!(),
        }
    }

    fn on_tick(&mut self) -> Option<crate::ui::UiStateReaction> {
        None
    }

    fn draw(&mut self, f: &mut tui::Frame<B>) {
        match self.mode {
            EditUiMode::List => self.list_draw(f),
            EditUiMode::Delete => todo!(),
            EditUiMode::Rename => todo!(),
        }
    }
}

pub fn edit(config: &LoadedConfig) {
    let mut list_ui = EditUi::new(config);
    crate::ui::run_ui(&mut list_ui);
}
