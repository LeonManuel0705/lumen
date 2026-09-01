use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
};
use x86_64::VirtAddr;

use super::allocator::LumenAllocator;

pub const HEAP_START: usize = 0x_4444_4444_0000;
pub const HEAP_SIZE: usize = 24 * 1024 * 1024;

#[global_allocator]
pub static ALLOCATOR: LumenAllocator = LumenAllocator::empty();

pub fn init(
    mapper: &mut impl Mapper<Size4KiB>,
    frames: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), &'static str> {
    let range = {
        let start = Page::containing_address(VirtAddr::new(HEAP_START as u64));
        let end = Page::containing_address(VirtAddr::new((HEAP_START + HEAP_SIZE - 1) as u64));
        Page::range_inclusive(start, end)
    };

    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
    for page in range {
        let frame = frames.allocate_frame().ok_or("out of physical frames")?;
        unsafe {
            mapper
                .map_to(page, frame, flags, frames)
                .map_err(map_error)?
                .flush();
        }
    }

    unsafe { ALLOCATOR.init(HEAP_START, HEAP_SIZE) };
    crate::serial_println!(
        "[mem] heap online at {:#x}, {} MiB",
        HEAP_START,
        HEAP_SIZE / 1024 / 1024
    );
    Ok(())
}

fn map_error(err: MapToError<Size4KiB>) -> &'static str {
    match err {
        MapToError::FrameAllocationFailed => "heap mapping: out of physical frames",
        MapToError::ParentEntryHugePage => "heap mapping: hit a huge page",
        MapToError::PageAlreadyMapped(_) => "heap mapping: page already in use",
    }
}
