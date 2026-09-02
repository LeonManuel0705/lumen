use core::alloc::{GlobalAlloc, Layout};
use core::ptr;
use spin::Mutex;

const GRAIN: usize = 16;
const HEADER: usize = core::mem::size_of::<Header>();
const MIN_BLOCK: usize = 32;

#[repr(C)]
struct Header {
    start: usize,
    size: usize,
}

#[repr(C)]
struct FreeNode {
    size: usize,
    next: *mut FreeNode,
}

struct Heap {
    head: *mut FreeNode,
    used: usize,
    peak: usize,
    capacity: usize,
}

unsafe impl Send for Heap {}

pub struct LumenAllocator {
    heap: Mutex<Heap>,
}

#[derive(Copy, Clone)]
pub struct Stats {
    pub used: usize,
    pub peak: usize,
    pub capacity: usize,
    pub free_blocks: usize,
    pub largest_free: usize,
}

impl LumenAllocator {
    pub const fn empty() -> Self {
        Self {
            heap: Mutex::new(Heap {
                head: ptr::null_mut(),
                used: 0,
                peak: 0,
                capacity: 0,
            }),
        }
    }

    pub unsafe fn init(&self, start: usize, size: usize) {
        let mut heap = self.heap.lock();
        let aligned = align_up(start, GRAIN);
        let usable = match size.checked_sub(aligned - start) {
            Some(n) => n & !(GRAIN - 1),
            None => 0,
        };
        if usable < MIN_BLOCK {
            crate::serial_println!("[mem] heap region of {} bytes is too small to use", size);
            return;
        }
        let node = aligned as *mut FreeNode;
        (*node).size = usable;
        (*node).next = ptr::null_mut();
        heap.head = node;
        heap.capacity = usable;
    }

    pub fn stats(&self) -> Stats {
        let heap = self.heap.lock();
        let mut free_blocks = 0;
        let mut largest_free = 0;
        let mut node = heap.head;
        while !node.is_null() {
            free_blocks += 1;
            let size = unsafe { (*node).size };
            if size > largest_free {
                largest_free = size;
            }
            node = unsafe { (*node).next };
        }
        Stats {
            used: heap.used,
            peak: heap.peak,
            capacity: heap.capacity,
            free_blocks,
            largest_free,
        }
    }
}

unsafe impl GlobalAlloc for LumenAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let align = layout.align().max(GRAIN);
        let mut heap = self.heap.lock();

        let mut link: *mut *mut FreeNode = &mut heap.head;
        loop {
            let node = *link;
            if node.is_null() {
                return ptr::null_mut();
            }

            let block_start = node as usize;
            let block_end = block_start + (*node).size;
            let payload = align_up(block_start + HEADER, align);
            let alloc_end = align_up(payload + layout.size().max(1), GRAIN);

            if alloc_end <= block_end {
                let next = (*node).next;
                let remainder = block_end - alloc_end;
                let taken_end = if remainder >= MIN_BLOCK {
                    let split = alloc_end as *mut FreeNode;
                    (*split).size = remainder;
                    (*split).next = next;
                    *link = split;
                    alloc_end
                } else {
                    *link = next;
                    block_end
                };

                let header = (payload - HEADER) as *mut Header;
                (*header).start = block_start;
                (*header).size = taken_end - block_start;

                heap.used += taken_end - block_start;
                if heap.used > heap.peak {
                    heap.peak = heap.used;
                }
                return payload as *mut u8;
            }

            link = &mut (*node).next;
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }
        let header = (ptr as usize - HEADER) as *mut Header;
        let start = (*header).start;
        let size = (*header).size;

        let mut heap = self.heap.lock();
        heap.used -= size;

        let mut prev: *mut FreeNode = ptr::null_mut();
        let mut link: *mut *mut FreeNode = &mut heap.head;
        while !(*link).is_null() && (*link as usize) < start {
            prev = *link;
            link = &mut (**link).next;
        }

        let node = start as *mut FreeNode;
        (*node).size = size;
        (*node).next = *link;
        *link = node;

        let next = (*node).next;
        if !next.is_null() && start + size == next as usize {
            (*node).size += (*next).size;
            (*node).next = (*next).next;
        }

        if !prev.is_null() && prev as usize + (*prev).size == start {
            (*prev).size += (*node).size;
            (*prev).next = (*node).next;
        }
    }
}

fn align_up(value: usize, align: usize) -> usize {
    (value + align - 1) & !(align - 1)
}
