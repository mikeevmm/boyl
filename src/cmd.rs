pub mod new {
    pub const CMD_STR: &str = "new";
    pub const TEMPLATE_ARG: &str = "TEMPLATE";
    pub const NAME_ARG: &str = "NAME";
    pub const LOCATION_ARG: &str = "LOCATION";
}

pub mod make {
    use std::{convert::TryInto, error::Error, fmt::Debug, path::PathBuf, str::FromStr};

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
        use std::{cmp::max, path::PathBuf};
        use termion::event::Key;
        use tui::{backend::Backend, layout::Rect, style::{Color, Style}, widgets::{Block, Borders, Paragraph}};

        use crate::ui::{layout::VisualBox, UiState, UiStateReaction};

        enum IgnoreUiMode {
            List,
            Input,
        }

        pub struct IgnoreUi<'paths> {
            base_path: PathBuf,
            ignore_patterns: Vec<Regex>,
            open: Vec<&'paths PathBuf>,
            mode: IgnoreUiMode,
        }

        impl<'paths> IgnoreUi<'paths> {
            pub fn new(base_path: PathBuf) -> Self {
                IgnoreUi {
                    base_path,
                    ignore_patterns: vec![],
                    open: vec![],
                    mode: IgnoreUiMode::List,
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

                make_help_box("Up/J", "Move up in list");
                make_help_box("Down/K", "Move down in list");
                make_help_box("O", "Open/Close folder");
                make_help_box("X", "Exclude/Include file");
                make_help_box("C", "Exclude pattern");
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

                Rect::new(size.left(), size.top(), size.width, size.height - new_height)
            }

            fn draw_list(&self, f: &mut tui::Frame<impl Backend>, size: Rect) {
                let block = Block::default().title("TODO LIST").borders(Borders::ALL);
                f.render_widget(block, size);
            }
        }

        impl<'paths, B> UiState<B> for IgnoreUi<'paths>
        where
            B: Backend,
        {
            fn require_ticking(&self) -> Option<std::time::Duration> {
                None
            }

            fn on_key(
                &mut self,
                key: termion::event::Key,
            ) -> Option<crate::ui::UiStateReaction<B>> {
                if let Key::Ctrl('c') = key {
                    Some(UiStateReaction::Exit)
                } else {
                    None
                }
            }

            fn on_tick(&mut self) -> Option<crate::ui::UiStateReaction<B>> {
                None
            }

            fn draw(&self, f: &mut tui::Frame<B>) {
                let remaining = self.draw_help(f, f.size());
                self.draw_list(f, remaining);
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
            let ui_state = Box::new(ignore_ui::IgnoreUi::new(template_dir.path_buf));
            ui::run_ui(ui_state);
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
                "\
    {}
      {}",
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
