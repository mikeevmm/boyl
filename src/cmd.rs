pub mod new {
    pub const CMD_STR: &str = "new";
    pub const TEMPLATE_ARG: &str = "TEMPLATE";
    pub const NAME_ARG: &str = "NAME";
    pub const LOCATION_ARG: &str = "LOCATION";
}

pub mod make {
    use crate::ui;

    pub const CMD_STR: &str = "make";

    /// FSM UI States for the Make command
    mod states {
        use termion::event::Key;
        use tui::{
            backend::Backend,
            widgets::{Block, Borders},
        };

        use crate::ui::{UiState, UiStateReaction};

        enum Highlighted {
            Name,
            Description,
        }

        impl Highlighted {
            fn cycle_up(&self) -> Highlighted {
                match self {
                    Highlighted::Name => Highlighted::Description,
                    Highlighted::Description => Highlighted::Name,
                }
            }

            fn cycle_down(&self) -> Highlighted {
                match self {
                    Highlighted::Name => Highlighted::Name,
                    Highlighted::Description => Highlighted::Description,
                }
            }
        }

        pub struct Form {
            highlighted: Highlighted,
            name: String,
            description: String,
        }

        impl Form {
            pub fn new() -> Self {
                Form {
                    highlighted: Highlighted::Name,
                    name: String::with_capacity(20_usize),
                    description: String::with_capacity(80_usize),
                }
            }
        }

        impl<B: Backend> UiState<B> for Form {
            fn require_ticking(&self) -> Option<std::time::Duration> {
                None
            }

            fn on_key(
                &mut self,
                key: termion::event::Key,
            ) -> Option<crate::ui::UiStateReaction<B>> {
                let field = match self.highlighted {
                    Highlighted::Name => &mut self.name,
                    Highlighted::Description => &mut self.description,
                };

                match key {
                    Key::Ctrl('c') => Some(UiStateReaction::Exit),
                    Key::Char(char) => {
                        todo!()
                    }
                    Key::Backspace => {
                        todo!()
                    }
                    Key::Up => {
                        self.highlighted = self.highlighted.cycle_up();
                        None
                    }
                    Key::Down => {
                        self.highlighted = self.highlighted.cycle_down();
                        None
                    }
                    _ => None,
                }
            }

            fn on_tick(&mut self) -> Option<crate::ui::UiStateReaction<B>> {
                None
            }

            fn draw(&self, f: &mut tui::Frame<B>) {
                let size = f.size();
            }
        }
    }

    pub fn make() {
        ui::run_ui(Box::new(states::Form::new()));
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
