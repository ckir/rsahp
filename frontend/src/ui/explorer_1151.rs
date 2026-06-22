to include a line number before every line, in the format: <line_number>: <original_line>. Please note that any changes targeting the original code should remove the line number, colon, and leading space.
use eframe::egui;
use super::document_window::DocumentState;
use egui_ltreeview::{TreeView, TreeViewState, NodeBuilder, Action, DirPosition};

pub enum Node {
    Directory(Directory),
    File(File),
}

pub struct Directory {
    pub id: usize,
    pub name: String,
    pub children: Vec<Node>,
}

pub struct File {
    pub id: usize,
    pub name: String,
    pub document_id: Option<usize>,
}

impl Node {
    pub fn id(&self) -> usize {
        match self {
            Node::Directory(d) => d.id,
            Node::File(f) => f.id,
        }
    }
    pub fn name(&self) -> &str {
        match self {
            Node::Directory(d) => &d.name,
            Node::File(f) => &f.name,
        }
    }
    pub fn remove(&mut self, id: usize) -> Option<Node> {
        match self {
            Node::Directory(dir) => {
                if let Some(index) = dir.children.iter().position(|n| n.id() == id) {
                    Some(dir.children.remove(index))
                } else {
                    for node in dir.children.iter_mut() {
                        if let Some(r) = node.remove(id) {
                            return Some(r);
                        }
                    }
                    None
                }
            }
            Node::File(_) => None,
        }
    }
    pub fn insert(&mut self, parent_id: usize, position: DirPosition<usize>, value: Node) -
<truncated 13617 bytes>
                        let doc_id = state.next_id;
                        let leaf = Node::File(File {
                            id: state.next_id,
                            name: name.clone(),
                            document_id: Some(doc_id),
                        });
                        let id = state.next_id;
                        state.next_id += 1;
                        let _ = state.tree.insert(parent_id, position, leaf);
                        state.tree_view_state.set_selected(vec![id]);
                        open_documents.push(DocumentState::new(doc_id as i32, &name));
                        state.modal_state = None;
                    } else {
                        submitted = false; // keep open
                    }
                }
                ModalAction::AddDir(parent_id, position) => {
                    if !modal.input_name.trim().is_empty() {
                        let name = modal.input_name.trim().to_string();
                        let dir = Node::Directory(Directory {
                            id: state.next_id,
                            name,
                            children: vec![],
                        });
                        let id = state.next_id;
                        state.next_id += 1;
                        let _ = state.tree.insert(parent_id, position, dir);
                        state.tree_view_state.set_selected(vec![id]);
                        state.modal_state = None;
                    } else {
                        submitted = false; // keep open
                    }
                }
            }
        }
        
        if (!is_open || close_requested) && !submitted {
            state.modal_state = None;
        }
    }
}
389: