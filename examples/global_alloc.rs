use segtree_alloc::SegTreeAllocator;

#[global_allocator]
static ALLOCATOR: SegTreeAllocator = SegTreeAllocator::new();

fn main() {
    let mut sum = 0u64;
    for i in 1..=1_000_000 {
        let mut v = std::iter::successors(Some(i), |&x| {
            if x == 1 {
                return None;
            }
            Some(if x % 2 == 0 { x / 2 } else { x * 3 + 1 })
        })
        .collect::<Vec<u32>>();
        v.sort();
        let max_delta = v.windows(2).map(|w| w[1] - w[0]).max().unwrap_or(0);
        sum += max_delta as u64;
    }
    println!("avg max delta = {:.3}", sum as f64 / 1000.0);
}
