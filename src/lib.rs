use std::fmt;

type Mask = u8;

const UNIT_SIZE: usize = 1;
const HEIGHT: u8 = 3;
const LEAF_LEN: usize = 1 << HEIGHT;
const TREE_LEN: usize = LEAF_LEN * 2;
const MAX_SIZE: usize = UNIT_SIZE * LEAF_LEN;

const USED: Mask = 0x80;

#[derive(Debug)]
pub struct SegTreeAlloc {
    tree: [Mask; TREE_LEN],
}

impl SegTreeAlloc {
    pub const fn new() -> Self {
        Self {
            tree: [0u8; TREE_LEN],
        }
    }

    fn lvl_for_size(size: usize) -> Result<u8, ()> {
        let size = size
            .checked_next_power_of_two()
            .filter(|&s| s < MAX_SIZE)
            .ok_or(())?;
        Ok(HEIGHT - (size / UNIT_SIZE).trailing_zeros() as u8)
    }

    #[allow(clippy::result_unit_err)]
    pub fn alloc(&mut self, size: usize) -> Result<usize, ()> {
        let lvl = Self::lvl_for_size(size)?;
        if self.tree[1] > lvl {
            return Err(());
        }
        let mut i = 1usize;
        // ? LChild + (curlvl + 1) <= lvl, where curlvl = 0..lvl
        // = LChild < lvl - curlvl, where curlvl = 0..lvl
        // Compiler will flip the loop direction.
        for l in 0..lvl {
            i = (i << 1) | (if self.tree[i << 1] < lvl - l { 0 } else { 1 });
        }
        // offset = (i - 2^lvl) * (2^H / 2^lvl) * U
        //        = (i * 2^(H - lvl) - 2^H) * U
        let off = ((i << (HEIGHT - lvl)) - (1 << HEIGHT)) * UNIT_SIZE;
        self.tree[i] = USED;
        self.push_up(i);
        Ok(off)
    }

    pub fn dealloc(&mut self, off: usize, size: usize) {
        let lvl = Self::lvl_for_size(size).unwrap();
        // index = 2^lvl + offset / U / (2^H / 2^lvl)
        //       = 2^lvl + offset / U / 2^(H-lvl)
        //       = (2^H + offset / U) / 2^(H-lvl)
        let i = ((1 << HEIGHT) + off / UNIT_SIZE) >> (HEIGHT - lvl);
        debug_assert_eq!(self.tree[i], USED);
        self.tree[i] = 0;
        self.push_up(i);
    }

    fn push_up(&mut self, mut i: usize) {
        while i > 1 {
            let a = self.tree[i];
            let b = self.tree[i ^ 1];
            self.tree[i >> 1] = a.min(b) + (a != 0 || b != 0) as u8;
            i >>= 1;
        }
    }
}

impl fmt::Display for SegTreeAlloc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for lvl in 0..=HEIGHT {
            let off = 1 << lvl;
            let cnt = 1 << lvl;
            let stride = LEAF_LEN / cnt;
            for (i, x) in self.tree[off..][..cnt].iter().enumerate() {
                if i != 0 {
                    write!(f, "{:len$}", "", len = (stride - 1) * 4)?;
                }
                write!(f, "{:4}", x + lvl)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    use super::*;

    #[test]
    fn simple() {
        let mut heap = SegTreeAlloc::new();

        assert_eq!(heap.alloc(1).unwrap(), 0);
        assert_eq!(heap.alloc(2).unwrap(), 2);
        assert_eq!(heap.alloc(1).unwrap(), 1);
        assert_eq!(heap.alloc(1).unwrap(), 4);
        heap.alloc(4).unwrap_err();

        heap.dealloc(4, 1);
        assert_eq!(heap.alloc(4).unwrap(), 4);
        heap.alloc(1).unwrap_err();

        heap.dealloc(2, 2);
        assert_eq!(heap.alloc(1).unwrap(), 2);
        assert_eq!(heap.alloc(1).unwrap(), 3);
    }

    #[test]
    fn cross() {
        let mut heap = SegTreeAlloc::new();
        assert_eq!(heap.alloc(2).unwrap(), 0);
        assert_eq!(heap.alloc(2).unwrap(), 2);
        assert_eq!(heap.alloc(2).unwrap(), 4);
        heap.dealloc(0, 2);

        heap.alloc(4).unwrap_err();
        assert_eq!(heap.alloc(2).unwrap(), 0);
    }

    #[test]
    fn random() {
        const SEED: u64 = 0x6868_4242_DEAD_BEEF;
        const ROUND: usize = 1_000_000;

        let mut rng = StdRng::seed_from_u64(SEED);
        let mut heap = SegTreeAlloc::new();
        let mut alloc_map = BTreeMap::new();
        let mut alloc_idx = Vec::new();
        let mut total_allocated = 0usize;

        let mut success_cnt = [(0u32, 0u32); MAX_SIZE + 1];

        for _ in 0..ROUND {
            let rest = MAX_SIZE - total_allocated;
            if rest != 0 && (alloc_map.is_empty() || rng.gen()) {
                let size = rng.gen_range(1..=rest.min(MAX_SIZE / 2));

                success_cnt[rest].0 += 1;
                if let Ok(off) = heap.alloc(size) {
                    success_cnt[rest].1 += 1;
                    total_allocated += size;

                    if let Some((&before_off, &(before_size, _))) =
                        alloc_map.range(..=off).next_back()
                    {
                        assert!(before_off + before_size <= off);
                    }
                    if let Some((&after_off, _)) = alloc_map.range(off..).next() {
                        assert!(off + size <= after_off);
                    }
                    let idx = alloc_idx.len();
                    assert_eq!(alloc_map.insert(off, (size, idx)), None);
                    alloc_idx.push(off);
                }
            } else {
                let idx = rng.gen_range(0..alloc_idx.len());
                let off = alloc_idx[idx];
                let (size, _) = alloc_map.remove(&off).unwrap();
                heap.dealloc(off, size);

                total_allocated -= size;

                if idx + 1 != alloc_idx.len() {
                    let last_off = *alloc_idx.last().unwrap();
                    alloc_map.get_mut(&last_off).unwrap().1 = idx;
                }
                alloc_idx.swap_remove(idx);
            }
        }

        for (size, &(total, success)) in success_cnt
            .iter()
            .enumerate()
            .step_by((success_cnt.len() / 20).max(1))
        {
            if total != 0 {
                println!(
                    "free={:.3} success={:.4}",
                    size as f32 / MAX_SIZE as f32,
                    success as f32 / total as f32
                );
            }
        }
    }
}
