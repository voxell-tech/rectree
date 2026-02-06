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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_morton_logic_consistency() {
        // Ensure x and y bits are interleaved correctly.
        // x=1 (01), y=0 (00) -> 01
        assert_eq!(morton_2d(1, 0), 1);
        // x=0 (00), y=1 (01) -> 10 (binary) -> 2
        assert_eq!(morton_2d(0, 1), 2);
        // x=1 (01), y=1 (01) -> 11 (binary) -> 3
        assert_eq!(morton_2d(1, 1), 3);
    }
}
