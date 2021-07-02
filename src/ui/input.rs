use std::cmp::min;

use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::Paragraph,
};

/// A single-line input field, with a caret.
///
/// This struct does not handle translating user input to actions on the input
/// field, but rather provides functions to act on the input.
#[derive(Clone)]
pub struct InputField {
    input_buffer: String,
    caret_position: usize,
    buffer_start: usize,
}

impl InputField {
    pub fn new() -> Self {
        Self::new_with_content(String::with_capacity(80))
    }

    pub fn new_with_content(mut content: String) -> Self {
        content.push(' ');

        InputField {
            input_buffer: content,
            caret_position: 0,
            buffer_start: 0,
        }
    }

    pub fn add_char(&mut self, c: char) {
        self.input_buffer.insert(self.caret_position, c);
        self.caret_position += 1;
    }

    pub fn backspace_char(&mut self) {
        if self.caret_position == 0 {
            return;
        }
        self.input_buffer.remove(self.caret_position - 1);
        self.caret_position = self.caret_position.saturating_sub(1);
    }

    pub fn delete_char(&mut self) {
        if self.caret_position == self.input_buffer.len() - 1 {
            return;
        }
        self.input_buffer.remove(self.caret_position);
    }

    pub fn caret_move_left(&mut self) {
        self.caret_position = self.caret_position.saturating_sub(1);
    }

    pub fn caret_move_right(&mut self) {
        self.caret_position = min(
            self.input_buffer.len().saturating_sub(1),
            self.caret_position + 1,
        );
    }

    /// Return the string that should be rendered when displaying this input field
    /// (in a `width`-wide viewport), and the character that should be highlighted/
    /// have a caret before it.
    pub fn render(&mut self, width: u16) -> (String, usize) {
        if self.caret_position < self.buffer_start + 1 {
            self.buffer_start = self.caret_position;
        } else if self.caret_position > self.buffer_start + width as usize - 1 {
            self.buffer_start = self.caret_position.saturating_sub(width as usize) + 1;
        }
        let buffer_start = self.buffer_start;
        let buffer_end = min(buffer_start + width as usize, self.input_buffer.len());
        let highlighted = self.caret_position - buffer_start;
        (
            self.input_buffer[buffer_start..buffer_end].to_string(),
            highlighted,
        )
    }

    /// Return the user input that this input field currently has.
    pub fn consume_input(&self) -> String {
        self.input_buffer[..self.input_buffer.len() - 1].to_string()
    }
}

pub fn draw_input(
    f: &mut tui::Frame<impl Backend>,
    size: Rect,
    input_field: &mut InputField,
    prompt_text: &str,
) -> Rect {
    let prompt_rect = Rect::new(size.left(), size.bottom() - 1, size.width, 1);
    let remaining = Rect::new(size.left(), size.top(), size.width, size.height - 1);

    let (shown_input, highlighted) = input_field.render(remaining.width - prompt_text.len() as u16);

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
