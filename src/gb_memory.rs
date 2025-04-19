use log::debug;

pub(crate) struct GbMemory {
    pub(crate) memory_array: [u8; 0xFFFF + 1],
}
const INTERRUPT_FLAGS_LOCATION: u16 = 0xFF0F;
const INTERRUPT_ENABLE_LOCATION: u16 = 0xFFFF;
const DIV_REGISTER_LOCATION: u16 = 0xFF04;
const TIMA_LOCATION: u16 = 0xFF05;
const TMA_LOCATION: u16 = 0xFF06;
const TAC_LOCATION: u16 = 0xFF07;
const JOYP_LOCATION: u16 = 0xFF00;

pub(crate) struct InterruptFlags {
    pub v_blank: bool,
    pub lcd: bool,
    pub timer: bool,
    pub serial: bool,
    pub joypad: bool,
}
impl InterruptFlags {
    pub fn get_flags_from_byte(byte: u8) -> InterruptFlags {
        InterruptFlags {
            v_blank: (0b00010000 & byte) > 0,
            lcd: (0b1000 & byte) > 0,
            timer: (0b0100 & byte) > 0,
            serial: (0b0010 & byte) > 0,
            joypad: (0b0001 & byte) > 0,
        }
    }
    pub fn get_byte_from_flag(&self) -> u8 {
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
        if address == JOYP_LOCATION {
            return 0xFF;
        }
        self.memory_array[address as usize]
    }
    pub(crate) fn write_byte(&mut self, address: u16, value: u8) {
        debug!("Writing 0x{:02x} to 0x{:04x}", value, address);

        if address == DIV_REGISTER_LOCATION {
            self.memory_array[DIV_REGISTER_LOCATION as usize] = 0;
        } else {
            self.memory_array[address as usize] = value;
        }
    }
    pub fn tick_div(&mut self) {
        let current_val = self.memory_array[DIV_REGISTER_LOCATION as usize];
        self.memory_array[DIV_REGISTER_LOCATION as usize] = current_val.wrapping_add(1);
    }
    pub fn get_tima_ticks_per_second(&mut self) -> u64 {
        let tac = self.memory_array[TAC_LOCATION as usize];
        if (tac & 0b100) == 0 {
            return 0;
        }
        match (tac & 0b11) {
            0x00 => 4096,
            0x01 => 262144,
            0x10 => 65536,
            0x11 => 16384,
            _ => panic!("Impossible tac state"),
        }
    }
    pub fn tick_tima(&mut self) {
        let current_val = self.memory_array[TIMA_LOCATION as usize];
        let (new_val, overflow) = current_val.overflowing_add(1);
        if !overflow {
            self.memory_array[TIMA_LOCATION as usize] = new_val;
        } else {
            self.memory_array[TIMA_LOCATION as usize] = self.memory_array[TMA_LOCATION as usize];
            let mut i_f = self.read_interrupt_flags();
            i_f.timer = true;
            self.set_interrupt_flags(i_f);
        }
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
