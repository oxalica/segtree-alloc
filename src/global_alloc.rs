use std::alloc::{GlobalAlloc, Layout};
use std::cell::SyncUnsafeCell;
use std::process::abort;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};

use rustix::mm::{mmap_anonymous, MapFlags, ProtFlags};

use crate::SegTreeAlloc;

type Heap = SegTreeAlloc<64, 24>;

pub struct SegTreeAllocator {
    guard: AtomicBool,
    start_ptr: SyncUnsafeCell<usize>,
    inner: SyncUnsafeCell<Heap>,
}

impl SegTreeAllocator {
    pub const fn new() -> Self {
        Self {
            guard: AtomicBool::new(false),
            start_ptr: SyncUnsafeCell::new(0),
            inner: SyncUnsafeCell::new(SegTreeAlloc::new()),
        }
    }

    fn with_guard<T>(&self, f: impl FnOnce(&mut usize, &mut Heap) -> T) -> T {
        if self.guard.swap(true, Ordering::Acquire) {
            abort();
        }
        let ret = unsafe { f(&mut *self.start_ptr.get(), &mut *self.inner.get()) };
        self.guard.store(false, Ordering::Release);
        ret
    }
}

#[cold]
fn mmap_all() -> usize {
    let ret = unsafe {
        mmap_anonymous(
            ptr::null_mut(),
            Heap::MAX_SIZE,
            ProtFlags::READ | ProtFlags::WRITE,
            MapFlags::PRIVATE,
        )
    };
    match ret {
        Ok(ptr) if !ptr.is_null() => ptr as usize,
        _ => abort(),
    }
}

unsafe impl GlobalAlloc for SegTreeAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());
        self.with_guard(|start_ptr, h| {
            if *start_ptr == 0 {
                *start_ptr = mmap_all();
            }
            match h.alloc(size) {
                Ok(off) => (*start_ptr + off) as *mut u8,
                Err(()) => ptr::null_mut(),
            }
        })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(layout.align());
        self.with_guard(|start_ptr, h| {
            let off = ptr as usize - *start_ptr;
            unsafe {
                h.dealloc(off, size).unwrap_unchecked();
            }
        });
    }
}