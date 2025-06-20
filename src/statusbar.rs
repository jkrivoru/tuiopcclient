use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};

use crate::client::ConnectionStatus;

#[derive(Debug, Clone, PartialEq)]
pub enum Screen {
    Main,
}

pub struct StatusBarRenderer {
    pub status_message: String,
    pub connection_status: ConnectionStatus,
    pub current_screen: Screen,
}

impl StatusBarRenderer {
    pub fn new() -> Self {
        Self {
            status_message: "Ready".to_string(),
            connection_status: ConnectionStatus::Disconnected,
            current_screen: Screen::Main,
        }
    }

    pub fn render_status_bar(
        &self,
        f: &mut Frame,
        area: Rect,
        active_menu: Option<&crate::menu::MenuType>,
    ) {
        let status_text = format!(
            " {} | Status: {:?} | Screen: {:?} | Menu: {:?}",
            self.status_message, self.connection_status, self.current_screen, active_menu
        );

        let status =
            Paragraph::new(status_text).style(Style::default().bg(Color::Blue).fg(Color::White));
        f.render_widget(status, area);
    }

    pub fn set_status_message(&mut self, message: String) {
        self.status_message = message;
    }

    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }
    pub fn set_current_screen(&mut self, screen: Screen) {
        self.current_screen = screen;
    }
}
