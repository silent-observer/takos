use log::info;

use super::blockdevice::RandomAccessDevice;

pub struct RamDisk(&'static mut [u8]);

impl RamDisk {
    pub fn new(ramdisk: &'static mut [u8]) -> Self {
        Self(ramdisk)
    }
}

impl RandomAccessDevice for RamDisk {
    fn read(&self, addr: usize, size: usize) -> &[u8] {
        info!("RamDisk read: 0x{:X}, 0x{:X}", addr, size);
        assert!(addr + size <= self.0.len());
        &self.0[addr..(addr + size)]
    }

    fn write(&mut self, addr: usize, data: &[u8]) {
        assert!(addr + data.len() <= self.0.len());
        self.0[addr..(addr + data.len())].copy_from_slice(data);
    }
}
