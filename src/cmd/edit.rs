use crate::{
    config::{LoadedConfig, TemplateKey},
    template::Template,
    ui::{self, input::InputField, layout::VisualBox, list::List, UiState, UiStateReaction},
};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Paragraph},
};

enum EditUiMode {
    List,
    Delete(TemplateKey, String),
    Error(String),
    Rename(TemplateKey),
}

struct EditUi<'conf> {
    config: &'conf mut LoadedConfig,
    mode: EditUiMode,
    list: List<'conf, Spans<'conf>>,
    input: InputField,
}

impl<'conf> EditUi<'conf> {
    fn new(config: &'conf mut LoadedConfig) -> Self {
        let list = List::new(Self::make_list_elements(config));
        EditUi {
            config,
            mode: EditUiMode::List,
            list,
            input: InputField::new(),
        }
    }

    /// Computes a single `Spans`, corresponding to one entry on the list for a `Template`.
    fn make_template_entry(template: &Template) -> Spans<'static> {
        Spans::from(vec![
            Span::raw(template.name.clone()),
            Span::raw(" "),
            Span::styled(
                template
                    .description
                    .as_deref()
                    .unwrap_or("(No description.)")
                    .to_string(),
                Style::default().fg(Color::Gray),
            ),
        ])
    }

    /// Computes the `Spans` to display the existing templates in a list.
    ///
    /// This is a reasonably expensive operation, as it iterates over every
    /// template in `config` and clones the names and descriptions, so it
    /// should be used sparsely if possible.
    fn make_list_elements(config: &LoadedConfig) -> Vec<Spans<'static>> {
        config
            .config
            .templates
            .values()
            .map(Self::make_template_entry)
            .collect::<Vec<Spans>>()
    }

    fn list_input(&mut self, key: Key) -> Option<crate::ui::UiStateReaction> {
        match key {
            Key::Up | Key::Char('k') => {
                self.list.go_up();
            }
            Key::Down | Key::Char('j') => {
                self.list.go_down();
            }
            Key::Ctrl('c') | Key::Char('q') | Key::Char('\n') | Key::Char('\r') => {
                return Some(UiStateReaction::Exit);
            }
            Key::Char('x') => {
                if self.list.len() > 0 {
                    let (&delete_key, template) = self
                        .config
                        .config
                        .templates
                        .iter()
                        .nth(self.list.highlight)
                        .unwrap();
                    let delete_name = template.name.clone();
                    self.mode = EditUiMode::Delete(delete_key, delete_name);
                }
            }
            Key::Char('e') => {
                if self.list.len() > 0 {
                    let rename_key = *self
                        .config
                        .config
                        .templates
                        .keys()
                        .nth(self.list.highlight)
                        .unwrap();
                    let current_description = self
                        .config
                        .config
                        .templates
                        .get(&rename_key)
                        .unwrap()
                        .description
                        .clone();
                    self.input = if let Some(description) = current_description {
                        InputField::new_with_content(description)
                    } else {
                        InputField::new()
                    };
                    self.mode = EditUiMode::Rename(rename_key);
                }
            }
            _ => {}
        }

        None
    }

    fn delete_input(
        &mut self,
        key: Key,
        template_key: &TemplateKey,
    ) -> Option<crate::ui::UiStateReaction> {
        match key {
            Key::Char('y') => {
                self.list.remove_entry(self.list.highlight);
                let template = self.config.config.templates.get(template_key).unwrap();
                let template_dir = template.path.clone(); // For use in error message.
                if let Err(err) = self.config.delete_template(template_key) {
                    match err {
                        crate::config::DeleteTemplateError::NoTemplate(_) => panic!(
                            "Tried to remove highlighted template, but config has no template of corresponding key."),
                        crate::config::DeleteTemplateError::IoErr(err) => {
                            let err_message = format!("There was an error deleting the template from disk. \
                            You may need to manually delete the following folder:\n\
                            {}\n\
                            Error:\n\
                            {}",
                    template_dir.to_string_lossy(),
                    err.to_string());
                            self.mode = EditUiMode::Error(err_message);
                        },
                    }
                } else {
                    self.mode = EditUiMode::List;
                }
            }
            _ => self.mode = EditUiMode::List,
        }

        None
    }

    fn rename_input(
        &mut self,
        key: Key,
        template_key: &TemplateKey,
    ) -> Option<crate::ui::UiStateReaction> {
        match key {
            Key::Left => self.input.caret_move_left(),
            Key::Right => self.input.caret_move_right(),
            Key::Backspace => self.input.backspace_char(),
            Key::Ctrl('c') => {
                self.mode = EditUiMode::List;
            }
            Key::Char('\n') | Key::Char('\r') => {
                let new_description = {
                    let new_description = self.input.consume_input();
                    if new_description.trim().is_empty() {
                        None
                    } else {
                        Some(new_description)
                    }
                };
                let template = self.config.config.templates.get_mut(template_key).unwrap();
                template.description = new_description;
                self.list
                    .replace_entry(self.list.highlight, Self::make_template_entry(&template));
                self.mode = EditUiMode::List;
            }
            Key::Char(c) => self.input.add_char(c),
            _ => {}
        }
        None
    }

    fn draw_help(&mut self, f: &mut tui::Frame<impl Backend>) -> Rect {
        let mut helps = vec![];
        if !self.config.config.templates.is_empty() {
            helps.extend(vec![
                ui::help::make_help_box("Up/K", "Move up in list"),
                ui::help::make_help_box("Down/J", "Move down in list"),
                ui::help::make_help_box("X", "Delete template"),
                ui::help::make_help_box("E", "Edit description"),
            ]);
        }
        helps.push(ui::help::make_help_box("Enter/Q", "Exit"));
        let (help_texts, help_boxes): (Vec<String>, Vec<VisualBox>) = helps.into_iter().unzip();
        ui::help::draw_help(help_texts, help_boxes, f, f.size())
    }

    fn draw_delete(&self, f: &mut tui::Frame<impl Backend>, name: &str) -> Rect {
        let size = f.size();
        let error_paragraph =
            Paragraph::new(format!("Are you sure you want to delete '{}'? [y/N]", name))
                .style(Style::default().bg(Color::Red).fg(Color::White));

        let paragraph_rect = Rect::new(size.left(), size.bottom().saturating_sub(1), size.width, 1);
        f.render_widget(error_paragraph, paragraph_rect);

        // Return remaining space to draw
        Rect::new(
            size.left(),
            size.top(),
            size.width,
            size.height.saturating_sub(1),
        )
    }

    fn draw_error(&self, f: &mut tui::Frame<impl Backend>, message: &'_ str) -> Rect {
        let size = f.size();
        let (message, newlines) = ui::layout::distribute_text(message, size.width);
        let height = std::cmp::min(size.height, newlines as u16);
        let paragraph_rect = Rect::new(size.left(), size.bottom() - height, size.width, height);
        let remaining = Rect::new(size.left(), size.top(), size.width, size.height - height);

        let error_paragraph =
            Paragraph::new(message).style(Style::default().bg(Color::Red).fg(Color::White));
        f.render_widget(error_paragraph, paragraph_rect);

        remaining
    }

    fn draw_prompt(&mut self, f: &mut tui::Frame<impl Backend>) -> Rect {
        let size = f.size();
        let prompt_text = if size.width > 45 {
            "New description: "
        } else {
            ":"
        };
        ui::input::draw_input(f, size, &mut self.input, prompt_text)
    }
}

impl<'conf, B: Backend> UiState<B> for EditUi<'conf> {
    fn require_ticking(&self) -> Option<std::time::Duration> {
        None
    }

    fn on_key(&mut self, key: Key) -> Option<crate::ui::UiStateReaction> {
        match self.mode {
            EditUiMode::List => self.list_input(key),
            EditUiMode::Delete(template_key, _) => self.delete_input(key, &template_key.clone()),
            EditUiMode::Rename(template_key) => self.rename_input(key, &template_key),
            EditUiMode::Error(_) => {
                self.mode = EditUiMode::List;
                None
            }
        }
    }

    fn on_tick(&mut self) -> Option<crate::ui::UiStateReaction> {
        None
    }

    fn draw(&mut self, f: &mut tui::Frame<B>) {
        let remaining = match &self.mode {
            EditUiMode::List => self.draw_help(f),
            EditUiMode::Delete(_key, name) => self.draw_delete(f, name),
            EditUiMode::Rename(_) => self.draw_prompt(f),
            EditUiMode::Error(err_message) => self.draw_error(f, err_message),
        };
        let block = Block::default().borders(Borders::ALL).title("Templates:");
        let block_inner = block.inner(remaining);
        f.render_widget(block, remaining);
        self.list.draw(f, block_inner);
    }
}

pub fn edit(config: &mut LoadedConfig) {
    let mut list_ui = EditUi::new(config);
    crate::ui::run_ui(&mut list_ui);
}
