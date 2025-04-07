use crate::{gb_memory, gb_registers, renderer};
use log::info;

pub(crate) struct Gb {
    pub(crate) registers: gb_registers::GbRegisters,
    pub(crate) gb_memory: gb_memory::GbMemory,
    pub(crate) interrupt_master_flag: bool,
    pub(crate) renderer: renderer::Renderer,
}
const LCDC_LOCATION: u16 = 0xFF40;

impl Gb {
    pub fn get_r8(&self, register_id: u8) -> u8 {
        if register_id != 6 {
            self.registers.internal_get_r8(register_id)
        } else {
            let read_location = self.registers.get_hl();
            self.gb_memory.read_byte(read_location)
        }
    }
    pub fn set_r8(&mut self, register_id: u8, new_value: u8) {
        if register_id != 6 {
            self.registers.internal_set_r8(register_id, new_value);
        } else {
            let write_location = self.registers.get_hl();
            self.gb_memory.write_byte(write_location, new_value);
        }
    }

    pub(crate) fn render(&mut self) {
        //load LCDC control register byte
        info!("attempting render");
        let lcdc = self.gb_memory.read_byte(LCDC_LOCATION);
        let lcdc_flags = renderer::RendererLcdcFlags::new(lcdc);
        if !lcdc_flags.lcd_enable {
            info!("lcd disabled");
            return;
        }
        self.renderer
            .render_bg(lcdc_flags, self.gb_memory.memory_array)
    }
    pub(crate) fn read_byte_and_advance_program_counter(&mut self) -> u8 {
        self.registers.program_counter += 1;
        self.gb_memory.read_byte(self.registers.program_counter - 1)
    }
    pub(crate) fn read_word_and_advance_program_counter(&mut self) -> u16 {
        let b1 = self.read_byte_and_advance_program_counter() as u16;
        let b2 = self.read_byte_and_advance_program_counter() as u16;
        (b2 << 8) | b1
    }
    //hope this works...
    pub(crate) fn read_byte_signed_and_advance_program_counter(&mut self) -> i8 {
        self.registers.program_counter += 1;
        let u8byte = self.gb_memory.read_byte(self.registers.program_counter - 1);
        unsafe { std::mem::transmute(u8byte) }
    }
    pub(crate) fn read_hl_indirection_offset(&self, offset: u16) -> u8 {
        let hl_location = self.registers.get_hl();
        self.gb_memory.read_byte(hl_location + offset)
    }
    pub(crate) fn read_hl_indirection(&self) -> u8 {
        self.read_hl_indirection_offset(0)
    }
    pub(crate) fn set_hl_indirection_offset(&mut self, offset: u16, new_value: u8) {
        let hl_location = self.registers.get_hl() + offset;
        self.gb_memory.write_byte(hl_location, new_value);
    }
    pub(crate) fn set_hl_indirection(&mut self, new_value: u8) {
        self.set_hl_indirection_offset(0, new_value);
    }
    pub(crate) fn pop_stack_byte(&mut self) -> u8 {
        let read_byte_location = self.registers.stack_pointer;
        self.registers.stack_pointer += 1;
        self.gb_memory.read_byte(read_byte_location)
    }
    pub(crate) fn pop_stack_word(&mut self) -> u16 {
        let read_byte_low = self.pop_stack_byte() as u16;
        let read_byte_high = (self.pop_stack_byte() as u16) << 8;
        read_byte_high | read_byte_low
    }
    pub(crate) fn push_stack_byte(&mut self, val: u8) {
        let write_byte_location = self.registers.stack_pointer;
        self.registers.stack_pointer -= 1;
        self.gb_memory.write_byte(write_byte_location, val);
    }
    pub(crate) fn push_stack_word(&mut self, val: u16) {
        let write_byte_low = (val & 0xFF) as u8;
        let write_byte_high = ((val & 0xFF00) >> 8) as u8;
        self.push_stack_byte(write_byte_high);
        self.push_stack_byte(write_byte_low);
    }
}
