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
                if node.parent_path.is_empty() {
                    node.name.clone()
                } else {
                    format!("{}/{}", node.parent_path, node.name)
                }
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
                let mut child_nodes = self.get_demo_children(&node_id, level + 1, &parent_path);
                
                // Restore expanded state for child nodes that were previously expanded
                for child in &mut child_nodes {
                    let child_path = if child.parent_path.is_empty() {
                        child.name.clone()
                    } else {
                        format!("{}/{}", child.parent_path, child.name)
                    };
                    
                    if self.expanded_nodes.contains(&child_path) {
                        child.is_expanded = true;
                    }
                }
                
                // Insert children after the current node
                for (i, child) in child_nodes.into_iter().enumerate() {
                    self.tree_nodes.insert(index + 1 + i, child);
                }
                
                // Recursively expand any child nodes that should be expanded
                self.restore_child_expansions(index + 1, level + 1);
            }
        }
    }fn collapse_node(&mut self, index: usize) {
        if index < self.tree_nodes.len() {
            let (node_path, node_level) = {
                let node = &self.tree_nodes[index];
                let path = if node.parent_path.is_empty() {
                    node.name.clone()
                } else {
                    format!("{}/{}", node.parent_path, node.name)
                };
                (path, node.level)
            };
            
            if self.expanded_nodes.contains(&node_path) {
                // Remove the current node from expanded set
                self.expanded_nodes.remove(&node_path);
                self.tree_nodes[index].is_expanded = false;
                
                // Store the current selected node info before removing children
                let was_selected_child_removed = self.selected_node_index > index;
                
                // NOTE: We intentionally DO NOT remove child paths from expanded_nodes
                // This preserves the expanded state of child nodes for when the parent is re-expanded
                
                // Count and remove all child nodes from the visual tree
                let mut removed_count = 0;
                let mut i = index + 1;
                while i < self.tree_nodes.len() && self.tree_nodes[i].level > node_level {
                    self.tree_nodes.remove(i);
                    removed_count += 1;
                    // Don't increment i since we removed an element
                }
                
                // Adjust selected index more carefully
                if was_selected_child_removed && self.selected_node_index > index {
                    if self.selected_node_index <= index + removed_count {
                        // Selected node was removed, stay on the collapsed parent
                        self.selected_node_index = index;
                    } else {
                        // Selected node was after the removed children, adjust index
                        self.selected_node_index -= removed_count;
                    }
                }
                
                // Ensure selected index is still valid
                if self.selected_node_index >= self.tree_nodes.len() {
                    self.selected_node_index = self.tree_nodes.len().saturating_sub(1);
                }
            }
        }
    }fn get_demo_children(&self, parent_id: &str, level: usize, parent_path: &str) -> Vec<TreeNode> {
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
                TreeNode {
                    name: "Simulation".to_string(),
                    node_id: "ns=2;s=Simulation".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "DataAccess".to_string(),
                    node_id: "ns=2;s=DataAccess".to_string(),
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
                TreeNode {
                    name: "DataTypes".to_string(),
                    node_id: "i=22".to_string(),
                    node_type: NodeType::DataType,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "ReferenceTypes".to_string(),
                    node_id: "i=31".to_string(),
                    node_type: NodeType::ReferenceType,
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
                    has_children: true,
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
                TreeNode {
                    name: "ServiceLevel".to_string(),
                    node_id: "i=2267".to_string(),
                    node_type: NodeType::Variable,
                    level,
                    has_children: false,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Auditing". to_string(),
                    node_id: "i=2994".to_string(),
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
                TreeNode {
                    name: "Device3".to_string(),
                    node_id: "ns=2;s=Device3".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Device4".to_string(),
                    node_id: "ns=2;s=Device4".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Device5".to_string(),
                    node_id: "ns=2;s=Device5".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
            ],
            "ns=2;s=Simulation" => vec![ // Simulation
                TreeNode {
                    name: "RandomValues".to_string(),
                    node_id: "ns=2;s=Simulation.RandomValues".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Counters".to_string(),
                    node_id: "ns=2;s=Simulation.Counters".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "SlowChangingValues".to_string(),
                    node_id: "ns=2;s=Simulation.SlowChangingValues".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
            ],
            "ns=2;s=DataAccess" => vec![ // DataAccess
                TreeNode {
                    name: "Dynamic".to_string(),
                    node_id: "ns=2;s=DataAccess.Dynamic".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Static".to_string(),
                    node_id: "ns=2;s=DataAccess.Static".to_string(),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
            ],
            // Add more detailed children for devices
            "ns=2;s=Device1" | "ns=2;s=Device2" | "ns=2;s=Device3" | "ns=2;s=Device4" | "ns=2;s=Device5" => vec![
                TreeNode {
                    name: "Temperature".to_string(),
                    node_id: format!("{}.Temperature", parent_id),
                    node_type: NodeType::Variable,
                    level,
                    has_children: false,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Pressure".to_string(),
                    node_id: format!("{}.Pressure", parent_id),
                    node_type: NodeType::Variable,
                    level,
                    has_children: false,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Status".to_string(),
                    node_id: format!("{}.Status", parent_id),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
                TreeNode {
                    name: "Configuration".to_string(),
                    node_id: format!("{}.Configuration", parent_id),
                    node_type: NodeType::Object,
                    level,
                    has_children: true,
                    is_expanded: false,
                    parent_path: parent_path.to_string(),
                },
            ],
            // Add many variables for testing scrolling
            "ns=2;s=Simulation.RandomValues" => {
                let mut vars = Vec::new();
                for i in 1..=20 {
                    vars.push(TreeNode {
                        name: format!("Random{}", i),
                        node_id: format!("ns=2;s=Simulation.RandomValues.Random{}", i),
                        node_type: NodeType::Variable,
                        level,
                        has_children: false,
                        is_expanded: false,
                        parent_path: parent_path.to_string(),
                    });
                }
                vars
            },
            "ns=2;s=Simulation.Counters" => {
                let mut vars = Vec::new();
                for i in 1..=15 {
                    vars.push(TreeNode {
                        name: format!("Counter{}", i),
                        node_id: format!("ns=2;s=Simulation.Counters.Counter{}", i),
                        node_type: NodeType::Variable,
                        level,
                        has_children: false,
                        is_expanded: false,
                        parent_path: parent_path.to_string(),
                    });
                }
                vars
            },
            "ns=2;s=DataAccess.Dynamic" => {
                let mut vars = Vec::new();
                for i in 1..=25 {
                    vars.push(TreeNode {
                        name: format!("DynamicVar{}", i),
                        node_id: format!("ns=2;s=DataAccess.Dynamic.DynamicVar{}", i),
                        node_type: NodeType::Variable,
                        level,
                        has_children: false,
                        is_expanded: false,
                        parent_path: parent_path.to_string(),
                    });
                }
                vars
            },
            _ => vec![],
        }
    }

    fn update_selected_attributes(&mut self) {
        if self.selected_node_index < self.tree_nodes.len() {
            let node = &self.tree_nodes[self.selected_node_index];
            
            // Generate demo attributes based on node type
            self.selected_attributes = match node.node_type {                NodeType::Variable => vec![
                    NodeAttribute {
                        name: "BrowseName".to_string(),
                        value: node.name.clone(),
                    },
                    NodeAttribute {
                        name: "DisplayName".to_string(),
                        value: node.name.clone(),
                    },
                    NodeAttribute {
                        name: "NodeId".to_string(),
                        value: node.node_id.clone(),
                    },
                    NodeAttribute {
                        name: "Value".to_string(),
                        value: "42.5".to_string(),
                    },
                    NodeAttribute {
                        name: "AccessLevel".to_string(),
                        value: "Read/Write".to_string(),
                    },
                    NodeAttribute {
                        name: "UserAccessLevel".to_string(),
                        value: "Read/Write".to_string(),
                    },
                ],
                _ => vec![
                    NodeAttribute {
                        name: "BrowseName".to_string(),
                        value: node.name.clone(),
                    },
                    NodeAttribute {
                        name: "DisplayName".to_string(),
                        value: node.name.clone(),
                    },
                    NodeAttribute {
                        name: "NodeId".to_string(),
                        value: node.node_id.clone(),
                    },
                    NodeAttribute {
                        name: "NodeClass".to_string(),
                        value: format!("{:?}", node.node_type),
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
            }            KeyCode::Left => {
                // Left key behavior:
                // 1. If current node is expanded, collapse it
                // 2. If current node is not expanded, move to parent
                if self.selected_node_index < self.tree_nodes.len() {
                    let node = &self.tree_nodes[self.selected_node_index];
                    if node.is_expanded {
                        // Collapse the current node
                        self.collapse_node(self.selected_node_index);
                        self.update_selected_attributes();
                    } else if node.level > 0 {
                        // Move to immediate parent node
                        self.move_to_parent();
                        self.update_selected_attributes();
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
    }    fn move_to_parent(&mut self) {
        if self.selected_node_index < self.tree_nodes.len() {
            let current_level = self.tree_nodes[self.selected_node_index].level;
            
            if current_level > 0 {
                // Find the immediate parent node (level = current_level - 1)
                for i in (0..self.selected_node_index).rev() {
                    if self.tree_nodes[i].level == current_level - 1 {
                        self.selected_node_index = i;
                        self.update_scroll();
                        break;
                    }
                }
            }
        }
    }fn update_scroll(&mut self) {
        // This will be updated with actual visible height in render
        let visible_height = 20;
        self.update_scroll_with_height(visible_height);
    }    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(0),    // Main content area
                Constraint::Length(1), // Status bar
            ])
            .split(area);

        // Main content area: Tree view on left, attributes on right
        let content_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Tree view
                Constraint::Percentage(30), // Attributes panel
            ])
            .split(main_chunks[0]);

        // Tree view
        self.render_tree_view(f, content_chunks[0]);

        // Attributes panel
        self.render_attributes_panel(f, content_chunks[1]);

        // Status bar
        self.render_status_bar(f, main_chunks[1]);
    }    fn render_status_bar(&self, f: &mut Frame, area: Rect) {
        let selected_node_info = if self.selected_node_index < self.tree_nodes.len() {
            let node = &self.tree_nodes[self.selected_node_index];
            format!("Selected: {} | NodeId: {}", node.name, node.node_id)
        } else {
            "No node selected".to_string()
        };

        let status_text = vec![
            Span::styled("OPC UA Server: ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(&self.server_url, Style::default().fg(Color::Cyan)),
            Span::raw(" | "),
            Span::styled("Connected", Style::default().fg(Color::Green)),
            Span::raw(" | "),
            Span::styled(&selected_node_info, Style::default().fg(Color::Yellow)),
            Span::raw(" | Use ←/→ expand/collapse, ↑/↓ navigate, q/Esc exit"),
        ];

        let status = Paragraph::new(Line::from(status_text))
            .style(Style::default().fg(Color::White).bg(Color::DarkGray));

        f.render_widget(status, area);
    }fn render_tree_view(&mut self, f: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(2) as usize; // Subtract borders
        self.update_scroll_with_height(visible_height);
        
        let start_idx = self.scroll_offset;
        let end_idx = (start_idx + visible_height).min(self.tree_nodes.len());
        let visible_nodes = if start_idx < self.tree_nodes.len() {
            &self.tree_nodes[start_idx..end_idx]
        } else {
            &[]
        };

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
                
                // Use consistent width for expand icons
                let expand_icon = if node.has_children {
                    if node.is_expanded { "▼" } else { "▶" }
                } else {
                    " "
                };

                // Format: [indent][expand_icon] [type_icon] [name]
                let name = format!("{}{} {} {}", indent, expand_icon, icon, node.name);
                
                let style = if is_selected {
                    Style::default().bg(Color::Blue).fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(name)).style(style)
            })
            .collect();

        // Add scroll indicator
        let title = if self.tree_nodes.len() > visible_height {
            format!("OPC UA Node Tree ({}/{} shown)", visible_nodes.len(), self.tree_nodes.len())
        } else {
            "OPC UA Node Tree".to_string()
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Gray))
            );

        f.render_widget(list, area);
        
        // Render scrollbar if needed
        if self.tree_nodes.len() > visible_height {
            self.render_tree_scrollbar(f, area, visible_height);
        }
    }

    fn render_tree_scrollbar(&self, f: &mut Frame, area: Rect, visible_height: usize) {
        let scrollbar_area = Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };

        let scrollbar_height = scrollbar_area.height as usize;
        if scrollbar_height < 3 {
            return; // Too small to render meaningful scrollbar
        }

        let total_items = self.tree_nodes.len();
        let track_height = scrollbar_height.saturating_sub(2); // Remove arrows
        
        // Calculate thumb size and position
        let thumb_size = ((visible_height as f64 / total_items as f64) * track_height as f64).round() as usize;
        let thumb_size = thumb_size.max(1).min(track_height);
        
        let max_scroll = total_items.saturating_sub(visible_height);
        let scroll_ratio = if max_scroll > 0 {
            self.scroll_offset as f64 / max_scroll as f64
        } else {
            0.0
        };
        
        let max_thumb_pos = track_height.saturating_sub(thumb_size);
        let thumb_pos = (scroll_ratio * max_thumb_pos as f64).round() as usize;

        // Render scrollbar components
        // Up arrow
        f.render_widget(
            Paragraph::new("↑").style(Style::default().fg(Color::White)),
            Rect { x: scrollbar_area.x, y: scrollbar_area.y, width: 1, height: 1 }
        );
        
        // Track
        for i in 0..track_height {
            let y = scrollbar_area.y + 1 + i as u16;
            let symbol = if i >= thumb_pos && i < thumb_pos + thumb_size {
                "█" // Thumb
            } else {
                "│" // Track
            };
            
            f.render_widget(
                Paragraph::new(symbol).style(Style::default().fg(Color::Gray)),
                Rect { x: scrollbar_area.x, y, width: 1, height: 1 }
            );
        }
        
        // Down arrow
        f.render_widget(
            Paragraph::new("↓").style(Style::default().fg(Color::White)),
            Rect { 
                x: scrollbar_area.x, 
                y: scrollbar_area.y + scrollbar_area.height - 1, 
                width: 1, 
                height: 1 
            }
        );
    }

    fn render_attributes_panel(&mut self, f: &mut Frame, area: Rect) {
        let visible_height = area.height.saturating_sub(4) as usize; // Subtract borders and header
        
        let start_idx = self.attribute_scroll_offset;
        let end_idx = (start_idx + visible_height).min(self.selected_attributes.len());
        let visible_attributes = if !self.selected_attributes.is_empty() {
            &self.selected_attributes[start_idx..end_idx]
        } else {
            &[]
        };        let rows: Vec<Row> = visible_attributes
            .iter()
            .map(|attr| {
                Row::new(vec![
                    Cell::from(attr.name.as_str()),
                    Cell::from(attr.value.as_str()),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            &[
                Constraint::Percentage(40),
                Constraint::Percentage(60),
            ]
        )
            .header(
                Row::new(vec!["Attribute", "Value"])
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

    fn restore_child_expansions(&mut self, start_index: usize, current_level: usize) {
        let mut i = start_index;
        while i < self.tree_nodes.len() && self.tree_nodes[i].level >= current_level {
            if self.tree_nodes[i].level == current_level {
                let node = &self.tree_nodes[i];
                if node.has_children && node.is_expanded {
                    // This child was previously expanded, so expand it again
                    let (node_id, level, parent_path) = {
                        let node = &self.tree_nodes[i];
                        let path = if node.parent_path.is_empty() {
                            node.name.clone()
                        } else {
                            format!("{}/{}", node.parent_path, node.name)
                        };
                        (node.node_id.clone(), node.level, path)
                    };
                    
                    // Add child nodes for this expanded node
                    let mut child_nodes = self.get_demo_children(&node_id, level + 1, &parent_path);
                    
                    // Restore expanded state for grandchildren
                    for child in &mut child_nodes {
                        let child_path = if child.parent_path.is_empty() {
                            child.name.clone()
                        } else {
                            format!("{}/{}", child.parent_path, child.name)
                        };
                        
                        if self.expanded_nodes.contains(&child_path) {
                            child.is_expanded = true;
                        }
                    }
                    
                    // Insert children after the current node
                    for (j, child) in child_nodes.into_iter().enumerate() {
                        self.tree_nodes.insert(i + 1 + j, child);
                    }
                    
                    // Skip over the newly inserted children and continue
                    let children_count = self.get_demo_children(&node_id, level + 1, &parent_path).len();
                    i += children_count + 1;
                    
                    // Recursively restore expansions for the newly added children
                    if children_count > 0 {
                        self.restore_child_expansions(i - children_count, level + 1);
                    }
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }
    }
}
