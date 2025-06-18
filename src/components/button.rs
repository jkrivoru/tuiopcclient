use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonState {
    Normal,
    Hovered,
    Pressed,
    MouseDown, // New state for mouse down but not released
    Disabled,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonColor {
    Red,      // Cancel/destructive actions
    Green,    // Positive/continue actions
    Blue,     // Neutral/back actions
    Default,  // Default button color
}

#[derive(Debug, Clone, PartialEq)]
pub enum ButtonAction {
    Clicked,
    None,
}

#[derive(Debug, Clone)]
pub struct Button {
    pub id: String,
    pub label: String,
    pub hotkey: Option<char>, // For Alt+key shortcuts
    pub ctrl_key: Option<char>, // For Ctrl+key shortcuts
    pub state: ButtonState,
    pub color: ButtonColor,
    pub enabled: bool,
    pub area: Option<Rect>, // Set during rendering for click detection
}

impl Button {
    pub fn new(id: &str, label: &str) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            hotkey: None,
            ctrl_key: None,
            state: ButtonState::Normal,
            color: ButtonColor::Default,
            enabled: true,
            area: None,
        }
    }

    pub fn with_hotkey(mut self, key: char) -> Self {
        self.hotkey = Some(key);
        self
    }

    pub fn with_ctrl_key(mut self, key: char) -> Self {
        self.ctrl_key = Some(key);
        self
    }

    pub fn with_color(mut self, color: ButtonColor) -> Self {
        self.color = color;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {        self.enabled = enabled;
        self.state = if enabled { ButtonState::Normal } else { ButtonState::Disabled };
        self
    }

    pub fn set_state(&mut self, state: ButtonState) {
        if self.enabled {
            self.state = state;
        }
    }    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        // Only change state if we're not in a mouse interaction
        if self.state != ButtonState::MouseDown {
            self.state = if enabled { ButtonState::Normal } else { ButtonState::Disabled };
        } else if !enabled {
            // If being disabled while in MouseDown state, force disable
            self.state = ButtonState::Disabled;
        }
    }

    pub fn handle_key_input(&self, key: KeyCode, modifiers: KeyModifiers) -> ButtonAction {
        if !self.enabled {
            return ButtonAction::None;
        }

        match key {
            KeyCode::Char(c) => {
                // Check Alt+key shortcuts
                if modifiers.contains(KeyModifiers::ALT) {
                    if let Some(hotkey) = self.hotkey {
                        if c.to_lowercase().to_string() == hotkey.to_lowercase().to_string() {
                            return ButtonAction::Clicked;
                        }
                    }
                }
                // Check Ctrl+key shortcuts
                if modifiers.contains(KeyModifiers::CONTROL) {
                    if let Some(ctrl_key) = self.ctrl_key {
                        if c.to_lowercase().to_string() == ctrl_key.to_lowercase().to_string() {
                            return ButtonAction::Clicked;
                        }
                    }
                }
            }            _ => {}
        }
        
        ButtonAction::None
    }

    pub fn handle_mouse_click(&self, column: u16, row: u16) -> ButtonAction {
        if !self.enabled {
            return ButtonAction::None;
        }

        if let Some(area) = self.area {
            if column >= area.x 
                && column < area.x + area.width 
                && row >= area.y 
                && row < area.y + area.height 
            {
                return ButtonAction::Clicked;
            }
        }        ButtonAction::None
    }

    pub fn handle_mouse_down(&mut self, column: u16, row: u16) -> bool {
        if !self.enabled {
            return false;
        }

        if let Some(area) = self.area {
            if column >= area.x 
                && column < area.x + area.width 
                && row >= area.y 
                && row < area.y + area.height 
            {
                self.state = ButtonState::MouseDown;
                return true;
            }
        }        false
    }

    pub fn handle_mouse_up(&mut self, column: u16, row: u16) -> ButtonAction {
        if !self.enabled {
            return ButtonAction::None;
        }

        if self.state == ButtonState::MouseDown {
            // Reset state first
            self.state = ButtonState::Normal;
            
            // Check if mouse up is still within button area
            if let Some(area) = self.area {
                if column >= area.x 
                    && column < area.x + area.width 
                    && row >= area.y 
                    && row < area.y + area.height 
                {
                    return ButtonAction::Clicked;
                }
            }
        }

        ButtonAction::None
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Store area for click detection
        self.area = Some(area);

        let (border_style, text_style, bg_color) = match self.state {
            ButtonState::Normal => {
                let bg = self.get_background_color();
                (
                    Style::default().fg(Color::White).bg(bg),
                    Style::default().fg(Color::White).bg(bg),
                    bg,
                )
            },
            ButtonState::Hovered => {
                let bg = self.get_background_color();
                (
                    Style::default().fg(Color::Yellow).bg(bg),
                    Style::default().fg(Color::Yellow).bg(bg),
                    bg,
                )
            },
            ButtonState::MouseDown => {
                let bg = self.get_lighter_background_color();
                (
                    Style::default().fg(Color::White).bg(bg),
                    Style::default().fg(Color::White).bg(bg),
                    bg,
                )
            },
            ButtonState::Pressed => {
                let bg = self.get_background_color();
                (
                    Style::default().fg(Color::Green).bg(bg),
                    Style::default().fg(Color::Green).bg(bg),
                    bg,
                )
            },
            ButtonState::Disabled => (
                Style::default().fg(Color::DarkGray).bg(Color::Black),
                Style::default().fg(Color::DarkGray).bg(Color::Black),
                Color::Black,
            ),
        };

        // Create button text with hotkey highlighting
        let button_text = self.create_button_text(text_style);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style);

        let paragraph = Paragraph::new(button_text)
            .block(block)
            .style(text_style);

        f.render_widget(paragraph, area);
    }

    fn get_background_color(&self) -> Color {
        match self.color {
            ButtonColor::Red => Color::Red,
            ButtonColor::Green => Color::Green,
            ButtonColor::Blue => Color::Blue,
            ButtonColor::Default => Color::DarkGray,
        }
    }

    fn get_lighter_background_color(&self) -> Color {
        match self.color {
            ButtonColor::Red => Color::LightRed,
            ButtonColor::Green => Color::LightGreen,
            ButtonColor::Blue => Color::LightBlue,
            ButtonColor::Default => Color::Gray,
        }
    }

    fn create_button_text(&self, base_style: Style) -> Line {
        let mut spans = Vec::new();
        
        if let Some(hotkey) = self.hotkey {
            // Find the hotkey character in the label and underline it
            let hotkey_lower = hotkey.to_lowercase().to_string();
            let label_chars: Vec<char> = self.label.chars().collect();
            let mut found_hotkey = false;
            
            for (_i, ch) in label_chars.iter().enumerate() {
                if !found_hotkey && ch.to_lowercase().to_string() == hotkey_lower {
                    // Underline the hotkey character
                    spans.push(Span::styled(
                        ch.to_string(),
                        base_style.add_modifier(Modifier::UNDERLINED),
                    ));
                    found_hotkey = true;
                } else {
                    spans.push(Span::styled(ch.to_string(), base_style));
                }
            }
            
            // Add hotkey hint if not found in label
            if !found_hotkey {
                spans.push(Span::styled(self.label.clone(), base_style));
                spans.push(Span::styled(
                    format!(" (Alt+{})", hotkey.to_uppercase()),
                    Style::default().fg(Color::DarkGray),
                ));
            }
        } else if let Some(ctrl_key) = self.ctrl_key {
            // Add ctrl key hint
            spans.push(Span::styled(self.label.clone(), base_style));
            spans.push(Span::styled(
                format!(" (Ctrl+{})", ctrl_key.to_uppercase()),
                Style::default().fg(Color::DarkGray),
            ));
        } else {
            spans.push(Span::styled(self.label.clone(), base_style));
        }

        Line::from(spans)
    }
}

// Button manager for handling multiple buttons
#[derive(Debug)]
pub struct ButtonManager {
    buttons: Vec<Button>,
    focused_button: Option<usize>,
}

impl ButtonManager {
    pub fn new() -> Self {
        Self {
            buttons: Vec::new(),
            focused_button: None,
        }
    }

    pub fn add_button(&mut self, button: Button) {
        self.buttons.push(button);
    }

    pub fn clear(&mut self) {
        self.buttons.clear();
        self.focused_button = None;
    }

    pub fn handle_key_input(&mut self, key: KeyCode, modifiers: KeyModifiers) -> Option<String> {
        // Handle Tab navigation between buttons
        if key == KeyCode::Tab && !self.buttons.is_empty() {
            self.focus_next_button();
            return None;
        }

        // Handle Enter on focused button
        if key == KeyCode::Enter {
            if let Some(focused_idx) = self.focused_button {
                if let Some(button) = self.buttons.get(focused_idx) {
                    if button.enabled {
                        return Some(button.id.clone());
                    }
                }
            }
        }

        // Check all buttons for hotkey matches
        for button in &self.buttons {
            if button.handle_key_input(key, modifiers) == ButtonAction::Clicked {
                return Some(button.id.clone());
            }        }
        
        None
    }

    pub fn handle_mouse_click(&mut self, column: u16, row: u16) -> Option<String> {
        for (idx, button) in self.buttons.iter().enumerate() {
            if button.handle_mouse_click(column, row) == ButtonAction::Clicked {                self.focused_button = Some(idx);
                return Some(button.id.clone());
            }
        }
        None
    }

    pub fn handle_mouse_down(&mut self, column: u16, row: u16) -> bool {
        for button in &mut self.buttons {
            if button.handle_mouse_down(column, row) {
                return true;
            }
        }
        false
    }

    pub fn handle_mouse_up(&mut self, column: u16, row: u16) -> Option<String> {
        for (idx, button) in self.buttons.iter_mut().enumerate() {
            if button.handle_mouse_up(column, row) == ButtonAction::Clicked {
                self.focused_button = Some(idx);
                return Some(button.id.clone());
            }        }
        None
    }

    pub fn render_buttons(&mut self, f: &mut Frame, areas: &[Rect]) {
        // Don't automatically reset button states - let them manage their own state
        // Only update focus highlighting when no buttons are in MouseDown state
        let has_mouse_down = self.buttons.iter().any(|b| b.state == ButtonState::MouseDown);
        
        if !has_mouse_down {
            for (idx, button) in self.buttons.iter_mut().enumerate() {
                if Some(idx) == self.focused_button && button.enabled {
                    button.set_state(ButtonState::Hovered);
                } else if button.enabled && button.state != ButtonState::MouseDown {
                    button.set_state(ButtonState::Normal);
                }
            }
        }

        // Render each button in its designated area
        for (idx, area) in areas.iter().enumerate() {
            if let Some(button) = self.buttons.get_mut(idx) {
                button.render(f, *area);
            }
        }
    }

    fn focus_next_button(&mut self) {
        if self.buttons.is_empty() {
            return;
        }

        let enabled_buttons: Vec<usize> = self.buttons
            .iter()
            .enumerate()
            .filter(|(_, btn)| btn.enabled)
            .map(|(idx, _)| idx)
            .collect();

        if enabled_buttons.is_empty() {
            self.focused_button = None;
            return;
        }

        match self.focused_button {
            None => {
                self.focused_button = Some(enabled_buttons[0]);
            }
            Some(current) => {
                if let Some(current_pos) = enabled_buttons.iter().position(|&x| x == current) {
                    let next_pos = (current_pos + 1) % enabled_buttons.len();
                    self.focused_button = Some(enabled_buttons[next_pos]);
                } else {
                    self.focused_button = Some(enabled_buttons[0]);
                }
            }
        }
    }

    pub fn get_button_mut(&mut self, id: &str) -> Option<&mut Button> {
        self.buttons.iter_mut().find(|b| b.id == id)
    }

    pub fn set_button_enabled(&mut self, id: &str, enabled: bool) {
        if let Some(button) = self.get_button_mut(id) {
            button.set_enabled(enabled);
        }
    }
}
