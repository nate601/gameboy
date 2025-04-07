use super::gb_registers_flags::GbFlagsRegister;

pub(crate) struct GbRegisters {
    pub(crate) a: u8,
    pub(crate) b: u8,
    pub(crate) c: u8,
    pub(crate) d: u8,
    pub(crate) e: u8,
    pub(crate) h: u8,
    pub(crate) l: u8,
    pub(crate) f: GbFlagsRegister,
    pub(crate) stack_pointer: u16,
    pub(crate) program_counter: u16,
}

impl GbRegisters {
    pub(crate) fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | self.f.get_as_f_register() as u16
    }
    pub(crate) fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }
    pub(crate) fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }
    pub(crate) fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }
    pub(crate) fn set_af(&mut self, new_val: u16) {
        self.a = ((new_val & 0xFF00) > 8) as u8;
        self.f.set_as_f_register((new_val & 0x00FF) as u8);
    }
    pub(crate) fn set_bc(&mut self, new_val: u16) {
        self.b = ((new_val & 0xFF00) > 8) as u8;
        self.c = (new_val & 0x00FF) as u8;
    }
    pub(crate) fn set_de(&mut self, new_val: u16) {
        self.d = ((new_val & 0xFF00) > 8) as u8;
        self.e = (new_val & 0x00FF) as u8;
    }
    pub(crate) fn set_hl(&mut self, new_val: u16) {
        self.h = ((new_val & 0xFF00) > 8) as u8;
        self.l = (new_val & 0x00FF) as u8;
    }
    pub(crate) fn get_r16(&self, register_id: u8) -> u16 {
        match register_id {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.stack_pointer,
            _ => panic!("Unable to get register r16 {}", register_id),
        }
    }
    pub(crate) fn set_r16(&mut self, register_id: u8, new_val: u16) {
        match register_id {
            0 => self.set_bc(new_val),
            1 => self.set_de(new_val),
            2 => self.set_hl(new_val),
            3 => self.stack_pointer = new_val,
            _ => panic!("Unable to get register r16 {}", register_id),
        }
    }
    pub(crate) fn get_r16mem(&mut self, register_id: u8) -> u16 {
        match register_id {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => {
                let ret_val = self.get_hl();
                self.set_hl(ret_val.wrapping_add(1));
                ret_val
            }
            3 => {
                let ret_val = self.get_hl();
                self.set_hl(ret_val.wrapping_sub(1));
                ret_val
            }
            _ => panic!("Unknown register_id"),
        }
    }
    pub(crate) fn get_r16stk(&self, register_id: u8) -> u16 {
        match register_id {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.get_af(),
            _ => panic!("Unknown register_id"),
        }
    }
    pub(crate) fn set_r16stk(&mut self, register_id: u8, new_value: u16) {
        match register_id {
            0 => self.set_bc(new_value),
            1 => self.set_de(new_value),
            2 => self.set_hl(new_value),
            3 => self.set_af(new_value),
            _ => panic!("Unknown register_id"),
        }
    }
    pub(crate) fn internal_get_r8(&self, register_id: u8) -> u8 {
        match register_id {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => unimplemented!("indirect read to [hl] thru r8 instruction"), //TODO: Figure this shit out
            7 => self.a,
            _ => panic!("Unable to get r8 of value {}", register_id),
        }
    }
    pub(crate) fn internal_set_r8(&mut self, register_id: u8, new_value: u8) {
        match register_id {
            0 => self.b = new_value,
            1 => self.c = new_value,
            2 => self.d = new_value,
            3 => self.e = new_value,
            4 => self.h = new_value,
            5 => self.l = new_value,
            6 => unimplemented!("indirect write to [hl] thru r8 instruction"), //TODO: Figure this shit out
            7 => self.a = new_value,
            _ => panic!(
                "Unable to set r8 register {} with value {}",
                register_id, new_value
            ),
        }
    }
}
