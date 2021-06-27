use self::list::FileList;
use super::input::InputField;
use crate::ui::{layout::VisualBox, UiState, UiStateReaction};
use std::{
    cmp::{max, min},
    path::Path,
};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Paragraph},
};

pub mod list;

#[derive(Clone, Copy)]
enum InputMode {
    IgnorePattern,
}

#[derive(Clone)]
enum UiMode {
    List,
    Input(InputMode, InputField),
    Error(String),
}

struct FileListWidget {
    buffer_start: usize,
}

impl Default for FileListWidget {
    fn default() -> Self {
        FileListWidget { buffer_start: 0 }
    }
}

pub struct FilePickerUi<'path> {
    base_path: &'path Path,
    pub file_list: FileList<'path>,
    file_widget: FileListWidget,
    mode: UiMode,
    pub aborted: bool,
}

impl<'path> FilePickerUi<'path> {
    pub fn new(base_path: &'path Path) -> Self {
        FilePickerUi {
            base_path,
            file_list: FileList::new(&base_path),
            file_widget: FileListWidget::default(),
            mode: UiMode::List,
            aborted: false,
        }
    }

    fn draw_help(&self, f: &mut tui::Frame<impl Backend>, buffer_rect: Rect) -> Rect {
        let mut help_texts = vec![];
        let mut help_boxes = vec![];
        let mut make_help_box = |button: &'static str, info: &'static str| {
            let help_text = format!("[{}] {}", button, info);
            let help_box = VisualBox::new(help_text.len() as u16, 1);
            help_texts.push(help_text);
            help_boxes.push(help_box);
        };

        make_help_box("Up/K", "Move up in list");
        make_help_box("Down/J", "Move down in list");
        make_help_box("O", "Open/Close folder");
        make_help_box("X", "Exclude/Include file");
        make_help_box("Z", "Exclude pattern");
        make_help_box("R", "Reset");
        make_help_box("Enter", "Finish");

        let positions = crate::ui::layout::distribute(buffer_rect.width, &help_boxes);
        let new_height = min(
            positions.last().unwrap().1 - positions[0].1 + 1,
            buffer_rect.height,
        );
        let start_y = max(
            buffer_rect.bottom().saturating_sub(new_height),
            buffer_rect.top(),
        );

        // Draw a green background (a bit hacky)
        f.render_widget(
            Block::default().style(Style::default().bg(Color::Green).fg(Color::Black)),
            Rect::new(buffer_rect.left(), start_y, buffer_rect.width, new_height),
        );
        // Draw the labels
        for ((x, y), text) in positions.iter().zip(help_texts) {
            let x = x + buffer_rect.left();
            let y = y + start_y;

            if y > buffer_rect.bottom() {
                break;
            }

            let width = text.len() as u16;
            let height = min(1, buffer_rect.height);
            let y = min(y, buffer_rect.bottom().saturating_sub(1));
            f.render_widget(Paragraph::new(text), Rect::new(x, y, width, height));
        }

        Rect::new(
            buffer_rect.left(),
            buffer_rect.top(),
            buffer_rect.width,
            buffer_rect.height - new_height,
        )
    }

    fn draw_prompt(
        &mut self,
        f: &mut tui::Frame<impl Backend>,
        size: Rect,
        input_field: &mut InputField,
    ) -> Rect {
        let prompt_rect = Rect::new(size.left(), size.bottom() - 1, size.width, 1);
        let remaining = Rect::new(size.left(), size.top(), size.width, size.height - 1);

        let prompt_text = if prompt_rect.width > 45 {
            "Ignore pattern: "
        } else {
            ":"
        };
        let (shown_input, highlighted) =
            input_field.render(remaining.width - prompt_text.len() as u16);

        f.render_widget(
            Paragraph::new(vec![Spans::from(vec![
                Span::styled(prompt_text, Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&shown_input[0..highlighted]),
                Span::styled(
                    shown_input.chars().nth(highlighted).unwrap().to_string(),
                    Style::default().bg(Color::White).fg(Color::Black),
                ),
                Span::raw(&shown_input[highlighted + 1..]),
            ])])
            .style(Style::default().bg(Color::Green).fg(Color::Black)),
            prompt_rect,
        );

        remaining
    }

    fn draw_error(&self, f: &mut tui::Frame<impl Backend>, message: &'_ str) -> Rect {
        let size = f.size();
        let newlines = message.lines().count() as u16;
        let height = min(size.height, newlines);
        let paragraph_rect = Rect::new(size.left(), size.bottom() - height, size.width, height);
        let remaining = Rect::new(size.left(), size.top(), size.width, size.height - height);

        let error_paragraph =
            Paragraph::new(message).style(Style::default().bg(Color::Red).fg(Color::White));
        f.render_widget(error_paragraph, paragraph_rect);

        remaining
    }

    fn draw_list(&mut self, f: &mut tui::Frame<impl Backend>, size: Rect) {
        if self.file_list.highlight < self.file_widget.buffer_start {
            self.file_widget.buffer_start = self.file_list.highlight;
        } else if self.file_list.highlight
            > (self.file_widget.buffer_start + size.height as usize).saturating_sub(1)
        {
            self.file_widget.buffer_start = self
                .file_list
                .highlight
                .saturating_sub(size.height as usize)
                + 1;
        }

        let list_size = self.file_list.len();
        let buffer_start = self.file_widget.buffer_start;
        let buffer_end = min(
            self.file_widget.buffer_start + size.height as usize,
            list_size,
        );
        for (i, list_elem) in self
            .file_list
            .iter_paths(buffer_start..buffer_end)
            .enumerate()
        {
            let show_up_indicator = i == 0 && buffer_start > 0;
            let show_down_indicator =
                list_size > size.height as usize && i == (buffer_end - buffer_start - 1);
            let highlighted = self.file_list.highlight == buffer_start + i;
            let render_y = size.top() + i as u16;

            if show_up_indicator || show_down_indicator {
                let indicator = if show_up_indicator { "▲" } else { "▼" };
                let render_to = Rect::new(size.right().saturating_sub(1), render_y, 1, 1);
                f.render_widget(Paragraph::new(indicator), render_to);
            }

            let mut line_width = size.width;
            if show_up_indicator || show_down_indicator {
                line_width = line_width.saturating_sub(1)
            }

            // We wish to have text left-aligned, but to show the ending of the path
            // if it is too big to fit in the frame.
            let file_width = max(20, (size.width as usize).saturating_sub(list_elem.depth));
            let file_name = list_elem.path.to_string_lossy();
            let file_name = &file_name[file_name.len().saturating_sub(file_width)..file_name.len()];

            let mut file_name_style = Style::default();
            if highlighted {
                file_name_style = file_name_style.bg(Color::DarkGray).fg(Color::White);
            }
            if !list_elem.included {
                file_name_style = file_name_style.add_modifier(Modifier::DIM);
            }
            if list_elem.path.is_dir() {
                file_name_style = file_name_style.add_modifier(Modifier::BOLD | Modifier::ITALIC);
            }
            let indented_file_name = format!("{}{}", " ".repeat(list_elem.depth), file_name);
            let file_name_paragraph = Paragraph::new(indented_file_name).style(file_name_style);
            let render_to = Rect::new(size.left(), render_y, line_width, 1);
            f.render_widget(file_name_paragraph, render_to);
        }
    }

    fn ignore_pattern(&mut self, pattern: String) -> Result<(), Box<dyn std::error::Error>> {
        self.file_list.exclude_pattern(&pattern)?;
        Ok(())
    }
}

impl<'paths, B> UiState<B> for FilePickerUi<'paths>
where
    B: Backend,
{
    fn require_ticking(&self) -> Option<std::time::Duration> {
        None
    }

    fn on_key(&mut self, key: termion::event::Key) -> Option<crate::ui::UiStateReaction> {
        match &mut self.mode {
            UiMode::List => {
                if let Key::Ctrl('c') = key {
                    self.aborted = true;
                    Some(UiStateReaction::Exit)
                } else {
                    match key {
                        Key::Up | Key::Char('k') => {
                            self.file_list.go_up();
                        }
                        Key::Down | Key::Char('j') => {
                            self.file_list.go_down();
                        }
                        Key::Char('o') => {
                            self.file_list.toggle_folder();
                        }
                        Key::Char('x') => {
                            self.file_list.toggle_exclude_file();
                        }
                        Key::Char('r') => {
                            self.file_list = FileList::new(self.base_path);
                        }
                        Key::Char('z') => {
                            self.mode = UiMode::Input(InputMode::IgnorePattern, InputField::new());
                        }
                        Key::Char('\n') | Key::Char('\r') => {
                            return Some(UiStateReaction::Exit);
                        }
                        _ => {}
                    };
                    None
                }
            }
            UiMode::Input(mode, input_field) => {
                match key {
                    Key::Ctrl('c') => {
                        // Abort.
                        self.mode = UiMode::List;
                    }
                    Key::Char('\n') | Key::Char('\r') => {
                        let pattern = input_field.consume_input();
                        match mode {
                            InputMode::IgnorePattern => {
                                self.mode = match self.ignore_pattern(pattern) {
                                    Ok(()) => UiMode::List,
                                    Err(err) => UiMode::Error(err.to_string()),
                                }
                            }
                        }
                    }
                    Key::Char('\t') => {}
                    Key::Char(c) => input_field.add_char(c),
                    Key::Backspace => input_field.backspace_char(),
                    Key::Delete => input_field.delete_char(),
                    Key::Left => input_field.caret_move_left(),
                    Key::Right => input_field.caret_move_right(),
                    _ => {}
                };
                None
            }
            UiMode::Error(_) => {
                self.mode = UiMode::List;
                None
            }
        }
    }

    fn on_tick(&mut self) -> Option<crate::ui::UiStateReaction> {
        None
    }

    fn draw(&mut self, f: &mut tui::Frame<B>) {
        let mut mode = self.mode.clone();
        let remaining = match &mut mode {
            UiMode::List => self.draw_help(f, f.size()),
            UiMode::Input(_, input_field) => self.draw_prompt(f, f.size(), input_field),
            UiMode::Error(err_msg) => self.draw_error(f, err_msg),
        };
        let list_block = Block::default().borders(tui::widgets::Borders::ALL);
        let block_inner = list_block.inner(remaining);
        f.render_widget(list_block, remaining);
        self.draw_list(f, block_inner);
    }
}