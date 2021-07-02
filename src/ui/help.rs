use tui::{backend::Backend, layout::Rect, style::Style, widgets::{Block, Paragraph}};

use crate::ui::layout::VisualBox;

pub fn make_help_box(button: &'static str, help: &'static str) -> (String, VisualBox) {
    let help_text = format!("[{}] {}", button, help);
    let help_box = VisualBox::new(help_text.len() as u16, 1);
    (help_text, help_box)
}

pub fn draw_help(help_texts: Vec<String>, help_boxes: Vec<VisualBox>, f: &mut tui::Frame<impl Backend>, buffer_rect: Rect) -> Rect {
    let positions = crate::ui::layout::distribute(buffer_rect.width, &help_boxes);
    let new_height = std::cmp::min(
        positions.last().unwrap().1 - positions[0].1 + 1,
        buffer_rect.height,
    );
    let start_y = std::cmp::max(
        buffer_rect.bottom().saturating_sub(new_height),
        buffer_rect.top(),
    );

    // Draw a green background (a bit hacky)
    f.render_widget(
        Block::default().style(
            Style::default()
                .bg(tui::style::Color::Green)
                .fg(tui::style::Color::Black),
        ),
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
        let height = std::cmp::min(1, buffer_rect.height);
        let y = std::cmp::min(y, buffer_rect.bottom().saturating_sub(1));
        f.render_widget(Paragraph::new(text), Rect::new(x, y, width, height));
    }

    Rect::new(
        buffer_rect.left(),
        buffer_rect.top(),
        buffer_rect.width,
        buffer_rect.height - new_height,
    )
}
