extern crate pretty_env_logger;
use log::{debug, info, log, warn};
use std::fs;
struct GbRegisters {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    h: u8,
    l: u8,
    f: GbFlagsRegister,
    stack_pointer: u16,
    program_counter: u16,
}
impl GbRegisters {
    fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | self.f.get_as_f_register() as u16
    }
    fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | self.c as u16
    }
    fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | self.e as u16
    }
    fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | self.l as u16
    }
    fn set_af(&mut self, new_val: u16) {
        self.a = ((new_val & 0xFF00) > 8) as u8;
        self.f.set_as_f_register((new_val & 0x00FF) as u8);
    }
    fn set_bc(&mut self, new_val: u16) {
        self.b = ((new_val & 0xFF00) > 8) as u8;
        self.c = (new_val & 0x00FF) as u8;
    }
    fn set_de(&mut self, new_val: u16) {
        self.d = ((new_val & 0xFF00) > 8) as u8;
        self.e = (new_val & 0x00FF) as u8;
    }
    fn set_hl(&mut self, new_val: u16) {
        self.h = ((new_val & 0xFF00) > 8) as u8;
        self.l = (new_val & 0x00FF) as u8;
    }
    fn get_r16(&self, register_id: u8) -> u16 {
        match register_id {
            0 => self.get_bc(),
            1 => self.get_de(),
            2 => self.get_hl(),
            3 => self.stack_pointer,
            _ => panic!("Unable to get register r16 {}", register_id),
        }
    }
    fn set_r16(&mut self, register_id: u8, new_val: u16) {
        match register_id {
            0 => self.set_bc(new_val),
            1 => self.set_de(new_val),
            2 => self.set_hl(new_val),
            3 => self.stack_pointer = new_val,
            _ => panic!("Unable to get register r16 {}", register_id),
        }
    }
    fn get_r8(&self, register_id: u8) -> u8 {
        match register_id {
            0 => self.b,
            1 => self.c,
            2 => self.d,
            3 => self.e,
            4 => self.h,
            5 => self.l,
            6 => unimplemented!(
                "I don't really understnad why [hl] is listed here... wouldn't that be a u16?"
            ), //TODO: Figure this shit out
            7 => self.a,
            _ => panic!("Unable to get r8 of value {}", register_id),
        }
    }
    fn set_r8(&mut self, register_id: u8, new_value: u8) {
        match register_id {
            0 => self.b = new_value,
            1 => self.c = new_value,
            2 => self.d = new_value,
            3 => self.e = new_value,
            4 => self.h = new_value,
            5 => self.l = new_value,
            6 => unimplemented!(
                "I don't really understnad why [hl] is listed here... wouldn't that be a u16?"
            ), //TODO: Figure this shit out
            7 => self.a = new_value,
            _ => panic!(
                "Unable to set r8 register {} with value {}",
                register_id, new_value
            ),
        }
    }
}
struct GbFlagsRegister {
    z: bool, // Zero flag
    n: bool, // Subtraction flag (BCD)
    h: bool, // Half Carry Flag (BCD)
    c: bool, // Carry Flag
}
impl GbFlagsRegister {
    fn get_as_f_register(&self) -> u8 {
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
        ret_val
    }
    fn set_as_f_register(&mut self, new_val: u8) {
        self.z = (new_val & 0b1000) > 0;
        self.n = (new_val & 0b0100) > 0;
        self.h = (new_val & 0b0010) > 0;
        self.c = (new_val & 0b0001) > 0;
    }
}
struct GbMemory {
    memory_array: [u8; 0xFFFF],
}
impl GbMemory {
    fn read_byte(&self, address: u16) -> u8 {
        self.memory_array[address as usize]
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        self.memory_array[address as usize] = value;
    }
}

struct Gb {
    registers: GbRegisters,
    gb_memory: GbMemory,
}
impl Gb {
    fn read_next_byte_and_advance_program_counter(&mut self) -> u8 {
        self.registers.program_counter += 1;
        self.gb_memory.read_byte(self.registers.program_counter - 1)
    }
    fn read_word_and_advance_program_counter(&mut self) -> u16 {
        let b1 = self.read_next_byte_and_advance_program_counter() as u16;
        let b2 = self.read_next_byte_and_advance_program_counter() as u16;
        (b2 << 8) | b1
    }
    fn read_hl_indirection_offset(&self, offset: u16) -> u8 {
        let hl_location = self.registers.get_hl();
        self.gb_memory.read_byte(hl_location + offset)
    }
    fn read_hl_indirection(&self) -> u8 {
        self.read_hl_indirection_offset(0)
    }
    fn set_hl_indirection_offset(&mut self, offset: u16, new_value: u8) {
        let hl_location = self.registers.get_hl() + offset;
        self.gb_memory.write_byte(hl_location, new_value);
    }
    fn set_hl_indirection(&mut self, new_value: u8) {
        self.set_hl_indirection_offset(0, new_value);
    }
}

fn main() {
    pretty_env_logger::init();
    let mut gb = Gb {
        registers: GbRegisters {
            a: 0x0,
            b: 0x0,
            c: 0x0,
            d: 0x0,
            e: 0x0,
            h: 0x0,
            l: 0x0,
            f: GbFlagsRegister {
                z: false,
                n: false,
                h: false,
                c: false,
            },
            stack_pointer: 0u16,
            program_counter: 0u16,
        },
        gb_memory: GbMemory {
            memory_array: [0u8; 0x0FFFF],
        },
    };
    read_rom(&mut gb.gb_memory);
    gb.registers.program_counter = 0x100;
    let mut i = 0;
    loop {
        let read_program_counter = gb.registers.program_counter;
        let query_byte = gb.read_next_byte_and_advance_program_counter();
        debug!("===");
        debug!("0x{:04x}: 0x{:02x}", read_program_counter, query_byte);
        // debug!("Query byte: {:#04x}", query_byte);
        match query_byte >> 6 {
            0x00 => {
                // debug!("Opcode group 0");
                //NOP
                if query_byte == 0 {
                    debug!("NOP");
                    continue;
                }
                //LD r16, imm16
                if query_byte & 0b1111 == 0b0001 {
                    debug!("LD r16, imm16");
                    let write_word = gb.read_word_and_advance_program_counter();
                    let register = (query_byte | 0b00110000) >> 4;
                    gb.registers.set_r16(register, write_word);
                    continue;
                }
                //LD r16mem, a
                if query_byte & 0b1111 == 0b0010 {
                    debug!("LD [r16mem], a");
                    let write_location = gb.registers.get_r16((query_byte & 0b00110000) >> 4);
                    let write_byte = gb.registers.a;
                    gb.gb_memory.write_byte(write_location, write_byte);
                    continue;
                }
                //LD a, r16mem
                if query_byte & 0b1111 == 0b1010 {
                    debug!("LD a, r16mem");
                    let read_location = gb.registers.get_r16((query_byte & 0b00110000) >> 4);
                    let write_byte = gb.gb_memory.read_byte(read_location);
                    gb.registers.a = write_byte;
                    continue;
                }
                //LD [imm16], sp
                if query_byte == 0b00001000 {
                    debug!("LD [imm16], sp");
                    let write_location = gb.read_word_and_advance_program_counter();
                    let write_byte = gb.registers.a;
                    gb.gb_memory.write_byte(write_location, write_byte);
                    continue;
                }
                //inc r16
                if (query_byte & 0b1111) == 0b0011 {
                    debug!("inc r16");
                    //Apparently this doesn't set any flags... :shrug:
                    let register_index = (0b00110000 & query_byte) >> 4;
                    let new_val = gb.registers.get_r16(register_index) + 1;
                    gb.registers.set_r16(register_index, new_val);
                    continue;
                }
                //dec r16
                if (query_byte & 0b1111) == 0b1011 {
                    debug!("dec r16");
                    //Apparently this doesn't set any flags... :shrug:
                    let register_index = (0b00110000 & query_byte) >> 4;
                    let new_val = gb.registers.get_r16(register_index).saturating_sub(1);
                    gb.registers.set_r16(register_index, new_val);
                    continue;
                }
                //add hl, r16
                if (query_byte & 0b1111) == 0b1001 {
                    debug!("add hl, r16");
                    let old_hl = gb.registers.get_hl();
                    let r16_id = (query_byte & 0b00110000) >> 4;
                    let r16 = gb.registers.get_r16(r16_id);
                    let (new_value, overflow) = old_hl.overflowing_add(r16);
                    gb.registers.f.n = false;
                    gb.registers.f.c = overflow;
                    gb.registers.f.h = false; //TODO: Implement half-carry.  Too tired to implement
                    //right now, my redbull is failing me
                    gb.registers.set_hl(new_value);
                    continue;
                }
                //inc r8
                if (query_byte & 0b111) == 0b100 {
                    debug!("inc r8");
                    let r8_id = (query_byte & 0b00111000) >> 3;
                    let old_val = gb.registers.get_r8(r8_id);
                    let (new_val, _) = old_val.overflowing_add(1);
                    gb.registers.set_r8(r8_id, new_val);
                    gb.registers.f.n = false;
                    gb.registers.f.z = new_val == 0;
                    gb.registers.f.h = false; //TODO: Implement half-carry
                }
                //dec r8
                if (query_byte & 0b111) == 0b101 {
                    debug!("dec r8");
                    let r8_id = (query_byte & 0b00111000) >> 3;
                    let old_val = gb.registers.get_r8(r8_id);
                    let (new_val, _) = old_val.overflowing_sub(1);
                    gb.registers.set_r8(r8_id, new_val);
                    gb.registers.f.n = true;
                    gb.registers.f.z = new_val == 0;
                    gb.registers.f.h = false; //TODO: Implement half-carry
                }
            }
            0x01 => {
                unimplemented!("Opcode group 1 not implemented");
            }
            0x11 => {
                unimplemented!("Opcode group 2 not implemented");
            }
            _ => {}
        }
        i += 1;
        if i > 10 {
            break;
        }
    }
}
fn read_rom(gb_memory: &mut GbMemory) {
    let mut contents = fs::read("tetris.gb").expect("Unable to read test rom.");
    // println!("{:#?}", contents);
    let cart_title = std::str::from_utf8(&contents[0x134..0x143])
        .expect("Improperly formatted ROM Header (Title)");
    info!("Game Title: {}", cart_title);
    let cart_type = contents[0x147];
    info!("Cartridge Type: 0x{:02x}", cart_type);
    let cart_rom_size_type = contents[0x148];
    info!("ROM Size Type: 0x{:02x}", cart_rom_size_type);
    let cart_ram_size_type = contents[0x149];
    info!("RAM Size Type: 0x{:02x}", cart_ram_size_type);
    let cart_destination_code = contents[0x014A];
    info!(
        "Cartridge Destination: {}",
        if cart_destination_code > 0 {
            "Overseas Only"
        } else {
            "Japan (or possibly overseas)"
        }
    );
    contents.resize(0xFFFF, 0u8);
    gb_memory.memory_array.copy_from_slice(&contents);
}
