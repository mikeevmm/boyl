use std::cmp::min;

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
        let mut input_buffer = String::with_capacity(80);
        input_buffer.push(' ');

        InputField {
            input_buffer,
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
