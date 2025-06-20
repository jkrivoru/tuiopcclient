use super::types::*;

impl ConnectScreen {
    pub fn handle_mouse_down(&mut self, column: u16, row: u16) -> bool {
        self.button_manager.handle_mouse_down(column, row)
    }

    pub fn handle_mouse_up(&mut self, column: u16, row: u16) -> Option<String> {
        self.button_manager.handle_mouse_up(column, row)
    }
}
