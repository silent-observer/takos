pub trait BlockDevice {
    const BLOCK_SIZE: usize;

    fn read_block(&self, index: usize) -> &[u8];
    fn write_block(&mut self, index: usize, data: &[u8]);
}

pub trait RandomAccessDevice {
    fn read(&self, addr: usize, size: usize) -> &[u8];
    fn write(&mut self, addr: usize, data: &[u8]);
}
