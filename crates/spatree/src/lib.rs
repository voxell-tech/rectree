#![doc = include_str!("../README.md")]
#![no_std]

extern crate alloc;

use core::ops::Deref;

use alloc::boxed::Box;
use alloc::vec;
use alloc::vec::Vec;
use kurbo::{Point, Rect};

/// **Spatree** implements a Linear Bounding Volume Hierarchy (LBVH).
///
/// It uses _Morton encoding_ to map 2D spaital coordinates onto a 1D
/// Z-order curve. Sorting these codes ensures spatially close objects
/// are adjacent in memory, allowing for efficient top-down hierarchy
/// generation.
#[derive(Default)]
pub struct Spatree {
    global_bound: Rect,
    rects: Vec<Rect>,
    nodes: Vec<Node>,
}

// Builders.
impl Spatree {
    /// Creates a new empty [`Spatree`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a new [`Rect`] into the spatial tree.
    ///
    /// If this is performed after [`Self::build()`], a rebuild will
    /// be required to cater for the change!
    pub fn push_rect(&mut self, rect: Rect) -> RectId {
        let index = self.rects.len();
        self.rects.push(rect);
        // Fit the global bound to the new rect.
        self.global_bound = self.global_bound.union(rect);
        RectId(index)
    }

    /// Get a specific [`Rect`] for a given [`RectId`].
    pub fn get_rect(&self, id: RectId) -> Option<&Rect> {
        self.rects.get(*id)
    }

    /// Obtain the global bounding box of the spatial tree.
    /// Thi global bound is accumulated during
    /// [`Self::push_rect()`] calls.
    pub fn global_bound(&self) -> &Rect {
        &self.global_bound
    }

    /// Constructs a spatial hierarchy (LBVH) from the current set of rectangles.
    ///
    /// ### Arguments
    ///
    /// - `point_from_rect`: A closure that determines the stable
    ///   representative point (e.g., center or top-left) of a `Rect`
    ///   used for Morton encoding.
    ///
    /// After construction, all internal node bounding boxes are computed.
    ///
    /// If [`Self::global_bound()`] has zero area, the tree is left
    /// empty since no meaningful spatial ordering can be derived.
    pub fn build<F>(&mut self, point_from_rect: F)
    where
        F: Fn(&Rect) -> Point,
    {
        let bound_size = self.global_bound.size();
        // There is point in building a spatial tree when there is no
        // space within the max bound.
        if bound_size.is_zero_area() {
            return;
        }

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

        // Build internal nodes.
        self.nodes = generate_hierarchy(&morton_codes);
        self.calculate_internal_bounds();
    }

    /// Calculate the bounds of all the internal nodes.
    fn calculate_internal_bounds(&mut self) {
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
                        self.rects[rect_id]
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
    }
}

/// Queries.
impl Spatree {
    /// Query for hits for an arbitrary target and hit condition.
    pub fn query<T, F>(
        &self,
        target: T,
        hit_condition: F,
    ) -> Vec<RectId>
    where
        F: Fn(&Rect, &T) -> bool,
    {
        let mut hits = Vec::new();

        if self.nodes.is_empty() {
            // There's no tree, if there's just one rect, do a hit
            // test for it.
            if let Some(rect) = self.rects.first()
                && hit_condition(rect, &target)
            {
                hits.push(RectId(0));
            }
        } else {
            // Traverse the tree.
            let mut stack = vec![0];

            while let Some(node_idx) = stack.pop() {
                let node = self.nodes[node_idx];

                // Skip the tree if it's not a hit.
                if !hit_condition(&node.rect, &target) {
                    continue;
                }

                for child in node.children.iter() {
                    match child {
                        NodeId::Internal(child_idx) => {
                            stack.push(*child_idx)
                        }
                        NodeId::Leaf(leaf_idx) => {
                            if hit_condition(
                                &self.rects[*leaf_idx],
                                &target,
                            ) {
                                hits.push(RectId(*leaf_idx));
                            }
                        }
                        NodeId::Invalid => continue,
                    }
                }
            }
        }

        hits
    }

    /// Query for rects that contains the given [`Point`].
    pub fn query_point(&self, point: Point) -> Vec<RectId> {
        self.query(
            point,
            #[inline(always)]
            |rect, point| rect.contains(*point),
        )
    }

    /// Query for rects that overlaps the given [`Rect`].
    pub fn query_rect(&self, rect: Rect) -> Vec<RectId> {
        self.query(
            rect,
            #[inline(always)]
            |rect, target_rect| rect.overlaps(*target_rect),
        )
    }
}

/// An internal node within the [`Spatree`].
#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub rect: Rect,
    pub parent: Option<usize>,
    pub children: [NodeId; 2],
}

impl Node {
    /// Empty node with zero area, no children, and no parent.
    pub const EMPTY: Self = Self {
        rect: Rect::ZERO,
        parent: None,
        children: [NodeId::Invalid; 2],
    };
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub enum NodeId {
    Internal(usize),
    Leaf(usize),
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

/// Stores the Morton code alongside their associated leaf index.
///
/// This struct is optimized for ordering based only on
/// [`Self::code`] without any consideration for [`Self::index`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MortonCode {
    pub code: u32,
    pub index: usize,
}

impl Ord for MortonCode {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.code.cmp(&other.code)
    }
}

impl PartialOrd for MortonCode {
    fn partial_cmp(
        &self,
        other: &Self,
    ) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Top down hierarchy building for single threaded algorithm.
pub fn generate_hierarchy(codes: &[MortonCode]) -> Vec<Node> {
    let len = codes.len();
    if len <= 1 {
        return Vec::new();
    }

    // A binary tree with N leaves has exactly N - 1 internal nodes.
    let mut internal_nodes = vec![Node::EMPTY; len - 1];

    /// Represents a range to be split and its connection to the tree.
    struct BuildStack {
        first: usize,
        last: usize,
        parent_idx: Option<usize>,
        /// `0` for left, `1` for right.
        child_slot: usize,
    }

    let mut stack = Vec::with_capacity(len);
    let mut node_idx = 0;

    // First build stakc will have the full range.
    stack.push(BuildStack {
        first: 0,
        last: internal_nodes.len(),
        parent_idx: None,
        child_slot: 0,
    });

    while let Some(task) = stack.pop() {
        let BuildStack {
            first,
            last,
            parent_idx,
            child_slot,
        } = task;

        let curr_node_id = if first == last {
            // Single element range represents a leaf node.
            NodeId::Leaf(codes[first].index)
        } else {
            // Internal node case.
            let node_id = NodeId::Internal(node_idx);
            let split = find_split(codes, first, last);

            // Push right sub-range then left sub-range (LIFO).
            stack.push(BuildStack {
                first,
                last: split,
                parent_idx: Some(node_idx),
                child_slot: 0,
            });
            stack.push(BuildStack {
                first: split + 1,
                last,
                parent_idx: Some(node_idx),
                child_slot: 1,
            });

            node_idx += 1;
            node_id
        };

        // Link the current node to its parent if it's not the root.
        if let Some(parent_idx) = parent_idx {
            internal_nodes[parent_idx].children[child_slot] =
                curr_node_id;

            // If the current node is internal, set its parent index.
            if let NodeId::Internal(curr_idx) = curr_node_id {
                internal_nodes[curr_idx].parent = Some(parent_idx);
            }
        }
    }

    internal_nodes
}

/// `x` & `y` must be within (and will be clamped into)
/// the `0..=1` range.
pub fn morton_2d_f64(x: f64, y: f64) -> u32 {
    const MAX: f64 = 65535.0;
    let x = (x.clamp(0.0, 1.0) * MAX) as u16;
    let y = (y.clamp(0.0, 1.0) * MAX) as u16;

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

/// Find the split point for a range of sorted Morton codes.
///
/// Locate the position where the shared bit prefix changes and
/// return the index used to divide the range into two clusters.
pub const fn find_split(
    morton_codes: &[MortonCode],
    first: usize,
    last: usize,
) -> usize {
    let first_code = morton_codes[first].code;
    let last_code = morton_codes[last].code;
    // Split the range in the middle for identical Morton codes.
    if first_code == last_code {
        return (first + last) >> 1;
    };

    let common_prefix = calc_common_prefix(first_code, last_code);

    // Use binary search to find where the next bit differs.
    // Specifically, we are looking for the highest object that
    // shares more than `common_prefix` bits with the first one.

    // Initial guess.
    let mut split = first;
    let mut step = last - first;
    while step > 1 {
        // Exponential decrease.
        step = (step + 1) >> 1;
        // Proposed new position.
        let new_split = split + step;

        if new_split < last {
            let split_code = morton_codes[new_split].code;
            let split_prefix =
                calc_common_prefix(first_code, split_code);

            if split_prefix > common_prefix {
                // Accept proposal.
                split = new_split
            };
        }
    }

    split
}

/// Measures the common prefix of two morton codes.
#[inline]
pub const fn calc_common_prefix(code_a: u32, code_b: u32) -> u32 {
    (code_a ^ code_b).leading_zeros()
}
