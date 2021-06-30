use std::marker::PhantomData;

use tui::{
    backend::Backend,
    layout::Rect,
    style::{Color, Style},
    text::{Spans, Text},
    widgets::Paragraph,
};

pub trait ListElement<'t> {
    fn get_list_element(&self) -> Text<'t>;
}

impl<'t> ListElement<'t> for Spans<'t> {
    fn get_list_element(&self) -> Text<'t> {
        self.clone().into()
    }
}

pub struct List<'t, T>
where
    T: ListElement<'t>,
{
    phantom: PhantomData<&'t T>,
    highlight: usize,
    buffer_start: usize,
    elements: Vec<T>,
}

impl<'t, T> List<'t, T>
where
    T: ListElement<'t>,
{
    pub fn new(elements: Vec<T>) -> Self {
        List {
            phantom: PhantomData,
            highlight: 0,
            buffer_start: 0,
            elements,
        }
    }

    fn len(&self) -> usize {
        self.elements.len()
    }

    pub fn go_up(&mut self) {
        self.highlight = if self.highlight == 0 {
            self.elements.len().saturating_sub(1)
        } else {
            self.highlight.saturating_sub(1)
        };
    }

    pub fn go_down(&mut self) {
        self.highlight = if self.highlight == self.elements.len().saturating_sub(1) {
            0
        } else {
            self.highlight.saturating_add(1)
        };
    }

    pub fn draw(&mut self, f: &mut tui::Frame<impl Backend>, size: Rect) {
        if self.highlight < self.buffer_start {
            self.buffer_start = self.highlight;
        } else if self.highlight > (self.buffer_start + size.height as usize).saturating_sub(1) {
            self.buffer_start = self.highlight.saturating_sub(size.height as usize) + 1;
        }

        let self_size = self.len();
        let buffer_start = self.buffer_start;
        let buffer_end = std::cmp::min(self.buffer_start + size.height as usize, self_size);
        for (i, list_elem) in self.elements[buffer_start..buffer_end].iter().enumerate() {
            let show_up_indicator = i == 0 && buffer_start > 0;
            let show_down_indicator =
                self_size > size.height as usize && i == (buffer_end - buffer_start - 1);
            let highlighted = self.highlight == buffer_start + i;
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

            let mut entry_style = Style::default();
            if highlighted {
                entry_style = entry_style.bg(Color::DarkGray).fg(Color::White);
            }
            let entry_paragraph = Paragraph::new(list_elem.get_list_element()).style(entry_style);
            let render_to = Rect::new(size.left(), render_y, line_width, 1);
            f.render_widget(entry_paragraph, render_to);
        }
    }
}
