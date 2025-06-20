use crate::client::ConnectionStatus;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Modifier},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Table, Row, Cell},
    Frame,
};

pub struct BrowseScreen {
    // Tree navigation state
    current_path: Vec<String>,
    tree_nodes: Vec<TreeNode>,
    selected_node_index: usize,
    expanded_nodes: std::collections::HashSet<String>,
    scroll_offset: usize,
    
    // Attributes panel state
    selected_attributes: Vec<NodeAttribute>,
    attribute_scroll_offset: usize,

    // Connection info
    server_url: String,
    connection_status: ConnectionStatus,
}

#[derive(Clone)]
pub struct TreeNode {
    pub name: String,
    pub node_id: String,
    pub node_type: NodeType,
    pub level: usize,
    pub has_children: bool,
    pub is_expanded: bool,
    pub parent_path: String,
}

#[derive(Clone)]
pub struct NodeAttribute {
    pub name: String,
    pub value: String,
    pub data_type: String,
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
            tree_nodes: Vec::new(),
            selected_node_index: 0,
            expanded_nodes: std::collections::HashSet::new(),
            scroll_offset: 0,
            selected_attributes: Vec::new(),
            attribute_scroll_offset: 0,
            server_url,
            connection_status: ConnectionStatus::Connected,
        };

        // Initialize with demo OPC UA tree structure
        browse_screen.load_demo_tree();
        browse_screen.update_selected_attributes();
        browse_screen
    }

    fn load_demo_tree(&mut self) {
        // Create a hierarchical OPC UA server structure
        self.tree_nodes = vec![
            TreeNode {
                name: "Objects".to_string(),
                node_id: "i=85".to_string(),
                node_type: NodeType::Object,
                level: 0,
                has_children: true,
                is_expanded: false,
                parent_path: "".to_string(),
            },
            TreeNode {
                name: "Types".to_string(),
                node_id: "i=86".to_string(),
                node_type: NodeType::Object,
                level: 0,
                has_children: true,
                is_expanded: false,
                parent_path: "".to_string(),
            },
            TreeNode {
                name: "Views".to_string(),
                node_id: "i=87".to_string(),
                node_type: NodeType::Object,
                level: 0,
                has_children: true,
                is_expanded: false,
                parent_path: "".to_string(),
            },
            TreeNode {
                name: "Server".to_string(),
                node_id: "i=2253".to_string(),
                node_type: NodeType::Object,
                level: 0,
                has_children: true,
                is_expanded: false,
                parent_path: "".to_string(),
            },
        ];
    }    fn expand_node(&mut self, index: usize) {
        if index < self.tree_nodes.len() && self.tree_nodes[index].has_children {
            let node_path = {
                let node = &self.tree_nodes[index];
                format!("{}/{}", node.parent_path, node.name)
            };
            
            if !self.expanded_nodes.contains(&node_path) {
                self.expanded_nodes.insert(node_path.clone());
                
                // Get node info before modifying the vector
                let (node_id, level, parent_path) = {
                    let node = &self.tree_nodes[index];
                    (node.node_id.clone(), node.level, node_path.clone())
                };
                
                self.tree_nodes[index].is_expanded = true;
                
                // Add child nodes (demo data)
                let child_nodes = self.get_demo_children(&node_id, level + 1, &parent_path);
                
                // Insert children after the current node
                for (i, child) in child_nodes.into_iter().enumerate() {
                    self.tree_nodes.insert(index + 1 + i, child);
                }
            }
        }
    }

    fn collapse_node(&mut self, index: usize) {
        if index < self.tree_nodes.len() {
            let (node_path, node_level) = {
                let node = &self.tree_nodes[index];
                (format!("{}/{}", node.parent_path, node.name), node.level)
            };
            
            if self.expanded_nodes.contains(&node_path) {
                self.expanded_nodes.remove(&node_path);
                self.tree_nodes[index].is_expanded = false;
                  // Remove all child nodes
                let mut i = index + 1;
                while i < self.tree_nodes.len() && self.tree_nodes[i].level > node_level {
                    self.tree_nodes.remove(i);
                }
            }
        }
    }

    fn get_demo_children(&self, parent_id: &str, level: usize, parent_path: &str) -> Vec<TreeNode> {
        match parent_id {
            "i=85" => vec![ // Objects
                TreeNode {
                    name: "Server".to_string(),
                    node_id: "i=2253".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "DeviceSet".to_string(),
                    node_id: "i=5001".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
            ],
            "i=86" => vec![ // Types
                TreeNode {
                    name: "ObjectTypes".to_string(),
                    node_id: "i=58".to_string(),
                    node_type: NodeType::ObjectType,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "VariableTypes".to_string(),
                    node_id: "i=62".to_string(),
                    node_type: NodeType::VariableType,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
            ],
            "i=2253" => vec![ // Server
                TreeNode {
                    name: "ServerCapabilities".to_string(),
                    node_id: "i=2268".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: false,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "ServerDiagnostics".to_string(),
                    node_id: "i=2274".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "ServerStatus".to_string(),
                    node_id: "i=2256".to_string(),
                    node_type: NodeType::Variable,
                    level,
                    has_children: false,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
            ],
            "i=5001" => vec![ // DeviceSet
                TreeNode {
                    name: "Device1".to_string(),
                    node_id: "ns=2;s=Device1".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Device2".to_string(),
                    node_id: "ns=2;s=Device2".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
            ],
            _ => vec![],
        }
    }

    fn update_selected_attributes(&mut self) {
        if self.selected_node_index < self.tree_nodes.len() {
            let node = &self.tree_nodes[self.selected_node_index];
            
            // Generate demo attributes based on node type
            self.selected_attributes = match node.node_type {
                NodeType::Variable => vec![
                    NodeAttribute {
                        name: "BrowseName".to_string(),
                        value: node.name.clone(),
                        data_type: "QualifiedName".to_string(),
                    },
                    NodeAttribute {
                        name: "DisplayName".to_string(),
                        value: node.name.clone(),
                        data_type: "LocalizedText".to_string(),
                    },
                    NodeAttribute {
                        name: "NodeId".to_string(),
                        value: node.node_id.clone(),
                        data_type: "NodeId".to_string(),
                    },
                    NodeAttribute {
                        name: "Value".to_string(),
                        value: "42.5".to_string(),
                        data_type: "Double".to_string(),
                    },
                    NodeAttribute {
                        name: "AccessLevel".to_string(),
                        value: "Read/Write".to_string(),
                        data_type: "Byte".to_string(),
                    },
                    NodeAttribute {
                        name: "UserAccessLevel".to_string(),
                        value: "Read/Write".to_string(),
                        data_type: "Byte".to_string(),
                    },
                ],
                _ => vec![
                    NodeAttribute {
                        name: "BrowseName".to_string(),
                        value: node.name.clone(),
                        data_type: "QualifiedName".to_string(),
                    },
                    NodeAttribute {
                        name: "DisplayName".to_string(),
                        value: node.name.clone(),
                        data_type: "LocalizedText".to_string(),
                    },
                    NodeAttribute {
                        name: "NodeId".to_string(),
                        value: node.node_id.clone(),
                        data_type: "NodeId".to_string(),
                    },
                    NodeAttribute {
                        name: "NodeClass".to_string(),
                        value: format!("{:?}", node.node_type),
                        data_type: "Int32".to_string(),
                    },
                ],
            };
        }
    }    pub async fn handle_input(
        &mut self,
        key: KeyCode,
        _modifiers: KeyModifiers,
    ) -> Result<Option<ConnectionStatus>> {
        match key {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Disconnect and return to connect screen
                Ok(Some(ConnectionStatus::Disconnected))
            }
            KeyCode::Up => {
                if self.selected_node_index > 0 {
                    self.selected_node_index -= 1;
                    self.update_scroll();
                    self.update_selected_attributes();
                }
                Ok(None)
            }
            KeyCode::Down => {
                if self.selected_node_index < self.tree_nodes.len().saturating_sub(1) {
                    self.selected_node_index += 1;
                    self.update_scroll();
                    self.update_selected_attributes();
                }
                Ok(None)
            }
            KeyCode::Right | KeyCode::Enter => {
                // Expand node if it has children
                if self.selected_node_index < self.tree_nodes.len() {
                    let node = &self.tree_nodes[self.selected_node_index];
                    if node.has_children && !node.is_expanded {
                        self.expand_node(self.selected_node_index);
                        self.update_selected_attributes();
                    }
                }
                Ok(None)
            }
            KeyCode::Left => {
                // Collapse node if it's expanded
                if self.selected_node_index < self.tree_nodes.len() {
                    let node = &self.tree_nodes[self.selected_node_index];
                    if node.is_expanded {
                        self.collapse_node(self.selected_node_index);
                    } else if node.level > 0 {
                        // Move to parent node
                        self.move_to_parent();
                    }
                }
                Ok(None)
            }
            KeyCode::PageUp => {
                let page_size = 10;
                self.selected_node_index = self.selected_node_index.saturating_sub(page_size);
                self.update_scroll();
                self.update_selected_attributes();
                Ok(None)
            }
            KeyCode::PageDown => {
                let page_size = 10;
                self.selected_node_index = (self.selected_node_index + page_size)
                    .min(self.tree_nodes.len().saturating_sub(1));
                self.update_scroll();
                self.update_selected_attributes();
                Ok(None)
            }
            KeyCode::Home => {
                self.selected_node_index = 0;
                self.scroll_offset = 0;
                self.update_selected_attributes();
                Ok(None)
            }
            KeyCode::End => {
                self.selected_node_index = self.tree_nodes.len().saturating_sub(1);
                self.update_scroll();
                self.update_selected_attributes();
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn move_to_parent(&mut self) {
        if self.selected_node_index < self.tree_nodes.len() {
            let current_level = self.tree_nodes[self.selected_node_index].level;
            
            // Find parent node (move backwards to find a node with level - 1)
            for i in (0..self.selected_node_index).rev() {
                if self.tree_nodes[i].level < current_level {
                    self.selected_node_index = i;
                    self.update_scroll();
                    self.update_selected_attributes();
                    break;
                }
            }        }
    }

    fn update_scroll(&mut self) {
        // Keep selected item visible in the tree view
        let visible_height = 20; // This will be calculated from the actual render area
        
        if self.selected_node_index < self.scroll_offset {
            self.scroll_offset = self.selected_node_index;
        } else if self.selected_node_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_node_index.saturating_sub(visible_height - 1);
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content area
            ])
            .split(area);

        // Header
        self.render_header(f, main_chunks[0]);

        // Main content area: Tree view on left, attributes on right
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Tree view
                Constraint::Percentage(30), // Attributes panel
            ])
            .split(main_chunks[1]);

        // Tree view
        self.render_tree_view(f, content_chunks[0]);

        // Attributes panel
        self.render_attributes_panel(f, content_chunks[1]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let header_text = vec![
            Span::styled("OPC UA Browser", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw(" - "),
            Span::styled(&self.server_url, Style::default().fg(Color::Cyan)),
            Span::raw("\nUse ←/→ to expand/collapse, ↑/↓ to navigate, q/Esc to exit"),
        ];

        let header = Paragraph::new(vec![
            Line::from(header_text[0..2].to_vec()),
            Line::from(header_text[2..].to_vec()),
        ])
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White));

        f.render_widget(header, area);
    }

    fn render_tree_view(&mut self, f: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        self.update_scroll_with_height(visible_height);
        
        let start_idx = self.scroll_offset;
        let end_idx = (start_idx + visible_height).min(self.tree_nodes.len());
        let visible_nodes = &self.tree_nodes[start_idx..end_idx];

        let items: Vec<ListItem> = visible_nodes
            .iter()
            .enumerate()
            .map(|(i, node)| {
                let actual_index = start_idx + i;
                let is_selected = actual_index == self.selected_node_index;
                
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

                // Create indentation based on level
                let indent = "  ".repeat(node.level);
                
                let expand_icon = if node.has_children {
                    if node.is_expanded { "▼ " } else { "▶ " }
                } else {
                    "  "
                };

                let name = format!("{}{}{} {}", indent, expand_icon, icon, node.name);
                
                let style = if is_selected {
                    Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(name).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("OPC UA Node Tree")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            );

        f.render_widget(list, area);
    }

    fn render_attributes_panel(&mut self, f: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(4) as usize; // Subtract borders and header
        
        let start_idx = self.attribute_scroll_offset;
        let end_idx = (start_idx + visible_height).min(self.selected_attributes.len());
        let visible_attributes = if !self.selected_attributes.is_empty() {
            &self.selected_attributes[start_idx..end_idx]
        } else {
            &[]
        };

        let rows: Vec<Row> = visible_attributes
            .iter()
            .map(|attr| {
                Row::new(vec![
                    Cell::from(attr.name.as_str()),
                    Cell::from(attr.value.as_str()),
                    Cell::from(attr.data_type.as_str()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(35),
                Constraint::Percentage(40),
                Constraint::Percentage(25),
            ]
        )
            .header(
                Row::new(vec!["Attribute", "Value", "Type"])
                    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            )
            .block(
                Block::default()
                    .title("Node Attributes")
                    .borders(Borders::ALL)                    .border_style(Style::default().fg(Color::Gray))
            )
            .column_spacing(1);

        f.render_widget(table, area);
    }    fn update_scroll_with_height(&mut self, visible_height: usize) {
        if self.selected_node_index < self.scroll_offset {
            self.scroll_offset = self.selected_node_index;
        } else if self.selected_node_index >= self.scroll_offset + visible_height {
            self.scroll_offset = self.selected_node_index.saturating_sub(visible_height - 1);
        }
    }
}
