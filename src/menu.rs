use ratatui::{
    layout::{Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::client::ConnectionStatus;

#[derive(Debug, Clone, PartialEq)]
pub enum MenuType {
    File,
    Browse,
}

pub struct MenuRenderer {
    pub active_menu: Option<MenuType>,
    pub connection_status: ConnectionStatus,
}

impl MenuRenderer {
    pub fn new() -> Self {
        Self {
            active_menu: None,
            connection_status: ConnectionStatus::Disconnected,
        }
    }

    pub fn render_menu_bar(&self, f: &mut Frame, area: Rect) {
        // Create menu items with proper highlighting
        let mut menu_items = vec![];
        
        // File menu
        let file_style = if self.active_menu == Some(MenuType::File) {
            Style::default().fg(Color::White).bg(Color::Blue)
        } else {
            Style::default().fg(Color::Black).bg(Color::Gray)
        };
        menu_items.push(Span::styled(" File ", file_style));
        menu_items.push(Span::raw("  "));
        
        // Browse menu (only enabled when connected)
        let browse_style = if self.connection_status == ConnectionStatus::Connected {
            if self.active_menu == Some(MenuType::Browse) {
                Style::default().fg(Color::White).bg(Color::Blue)
            } else {
                Style::default().fg(Color::Black).bg(Color::Gray)
            }
        } else {
            Style::default().fg(Color::DarkGray).bg(Color::Gray)
        };
        menu_items.push(Span::styled(" Browse ", browse_style));
        menu_items.push(Span::raw("  "));
        
        // Additional menu items (not clickable yet)
        menu_items.push(Span::styled(" Subscribe ", Style::default().fg(Color::Black).bg(Color::Gray)));
        menu_items.push(Span::raw("  "));
        menu_items.push(Span::styled(" Help ", Style::default().fg(Color::Black).bg(Color::Gray)));
        
        let menu = Paragraph::new(Line::from(menu_items))
            .style(Style::default().bg(Color::Blue).fg(Color::White));
        f.render_widget(menu, area);
    }

    pub fn render_dropdown_menu(&self, f: &mut Frame, menu_type: MenuType) {
        match menu_type {
            MenuType::File => {
                // File dropdown menu
                let dropdown_area = Rect {
                    x: 1,  // Start just under "File"
                    y: 1,  // Row below menu bar
                    width: 25,  // Wide enough for menu items
                    height: 6,  // Height for 4 items + borders
                };
                
                let menu_items = ["Connect...       Ctrl+C",
                    "Server Info...   Ctrl+I", 
                    "─────────────────────────",
                    "Exit             Alt+X"];
                
                let dropdown = Paragraph::new(menu_items.join("\n"))
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)))
                    .style(Style::default().bg(Color::White).fg(Color::Black));
                
                // Clear the area first to ensure dropdown is visible
                f.render_widget(Clear, dropdown_area);
                f.render_widget(dropdown, dropdown_area);
            }
            MenuType::Browse => {
                // Browse dropdown menu
                let dropdown_area = Rect {
                    x: 9,  // Start just under "Browse"
                    y: 1,  // Row below menu bar
                    width: 20,  // Wide enough for menu items
                    height: 4,  // Height for 2 items + borders
                };
                
                let menu_items = ["Browse Server    F5",
                    "Refresh          F6"];
                
                let dropdown = Paragraph::new(menu_items.join("\n"))
                    .block(Block::default()
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::Yellow)))
                    .style(Style::default().bg(Color::White).fg(Color::Black));
                
                // Clear the area first to ensure dropdown is visible
                f.render_widget(Clear, dropdown_area);
                f.render_widget(dropdown, dropdown_area);
            }
        }
    }

    pub fn handle_menu_click(&mut self, column: u16) -> Option<MenuType> {
        // Menu positions: File (1-6), Browse (8-15)
        if (1..=6).contains(&column) {
            if self.active_menu == Some(MenuType::File) {
                self.active_menu = None;
            } else {
                self.active_menu = Some(MenuType::File);
            }
            Some(MenuType::File)
        } else if (8..=15).contains(&column) && self.connection_status == ConnectionStatus::Connected {
            if self.active_menu == Some(MenuType::Browse) {
                self.active_menu = None;
            } else {
                self.active_menu = Some(MenuType::Browse);
            }
            Some(MenuType::Browse)
        } else {
            self.active_menu = None;
            None
        }
    }

    pub fn close_menu(&mut self) {
        self.active_menu = None;
    }

    pub fn set_connection_status(&mut self, status: ConnectionStatus) {
        self.connection_status = status;
    }

    pub fn get_active_menu(&self) -> Option<MenuType> {
        self.active_menu.clone()
    }
}
