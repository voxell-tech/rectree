//! # Mutation Concepts
//!
//! Whenever a [`RectNode`] is being mutated, it's

#![no_std]

extern crate alloc;

use core::ops::Deref;

use alloc::collections::btree_set::BTreeSet;
use alloc::vec;
use alloc::vec::Vec;
use hashbrown::HashSet;
use kurbo::{Rect, Vec2};

use crate::mut_detect::MutDetect;
use crate::sparse_map::{Key, SparseMap};

pub use kurbo;

pub mod mut_detect;
pub mod sparse_map;

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
pub struct NodeId(pub Key);

impl Deref for NodeId {
    type Target = Key;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Default, Debug)]
pub struct Rectree {
    root_ids: HashSet<NodeId>,
    nodes: SparseMap<RectNode>,

    mutated_rects: BTreeSet<MutatedNode>,
    mutated_translations: BTreeSet<MutatedNode>,

    /// A heap allocated translation stack for tree traversal.
    translation_stack: NodeStack<Vec2>,
}

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord,
)]
struct MutatedNode {
    depth: u32,
    id: NodeId,
}

impl MutatedNode {
    fn new(depth: u32, id: NodeId) -> Self {
        Self { depth, id }
    }
}

impl Rectree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Inserts a node into the tree while keeping track of the
    /// parent-child relationship.
    pub fn insert_node(&mut self, mut node: RectNode) -> NodeId {
        let key = self.nodes.insert_with_key(|nodes, key| {
            let id = NodeId(key);
            // TODO: Log error, or panic if parent is some but does
            // not exists.
            if let Some(parent) = node.parent
                && let Some(parent_node) = nodes.get_mut(&parent)
            {
                parent_node.children.insert(id);
                node.depth = parent_node.depth + 1;
            } else {
                // No parent, meaning that it's a root id.
                self.root_ids.insert(id);
            }

            let mutated_node = MutatedNode::new(node.depth, id);
            self.mutated_rects.insert(mutated_node);
            self.mutated_translations.insert(mutated_node);

            node
        });

        NodeId(key)
    }

    pub fn root_ids(&self) -> &HashSet<NodeId> {
        &self.root_ids
    }

    pub fn get_node(&self, id: &NodeId) -> Option<&RectNode> {
        self.nodes.get(id)
    }

    pub fn with_node_mut<F, R>(
        &mut self,
        id: &NodeId,
        f: F,
    ) -> Option<R>
    where
        F: FnOnce(&mut RectNode) -> R,
    {
        self.nodes.get_mut(id).map(|node| {
            let result = f(node);

            // Record changes.
            if node.size.mutated() {
                self.mutated_rects
                    .insert(MutatedNode::new(node.depth, *id));
            }
            if node.local_translation.mutated() {
                self.mutated_translations
                    .insert(MutatedNode::new(node.depth, *id));
            }

            result
        })
    }

    pub fn update_translations(&mut self) {
        let mutated_nodes =
            core::mem::take(&mut self.mutated_translations);

        for MutatedNode { id, .. } in mutated_nodes.into_iter() {
            let Some(node) = self.nodes.get(&id) else {
                // TODO: Log error, or panic?
                continue;
            };

            // Translation could have already been resolved by a
            // previous iteration.
            if !node.local_translation.mutated() {
                continue;
            }

            self.propagate_translation(id);
        }
    }

    /// Propagrate the world translation from a given [`NodeId`].
    fn propagate_translation(&mut self, id: NodeId) {
        self.translation_stack.init(id, Vec2::ZERO);

        while let Some(NodeStackEl { id, buffer_index }) =
            self.translation_stack.elements.pop()
        {
            let Some(node) = self.nodes.get_mut(&id) else {
                // TODO: Log error, or panic?
                continue;
            };

            node.world_translation = *node.local_translation
                + self.translation_stack.buffer[buffer_index];

            // Reset the mutation state once the world translation
            // is being updated.
            node.local_translation.reset_mutation();

            self.translation_stack.push_data(node.world_translation);

            for child in node.children.iter() {
                self.translation_stack.push_node(*child);
            }
        }
    }

    /// Update self rect and all children rect.
    pub fn relayout<F>(&mut self, mut relayout: F)
    where
        F: FnMut(&mut Self, NodeId),
    {
        let mutated_nodes = core::mem::take(&mut self.mutated_rects);

        for MutatedNode { id, .. } in mutated_nodes.into_iter().rev()
        {
            let Some(node) = self.nodes.get(&id) else {
                // TODO: Log error, or panic?
                continue;
            };

            // Node could have been relayouted by a previous
            // iteration.
            if !node.size.mutated() {
                continue;
            }

            self.propagate_relayout(id, &mut relayout);
        }
    }

    fn propagate_relayout<F>(&mut self, id: NodeId, relayout: &mut F)
    where
        F: FnMut(&mut Self, NodeId),
    {
        let mut node_stack = vec![id];

        while let Some(id) = node_stack.pop() {
            let Some(node) = self.nodes.get_mut(&id) else {
                // TODO: Log error, or panic?
                continue;
            };

            // self.rect_stack.push_data(node.world_translation);

            if let Some(parent) = node.parent {
                node_stack.push(parent);
            }

            // Reset the mutation state before the relayout happens.
            // This allows us to recapture changes if the relayout happen to
            // update itself.
            node.size.reset_mutation();
            relayout(self, id);
        }
    }
}

#[derive(Debug)]
pub struct NodeStackEl {
    pub id: NodeId,
    pub buffer_index: usize,
}

#[derive(Debug)]
pub struct NodeStack<T> {
    pub elements: Vec<NodeStackEl>,
    pub buffer: Vec<T>,
    pub last_buffer_index: usize,
}

impl<T> Default for NodeStack<T> {
    fn default() -> Self {
        Self {
            elements: Vec::new(),
            buffer: Vec::new(),
            last_buffer_index: 0,
        }
    }
}

impl<T> NodeStack<T> {
    /// Clears the stack and initialize with default values.
    pub fn init(&mut self, root: NodeId, initial_data: T) {
        self.clear();
        self.push_data(initial_data);
        self.push_node(root);
    }

    pub fn push_node(&mut self, id: NodeId) {
        self.elements.push(NodeStackEl {
            id,
            buffer_index: self.last_buffer_index,
        });
    }

    pub fn push_data(&mut self, data: T) {
        self.last_buffer_index = self.buffer.len();
        self.buffer.push(data);
    }

    fn clear(&mut self) {
        self.elements.clear();
        self.buffer.clear();
        self.last_buffer_index = 0;
    }
}

// TODO: Docs must mention about this being assumed to be axis-aligned.
#[derive(Default, Debug, Clone)]
pub struct RectNode {
    pub local_translation: MutDetect<Vec2>,
    pub size: MutDetect<Vec2>,
    pub(crate) world_translation: Vec2,
    pub(crate) parent: Option<NodeId>,
    pub(crate) children: MutDetect<HashSet<NodeId>>,
    /// How deep in the hierarchy is this node (0 for root nodes).
    pub(crate) depth: u32,
}

impl RectNode {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_translation(translation: Vec2) -> Self {
        Self::new().with_translation(translation)
    }

    pub fn from_size(size: Vec2) -> Self {
        Self::new().with_size(size)
    }

    pub fn from_translation_size(
        translation: Vec2,
        size: Vec2,
    ) -> Self {
        Self::new().with_translation(translation).with_size(size)
    }

    pub fn from_rect(rect: Rect) -> Self {
        Self::new()
            .with_translation(rect.origin().to_vec2())
            .with_size(rect.size().to_vec2())
    }

    pub fn with_translation(mut self, translation: Vec2) -> Self {
        *self.local_translation = translation;
        self
    }

    pub fn with_size(mut self, size: Vec2) -> Self {
        *self.size = size;
        self
    }

    pub fn with_parent(mut self, parent: NodeId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn world_translation(&self) -> Vec2 {
        self.world_translation
    }

    pub fn children(&self) -> &HashSet<NodeId> {
        &self.children
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }
}
