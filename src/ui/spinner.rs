use std::str;

const SPINNER_CHARS: &[&str] = &[
    "⠉", "⠋", "⠍", "⠎", "⡅", "⡇", "⡆", "⣄", "⣠", "⣈", "⣘", "⢱",
];

pub struct Spinner {
    idx: usize,
}

impl Spinner {
    pub fn new() -> Self {
        Spinner { idx: 0 }
    }

    pub fn tick(&mut self) -> &'static str {
        self.idx = (self.idx + 1) % SPINNER_CHARS.len();
        &SPINNER_CHARS[self.idx]
    }
}
