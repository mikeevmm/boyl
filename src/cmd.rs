pub mod new {
    pub const CMD_STR: &str = "new";
    pub const TEMPLATE_ARG: &str = "TEMPLATE";
    pub const NAME_ARG: &str = "NAME";
    pub const LOCATION_ARG: &str = "LOCATION";
}

pub mod make {
    use std::{error::Error, fmt::Debug, path::PathBuf, str::FromStr};

    use crate::ui;
    use colored::Colorize;
    use read_input::prelude::*;

    pub const CMD_STR: &str = "make";

    const ERR_PATH: &str = "Cannot understand path.";
    const ERR_NO_EXIST: &str = "Path does not exist.";
    const ERR_NOT_DIR: &str = "Path is not a directory.";

    #[derive(Clone)]
    struct UserPath {
        path_buf: PathBuf,
    }

    impl Debug for UserPath {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.path_buf.fmt(f)
        }
    }

    impl FromStr for UserPath {
        type Err = Box<dyn Error>;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let expanded = shellexpand::full(s)?;
            let path_buf = PathBuf::from_str(&expanded)?;
            Ok(UserPath { path_buf })
        }
    }

    impl From<PathBuf> for UserPath {
        fn from(path_buf: PathBuf) -> Self {
            UserPath { path_buf }
        }
    }

    mod ignore_ui {
        use regex::Regex;
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

        use crate::ui::{
            layout::{FileList, InputField, VisualBox},
            UiState, UiStateReaction,
        };

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

        pub struct IgnoreUi<'path> {
            file_list: FileList<'path>,
            file_widget: FileListWidget,
            mode: UiMode,
        }

        impl<'path> IgnoreUi<'path> {
            pub fn new(base_path: &'path Path) -> Self {
                IgnoreUi {
                    file_list: FileList::new(&base_path),
                    file_widget: FileListWidget::default(),
                    mode: UiMode::List,
                }
            }

            fn draw_help(&self, f: &mut tui::Frame<impl Backend>, size: Rect) -> Rect {
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
                make_help_box("Enter", "Finish");

                let buffer_rect = size;
                let positions = crate::ui::layout::distribute(buffer_rect.width, &help_boxes);
                let new_height = positions.last().unwrap().1 - positions[0].1 + 1;
                let start_y = max(buffer_rect.bottom() - new_height, buffer_rect.top());

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
                    f.render_widget(Paragraph::new(text), Rect::new(x, y, width, 1));
                }

                Rect::new(
                    size.left(),
                    size.top(),
                    size.width,
                    size.height - new_height,
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
                let paragraph_rect =
                    Rect::new(size.left(), size.bottom() - height, size.width, height);
                let remaining =
                    Rect::new(size.left(), size.top(), size.width, size.height - height);

                let error_paragraph =
                    Paragraph::new(message).style(Style::default().bg(Color::Red).fg(Color::White));
                f.render_widget(error_paragraph, paragraph_rect);

                remaining
            }

            fn draw_list(&mut self, f: &mut tui::Frame<impl Backend>, size: Rect) {
                if self.file_list.highlight < self.file_widget.buffer_start {
                    self.file_widget.buffer_start = self.file_list.highlight;
                } else if self.file_list.highlight
                    > self.file_widget.buffer_start + size.height as usize - 1
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
                    let file_name =
                        &file_name[file_name.len().saturating_sub(file_width)..file_name.len()];

                    let mut file_name_style = Style::default();
                    if highlighted {
                        file_name_style = file_name_style.bg(Color::DarkGray).fg(Color::White);
                    }
                    if !list_elem.included {
                        file_name_style = file_name_style.fg(Color::Gray);
                    }
                    if list_elem.path.is_dir() {
                        file_name_style = file_name_style.add_modifier(Modifier::BOLD);
                    }
                    let indented_file_name =
                        format!("{}{}", " ".repeat(list_elem.depth), file_name);
                    let file_name_paragraph =
                        Paragraph::new(indented_file_name).style(file_name_style);
                    let render_to = Rect::new(size.left(), render_y, line_width, 1);
                    f.render_widget(file_name_paragraph, render_to);
                }
            }

            fn ignore_pattern(&mut self, pattern: String) -> Result<(), regex::Error> {
                let regex = Regex::new(&pattern)?;
                self.file_list.exclude_pattern(regex);
                Ok(())
            }
        }

        impl<'paths, B> UiState<B> for IgnoreUi<'paths>
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
                                    self.file_list.exclude_file();
                                }
                                Key::Char('z') => {
                                    self.mode =
                                        UiMode::Input(InputMode::IgnorePattern, InputField::new());
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
                                    _ => todo!(),
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
    }

    pub fn make() {
        let current_dir = std::env::current_dir().ok();

        let template_dir = {
            let template_dir_default = current_dir.map(UserPath::from);
            let prompt = match &template_dir_default {
                Some(default) => format!(
                    "Template directory {}:",
                    format!("[default: {}]", default.path_buf.to_string_lossy()).dimmed()
                ),
                None => "Template directory:".to_string(),
            };
            match template_dir_default {
                Some(default) => input()
                    .repeat_msg(prompt)
                    .default(default)
                    .err(ERR_PATH.red())
                    .add_err_test(|p| p.path_buf.exists(), ERR_NO_EXIST.red())
                    .add_err_test(|p| p.path_buf.is_dir(), ERR_NOT_DIR.red())
                    .get(),
                None => input::<UserPath>()
                    .repeat_msg(prompt)
                    .err(ERR_PATH.red())
                    .add_err_test(|p| p.path_buf.exists(), ERR_NO_EXIST.red())
                    .add_err_test(|p| p.path_buf.is_dir(), ERR_NOT_DIR.red())
                    .get(),
            }
        };

        let template_name = {
            let template_name_default = template_dir
                .path_buf
                .file_name()
                .map(|s| s.to_string_lossy().to_string());
            let prompt = match &template_name_default {
                Some(default) => format!(
                    "Template name {}:",
                    format!("[default: {}]", default).dimmed()
                ),
                None => "Template name:".to_string(),
            };
            match template_name_default {
                Some(default) => input().repeat_msg(prompt).default(default).get(),
                None => input::<String>().repeat_msg(prompt).get(),
            }
        };

        let ignore_list = {
            let mut ui_state = ignore_ui::IgnoreUi::new(&template_dir.path_buf);
            ui::run_ui(&mut ui_state);
        };
    }
}

pub mod list {
    use crate::config::LoadedConfig;
    use colored::Colorize;

    pub const CMD_STR: &str = "list";

    pub fn list(config: &LoadedConfig) {
        for template in config.config.templates.values() {
            println!(
                "{}\n  {}",
                template.name.bold(),
                template
                    .description
                    .as_ref()
                    .unwrap_or(&"No description.".italic().to_string()),
            );
        }
    }
}

pub mod tree {
    pub const CMD_STR: &str = "tree";
}
