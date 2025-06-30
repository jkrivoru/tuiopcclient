use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Paragraph};

/// Utility functions for common UI layouts and components
pub struct LayoutUtils;

impl LayoutUtils {
    /// Create a standard vertical layout with title and content areas
    #[allow(dead_code)]
    pub fn create_title_content_layout(area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Content
            ])
            .split(area)
            .to_vec()
    }
    /// Create a standard paragraph with title styling
    pub fn create_title_paragraph(title_text: &str) -> Paragraph {
        Paragraph::new(title_text)
            .style(Style::default().fg(Color::White).bg(Color::Blue))
            .block(Block::default().borders(Borders::ALL))
    }
    /// Create a two-column layout for forms
    #[allow(dead_code)]
    pub fn create_two_column_layout(area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area)
            .to_vec()
    }
    /// Create a vertical form layout with multiple rows
    pub fn create_form_layout(area: Rect, num_rows: usize) -> Vec<Rect> {
        let constraints: Vec<Constraint> = (0..num_rows).map(|_| Constraint::Length(3)).collect();

        Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area)
            .to_vec()
    }
    /// Create a horizontal button layout with margins and spacing
    pub fn create_button_layout(area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),  // Left margin
                Constraint::Length(18), // Left button (12 * 1.5 = 18)
                Constraint::Min(0),     // Space between
                Constraint::Length(18), // Right button (12 * 1.5 = 18)
                Constraint::Length(2),  // Right margin
            ])
            .split(area)
            .to_vec()
    }
}
