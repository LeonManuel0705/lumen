pub mod allocator;
pub mod frame;
pub mod heap;

use bootloader_api::BootInfo;
use x86_64::structures::paging::OffsetPageTable;
use x86_64::VirtAddr;

pub use frame::BootFrameAllocator;

pub fn init(boot_info: &BootInfo) -> Result<(), &'static str> {
    let offset = boot_info
        .physical_memory_offset
        .into_option()
        .ok_or("bootloader did not map physical memory")?;
    let phys_offset = VirtAddr::new(offset);

    let mut mapper = unsafe { active_page_table(phys_offset) };
    let mut frames = unsafe { BootFrameAllocator::new(&boot_info.memory_regions) };

    crate::serial_println!(
        "[mem] physical offset {:#x}, {} usable frames ({} MiB)",
        offset,
        frames.total_frames(),
        frames.total_frames() * 4 / 1024
    );

    heap::init(&mut mapper, &mut frames)?;
    crate::serial_println!("[mem] {} frames mapped for the heap", frames.handed_out());
    Ok(())
}

pub fn selftest() {
    use alloc::vec::Vec;

    let before = heap::ALLOCATOR.stats();

    {
        let mut keep: Vec<Vec<u8>> = Vec::new();
        let mut drop_soon: Vec<Vec<u8>> = Vec::new();
        for i in 0..64 {
            let len = 1024 + i * 97;
            let mut block = alloc::vec![0u8; len];
            block[0] = i as u8;
            block[len - 1] = i as u8;
            if i % 2 == 0 { keep.push(block) } else { drop_soon.push(block) }
        }
        drop(drop_soon);
        let big = alloc::vec![7u8; 512 * 1024];
        for (i, block) in keep.iter().enumerate() {
            let tag = (i * 2) as u8;
            if block[0] != tag || block[block.len() - 1] != tag {
                panic!("heap selftest: block {} was corrupted by a neighbour", i * 2);
            }
        }
        if big.len() != 512 * 1024 {
            panic!("heap selftest: large allocation came back short");
        }
    }

    let after = heap::ALLOCATOR.stats();
    if after.used != before.used || after.free_blocks != before.free_blocks {
        crate::serial_println!(
            "[mem] selftest leaked: {} bytes and {} blocks left over",
            after.used - before.used,
            after.free_blocks as i32 - before.free_blocks as i32
        );
    } else {
        crate::serial_println!(
            "[mem] selftest ok, peak {} KiB of {} KiB, largest free block {} KiB",
            after.peak / 1024,
            after.capacity / 1024,
            after.largest_free / 1024
        );
    }
}

unsafe fn active_page_table(phys_offset: VirtAddr) -> OffsetPageTable<'static> {
    use x86_64::registers::control::Cr3;

    let (level_4_frame, _) = Cr3::read();
    let virt = phys_offset + level_4_frame.start_address().as_u64();
    let table: *mut x86_64::structures::paging::PageTable = virt.as_mut_ptr();
    OffsetPageTable::new(&mut *table, phys_offset)
}
