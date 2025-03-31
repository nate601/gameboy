pub(crate) struct GbMemory {
    pub(crate) memory_array: [u8; 0xFFFF],
}

impl GbMemory {
    pub(crate) fn read_byte(&self, address: u16) -> u8 {
        self.memory_array[address as usize]
    }
    pub(crate) fn write_byte(&mut self, address: u16, value: u8) {
        self.memory_array[address as usize] = value;
    }
}
