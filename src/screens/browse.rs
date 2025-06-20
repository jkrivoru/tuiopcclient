use crate::client::ConnectionStatus;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub struct BrowseScreen {
    // Navigation state
    current_path: Vec<String>,
    nodes: Vec<NodeItem>,
    selected_node: Option<usize>,
    list_state: ListState,

    // Connection info
    server_url: String,
    connection_status: ConnectionStatus,
}

#[derive(Clone)]
pub struct NodeItem {
    pub name: String,
    pub node_id: String,
    pub node_type: NodeType,
    pub has_children: bool,
}

#[derive(Clone, Debug)]
pub enum NodeType {
    Object,
    Variable,
    Method,
    View,
    ObjectType,
    VariableType,
    DataType,
    ReferenceType,
}

impl BrowseScreen {
    pub fn new(server_url: String) -> Self {
        let mut browse_screen = Self {
            current_path: vec!["Root".to_string()],
            nodes: Vec::new(),
            selected_node: None,
            list_state: ListState::default(),
            server_url,
            connection_status: ConnectionStatus::Connected,
        };

        // Initialize with some demo nodes
        browse_screen.load_demo_nodes();
        browse_screen
    }

    fn load_demo_nodes(&mut self) {
        // Demo OPC UA server structure
        self.nodes = vec![
            NodeItem {
                name: "Objects".to_string(),
                node_id: "i=85".to_string(),
                node_type: NodeType::Object,
                has_children: true,
            },
            NodeItem {
                name: "Types".to_string(),
                node_id: "i=86".to_string(),
                node_type: NodeType::Object,
                has_children: true,
            },
            NodeItem {
                name: "Views".to_string(),
                node_id: "i=87".to_string(),
                node_type: NodeType::Object,
                has_children: true,
            },
            NodeItem {
                name: "Server".to_string(),
                node_id: "i=2253".to_string(),
                node_type: NodeType::Object,
                has_children: true,
            },
        ];

        // Select first item
        if !self.nodes.is_empty() {
            self.selected_node = Some(0);
            self.list_state.select(Some(0));
        }
    }

    pub async fn handle_input(
        &mut self,
        key: KeyCode,
        modifiers: KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        match key {
            KeyCode::Up => {
                if let Some(selected) = self.selected_node {
                    if selected > 0 {
                        self.selected_node = Some(selected - 1);
                        self.list_state.select(Some(selected - 1));
                    }
                }
                Ok(None)
            }
            KeyCode::Down => {
                if let Some(selected) = self.selected_node {
                    if selected < self.nodes.len() - 1 {
                        self.selected_node = Some(selected + 1);
                        self.list_state.select(Some(selected + 1));
                    }
                } else if !self.nodes.is_empty() {
                    self.selected_node = Some(0);
                    self.list_state.select(Some(0));
                }
                Ok(None)
            }
            KeyCode::Enter => {
                if let Some(selected) = self.selected_node {
                    if let Some(node) = self.nodes.get(selected) {
                        if node.has_children {
                            // Navigate into the node
                            self.navigate_into_node(node.clone());
                        }
                    }
                }
                Ok(None)
            }
            KeyCode::Backspace | KeyCode::Left => {
                // Navigate back
                self.navigate_back();
                Ok(None)
            }
            KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                // Quit the application
                Ok(Some(ConnectionStatus::Disconnected))
            }
            KeyCode::Char('r') if modifiers.contains(KeyModifiers::CONTROL) => {
                // Refresh current view
                self.refresh_current_view();
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn navigate_into_node(&mut self, node: NodeItem) {
        self.current_path.push(node.name.clone());

        // Load child nodes (demo implementation)
        match node.node_id.as_str() {
            "i=85" => {
                // Objects
                self.nodes = vec![
                    NodeItem {
                        name: "DeviceSet".to_string(),
                        node_id: "i=5001".to_string(),
                        node_type: NodeType::Object,
                        has_children: true,
                    },
                    NodeItem {
                        name: "Demo".to_string(),
                        node_id: "ns=2;s=Demo".to_string(),
                        node_type: NodeType::Object,
                        has_children: true,
                    },
                ];
            }
            "i=86" => {
                // Types
                self.nodes = vec![
                    NodeItem {
                        name: "ObjectTypes".to_string(),
                        node_id: "i=88".to_string(),
                        node_type: NodeType::ObjectType,
                        has_children: true,
                    },
                    NodeItem {
                        name: "VariableTypes".to_string(),
                        node_id: "i=89".to_string(),
                        node_type: NodeType::VariableType,
                        has_children: true,
                    },
                ];
            }
            _ => {
                // Default child nodes
                self.nodes = vec![
                    NodeItem {
                        name: "Variable1".to_string(),
                        node_id: format!("{}.Variable1", node.node_id),
                        node_type: NodeType::Variable,
                        has_children: false,
                    },
                    NodeItem {
                        name: "Variable2".to_string(),
                        node_id: format!("{}.Variable2", node.node_id),
                        node_type: NodeType::Variable,
                        has_children: false,
                    },
                ];
            }
        }

        // Reset selection
        self.selected_node = if !self.nodes.is_empty() {
            Some(0)
        } else {
            None
        };
        self.list_state.select(self.selected_node);
    }

    fn navigate_back(&mut self) {
        if self.current_path.len() > 1 {
            self.current_path.pop();

            // Reload parent nodes (simplified demo)
            if self.current_path.len() == 1 {
                self.load_demo_nodes();
            } else {
                // For demo, just show some generic nodes
                self.nodes = vec![
                    NodeItem {
                        name: "ChildNode1".to_string(),
                        node_id: "demo.child1".to_string(),
                        node_type: NodeType::Object,
                        has_children: true,
                    },
                    NodeItem {
                        name: "ChildNode2".to_string(),
                        node_id: "demo.child2".to_string(),
                        node_type: NodeType::Variable,
                        has_children: false,
                    },
                ];
            }

            // Reset selection
            self.selected_node = if !self.nodes.is_empty() {
                Some(0)
            } else {
                None
            };
            self.list_state.select(self.selected_node);
        }
    }

    fn refresh_current_view(&mut self) {
        // For demo, just reload the current view
        log::info!("Refreshing current view: {}", self.current_path.join(" > "));
        // In a real implementation, this would re-query the OPC UA server
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Node list
                Constraint::Length(3), // Details
            ])
            .split(area);

        // Header
        self.render_header(f, chunks[0]);

        // Node list
        self.render_node_list(f, chunks[1]);

        // Details
        self.render_details(f, chunks[2]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let header_text = vec![
            Span::styled("OPC UA Browser", Style::default().fg(Color::Green)),
            Span::raw(" - "),
            Span::styled(&self.server_url, Style::default().fg(Color::Cyan)),
            Span::raw("\nPath: "),
            Span::styled(
                self.current_path.join(" > "),
                Style::default().fg(Color::Yellow),
            ),
        ];

        let header = Paragraph::new(Line::from(header_text))
            .block(Block::default().borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        f.render_widget(header, area);
    }

    fn render_node_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .nodes
            .iter()
            .map(|node| {
                let icon = match node.node_type {
                    NodeType::Object => "📁",
                    NodeType::Variable => "📊",
                    NodeType::Method => "⚙️",
                    NodeType::View => "👁️",
                    NodeType::ObjectType => "🏷️",
                    NodeType::VariableType => "🔧",
                    NodeType::DataType => "📝",
                    NodeType::ReferenceType => "🔗",
                };

                let name = if node.has_children {
                    format!("{} {} >", icon, node.name)
                } else {
                    format!("{} {}", icon, node.name)
                };

                ListItem::new(name)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().title("Nodes").borders(Borders::ALL))
            .style(Style::default().fg(Color::White))
            .highlight_style(Style::default().bg(Color::Blue).fg(Color::White))
            .highlight_symbol("▶ ");

        f.render_stateful_widget(list, area, &mut self.list_state);
    }

    fn render_details(&self, f: &mut Frame, area: Rect) {
        let details_text = if let Some(selected) = self.selected_node {
            if let Some(node) = self.nodes.get(selected) {
                vec![
                    Span::raw("Node ID: "),
                    Span::styled(&node.node_id, Style::default().fg(Color::Green)),
                    Span::raw(" | Type: "),
                    Span::styled(
                        format!("{:?}", node.node_type),
                        Style::default().fg(Color::Yellow),
                    ),
                ]
            } else {
                vec![Span::raw("No node selected")]
            }
        } else {
            vec![Span::raw("No nodes available")]
        };

        let details = Paragraph::new(Line::from(details_text))
            .block(Block::default().title("Details").borders(Borders::ALL))
            .style(Style::default().fg(Color::White));

        f.render_widget(details, area);
    }

    pub fn render_help_line(&self, f: &mut Frame, area: Rect) {
        let help_text =
            "↑↓ - Navigate | Enter - Expand | Backspace - Back | Ctrl+R - Refresh | Ctrl+Q - Quit";
        let help = Paragraph::new(help_text).style(Style::default().fg(Color::Gray));
        f.render_widget(help, area);
    }
}
