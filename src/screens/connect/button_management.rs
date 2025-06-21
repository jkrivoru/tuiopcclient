use super::types::*;
use crate::client::ConnectionStatus;
use crate::components::{Button, ButtonColor};
use anyhow::Result;

pub struct ButtonConfig {
    pub id: &'static str,
    pub label: &'static str,
    pub hotkey: char,
    pub color: ButtonColor,
}

impl ConnectScreen {
    pub fn setup_buttons_for_current_step(&mut self) {
        self.button_manager.clear();

        let configs = self.get_button_configs();
        for config in configs {
            let enabled = self.is_button_enabled(config.id);
            
            self.button_manager.add_button(
                Button::new(config.id, config.label)
                    .with_hotkey(config.hotkey)
                    .with_color(config.color)
                    .with_enabled(enabled)
            );
        }
    }

    fn get_button_configs(&self) -> Vec<ButtonConfig> {
        match self.step {
            ConnectDialogStep::ServerUrl => vec![
                ButtonConfig {
                    id: "cancel",
                    label: "Cancel",
                    hotkey: 'c',
                    color: ButtonColor::Red,
                },
                ButtonConfig {
                    id: "next",
                    label: "Next",
                    hotkey: 'n',
                    color: ButtonColor::Green,
                },
            ],
            ConnectDialogStep::EndpointSelection => vec![
                ButtonConfig {
                    id: "cancel",
                    label: "Cancel",
                    hotkey: 'c',
                    color: ButtonColor::Red,
                },
                ButtonConfig {
                    id: "back",
                    label: "Back",
                    hotkey: 'b',
                    color: ButtonColor::Blue,
                },
                ButtonConfig {
                    id: "next",
                    label: "Next",
                    hotkey: 'n',
                    color: ButtonColor::Green,
                },
            ],
            ConnectDialogStep::SecurityConfiguration => vec![
                ButtonConfig {
                    id: "cancel",
                    label: "Cancel",
                    hotkey: 'c',
                    color: ButtonColor::Red,
                },
                ButtonConfig {
                    id: "back",
                    label: "Back",
                    hotkey: 'b',
                    color: ButtonColor::Blue,
                },
                ButtonConfig {
                    id: "next",
                    label: "Next",
                    hotkey: 'n',
                    color: ButtonColor::Green,
                },
            ],
            ConnectDialogStep::Authentication => vec![
                ButtonConfig {
                    id: "cancel",
                    label: "Cancel",
                    hotkey: 'c',
                    color: ButtonColor::Red,
                },
                ButtonConfig {
                    id: "back",
                    label: "Back",
                    hotkey: 'b',
                    color: ButtonColor::Blue,
                },
                ButtonConfig {
                    id: "connect",
                    label: "Connect",
                    hotkey: 'n',
                    color: ButtonColor::Green,
                },
            ],
        }
    }

    fn is_button_enabled(&self, button_id: &str) -> bool {
        match button_id {
            "next" | "connect" => !self.connect_in_progress,
            _ => true,
        }
    }

    pub async fn handle_button_action(&mut self, button_id: &str) -> Result<Option<ConnectionStatus>> {
        match button_id {
            "cancel" => Ok(Some(ConnectionStatus::Disconnected)),
            "next" => {
                match self.step {
                    ConnectDialogStep::ServerUrl |
                    ConnectDialogStep::EndpointSelection |
                    ConnectDialogStep::SecurityConfiguration => {
                        self.advance_to_next_step()?;
                        Ok(None)
                    }
                    _ => Ok(None),
                }
            }
            "back" => {
                self.handle_back_navigation();
                Ok(None)
            }
            "connect" => self.connect_with_settings().await,
            _ => Ok(None),
        }
    }

    fn handle_back_navigation(&mut self) {
        match self.step {
            ConnectDialogStep::EndpointSelection => {
                self.step = ConnectDialogStep::ServerUrl;
                self.input_mode = InputMode::Editing;
                self.setup_buttons_for_current_step();
            }
            ConnectDialogStep::SecurityConfiguration => {
                self.step = ConnectDialogStep::EndpointSelection;
                self.input_mode = InputMode::Normal;
                self.setup_buttons_for_current_step();
            }
            ConnectDialogStep::Authentication => {
                self.navigate_back_from_auth();
            }
            _ => {}
        }
    }
}
