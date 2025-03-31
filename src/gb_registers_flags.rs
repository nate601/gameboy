pub(crate) struct GbFlagsRegister {
    pub(crate) z: bool, // Zero flag
    pub(crate) n: bool, // Subtraction flag (BCD)
    pub(crate) h: bool, // Half Carry Flag (BCD)
    pub(crate) c: bool, // Carry Flag
}

impl GbFlagsRegister {
    pub(crate) fn get_as_f_register(&self) -> u8 {
        let mut ret_val = 0b0000u8;
        if self.z {
            ret_val |= 0b1000u8
        }
        if self.n {
            ret_val |= 0b0100u8
        }
        if self.h {
            ret_val |= 0b0010u8
        }
        if self.c {
            ret_val |= 0b0001u8
        }
        ret_val << 4
    }
    pub(crate) fn set_as_f_register(&mut self, new_val: u8) {
        let new_f = new_val >> 4;
        self.z = (new_f & 0b1000) > 0;
        self.n = (new_f & 0b0100) > 0;
        self.h = (new_f & 0b0010) > 0;
        self.c = (new_f & 0b0001) > 0;
    }
    pub(crate) fn check_condition(&self, condition_id: u8) -> bool {
        match condition_id {
            0 => !self.z,
            1 => self.z,
            2 => !self.c,
            3 => self.c,
            _ => panic!("Unknown conditionID"),
        }
    }
}
