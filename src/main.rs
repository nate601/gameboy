extern crate pretty_env_logger;
use gb_memory::InterruptFlags;
use log::{debug, error, info};
use sdl2::{event::Event, keyboard::Keycode, pixels::Color, render::WindowCanvas};
use std::{fs, ops::ControlFlow, time::Duration};
mod gameboy;
mod gb_memory;
mod gb_registers;
mod gb_registers_flags;
mod renderer;

const PANIC_ON_UNDEFINED_OPCODE: bool = true;

const OPS_PER_SEC: u32 = 4_194_304;
const NS_PER_SEC: u32 = 1_000_000_000;
const NS_PER_OP: u32 = NS_PER_SEC / OPS_PER_SEC;

const DIV_PER_SEC: u32 = 16_384;
const NS_PER_DIV: u32 = NS_PER_SEC / DIV_PER_SEC;

fn calculate_byte_half_carry_add(a: u8, b: u8) -> bool {
    //if we add the low bytes together, would it result in a result
    //bigger than a nibble?
    //if yes, then it's a half carry
    (a & 0x0F) + (b & 0x0F) > 0x0F
}
fn calculate_word_half_carry_add(a: u16, b: u16) -> bool {
    (a & 0xFF) + (b & 0xFF) > 0xFF
}
fn calculate_byte_half_carry_sub(a: u8, b: u8) -> bool {
    unimplemented!("uhh")
    // uhh
}

fn main() {
    //pre-init
    pretty_env_logger::init();
    let mut renderer = renderer::Renderer::renderer_init().expect("Unable to initizlize renderer");

    //init
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
            memory_array: [0u8; 0x0FFFF + 1],
        },
        interrupt_master_flag: false,
        renderer,
    };
    read_rom(&mut gb.gb_memory);
    gb.registers.program_counter = 0x100;

    let mut elapsed_time = 0u32;
    let mut render_counter = 0u32;
    let mut div_timer = 0u32;
    let mut tima_timer = 0u32;
    //Main loop
    'mainloop: loop {
        //interrupt checking
        if gb.interrupt_master_flag {
            let i_e = gb.gb_memory.read_interrupt_enable();
            let i_f = gb.gb_memory.read_interrupt_flags();
            let interupts = InterruptFlags::get_flags_from_byte(
                i_e.get_byte_from_flag() | i_f.get_byte_from_flag(),
            );
            let interrupt_call_location = match interupts {
                InterruptFlags { v_blank: true, .. } => {
                    let mut new_if = i_f;
                    new_if.v_blank = false;
                    gb.gb_memory.set_interrupt_flags(new_if);
                    0x40u16
                }
                InterruptFlags { lcd: true, .. } => {
                    let mut new_if = i_f;
                    new_if.lcd = false;
                    gb.gb_memory.set_interrupt_flags(new_if);
                    0x48u16
                }
                InterruptFlags { timer: true, .. } => {
                    let mut new_if = i_f;
                    new_if.timer = false;
                    gb.gb_memory.set_interrupt_flags(new_if);
                    0x50u16
                }
                InterruptFlags { serial: true, .. } => {
                    let mut new_if = i_f;
                    new_if.serial = false;
                    gb.gb_memory.set_interrupt_flags(new_if);
                    0x58u16
                }
                InterruptFlags { joypad: true, .. } => {
                    let mut new_if = i_f;
                    new_if.joypad = false;
                    gb.gb_memory.set_interrupt_flags(new_if);
                    0x60u16
                }
                _ => 0x0u16,
            };
            if interrupt_call_location != 0 {
                info!(
                    "Interrupt called! Sending you to 0x{:2x}",
                    interrupt_call_location
                );
                gb.interrupt_master_flag = false;
                gb.push_stack_word(gb.registers.program_counter);
                gb.registers.program_counter = interrupt_call_location;
            }
        }

        //input parsing

        'input_parsing: for event in gb.renderer.event_pump.poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => break 'mainloop,
                _ => (),
            }
        }

        //opcode parsing
        let read_program_counter = gb.registers.program_counter;
        let query_byte = gb.read_byte_and_advance_program_counter();
        debug!("=== === ===");
        debug!("0x{:04x}: 0x{:02x}", read_program_counter, query_byte);
        if let ControlFlow::Break(_) = execute_op(&mut gb, query_byte) {
            //
        } else {
            error!("Previous opcode is undefined! 0x{:02x}", query_byte);
            if PANIC_ON_UNDEFINED_OPCODE {
                unimplemented!(
                    "Undefined opcode! 0x{:04x}: 0x{:02x} / 0b{:08b}",
                    read_program_counter,
                    query_byte,
                    query_byte
                );
            }
        }
        ::std::thread::sleep(Duration::new(0, NS_PER_OP)); // 4.194304 MHz
        elapsed_time += NS_PER_OP;
        render_counter += NS_PER_OP;
        div_timer += NS_PER_OP;
        tima_timer += NS_PER_OP;

        let tima_tps = gb.gb_memory.get_tima_ticks_per_second();
        if tima_tps != 0 {
            let ns_per_tick = NS_PER_SEC / tima_tps;
            if tima_timer >= ns_per_tick {
                gb.gb_memory.tick_tima();
                tima_timer = 0;
            }
        }

        if div_timer >= NS_PER_DIV {
            let times_to_tick = div_timer / NS_PER_DIV;
            for _ in 1..times_to_tick {
                gb.gb_memory.tick_div();
                div_timer = 0;
            }
        }

        //rendering
        if render_counter >= NS_PER_OP * 10 {
            gb.render();
            render_counter = 0;
        }
    }
}

fn execute_op(gb: &mut gameboy::Gb, query_byte: u8) -> ControlFlow<()> {
    match query_byte >> 6 {
        0b00 => {
            // debug!("Opcode group 0");
            //NOP
            if query_byte == 0 {
                debug!("NOP");
                return ControlFlow::Break(());
            }
            //LD r16, imm16
            if query_byte & 0b1111 == 0b0001 {
                debug!("LD r16, imm16");
                let write_word = gb.read_word_and_advance_program_counter();
                let register = (query_byte | 0b00110000) >> 4;
                gb.registers.set_r16(register, write_word);
                return ControlFlow::Break(());
            }
            //LD r16mem, a
            if query_byte & 0b1111 == 0b0010 {
                debug!("LD [r16mem], a");
                let register_id = (query_byte & 0b00110000) >> 4;
                let write_location = gb.registers.get_r16mem(register_id);
                let write_byte = gb.registers.a;
                gb.gb_memory.write_byte(write_location, write_byte);
                return ControlFlow::Break(());
            }
            //LD a, r16mem
            if query_byte & 0b1111 == 0b1010 {
                debug!("LD a, r16mem");
                let register_id = (query_byte & 0b00110000) >> 4;
                let read_location = gb.registers.get_r16mem(register_id);
                let write_byte = gb.gb_memory.read_byte(read_location);
                gb.registers.a = write_byte;
                return ControlFlow::Break(());
            }
            //LD [imm16], sp
            if query_byte == 0b00001000 {
                debug!("LD [imm16], sp");
                let write_location = gb.read_word_and_advance_program_counter();
                let write_byte = gb.registers.a;
                gb.gb_memory.write_byte(write_location, write_byte);
                return ControlFlow::Break(());
            }
            //inc r16
            if (query_byte & 0b1111) == 0b0011 {
                debug!("inc r16");
                //Apparently this doesn't set any flags... :shrug:
                let register_index = (0b00110000 & query_byte) >> 4;
                let new_val = gb.registers.get_r16(register_index) + 1;
                gb.registers.set_r16(register_index, new_val);
                return ControlFlow::Break(());
            }
            //dec r16
            if (query_byte & 0b1111) == 0b1011 {
                debug!("dec r16");
                //Apparently this doesn't set any flags... :shrug:
                let register_index = (0b00110000 & query_byte) >> 4;
                let new_val = gb.registers.get_r16(register_index).saturating_sub(1);
                gb.registers.set_r16(register_index, new_val);
                return ControlFlow::Break(());
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
                gb.registers.f.h = calculate_word_half_carry_add(old_hl, r16);
                gb.registers.set_hl(new_value);
                return ControlFlow::Break(());
            }
            //inc r8
            if (query_byte & 0b111) == 0b100 {
                debug!("inc r8");
                let r8_id = (query_byte & 0b00111000) >> 3;
                let old_val = gb.get_r8(r8_id);
                let (new_val, _) = old_val.overflowing_add(1);
                gb.set_r8(r8_id, new_val);
                gb.registers.f.n = false;
                gb.registers.f.z = new_val == 0;
                gb.registers.f.h = calculate_byte_half_carry_add(old_val, 1);
                return ControlFlow::Break(());
            }
            //dec r8
            if (query_byte & 0b111) == 0b101 {
                debug!("dec r8");
                let r8_id = (query_byte & 0b00111000) >> 3;
                let old_val = gb.get_r8(r8_id);
                let (new_val, _) = old_val.overflowing_sub(1);
                gb.set_r8(r8_id, new_val);
                gb.registers.f.n = true;
                gb.registers.f.z = new_val == 0;
                gb.registers.f.h = false; //TODO: Implement half-carry
                return ControlFlow::Break(());
            }
            if (query_byte & 0b11000111) == 0b00000110 {
                debug!("ld r8, imm8");
                let write_byte = gb.read_byte_and_advance_program_counter();
                let r8_id = (query_byte & 0b0011000) >> 3;
                gb.set_r8(r8_id, write_byte);
                return ControlFlow::Break(());
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
                return ControlFlow::Break(());
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
                return ControlFlow::Break(());
            }
            //stop
            //TODO: implement CPU mode switching if I later decide to support gbc games
            if query_byte == 0b00010000 {
                let _ = gb.read_byte_and_advance_program_counter(); // pull but is unused
                return ControlFlow::Break(());
            }
            match query_byte {
                0b111 => {
                    debug!("rlca");
                    gb.registers.f.set_as_f_register(0);
                    let reg_a = gb.registers.a;
                    let new_val = reg_a.rotate_left(1);
                    gb.registers.f.c = (0b10000000 & reg_a) > 0;
                    gb.registers.a = new_val;
                    return ControlFlow::Break(());
                }
                0b1111 => {
                    debug!("rrca");
                    gb.registers.f.set_as_f_register(0);
                    let reg_a = gb.registers.a;
                    let new_val = reg_a.rotate_right(1);
                    gb.registers.f.c = (0b1 & reg_a) > 0;
                    gb.registers.a = new_val;
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
                }
                0b110111 => {
                    debug!("scf");
                    gb.registers.f.n = false;
                    gb.registers.f.h = false;
                    gb.registers.f.c = true;
                    return ControlFlow::Break(());
                }
                0b111111 => {
                    debug!("ccf");
                    let old_carry = gb.registers.f.c;
                    gb.registers.f.c = !old_carry;
                    return ControlFlow::Break(());
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
            let write_byte = gb.get_r8(src_r8_id);
            gb.set_r8(dest_r8_id, write_byte);
            return ControlFlow::Break(());
        }
        0b10 => {
            let operand_id = query_byte & 0b111;
            let original_operand_value = gb.get_r8(operand_id);
            let group_2_id = query_byte >> 3;
            let original_a = gb.registers.a;
            match group_2_id {
                0b10000 => {
                    debug!("add a, r8");
                    let (new_value, overflow) = original_operand_value.overflowing_add(original_a);
                    gb.registers.f.n = false;
                    gb.registers.f.z = new_value == 0;
                    gb.registers.f.c = overflow;
                    gb.registers.f.h =
                        calculate_byte_half_carry_add(original_operand_value, original_a);
                    gb.registers.a = new_value;
                    return ControlFlow::Break(());
                }
                0b10001 => {
                    debug!("adc a, r8");
                    let carry_addition = if gb.registers.f.c { 0b1 } else { 0b0 };
                    let (new_value, overflow) =
                        original_operand_value.overflowing_add(original_a + carry_addition);
                    gb.registers.f.n = false;
                    gb.registers.f.z = new_value == 0;
                    gb.registers.f.h = calculate_byte_half_carry_add(
                        original_operand_value,
                        original_a + carry_addition,
                    );
                    gb.registers.f.c = overflow;
                    gb.registers.a = new_value;
                    return ControlFlow::Break(());
                }
                0b10010 => {
                    debug!("sub a, r8");
                    let (new_value, overflow) = original_a.overflowing_sub(original_operand_value);
                    gb.registers.f.n = true;
                    gb.registers.f.z = new_value == 0;
                    gb.registers.f.h = false; //TODO: implement half carry
                    gb.registers.f.c = overflow;
                    gb.registers.a = new_value;
                    return ControlFlow::Break(());
                }
                0b10011 => {
                    debug!("sbc a, r8");
                    let (new_value, overflow) = original_a.overflowing_sub(original_operand_value);
                    gb.registers.f.n = true;
                    gb.registers.f.z = new_value == 0;
                    gb.registers.f.h = false; //TODO: implement half carry
                    gb.registers.f.c = overflow;
                    gb.registers.a = new_value;
                    return ControlFlow::Break(());
                }
                0b10100 => {
                    debug!("and a, r8");
                    let new_value = original_a & original_operand_value;
                    gb.registers.a = new_value;
                    gb.registers.f.n = false;
                    gb.registers.f.h = true;
                    gb.registers.f.c = false;
                    gb.registers.f.z = new_value == 0;
                    return ControlFlow::Break(());
                }
                0b10101 => {
                    debug!("xor a, r8");
                    let new_value = original_a ^ original_operand_value;
                    gb.registers.a = new_value;
                    gb.registers.f.n = false;
                    gb.registers.f.h = false;
                    gb.registers.f.c = false;
                    gb.registers.f.z = new_value == 0;
                    return ControlFlow::Break(());
                }
                0b10110 => {
                    debug!("or a, r8");
                    let new_value = original_a | original_operand_value;
                    gb.registers.a = new_value;
                    gb.registers.f.n = false;
                    gb.registers.f.h = false;
                    gb.registers.f.c = false;
                    gb.registers.f.z = new_value == 0;
                    return ControlFlow::Break(());
                }
                0b10111 => {
                    debug!("cp a, r8");
                    let (result, _) = original_a.overflowing_sub(original_operand_value);
                    gb.registers.f.n = true;
                    gb.registers.f.z = result == 0;
                    gb.registers.f.h = false; //TODO: Guess what! Still need to implement h/c
                    gb.registers.f.c = original_operand_value > original_a;
                    return ControlFlow::Break(());
                }
                _ => (),
            }
        }
        0b11 => {
            // unimplemented!("Opcode group 3 not implemented");
            // debug!("opcode group 3");

            if (query_byte & 0b11100111) == 0b11000000 {
                debug!("ret cond");
                let cond_id = (query_byte & 0b00011000) >> 3;
                let cond_state = gb.registers.f.check_condition(cond_id);
                if cond_state {
                    let new_pc = gb.pop_stack_word();
                    gb.registers.program_counter = new_pc;
                }
                return ControlFlow::Break(());
            }
            if (query_byte & 0b11100111) == 0b11000010 {
                debug!("jp cond, imm16");
                let jp_location = gb.read_word_and_advance_program_counter();
                let cond_id = (query_byte & 0b00011000) >> 3;
                let cond_state = gb.registers.f.check_condition(cond_id);
                if cond_state {
                    gb.registers.program_counter = jp_location;
                }
                return ControlFlow::Break(());
            }
            if (query_byte & 0b11100111) == 0b11000100 {
                debug!("call cond, imm16");
                let call_location = gb.read_word_and_advance_program_counter();
                let cond_id = (query_byte & 0b00011000) >> 3;
                let cond_state = gb.registers.f.check_condition(cond_id);
                let return_pc = gb.registers.program_counter;
                if cond_state {
                    gb.push_stack_word(return_pc);
                    gb.registers.program_counter = call_location;
                }
                return ControlFlow::Break(());
            }
            if (query_byte & 0b11000111) == 0b11000111 {
                debug!("rst vec");
                let tgt3 = (query_byte & 0b00111000) >> 3;
                let vec = (tgt3 * 8) as u16;
                let return_pc = gb.registers.program_counter;
                gb.push_stack_word(return_pc);
                gb.registers.program_counter = vec;
                return ControlFlow::Break(());
            }
            if (query_byte & 0b11001111) == 0b11000001 {
                debug!("pop r16stk");
                let r16stk_id = (query_byte & 0b00110000) >> 4;
                // let r16 = gb.registers.get_r16stk(r16stk_id);
                let read_word = gb.pop_stack_word();
                gb.registers.set_r16stk(r16stk_id, read_word);
                return ControlFlow::Break(());
            }
            if (query_byte & 0b11001111) == 0b11000101 {
                debug!("push r16stk");
                let r16stk_id = (query_byte & 0b00110000) >> 4;
                let r16 = gb.registers.get_r16stk(r16stk_id);
                gb.push_stack_word(r16);
                return ControlFlow::Break(());
            }

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
                    return ControlFlow::Break(());
                }
                0b11001110 => {
                    debug!("adc a, imm8");
                    let old_a = gb.registers.a;
                    let next_byte = gb.read_byte_and_advance_program_counter();
                    let carry_addition = if gb.registers.f.c { 0b1 } else { 0b0 };
                    let (result, overflow) = old_a.overflowing_add(next_byte + carry_addition);
                    gb.registers.f.z = result == 0;
                    gb.registers.f.n = false;
                    gb.registers.f.h =
                        calculate_byte_half_carry_add(old_a, next_byte + carry_addition);
                    gb.registers.f.c = overflow;
                    gb.registers.a = result;
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
                }
                0b11001001 => {
                    debug!("ret");
                    let new_pc = gb.pop_stack_word();
                    gb.registers.program_counter = new_pc;
                    return ControlFlow::Break(());
                }
                0b11011001 => {
                    debug!("reti");
                    gb.interrupt_master_flag = true;
                    let new_pc = gb.pop_stack_word();
                    gb.registers.program_counter = new_pc;
                    return ControlFlow::Break(());
                }
                0b11000011 => {
                    debug!("jp imm16");
                    let new_pc = gb.read_word_and_advance_program_counter();
                    gb.registers.program_counter = new_pc;
                    return ControlFlow::Break(());
                }
                0b11101001 => {
                    debug!("jp hl");
                    let new_pc = gb.registers.get_hl();
                    gb.registers.program_counter = new_pc;
                    return ControlFlow::Break(());
                }
                0b11001101 => {
                    debug!("call imm16");
                    let return_pc = gb.registers.program_counter; // I believe that it will be
                    // already increased by 1 at this point, so this is the value we need to
                    // add to the stack
                    gb.push_stack_word(return_pc);
                    let new_pc = gb.read_word_and_advance_program_counter();
                    gb.registers.program_counter = new_pc;
                    return ControlFlow::Break(());
                }
                0b11100010 => {
                    debug!("ldh [$FF00 + c], a");
                    let write_byte = gb.registers.a;
                    let write_byte_location = gb.registers.c as u16 + 0xFF00;
                    gb.gb_memory.write_byte(write_byte_location, write_byte);
                    return ControlFlow::Break(());
                }
                0b11100000 => {
                    debug!("ldh [imm8], a");
                    let write_byte = gb.registers.a;
                    let write_byte_location_low = gb.registers.c as u16;
                    let write_byte_location = 0xFF00u16 | write_byte_location_low;
                    gb.gb_memory.write_byte(write_byte_location, write_byte);
                    return ControlFlow::Break(());
                }
                0b11101010 => {
                    debug!("ld [imm16], a");
                    let write_byte = gb.registers.a;
                    let write_location = gb.read_word_and_advance_program_counter();
                    gb.gb_memory.write_byte(write_location, write_byte);
                    return ControlFlow::Break(());
                }
                0b11110010 => {
                    debug!("ldh a, [$FF00+c]");
                    let read_location_low = gb.registers.c as u16;
                    let read_location = 0xFF00u16 | read_location_low;
                    let read_byte = gb.gb_memory.read_byte(read_location);
                    gb.registers.a = read_byte;
                    return ControlFlow::Break(());
                }
                0b11110000 => {
                    debug!("ldh a, [imm8]");
                    let read_location_low = gb.read_byte_and_advance_program_counter() as u16;
                    let read_location = 0xFF00u16 | read_location_low;
                    let read_byte = gb.gb_memory.read_byte(read_location);
                    gb.registers.a = read_byte;
                    return ControlFlow::Break(());
                }
                0b11111010 => {
                    debug!("ld a, [imm16]");
                    let imm16 = gb.read_word_and_advance_program_counter();
                    let write_byte = gb.gb_memory.read_byte(imm16);
                    gb.registers.a = write_byte;
                    return ControlFlow::Break(());
                }
                0b11101000 => {
                    debug!("add sp, imm8");
                    let imm8 = gb.read_byte_and_advance_program_counter();
                    let sp = gb.registers.stack_pointer;
                    let (new_sp, _) = sp.overflowing_add(imm8 as u16);
                    gb.registers.f.h = calculate_byte_half_carry_add((0xFF & sp) as u8, imm8);
                    gb.registers.f.c = new_sp > 0xFF; //Does this work this way?
                    gb.registers.stack_pointer = new_sp;
                    gb.registers.f.z = false;
                    gb.registers.f.n = false;
                    return ControlFlow::Break(());
                }
                0b11111000 => {
                    debug!("ld hl,sp+imm8");
                    let imm8 = gb.read_byte_and_advance_program_counter();
                    let sp = gb.registers.stack_pointer;
                    let (new_hl, _) = sp.overflowing_add(imm8 as u16);
                    gb.registers.f.h = calculate_byte_half_carry_add((0xFF & sp) as u8, imm8);
                    gb.registers.f.c = new_hl > 0xFF; //Does this work this way?
                    gb.registers.set_hl(new_hl);
                    gb.registers.f.z = false;
                    gb.registers.f.n = false;
                    return ControlFlow::Break(());
                }
                0b11111001 => {
                    debug!("ld sp, hl");
                    let read_byte = gb.registers.get_hl();
                    gb.registers.stack_pointer = read_byte;
                    return ControlFlow::Break(());
                }
                0b11110011 => {
                    debug!("di");
                    gb.interrupt_master_flag = false;
                    return ControlFlow::Break(());
                }
                0b11111011 => {
                    debug!("ei");
                    gb.interrupt_master_flag = true;
                    return ControlFlow::Break(());
                }
                _ => (),
            }
        }
        _ => {
            error!("This opcode starts with some WIIIILD SHIT!")
        }
    }
    ControlFlow::Continue(())
}
fn read_rom(gb_memory: &mut gb_memory::GbMemory) {
    let mut contents = fs::read("01-special.gb").expect("Unable to read test rom.");
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
    contents.resize(0xFFFF + 1, 0u8);
    gb_memory.memory_array.copy_from_slice(&contents);
}
