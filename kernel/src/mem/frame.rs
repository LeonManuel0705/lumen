use bootloader_api::info::{MemoryRegionKind, MemoryRegions};
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

/// Hands out 4 KiB frames by walking the usable regions of the bootloader
/// memory map in order. Frames are never returned, which is fine: the only
/// caller is heap setup, and the heap itself manages its own reuse.
pub struct BootFrameAllocator<'a> {
    regions: &'a MemoryRegions,
    region_idx: usize,
    next_addr: u64,
    handed_out: usize,
}

impl<'a> BootFrameAllocator<'a> {
    /// # Safety
    /// The memory map must be accurate and the regions it marks usable must not
    /// be in use by anything else.
    pub unsafe fn new(regions: &'a MemoryRegions) -> Self {
        let mut me = Self {
            regions,
            region_idx: 0,
            next_addr: 0,
            handed_out: 0,
        };
        me.seek_usable();
        me
    }

    pub fn total_frames(&self) -> u64 {
        self.regions
            .iter()
            .filter(|r| r.kind == MemoryRegionKind::Usable)
            .map(|r| (r.end - r.start) / 4096)
            .sum()
    }

    pub fn handed_out(&self) -> usize {
        self.handed_out
    }

    /// Advances to the first usable region at or after `region_idx` that still
    /// has a frame left, positioning `next_addr` on a frame boundary in it.
    fn seek_usable(&mut self) {
        while self.region_idx < self.regions.len() {
            let region = &self.regions[self.region_idx];
            if region.kind == MemoryRegionKind::Usable {
                let start = align_up(region.start.max(self.next_addr));
                if start + 4096 <= region.end {
                    self.next_addr = start;
                    return;
                }
            }
            self.region_idx += 1;
            self.next_addr = 0;
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        self.seek_usable();
        if self.region_idx >= self.regions.len() {
            return None;
        }
        let addr = self.next_addr;
        self.next_addr += 4096;
        self.handed_out += 1;
        Some(PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

fn align_up(addr: u64) -> u64 {
    (addr + 4095) & !4095
}
