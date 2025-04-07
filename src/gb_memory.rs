pub(crate) struct GbMemory {
    pub(crate) memory_array: [u8; 0xFFFF],
}
const INTERRUPT_FLAGS_LOCATION: u16 = 0xFF0F;
const INTERRUPT_ENABLE_LOCATION: u16 = 0xFFFF;

pub(crate) struct InterruptFlags {
    v_blank: bool,
    lcd: bool,
    timer: bool,
    serial: bool,
    joypad: bool,
}
impl InterruptFlags {
    fn get_flags_from_byte(byte: u8) -> Self {
        Self {
            v_blank: (0b00010000 & byte) > 0,
            lcd: (0b1000 & byte) > 0,
            timer: (0b0100 & byte) > 0,
            serial: (0b0010 & byte) > 0,
            joypad: (0b0001 & byte) > 0,
        }
    }
    fn get_byte_from_flag(&self) -> u8 {
        let v_blank = if self.v_blank { 0b10000 } else { 0 };
        let lcd = if self.lcd { 0b1000 } else { 0 };
        let timer = if self.timer { 0b100 } else { 0 };
        let serial = if self.serial { 0b10 } else { 0 };
        let joypad = if self.joypad { 0b1 } else { 0 };
        v_blank | lcd | timer | serial | joypad
    }
}

impl GbMemory {
    pub(crate) fn read_byte(&self, address: u16) -> u8 {
        self.memory_array[address as usize]
    }
    pub(crate) fn write_byte(&mut self, address: u16, value: u8) {
        self.memory_array[address as usize] = value;
    }
    pub(crate) fn read_interrupt_enable(&self) -> InterruptFlags {
        let byte = self.read_byte(INTERRUPT_ENABLE_LOCATION);
        InterruptFlags::get_flags_from_byte(byte)
    }
    pub(crate) fn read_interrupt_flags(&self) -> InterruptFlags {
        let byte = self.read_byte(INTERRUPT_FLAGS_LOCATION);
        InterruptFlags::get_flags_from_byte(byte)
    }
    pub(crate) fn set_interrupt_flags(&mut self, i_f: InterruptFlags) {
        let byte = i_f.get_byte_from_flag();
        self.write_byte(INTERRUPT_FLAGS_LOCATION, byte);
    }
}
