use core::ops::Deref;

use alloc::vec::Vec;
use alloc::{boxed::Box, vec};
use kurbo::{Point, Rect};

#[derive(Default)]
pub struct Spatree {
    pub bound: Rect,
    pub rects: Vec<Rect>,
    pub nodes: Vec<Node>,
}

impl Spatree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_rect(&mut self, rect: Rect) -> RectId {
        let index = self.rects.len();
        self.bound = self.bound.union(rect);
        self.rects.push(rect);
        RectId(index)
    }

    pub fn build(&mut self, point_from_rect: fn(&Rect) -> Point) {
        let internal_node_len = self.rects.len() - 1;
        if internal_node_len == 0 {
            return;
        }

        let bound_size = self.bound.size();
        let mut morton_codes = self
            .rects
            .iter()
            .enumerate()
            .map(|(index, rect)| {
                let point = point_from_rect(rect);
                let x = point.x / bound_size.width;
                let y = point.y / bound_size.height;

                let code = morton_2d_f64(x, y);
                MortonCode { code, index }
            })
            .collect::<Box<_>>();

        morton_codes.sort_unstable();

        // Top down hierarchy building for single threaded algorithm.
        self.nodes = generate_hierarchy_iterative(&morton_codes);
    }

    pub fn calculate_bounds(&mut self) {
        if self.nodes.is_empty() {
            return;
        }

        // Because internal nodes were allocated top-down, children
        // always have a higher index than their parents. By iterating
        // backwards, we process the tree bottom-up.
        for i in (0..self.nodes.len()).rev() {
            let mut combined_rect = None;

            // Check both children to compute the unioned bounding box
            for child_id in self.nodes[i].children {
                let child_rect = match child_id {
                    NodeId::Leaf(rect_id) => {
                        // Leaf bounds are already known from the input rects
                        self.rects[*rect_id]
                    }
                    NodeId::Internal(idx) => {
                        // Because idx > i, this child's rect was
                        // already calculated in a previous iteration of this loop.
                        self.nodes[idx].rect
                    }
                    NodeId::Invalid => Rect::ZERO,
                };

                // Union the child's rect into the parent's rect
                combined_rect = Some(match combined_rect {
                    None => child_rect,
                    Some(existing) => child_rect.union(existing),
                });
            }

            if let Some(final_rect) = combined_rect {
                self.nodes[i].rect = final_rect;
            }
        }

        // Optionally update the global Spatree bound to the root's bound.
        if let Some(root) = self.nodes.first() {
            self.bound = root.rect;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub rect: Rect,
    pub parent: NodeId,
    pub children: [NodeId; 2],
}

#[derive(
    Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum NodeId {
    Internal(usize),
    Leaf(RectId),
    #[default]
    Invalid,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct RectId(usize);

impl Deref for RectId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// `x` & `y` must be within the `0..=1` range.
pub fn morton_2d_f64(x: f64, y: f64) -> u32 {
    const MAX: f64 = 65535.0;
    let x = (x * MAX) as u16;
    let y = (y * MAX) as u16;

    morton_2d(x, y)
}

/// Combine 2 [`u16`] integers into a [`u32`] morton code.
pub fn morton_2d(x: u16, y: u16) -> u32 {
    fn expand(mut v: u32) -> u32 {
        v = (v | (v << 8)) & 0x00FF00FF;
        v = (v | (v << 4)) & 0x0F0F0F0F;
        v = (v | (v << 2)) & 0x33333333;
        v = (v | (v << 1)) & 0x55555555;
        v
    }
    expand(x as u32) | (expand(y as u32) << 1)
}

pub const fn find_split(
    morton_codes: &[MortonCode],
    first: usize,
    last: usize,
) -> usize {
    let first_code = morton_codes[first].code;
    let last_code = morton_codes[last].code;
    // Handle duplicated morton code separately.
    if first_code == last_code {
        return (first + last) / 2;
    };

    let common_prefix = (first_code ^ last_code).leading_zeros();

    // Use binary search to find where the next bit differs.
    // Specifically, we are looking for the highest object that
    // shares more than `common_prefix` bits with the first one.

    let mut split = first; // initial guess
    let mut step = last - first;
    loop {
        // Exponential decrease.
        step = (step + 1) >> 1;
        // Proposed new position.
        let new_split = split + step;

        if new_split < last {
            let split_code = morton_codes[new_split].code;
            let split_prefix =
                (first_code ^ split_code).leading_zeros();

            if split_prefix > common_prefix {
                // Accept proposal.
                split = new_split
            };
        }

        if step <= 1 {
            break;
        }
    }

    split
}

/// Delta operator measures the common prefix of two morton codes if
/// `j` is not in the range of the morton code array, delta operator
/// returns `None`.
pub const fn delta(
    i: usize,
    j: usize,
    morton_codes: &[MortonCode],
) -> u32 {
    (morton_codes[i].code ^ morton_codes[j].code).leading_zeros()
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct MortonCode {
    pub code: u32,
    pub index: usize,
}

pub fn generate_hierarchy_iterative(
    codes: &[MortonCode],
) -> Vec<Node> {
    let len = codes.len();
    if len <= 1 {
        return Vec::new();
    }

    // A binary tree with N leaves has exactly N - 1 internal nodes.
    let mut internal_nodes = vec![
        Node {
            rect: Rect::default(),
            parent: NodeId::Invalid,
            children: [NodeId::Invalid; 2],
        };
        len - 1
    ];

    let mut stack = Vec::with_capacity(len);
    let mut next_internal_idx = 0;

    /// Represents a range to be split and its connection to the tree.
    struct BuildTask {
        first: usize,
        last: usize,
        parent_idx: Option<usize>,
        /// `0` for left, `1` for right.
        child_slot: usize,
    }

    // Push the root task: the full range of sorted Morton codes.
    stack.push(BuildTask {
        first: 0,
        last: len - 1,
        parent_idx: None,
        child_slot: 0,
    });

    while let Some(task) = stack.pop() {
        let BuildTask {
            first,
            last,
            parent_idx,
            child_slot,
        } = task;

        let current_node_id = if first == last {
            // Is leaf node: use the index stored in `MortonCode` .
            NodeId::Leaf(RectId(codes[first].index))
        } else {
            // Internal node case.
            let node_idx = next_internal_idx;
            next_internal_idx += 1;

            let split = find_split(codes, first, last);

            // Push right sub-range then left sub-range (LIFO)
            stack.push(BuildTask {
                first: split + 1,
                last,
                parent_idx: Some(node_idx),
                child_slot: 1,
            });

            stack.push(BuildTask {
                first,
                last: split,
                parent_idx: Some(node_idx),
                child_slot: 0,
            });

            NodeId::Internal(node_idx)
        };

        // Link the current node to its parent if it's not the root
        if let Some(parent_idx) = parent_idx {
            internal_nodes[parent_idx].children[child_slot] =
                current_node_id;

            // If the current node is internal, set its parent pointer
            if let NodeId::Internal(curr_idx) = current_node_id {
                internal_nodes[curr_idx].parent =
                    NodeId::Internal(parent_idx);
            }
        }
    }

    internal_nodes
}
