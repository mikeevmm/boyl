use self::list::FileList;
use super::{
    help,
    input::{self, InputField},
};
use crate::ui::{
    layout::{self, VisualBox},
    UiState, UiStateReaction,
};
use std::{cmp::min, path::Path};
use termion::event::Key;
use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
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
        let (help_texts, help_boxes): (Vec<String>, Vec<VisualBox>) = vec![
            super::help::make_help_box("Up/K", "Move up in list"),
            super::help::make_help_box("Down/J", "Move down in list"),
            super::help::make_help_box("O", "Open/Close folder"),
            super::help::make_help_box("X", "Exclude/Include file"),
            super::help::make_help_box("Z", "Exclude pattern"),
            super::help::make_help_box("R", "Reset"),
            super::help::make_help_box("Enter", "Finish"),
        ]
        .into_iter()
        .unzip();
        help::draw_help(help_texts, help_boxes, f, buffer_rect)
    }

    fn draw_prompt(
        &mut self,
        f: &mut tui::Frame<impl Backend>,
        size: Rect,
        input_field: &mut InputField,
    ) -> Rect {
        let prompt_text = if size.width > 45 {
            "Ignore pattern: "
        } else {
            ":"
        };
        input::draw_input(f, size, input_field, prompt_text)
    }

    fn draw_error(&self, f: &mut tui::Frame<impl Backend>, message: &'_ str) -> Rect {
        let size = f.size();
        let (message, newlines) = layout::distribute_text(message, size.width);
        let height = std::cmp::min(size.height, newlines as u16);
        let paragraph_rect = Rect::new(size.left(), size.bottom() - height, size.width, height);
        let remaining = Rect::new(size.left(), size.top(), size.width, size.height - height);

        let error_paragraph =
            Paragraph::new(message).style(Style::default().bg(Color::Red).fg(Color::White));
        f.render_widget(error_paragraph, paragraph_rect);

        remaining
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
        draw_list(&mut self.file_list, &mut self.file_widget, f, block_inner);
    }
}

pub struct FileTreeUi<'path> {
    file_list: FileList<'path>,
    file_widget: FileListWidget,
}

impl<'path> FileTreeUi<'path> {
    pub fn new(base_dir: &'path Path) -> Self {
        FileTreeUi {
            file_list: FileList::new(base_dir),
            file_widget: FileListWidget::default(),
        }
    }
}

impl<'path, B: Backend> UiState<B> for FileTreeUi<'path> {
    fn require_ticking(&self) -> Option<std::time::Duration> {
        None
    }

    fn on_key(&mut self, key: Key) -> Option<crate::ui::UiStateReaction> {
        match key {
            Key::Char('k') | Key::Up => {
                self.file_list.go_up();
                None
            }
            Key::Char('j') | Key::Down => {
                self.file_list.go_down();
                None
            }
            Key::Char('o') => {
                self.file_list.toggle_folder();
                None
            }
            Key::Char('\n') | Key::Char('\r') | Key::Ctrl('c') | Key::Char('q') => Some(UiStateReaction::Exit),
            _ => None,
        }
    }

    fn on_tick(&mut self) -> Option<crate::ui::UiStateReaction> {
        None
    }

    fn draw(&mut self, f: &mut tui::Frame<B>) {
        let (help_texts, help_boxes): (Vec<String>, Vec<VisualBox>) = vec![
            super::help::make_help_box("Up/K", "Move up in list"),
            super::help::make_help_box("Down/J", "Move down in list"),
            super::help::make_help_box("O", "Open/Close folder"),
            super::help::make_help_box("Enter/Q", "Exit"),
        ]
        .into_iter()
        .unzip();
        let remaining = crate::ui::help::draw_help(help_texts, help_boxes, f, f.size());
        let list_block = Block::default().borders(tui::widgets::Borders::ALL);
        let block_inner = list_block.inner(remaining);
        f.render_widget(list_block, remaining);
        draw_list(&mut self.file_list, &mut self.file_widget, f, block_inner);
    }
}

fn draw_list(
    file_list: &mut FileList,
    file_widget: &mut FileListWidget,
    f: &mut tui::Frame<impl Backend>,
    size: Rect,
) {
    if file_list.len() == 0 {
        return;
    }
    if file_list.highlight < file_widget.buffer_start {
        file_widget.buffer_start = file_list.highlight;
    } else if file_list.highlight
        > (file_widget.buffer_start + size.height as usize).saturating_sub(1)
    {
        file_widget.buffer_start = file_list.highlight.saturating_sub(size.height as usize) + 1;
    }

    let list_size = file_list.len();
    let buffer_start = file_widget.buffer_start;
    let buffer_end = min(file_widget.buffer_start + size.height as usize, list_size);
    for (i, list_elem) in file_list.iter_paths(buffer_start..buffer_end).enumerate() {
        let show_up_indicator = i == 0 && buffer_start > 0;
        let show_down_indicator =
            list_size > size.height as usize && i == (buffer_end - buffer_start - 1);
        let highlighted = file_list.highlight == buffer_start + i;
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
        let file_width = std::cmp::max(20, (size.width as usize).saturating_sub(list_elem.depth));
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
