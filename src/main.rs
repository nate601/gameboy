extern crate pretty_env_logger;
use log::{debug, error, info};
use std::fs;
mod gameboy;
mod gb_memory;
mod gb_registers;
mod gb_registers_flags;

const PANIC_ON_UNDEFINED_OPCODE: bool = true;

fn main() {
    pretty_env_logger::init();
    let mut gb = gameboy::Gb {
        registers: gb_registers::GbRegisters {
            a: 0x0,
            b: 0x0,
            c: 0x0,
            d: 0x0,
            e: 0x0,
            h: 0x0,
            l: 0x0,
            f: gb_registers_flags::GbFlagsRegister {
                z: false,
                n: false,
                h: false,
                c: false,
            },
            stack_pointer: 0xFFFE, //stack_pointer starts at 0xfffe per docs!
            program_counter: 0u16,
        },
        gb_memory: gb_memory::GbMemory {
            memory_array: [0u8; 0x0FFFF],
        },
    };
    read_rom(&mut gb.gb_memory);
    gb.registers.program_counter = 0x100;
    let mut i = 0;
    loop {
        let read_program_counter = gb.registers.program_counter;
        let query_byte = gb.read_byte_and_advance_program_counter();
        debug!("===");
        debug!("0x{:04x}: 0x{:02x}", read_program_counter, query_byte);
        // debug!("Query byte: {:#04x}", query_byte);
        match query_byte >> 6 {
            0b00 => {
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
                    let register_id = (query_byte & 0b00110000) >> 4;
                    let write_location = gb.registers.get_r16mem(register_id);
                    let write_byte = gb.registers.a;
                    gb.gb_memory.write_byte(write_location, write_byte);
                    continue;
                }
                //LD a, r16mem
                if query_byte & 0b1111 == 0b1010 {
                    debug!("LD a, r16mem");
                    let register_id = (query_byte & 0b00110000) >> 4;
                    let read_location = gb.registers.get_r16mem(register_id);
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
                    continue;
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
                    continue;
                }
                if (query_byte & 0b1100111) == 0b0000110 {
                    debug!("ld r8, imm8");
                    let write_byte = gb.read_byte_and_advance_program_counter();
                    let r8_id = (query_byte & 0b0011000) >> 3;
                    gb.registers.set_r8(r8_id, write_byte);
                    continue;
                }
                //jr imm8
                if query_byte == 0b00011000 {
                    debug!("jr imm8");
                    let offset: i16 = gb.read_byte_signed_and_advance_program_counter().into();
                    // let current_pc: i32 = gb.registers.program_counter.into();
                    // let new_pc = current_pc().checked_add(offset);
                    let current_pc = gb.registers.program_counter;
                    let new_pc = current_pc.wrapping_add_signed(offset);
                    gb.registers.program_counter = new_pc;
                    continue;
                }
                //jr cond, imm8
                //Has to be checked after checking for JR imm8 because that is just a special
                //conditonID.  Maybe I should just implement it as a special conditionID if other
                //condition uses are the same going forward... Pending
                if (query_byte & 0b11100111) == 0b00100000 {
                    debug!("jr cond, imm8");
                    let offset: i16 = gb.read_byte_signed_and_advance_program_counter().into();
                    let current_pc = gb.registers.program_counter;
                    let condition_id = (query_byte & 0b00011000) >> 3;
                    if gb.registers.f.check_condition(condition_id) {
                        let new_pc = current_pc.wrapping_add_signed(offset);
                        gb.registers.program_counter = new_pc
                    }
                    continue;
                }
                //stop
                //TODO: implement CPU mode switching if I later decide to support gbc games
                if query_byte == 0b00010000 {
                    let _ = gb.read_byte_and_advance_program_counter();
                    continue;
                }
                match query_byte {
                    0b111 => {
                        debug!("rlca");
                        gb.registers.f.set_as_f_register(0);
                        let reg_a = gb.registers.a;
                        let new_val = reg_a.rotate_left(1);
                        gb.registers.f.c = (0b10000000 & reg_a) > 0;
                        gb.registers.a = new_val;
                        continue;
                    }
                    0b1111 => {
                        debug!("rrca");
                        gb.registers.f.set_as_f_register(0);
                        let reg_a = gb.registers.a;
                        let new_val = reg_a.rotate_right(1);
                        gb.registers.f.c = (0b1 & reg_a) > 0;
                        gb.registers.a = new_val;
                        continue;
                    }
                    0b10111 => {
                        debug!("rla");
                        let old_c = gb.registers.f.c;
                        let reg_a = gb.registers.a;
                        gb.registers.f.set_as_f_register(0);
                        let (mut new_a, overflow) = reg_a.overflowing_shl(1);
                        new_a |= if old_c { 0b1 } else { 0b0 };
                        gb.registers.a = new_a;
                        gb.registers.f.c = overflow;
                        continue;
                    }
                    0b11111 => {
                        debug!("rra");
                        let old_c = gb.registers.f.c;
                        let reg_a = gb.registers.a;
                        gb.registers.f.set_as_f_register(0);
                        let (mut new_a, overflow) = reg_a.overflowing_shr(1);
                        new_a |= if old_c { 0b10000000 } else { 0b0 };
                        gb.registers.a = new_a;
                        gb.registers.f.c = overflow;
                        continue;
                    }
                    0b100111 => {
                        debug!("daa");
                        unimplemented!("DAA not implemented presently...")
                    }
                    0b101111 => {
                        debug!("cpl");
                        let old_a = gb.registers.a;
                        gb.registers.f.n = true;
                        gb.registers.f.h = true;
                        gb.registers.a = !old_a;
                        continue;
                    }
                    0b110111 => {
                        debug!("scf");
                        gb.registers.f.n = false;
                        gb.registers.f.h = false;
                        gb.registers.f.c = true;
                        continue;
                    }
                    0b111111 => {
                        debug!("ccf");
                        let old_carry = gb.registers.f.c;
                        gb.registers.f.c = !old_carry;
                        continue;
                    }
                    _ => (),
                }
            }
            0b01 => {
                //halt
                if query_byte == 0b01110110 {
                    debug!("halt");
                    panic!("Halt called");
                }
                //ld r8, r8
                debug!("ld r8, r8");
                let dest_r8_id = (query_byte & 0b00111000) >> 3;
                let src_r8_id = query_byte & 0b111;
                let write_byte = gb.registers.get_r8(src_r8_id);
                gb.registers.set_r8(dest_r8_id, write_byte);
                continue;
            }
            0b10 => {
                let operand_id = query_byte & 0b111;
                let original_operand_value = gb.registers.get_r8(operand_id);
                let group_2_id = query_byte >> 3;
                let original_a = gb.registers.a;
                match group_2_id {
                    0b10000 => {
                        debug!("add a, r8");
                        let (new_value, overflow) =
                            original_operand_value.overflowing_add(original_a);
                        gb.registers.f.n = false;
                        gb.registers.f.z = new_value == 0;
                        gb.registers.f.c = overflow;
                        gb.registers.f.h = false; //TODO: implement half carry
                        gb.registers.a = new_value;
                        continue;
                    }
                    0b10001 => {
                        debug!("adc a, r8");
                        let carry_addition = if gb.registers.f.c { 0b1 } else { 0b0 };
                        let (new_value, overflow) =
                            original_operand_value.overflowing_add(original_a + carry_addition);
                        gb.registers.f.n = false;
                        gb.registers.f.z = new_value == 0;
                        gb.registers.f.h = false; //TODO: implement half carry
                        gb.registers.f.c = overflow;
                        gb.registers.a = new_value;
                        continue;
                    }
                    0b10010 => {
                        debug!("sub a, r8");
                        let (new_value, overflow) =
                            original_a.overflowing_sub(original_operand_value);
                        gb.registers.f.n = true;
                        gb.registers.f.z = new_value == 0;
                        gb.registers.f.h = false; //TODO: implement half carry
                        gb.registers.f.c = overflow;
                        gb.registers.a = new_value;
                        continue;
                    }
                    0b10011 => {
                        debug!("sbc a, r8");
                        let (new_value, overflow) =
                            original_a.overflowing_sub(original_operand_value);
                        gb.registers.f.n = true;
                        gb.registers.f.z = new_value == 0;
                        gb.registers.f.h = false; //TODO: implement half carry
                        gb.registers.f.c = overflow;
                        gb.registers.a = new_value;
                        continue;
                    }
                    0b10100 => {
                        debug!("and a, r8");
                        let new_value = original_a & original_operand_value;
                        gb.registers.a = new_value;
                        gb.registers.f.n = false;
                        gb.registers.f.h = true;
                        gb.registers.f.c = false;
                        gb.registers.f.z = new_value == 0;
                        continue;
                    }
                    0b10101 => {
                        debug!("xor a, r8");
                        let new_value = original_a ^ original_operand_value;
                        gb.registers.a = new_value;
                        gb.registers.f.n = false;
                        gb.registers.f.h = false;
                        gb.registers.f.c = false;
                        gb.registers.f.z = new_value == 0;
                        continue;
                    }
                    0b10110 => {
                        debug!("or a, r8");
                        let new_value = original_a | original_operand_value;
                        gb.registers.a = new_value;
                        gb.registers.f.n = false;
                        gb.registers.f.h = false;
                        gb.registers.f.c = false;
                        gb.registers.f.z = new_value == 0;
                        continue;
                    }
                    0b10111 => {
                        debug!("cp a, r8");
                        let (result, _) = original_a.overflowing_sub(original_operand_value);
                        gb.registers.f.n = true;
                        gb.registers.f.z = result == 0;
                        gb.registers.f.h = false; //TODO: Guess what! Still need to implement h/c
                        gb.registers.f.c = original_operand_value > original_a;
                        continue;
                    }
                    _ => (),
                }
            }
            0b11 => {
                // unimplemented!("Opcode group 3 not implemented");
                debug!("opcode group 3");
                match query_byte {
                    0b11000110 => {
                        debug!("add a, imm8");
                        let old_a = gb.registers.a;
                        let next_byte = gb.read_byte_and_advance_program_counter();
                        let (result, overflow) = old_a.overflowing_add(next_byte);
                        gb.registers.f.z = result == 0;
                        gb.registers.f.n = false;
                        gb.registers.f.h = false; //TODO: implement h/c
                        gb.registers.f.c = overflow;
                        gb.registers.a = result;
                        continue;
                    }
                    0b11001110 => {
                        debug!("adc a, imm8");
                        let old_a = gb.registers.a;
                        let next_byte = gb.read_byte_and_advance_program_counter();
                        let carry_addition = if gb.registers.f.c { 0b1 } else { 0b0 };
                        let (result, overflow) = old_a.overflowing_add(next_byte + carry_addition);
                        gb.registers.f.z = result == 0;
                        gb.registers.f.n = false;
                        gb.registers.f.h = false; //TODO: implement h/c
                        gb.registers.f.c = overflow;
                        gb.registers.a = result;
                        continue;
                    }
                    0b11010110 => {
                        debug!("sub a, imm8");
                        let old_a = gb.registers.a;
                        let next_byte = gb.read_byte_and_advance_program_counter();
                        let (result, overflow) = old_a.overflowing_sub(next_byte);
                        gb.registers.f.z = result == 0;
                        gb.registers.f.n = true;
                        gb.registers.f.h = false; //TODO: implement h/c
                        gb.registers.f.c = overflow;
                        gb.registers.a = result;
                        continue;
                    }
                    0b11011110 => {
                        debug!("sbc a, imm8");
                        let old_a = gb.registers.a;
                        let next_byte = gb.read_byte_and_advance_program_counter();
                        let carry_sub = if gb.registers.f.c { 0b1 } else { 0b0 };
                        let (result, overflow) = old_a.overflowing_sub(next_byte + carry_sub);
                        gb.registers.f.z = result == 0;
                        gb.registers.f.n = true;
                        gb.registers.f.h = false; //TODO: implement h/c
                        gb.registers.f.c = overflow;
                        gb.registers.a = result;
                        continue;
                    }
                    0b11100110 => {
                        debug!("and a, imm8");
                        let old_a = gb.registers.a;
                        let next_byte = gb.read_byte_and_advance_program_counter();
                        let result = old_a & next_byte;
                        gb.registers.a = result;
                        gb.registers.f.z = result == 0;
                        gb.registers.f.n = false;
                        gb.registers.f.h = true;
                        gb.registers.f.c = false;
                        continue;
                    }
                    0b11101110 => {
                        debug!("xor a, imm8");
                        let old_a = gb.registers.a;
                        let next_byte = gb.read_byte_and_advance_program_counter();
                        let result = old_a ^ next_byte;
                        gb.registers.a = result;
                        gb.registers.f.z = result == 0;
                        gb.registers.f.n = false;
                        gb.registers.f.h = false;
                        gb.registers.f.c = false;
                        continue;
                    }
                    0b11110110 => {
                        debug!("or a, imm8");
                        let old_a = gb.registers.a;
                        let next_byte = gb.read_byte_and_advance_program_counter();
                        let result = old_a | next_byte;
                        gb.registers.a = result;
                        gb.registers.f.z = result == 0;
                        gb.registers.f.n = false;
                        gb.registers.f.h = false;
                        gb.registers.f.c = false;
                        continue;
                    }
                    0b11111110 => {
                        debug!("cp a, imm8");
                        let old_a = gb.registers.a;
                        let next_byte = gb.read_byte_and_advance_program_counter();
                        let (result, overflow) = old_a.overflowing_sub(next_byte);
                        gb.registers.f.z = result == 0;
                        gb.registers.f.n = true;
                        gb.registers.f.h = false; //TODO: implement h/c
                        gb.registers.f.c = overflow;
                        continue;
                    }
                    0b11001001 => {
                        debug!("ret");
                        let new_pc = gb.pop_stack_word();
                        gb.registers.program_counter = new_pc;
                        continue;
                    }
                    0b11000011 => {
                        debug!("jp imm16");
                        let new_pc = gb.read_word_and_advance_program_counter();
                        gb.registers.program_counter = new_pc;
                        continue;
                    }
                    0b11101001 => {
                        debug!("jp hl");
                        let new_pc = gb.registers.get_hl();
                        gb.registers.program_counter = new_pc;
                        continue;
                    }
                    0b11001101 => {
                        debug!("call imm16");
                        let return_pc = gb.registers.program_counter; // I believe that it will be
                        // already increased by 1 at this point, so this is the value we need to
                        // add to the stack
                        gb.push_stack_word(return_pc);
                        let new_pc = gb.read_word_and_advance_program_counter();
                        gb.registers.program_counter = new_pc;
                        continue;
                    }
                    0b11100010 => {
                        debug!("ldh [$FF00 + c], a");
                        let write_byte = gb.registers.a;
                        let write_byte_location = gb.registers.c as u16 + 0xFF00;
                        gb.gb_memory.write_byte(write_byte_location, write_byte);
                        continue;
                    }
                    _ => (),
                }
            }
            _ => {
                error!("This opcode starts with some WIIIILD SHIT!")
            }
        }
        error!("Previous opcode is undefined!");
        if PANIC_ON_UNDEFINED_OPCODE {
            panic!("Undefined opcode!");
        } else {
            i += 1;
            if i > 10 {
                break;
            }
        }
    }
}
fn read_rom(gb_memory: &mut gb_memory::GbMemory) {
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
