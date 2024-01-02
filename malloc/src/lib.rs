#![no_std]
#![allow(internal_features)]
#![feature(core_intrinsics)]

use core::alloc::{GlobalAlloc, Layout};
use core::ffi::c_int;
use core::intrinsics::abort;
use core::panic::PanicInfo;
use core::sync::atomic::{AtomicU8, Ordering};
use core::{fmt, ptr};

use segtree_alloc::SegTreeAllocator;

#[panic_handler]
fn panic_handler(_: &PanicInfo<'_>) -> ! {
    abort();
}

static ALLOCATOR: SegTreeAllocator = SegTreeAllocator::new();
static ENABLE_DEBUG: AtomicU8 = AtomicU8::new(0xFF);

const DEBUG_ENV_CSTR: &[u8] = b"SGTMALLOC_DEBUG\0";

#[cfg(target_arch = "x86_64")]
const MIN_ALIGN: usize = 16;

macro_rules! debug {
    ($($tt:tt)*) => {
        debug_print(format_args!("{}\n", format_args!( $($tt)*)))
    };
}

fn debug_print(f: fmt::Arguments<'_>) {
    extern "C" {
        fn getenv(name: *const u8) -> *const u8;
    }

    let enabled = match ENABLE_DEBUG.load(Ordering::Relaxed) {
        0 => false,
        1 => true,
        _ => {
            let enabled = unsafe { !getenv(DEBUG_ENV_CSTR.as_ptr()).is_null() };
            ENABLE_DEBUG.store(enabled as u8, Ordering::Relaxed);
            enabled
        }
    };

    struct Writer;

    impl fmt::Write for Writer {
        fn write_str(&mut self, s: &str) -> fmt::Result {
            match unsafe { rustix::io::write(rustix::stdio::stderr(), s.as_bytes()) } {
                Ok(_) => Ok(()),
                Err(_) => Err(fmt::Error),
            }
        }
    }

    if enabled {
        let _ = fmt::write(&mut Writer, f);
    }
}

#[no_mangle]
unsafe extern "C" fn malloc(size: usize) -> *mut u8 {
    debug!("malloc size={size}");
    let layout = Layout::from_size_align_unchecked(size, MIN_ALIGN);
    ALLOCATOR.alloc(layout)
}

#[no_mangle]
unsafe extern "C" fn free(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    debug!("free {ptr:?}");
    ALLOCATOR.dealloc_auto_size(ptr);
}

#[no_mangle]
unsafe extern "C" fn calloc(num: usize, size: usize) -> *mut u8 {
    debug!("calloc num={num} size={size}");
    let Some(size) = num.checked_mul(size) else {
        return ptr::null_mut();
    };
    let layout = Layout::from_size_align_unchecked(size, MIN_ALIGN);
    ALLOCATOR.alloc_zeroed(layout)
}

#[no_mangle]
unsafe extern "C" fn aligned_alloc(align: usize, size: usize) -> *mut u8 {
    debug!("aligned_alloc align={align} size={size}");
    if !align.is_power_of_two() {
        return ptr::null_mut();
    }
    ALLOCATOR.alloc(Layout::from_size_align_unchecked(size, align))
}

#[no_mangle]
unsafe extern "C" fn posix_memalign(ret: *mut *mut u8, align: usize, size: usize) -> c_int {
    debug!("posix_memalign align={align} size={size}");
    let ptr = ALLOCATOR.alloc(Layout::from_size_align_unchecked(size, align));
    *ret = ptr;
    if ptr.is_null() {
        1
    } else {
        0
    }
}

#[no_mangle]
unsafe extern "C" fn realloc(ptr: *mut u8, new_size: usize) -> *mut u8 {
    debug!("realloc ptr={ptr:?} new_size={new_size}");
    if ptr.is_null() {
        return malloc(new_size);
    }
    let prev_size = ALLOCATOR.alloc_size_of(ptr);
    let prev_layout = Layout::from_size_align_unchecked(prev_size, MIN_ALIGN);
    ALLOCATOR.realloc(ptr, prev_layout, new_size)
}
