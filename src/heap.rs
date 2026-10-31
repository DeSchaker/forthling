//! The heap of ALLOCATE, FREE and RESIZE. It lives in Forth memory and
//! grows down from the top towards HERE, so data space and heap share the
//! free space between them.

use crate::*;
use std::collections::BTreeMap;

/// The allocated blocks: start address to size in bytes, a whole number of
/// cells. Free space is whatever lies between them and below `top`.
pub(crate) struct Heap {
    blocks: BTreeMap<usize, usize>,
    top: usize,
}

impl Heap {
    /// An empty heap that ends at `top`, the end of memory.
    pub(crate) fn new(top: usize) -> Self {
        Heap { blocks: BTreeMap::new(), top }
    }

    /// The lowest address in use by the heap; data space ends here.
    pub(crate) fn floor(&self) -> usize {
        self.blocks.keys().next().copied().unwrap_or(self.top)
    }

    /// An aligned block of at least n bytes, at or above `limit` (HERE).
    /// Takes the first gap between blocks that fits, else extends the heap
    /// downwards.
    pub(crate) fn allocate(&mut self, n: usize, limit: usize) -> Option<usize> {
        let size = n.max(1).checked_add(CELL - 1)? & !(CELL - 1);
        let mut gap_start = None;
        let mut prev_end = None;
        for (&start, &len) in &self.blocks {
            if let Some(end) = prev_end {
                if start - end >= size {
                    gap_start = Some(end);
                    break;
                }
            }
            prev_end = Some(start + len);
        }
        let a = match (gap_start, prev_end) {
            (Some(a), _) => a,
            (None, Some(end)) if self.top - end >= size => end,
            _ => {
                let a = self.floor().checked_sub(size)?;
                if a < limit {
                    return None;
                }
                a
            }
        };
        self.blocks.insert(a, size);
        Some(a)
    }

    /// Frees the block that starts at a; false when there is none.
    pub(crate) fn free(&mut self, a: usize) -> bool {
        self.blocks.remove(&a).is_some()
    }

    /// The size of the block that starts at a.
    pub(crate) fn size(&self, a: usize) -> Option<usize> {
        self.blocks.get(&a).copied()
    }

    /// The end of the block that a lies in; outside the blocks, the start
    /// of the next block or the end of memory.
    pub(crate) fn region_end(&self, a: usize) -> usize {
        match self.blocks.range(..=a).next_back() {
            Some((&start, &len)) if a < start + len => start + len,
            _ => self.blocks.range(a + 1..).next().map_or(self.top, |(&s, _)| s),
        }
    }

    /// Grows or shrinks the block at a to n bytes where it is, if the space
    /// after it allows; false otherwise.
    pub(crate) fn resize_in_place(&mut self, a: usize, n: usize) -> bool {
        let Some(size) = n.max(1).checked_add(CELL - 1).map(|s| s & !(CELL - 1)) else {
            return false;
        };
        let next = self.blocks.range(a + 1..).next().map_or(self.top, |(&s, _)| s);
        if a + size > next {
            return false;
        }
        self.blocks.insert(a, size);
        true
    }
}

impl Forth {
    /// Where data space must stop: the bottom of the heap.
    pub(crate) fn data_end(&self) -> usize {
        self.heap.floor()
    }

    /// RESIZE: the new address of the block, which keeps its contents up to
    /// the smaller of both sizes. On failure the block is left as it was.
    pub(crate) fn resize(&mut self, a: usize, n: usize) -> Option<usize> {
        let old = self.heap.size(a)?;
        if self.heap.resize_in_place(a, n) {
            return Some(a);
        }
        let new = self.heap.allocate(n, self.here)?;
        self.mem.copy_within(a..a + old.min(n), new);
        self.heap.free(a);
        Some(new)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks() {
        let mut h = Heap::new(1000);
        let a = h.allocate(10, 0).unwrap();
        assert_eq!((a, h.size(a)), (984, Some(16)));
        let b = h.allocate(0, 0).unwrap();
        assert_eq!(b, a - 8);
        assert_eq!(h.floor(), b);
        assert!(h.free(a) && !h.free(a));
        // the freed block is reused before the heap grows
        assert_eq!(h.allocate(16, 0), Some(a));
        assert!(!h.resize_in_place(b, 9));
        assert!(h.resize_in_place(a, 16) && h.resize_in_place(a, 3));
        assert_eq!(h.size(a), Some(8));
        assert_eq!(h.allocate(1000, 0), None);
        assert_eq!(h.allocate(usize::MAX, 0), None);
        assert_eq!(h.allocate(100, b - 50), None);
    }
}
