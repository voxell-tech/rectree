use core::ops::Deref;

use alloc::boxed::Box;
use alloc::vec::Vec;
use kurbo::Rect;

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

    pub fn build(&mut self) {
        let bound_size = self.bound.size();
        let mut rect_codes = self
            .rects
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let center = r.center();
                let x = center.x / bound_size.width;
                let y = center.y / bound_size.height;

                let code = morton_2d_f64(x, y);
                (code, i)
            })
            .collect::<Box<_>>();

        rect_codes.sort_unstable();
    }
}

pub struct Node {
    pub rect: Rect,
    pub children: [NodeId; 2],
}

pub enum NodeId {
    Internal(usize),
    Leaf(RectId),
}

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

/// Delta operator measures the common prefix of two morton codes if
/// `j` is not in the range of the morton code array, delta operator
/// returns `None`.
pub const fn delta(i: usize, j: usize, morton_codes: &[u32]) -> u32 {
    (morton_codes[i] ^ morton_codes[j]).leading_zeros()
}

pub const fn find_split(
    morton_codes: &[u32],
    first: usize,
    last: usize,
) -> usize {
    let first_code = morton_codes[first];
    let last_code = morton_codes[last];
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
            let split_code = morton_codes[new_split];
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
