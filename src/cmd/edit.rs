use crate::{
    config::LoadedConfig,
    ui::{help, input::InputField, layout::VisualBox, list::List, UiState, UiStateReaction},
};
use colored::Colorize;
use termion::event::Key;
use tui::{backend::Backend, style::{Color, Modifier, Style}, text::{Span, Spans, Text}, widgets::{Block, Borders}};


struct EditMenuUi<'conf> {
    config: &'conf LoadedConfig,
}

impl<'conf, B: Backend> UiState<B> for EditMenuUi<'conf> {
    fn require_ticking(&self) -> Option<std::time::Duration> {
        None
    }

    fn on_key(&mut self, key: Key) -> Option<crate::ui::UiStateReaction> {
        None
    }

    fn on_tick(&mut self) -> Option<crate::ui::UiStateReaction> {
        None
    }

    fn draw(&mut self, f: &mut tui::Frame<B>) {
    }
}

pub fn edit(config: &LoadedConfig) {
    let mut menu_ui = EditMenuUi{config};
    crate::ui::run_ui(&mut menu_ui);
}
