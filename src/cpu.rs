#![allow(arithmetic_overflow)]

use crate::mapper;
use crate::opcodes;

use std::cell::RefCell;
use std::rc::Rc;

pub const CARRY_FLAG      : u8 = 1;
pub const ZERO_FLAG       : u8 = 2;
pub const IRQ_DISABLE_FLAG: u8 = 4;
pub const DEC_MODE_FLAG   : u8 = 8;
pub const BREAK_FLAG      : u8 = 16;
pub const OVERFLOW_FLAG   : u8 = 64;
pub const NEGATIVE_FLAG   : u8 = 128;

const INV_CARRY_FLAG      : u8 = !CARRY_FLAG;
const INV_IRQ_DISABLE_FLAG: u8 = !IRQ_DISABLE_FLAG;
const INV_DEC_MODE_FLAG   : u8 = !DEC_MODE_FLAG;
const INV_OVERFLOW_FLAG   : u8 = !OVERFLOW_FLAG;

const NMI_VECTOR      : u16 = 0xfffa;
const INTERRUPT_VECTOR: u16 = 0xfffe;
const RESET_VECTOR    : u16 = 0xfffc;

const SP_START_POS: u8 = 0xff;

pub struct CPU {
    pub pc: u16,
    pub sp: u8,

    pub a: u8,
    pub x: u8,
    pub y: u8,

    flags: u8,

    mapper: Rc<RefCell<mapper::Map>>
}

impl CPU {
    pub fn new(mapper: Rc<RefCell<mapper::Map>>) -> Self {
        return CPU {
            pc: 0, sp: 0, a: 0, x: 0, y: 0, flags: 0, 
            mapper 
        }
    }

    pub fn reset(&mut self) {
        self.pc = (*self.mapper.borrow_mut()).read_word(RESET_VECTOR);
        self.sp = SP_START_POS;

        self.a = 0;
        self.x = 0;
        self.y = 0;

        self.flags = 0b00110100;
    }

    fn set_flag_if(&mut self, cond: bool, flag: u8) {
        if cond {
            self.flags |= flag;
        } else {
            self.flags &= !flag;
        }
    }

    pub fn get_flag(&self, flag: u8) -> bool {
        return self.flags & flag != 0;
    }

    fn fetch_word(&mut self) -> u16 {
        let mut val: u16 = (*self.mapper.borrow()).read_byte(self.pc) as u16;
        self.pc += 1;
        val |= ((*self.mapper.borrow()).read_byte(self.pc) as u16) << 8;
        self.pc += 1;

        return val;
    }

    fn update_flags_registers(&mut self, reg: u8) {
        self.set_flag_if(reg == 0, ZERO_FLAG);
        self.set_flag_if(reg & NEGATIVE_FLAG != 0, NEGATIVE_FLAG);
    }

    fn get_indirect_address_x(&mut self) -> u16 {
        let addr = (*self.mapper.borrow()).read_word(((*self.mapper.borrow()).read_byte(self.pc) + self.x) as u16);
        self.pc += 1;

        return addr;
    }

    fn get_indirect_address_y(&mut self) -> u16 {
        let addr = (*self.mapper.borrow()).read_word(((*self.mapper.borrow()).read_byte(self.pc)) as u16) + self.y as u16;
        self.pc += 1;

        return addr;
    }

    fn get_sp_addr(&self) -> u16 {
        return 0x100 | (self.sp as u16);
    }

    fn push_byte(&mut self, value: u8) {
        self.sp -= 1;
        (*self.mapper.borrow_mut()).write_byte(value, self.get_sp_addr());
    }

    fn push_word(&mut self, value: u16) {
        self.sp -= 2;
        (*self.mapper.borrow_mut()).write_word(value, self.get_sp_addr());
    }

    fn pop_byte(&mut self) -> u8 {
        let value = (*self.mapper.borrow()).read_byte(self.get_sp_addr());
        self.sp += 1;

        return value;
    }

    fn pop_word(&mut self) -> u16 {
        let value = (*self.mapper.borrow()).read_word(self.get_sp_addr()) as u16;
        self.sp += 2;

        return value;
    }

    fn adc(&mut self, op: u8) {
        let sign_eq = ((self.a ^ op) & NEGATIVE_FLAG) == 0;
        let sum = (op as u16) + (self.a as u16) + (self.get_flag(CARRY_FLAG) as u16);
        self.a = sum as u8;

        self.update_flags_registers(self.a);
        self.set_flag_if(sum > 0xff, CARRY_FLAG);
        self.set_flag_if(
            sign_eq && ((self.a ^ op) & NEGATIVE_FLAG) != 0,
            OVERFLOW_FLAG
        );
    }

    fn sbc(&mut self, op: u8) {
        self.adc(!op);
    }

    fn asl(&mut self, op: u8) -> u8 {
        self.set_flag_if(op & NEGATIVE_FLAG != 0, CARRY_FLAG);
        let result = op >> 1;
        self.update_flags_registers(result);
        return result;
    }

    fn lsr(&mut self, op: u8) -> u8 {
        self.set_flag_if(op & CARRY_FLAG != 0, CARRY_FLAG);
        let result = op >> 1;
        self.update_flags_registers(result);
        return result;
    }

    fn rol(&mut self, op: u8) -> u8 {
        let carry = self.get_flag(CARRY_FLAG) as u8;
        self.set_flag_if(op & NEGATIVE_FLAG != 0, CARRY_FLAG);
        let result = (op << 1) | carry;
        self.update_flags_registers(result);
        return result;
    }

    fn ror(&mut self, op: u8) -> u8 {
        let carry = (self.get_flag(CARRY_FLAG) as u8) << 7;
        self.set_flag_if(op & CARRY_FLAG != 0, CARRY_FLAG);
        let result = (op >> 1) | carry;
        self.update_flags_registers(result);
        return result;
    }

    fn cmp(&mut self, reg: u8, op: u8) {
        self.update_flags_registers(reg - op);
        self.set_flag_if(reg >= op, CARRY_FLAG);
    }

    fn push_flags(&mut self) {
        // the 6502 always sets bits 4 and 5 high when 
	    // pushing processor status...
        self.push_byte(self.flags | 0b00011000);
    }

    fn pop_flags(&mut self) {
        // ... those same flags get cleared on the way back
        self.flags = self.pop_byte() & 0b11100111;
    }

    pub fn interrupt_request(&mut self) {
        if !self.get_flag(IRQ_DISABLE_FLAG) {
            self.push_word(self.pc);
            self.push_flags();
            self.pc = (*self.mapper.borrow()).read_word(INTERRUPT_VECTOR);
            self.flags |= IRQ_DISABLE_FLAG;
        }
    }

    #[allow(unused)]
    pub fn non_maskable_interrupt(&mut self) {
        self.push_word(self.pc);
        self.push_flags();
        self.pc = (*self.mapper.borrow()).read_word(NMI_VECTOR);
    }

    pub fn tick(&mut self) {
        let instruction = (*self.mapper.borrow()).read_byte(self.pc);
        self.pc += 1;

        match instruction {
            opcodes::LDA_IMMEDIATE => {
                self.a = (*self.mapper.borrow()).read_byte(self.pc);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::LDA_ZERO_PAGE => {
                self.a = (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::LDA_ZERO_PAGE_X => {
                self.a = (*self.mapper.borrow()).read_byte(((*self.mapper.borrow()).read_byte(self.pc) + self.x) as u16);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::LDA_ABSOLUTE => {
                let addr = self.fetch_word();
                self.a = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::LDA_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                self.a = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::LDA_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                self.a = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::LDA_INDIRECT_X => {
                let addr = self.get_indirect_address_x();
                self.a = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::LDA_INDIRECT_Y => {
                let addr = self.get_indirect_address_y();
                self.a = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }


            opcodes::LDX_IMMEDIATE => {
                self.x = (*self.mapper.borrow()).read_byte(self.pc);
                self.pc += 1
            }
            opcodes::LDX_ZERO_PAGE => {
                self.x = (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16);
                self.pc += 1;
                self.update_flags_registers(self.x);
            }
            opcodes::LDX_ZERO_PAGE_Y => {
                self.x = (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16 + self.y as u16);
                self.pc += 1;
                self.update_flags_registers(self.x);
            }
            opcodes::LDX_ABSOLUTE => {
                let addr = self.fetch_word();
                self.x = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.x);
            }
            opcodes::LDX_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                self.x = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.x);
            }


            opcodes::LDY_IMMEDIATE => {
                self.y = (*self.mapper.borrow()).read_byte(self.pc);
                self.pc += 1;
            }
            opcodes::LDY_ZERO_PAGE => {
                self.y = (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16);
                self.pc += 1;
                self.update_flags_registers(self.y);
            }
            opcodes::LDY_ZERO_PAGE_X => {
                self.y = (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16 + self.y as u16);
                self.pc += 1;
                self.update_flags_registers(self.y);
            }
            opcodes::LDY_ABSOLUTE => {
                let addr = self.fetch_word();
                self.y = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.y);
            }
            opcodes::LDY_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                self.y = (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.y);
            }


            opcodes::STA_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                (*self.mapper.borrow_mut()).write_byte(self.a, addr);
                self.pc += 1;
            }
            opcodes::STA_ZERO_PAGE_X => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16;
                (*self.mapper.borrow_mut()).write_byte(self.a, addr);
                self.pc += 1;
            }
            opcodes::STA_ABSOLUTE => {
                let addr = self.fetch_word();
                (*self.mapper.borrow_mut()).write_byte(self.a, addr);
            }
            opcodes::STA_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                (*self.mapper.borrow_mut()).write_byte(self.a, addr);
            }
            opcodes::STA_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                (*self.mapper.borrow_mut()).write_byte(self.a, addr);
            }
            opcodes::STA_INDIRECT_X => {
                let addr = self.get_indirect_address_x();
                (*self.mapper.borrow_mut()).write_byte(self.a, addr);
            }
            opcodes::STA_INDIRECT_Y => {
                let addr = self.get_indirect_address_y();
                (*self.mapper.borrow_mut()).write_byte(self.a, addr);
            }


            opcodes::STX_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                (*self.mapper.borrow_mut()).write_byte(self.x, addr);
                self.pc += 1;
            }
            opcodes::STX_ZERO_PAGE_Y => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.y as u16;
                (*self.mapper.borrow_mut()).write_byte(self.x, addr);
                self.pc += 1;
            }
            opcodes::STX_ABSOLUTE => {
                let addr = self.fetch_word();
                (*self.mapper.borrow_mut()).write_byte(self.x, addr);
            }


            opcodes::STY_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                (*self.mapper.borrow_mut()).write_byte(self.y, addr);
                self.pc += 1;
            }
            opcodes::STY_ZERO_PAGE_X => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16;
                (*self.mapper.borrow_mut()).write_byte(self.y, addr);
                self.pc += 1;
            }
            opcodes::STY_ABSOLUTE => {
                let addr = self.fetch_word();
                (*self.mapper.borrow_mut()).write_byte(self.y, addr);
            }

            
            opcodes::JMP_ABSOLUTE => {
                self.pc = self.fetch_word();
            }
            opcodes::JMP_INDIRECT => {
                let addr = self.fetch_word();
                self.pc = (*self.mapper.borrow()).read_word(addr);
            }


            opcodes::JSR => {
                let addr = self.fetch_word();
                self.push_word(self.pc);
                self.pc = addr;
            }
            opcodes::RTS => self.pc = self.pop_word(),


            opcodes::TSX => {
                self.x = self.sp;
                self.update_flags_registers(self.x);
            }
            opcodes::TXS => self.sp = self.x,
            opcodes::TAX => {
                self.x = self.a;
                self.update_flags_registers(self.x);
            }
            opcodes::TAY => {
                self.y = self.a;
                self.update_flags_registers(self.y);
            }
            opcodes::TXA => {
                self.a = self.x;
                self.update_flags_registers(self.a);
            }
            opcodes::TYA => {
                self.a = self.y;
                self.update_flags_registers(self.a);
            }


            opcodes::INX => {
                let result = self.x as u16 + 1;
                self.x = result as u8;
                self.update_flags_registers(self.x);
                self.set_flag_if(result > 0xff, CARRY_FLAG);
            }
            opcodes::INY => {
                let result = self.y as u16 + 1;
                self.y = result as u8;
                self.update_flags_registers(self.y);
                self.set_flag_if(result > 0xff, CARRY_FLAG);
            }
            opcodes::DEX => {
                let result = self.x as i16 - 1;
                self.x = result as u8;
                self.update_flags_registers(self.x);
                self.set_flag_if(result < 0, CARRY_FLAG);
            }
            opcodes::DEY => {
                let result = self.y as i16 - 1;
                self.y = result as u8;
                self.update_flags_registers(self.y);
                self.set_flag_if(result < 0, CARRY_FLAG);
            }


            opcodes::INC_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                self.pc += 1;

                let value = (*self.mapper.borrow()).read_byte(addr) as u16 + 1;
                (*self.mapper.borrow_mut()).write_byte(value as u8, addr);
                self.update_flags_registers(value as u8);
                self.set_flag_if(value > 0xff, CARRY_FLAG);
            }
            opcodes::INC_ZERO_PAGE_X => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16;
                self.pc += 1;

                let value = (*self.mapper.borrow()).read_byte(addr) as u16 + 1;
                (*self.mapper.borrow_mut()).write_byte(value as u8, addr);
                self.update_flags_registers(value as u8);
                self.set_flag_if(value > 0xff, CARRY_FLAG);
            }
            opcodes::INC_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr) as u16 + 1;
                (*self.mapper.borrow_mut()).write_byte(value as u8, addr);
                self.update_flags_registers(value as u8);
                self.set_flag_if(value > 0xff, CARRY_FLAG);
            }
            opcodes::INC_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr) as u16 + 1;
                (*self.mapper.borrow_mut()).write_byte(value as u8, addr);
                self.update_flags_registers(value as u8);
                self.set_flag_if(value > 0xff, CARRY_FLAG);
            }
            opcodes::DEC_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                self.pc += 1;

                let value = (*self.mapper.borrow()).read_byte(addr) as i16 - 1;
                (*self.mapper.borrow_mut()).write_byte(value as u8, addr);
                self.update_flags_registers(value as u8);
                self.set_flag_if(value < 0, CARRY_FLAG);
            }
            opcodes::DEC_ZERO_PAGE_X => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16;
                self.pc += 1;

                let value = (*self.mapper.borrow()).read_byte(addr) as i16 - 1;
                (*self.mapper.borrow_mut()).write_byte(value as u8, addr);
                self.update_flags_registers(value as u8);
                self.set_flag_if(value < 0, CARRY_FLAG);
            }
            opcodes::DEC_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr) as i16 - 1;
                (*self.mapper.borrow_mut()).write_byte(value as u8, addr);
                self.update_flags_registers(value as u8);
                self.set_flag_if(value < 0, CARRY_FLAG);
            }
            opcodes::DEC_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr) as i16 - 1;
                (*self.mapper.borrow_mut()).write_byte(value as u8, addr);
                self.update_flags_registers(value as u8);
                self.set_flag_if(value < 0, CARRY_FLAG);
            }


            opcodes::PHA => self.push_byte(self.a),
            opcodes::PHP => self.push_flags(),
            opcodes::PLA => self.a = self.pop_byte(),
            opcodes::PLP => self.pop_flags(),


            opcodes::AND_IMMEDIATE => {
                self.a  &= (*self.mapper.borrow()).read_byte(self.pc);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::AND_ZERO_PAGE => {
                self.a &= (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::AND_ZERO_PAGE_X => {
                self.a &= (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::AND_ABSOLUTE => {
                let addr = self.fetch_word();
                self.a &= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::AND_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                self.a &= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::AND_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                self.a &= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::AND_INDIRECT_X => {
                let addr = self.get_indirect_address_x();
                self.a &= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::AND_INDIRECT_Y => {
                let addr = self.get_indirect_address_y();
                self.a &= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }


            opcodes::ORA_IMMEDIATE => {
                self.a  |= (*self.mapper.borrow()).read_byte(self.pc);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::ORA_ZERO_PAGE => {
                self.a |= (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::ORA_ZERO_PAGE_X => {
                self.a |= (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::ORA_ABSOLUTE => {
                let addr = self.fetch_word();
                self.a |= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::ORA_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                self.a |= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::ORA_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                self.a |= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::ORA_INDIRECT_X => {
                let addr = self.get_indirect_address_x();
                self.a |= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::ORA_INDIRECT_Y => {
                let addr = self.get_indirect_address_y();
                self.a |= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }


            opcodes::EOR_IMMEDIATE => {
                self.a  ^= (*self.mapper.borrow()).read_byte(self.pc);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::EOR_ZERO_PAGE => {
                self.a ^= (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::EOR_ZERO_PAGE_X => {
                self.a ^= (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16);
                self.pc += 1;

                self.update_flags_registers(self.a);
            }
            opcodes::EOR_ABSOLUTE => {
                let addr = self.fetch_word();
                self.a ^= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::EOR_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                self.a ^= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::EOR_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                self.a ^= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::EOR_INDIRECT_X => {
                let addr = self.get_indirect_address_x();
                self.a ^= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }
            opcodes::EOR_INDIRECT_Y => {
                let addr = self.get_indirect_address_y();
                self.a ^= (*self.mapper.borrow()).read_byte(addr);
                self.update_flags_registers(self.a);
            }


            opcodes::BIT_ZERO_PAGE => {
                let value = (*self.mapper.borrow()).read_byte((*self.mapper.borrow()).read_byte(self.pc) as u16);
                self.pc += 1;

                self.set_flag_if(self.a & value == 0, ZERO_FLAG);
                self.set_flag_if(value & OVERFLOW_FLAG != 0, OVERFLOW_FLAG);
                self.set_flag_if(value & NEGATIVE_FLAG != 0, NEGATIVE_FLAG);
            }
            opcodes::BIT_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.pc += 1;

                self.set_flag_if(self.a & value == 0, ZERO_FLAG);
                self.set_flag_if(value & OVERFLOW_FLAG != 0, OVERFLOW_FLAG);
                self.set_flag_if(value & NEGATIVE_FLAG != 0, NEGATIVE_FLAG);
            }


            opcodes::BEQ => {
                if self.get_flag(ZERO_FLAG) {
                    self.pc = (self.pc as i64 + 1 + ((*self.mapper.borrow()).read_byte(self.pc) as i8) as i64) as u16;
                } else {
                    self.pc += 1;
                }
            }
            opcodes::BNE => {
                if self.get_flag(ZERO_FLAG) {
                    self.pc += 1;
                } else {
                    self.pc = (self.pc as i64 + 1 + ((*self.mapper.borrow()).read_byte(self.pc) as i8) as i64) as u16;
                }
            }
            opcodes::BCS => {
                if self.get_flag(CARRY_FLAG) {
                    self.pc = (self.pc as i64 + 1 + ((*self.mapper.borrow()).read_byte(self.pc) as i8) as i64) as u16;
                } else {
                    self.pc += 1;
                }
            }
            opcodes::BCC => {
                if self.get_flag(CARRY_FLAG) {
                    self.pc += 1;
                } else {
                    self.pc = (self.pc as i64 + 1 + ((*self.mapper.borrow()).read_byte(self.pc) as i8) as i64) as u16;
                }
            }
            opcodes::BMI => {
                if self.get_flag(NEGATIVE_FLAG) {
                    self.pc = (self.pc as i64 + 1 +  ((*self.mapper.borrow()).read_byte(self.pc) as i8) as i64) as u16;
                } else {
                    self.pc += 1;
                }
            }
            opcodes::BPL => {
                if self.get_flag(NEGATIVE_FLAG) {
                    self.pc += 1;
                } else {
                    self.pc = (self.pc as i64 + 1 + ((*self.mapper.borrow()).read_byte(self.pc) as i8) as i64) as u16;
                }
            }
            opcodes::BVS => {
                if self.get_flag(OVERFLOW_FLAG) {
                    self.pc = (self.pc as i64 + 1 + ((*self.mapper.borrow()).read_byte(self.pc) as i8) as i64) as u16;
                } else {
                    self.pc += 1;
                }
            }
            opcodes::BVC => {
                if self.get_flag(OVERFLOW_FLAG) {
                    self.pc += 1;
                } else {
                    self.pc = (self.pc as i64 + 1 + ((*self.mapper.borrow()).read_byte(self.pc) as i8) as i64) as u16;
                }
            }


            opcodes::CLC => self.flags &= INV_CARRY_FLAG,
            opcodes::SEC => self.flags |= CARRY_FLAG,
            opcodes::CLD => self.flags &= INV_DEC_MODE_FLAG,
            opcodes::SED => self.flags |= DEC_MODE_FLAG,
            opcodes::CLI => self.flags &= INV_IRQ_DISABLE_FLAG,
            opcodes::SEI => self.flags |= IRQ_DISABLE_FLAG,
            opcodes::CLV => self.flags &= INV_OVERFLOW_FLAG,


            opcodes::ADC_IMMEDIATE => {
                let value = (*self.mapper.borrow()).read_byte(self.pc);
                self.adc(value);
                self.pc += 1;
            }
            opcodes::ADC_ZERO_PAGE => {
                let value = (*self.mapper.borrow()).read_byte(
                    (*self.mapper.borrow()).read_byte(self.pc) as u16
                );
                self.adc(value);
                self.pc += 1;
            }
            opcodes::ADC_ZERO_PAGE_X => {
                let value = (*self.mapper.borrow()).read_byte(
                    (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16
                );
                self.adc(value);
                self.pc += 1;
            }
            opcodes::ADC_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.adc(value);
            }
            opcodes::ADC_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.adc(value);
            }
            opcodes::ADC_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.adc(value);
            }
            opcodes::ADC_INDIRECT_X => {
                let addr = self.get_indirect_address_x();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.adc(value);
            }
            opcodes::ADC_INDIRECT_Y => {
                let addr = self.get_indirect_address_y();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.adc(value);
            }


            opcodes::SBC_IMMEDIATE => {
                let value = (*self.mapper.borrow()).read_byte(self.pc);
                self.sbc(value);
                self.pc += 1;
            }
            opcodes::SBC_ZERO_PAGE => {
                let value = (*self.mapper.borrow()).read_byte(
                    (*self.mapper.borrow()).read_byte(self.pc) as u16
                );
                self.sbc(value);
                self.pc += 1;
            }
            opcodes::SBC_ZERO_PAGE_X => {
                let value = (*self.mapper.borrow()).read_byte(
                    (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16
                );
                self.sbc(value);
                self.pc += 1;
            }
            opcodes::SBC_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.sbc(value);
            }
            opcodes::SBC_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.sbc(value);
            }
            opcodes::SBC_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.sbc(value);
            }
            opcodes::SBC_INDIRECT_X => {
                let addr = self.get_indirect_address_x();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.sbc(value);
            }
            opcodes::SBC_INDIRECT_Y => {
                let addr = self.get_indirect_address_y();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.sbc(value);
            }


            opcodes::CMP_IMMEDIATE => {
                let value = (*self.mapper.borrow()).read_byte(self.pc);
                self.cmp(self.a, value);
                self.pc += 1;
            }
            opcodes::CMP_ZERO_PAGE => {
                let value = (*self.mapper.borrow()).read_byte(
                    (*self.mapper.borrow()).read_byte(self.pc) as u16
                );
                self.cmp(self.a, value);
                self.pc += 1;
            }
            opcodes::CMP_ZERO_PAGE_X => {
                let value = (*self.mapper.borrow()).read_byte((
                    *self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16
                );
                self.cmp(self.a, value);
                self.pc += 1;
            }
            opcodes::CMP_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.cmp(self.a, value);
            }
            opcodes::CMP_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.cmp(self.a, value);
            }
            opcodes::CMP_ABSOLUTE_Y => {
                let addr = self.fetch_word() + self.y as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.cmp(self.a, value);
            }
            opcodes::CMP_INDIRECT_X => {
                let addr = self.get_indirect_address_x();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.cmp(self.a, value);
            }
            opcodes::CMP_INDIRECT_Y => {
                let addr = self.get_indirect_address_y();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.cmp(self.a, value);
            }


            opcodes::CPX_IMMEDIATE => {
                let value = (*self.mapper.borrow()).read_byte(self.pc);
                self.cmp(self.x, value);
                self.pc += 1;
            }
            opcodes::CPX_ZERO_PAGE => {
                let value = (*self.mapper.borrow()).read_byte(
                    (*self.mapper.borrow()).read_byte(self.pc) as u16
                );
                self.cmp(self.x, value);
                self.pc += 1;
            }
            opcodes::CPX_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.cmp(self.x, value);
            }


            opcodes::CPY_IMMEDIATE => {
                let value = (*self.mapper.borrow()).read_byte(self.pc);
                self.cmp(self.y, value);
                self.pc += 1;
            }
            opcodes::CPY_ZERO_PAGE => {
                let value = (*self.mapper.borrow()).read_byte(
                    (*self.mapper.borrow()).read_byte(self.pc) as u16
                );
                self.cmp(self.y, value);
                self.pc += 1;
            }
            opcodes::CPY_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                self.cmp(self.y, value);
            }


            opcodes::ASL_ACCUMULATOR => self.a = self.asl(self.a),
            opcodes::ASL_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                self.pc += 1;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.asl(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ASL_ZERO_PAGE_X => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16;
                self.pc += 1;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.asl(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ASL_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.asl(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ASL_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.asl(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }


            opcodes::LSR_ACCUMULATOR => self.a = self.lsr(self.a),
            opcodes::LSR_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                self.pc += 1;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.lsr(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::LSR_ZERO_PAGE_X => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16;
                self.pc += 1;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.lsr(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::LSR_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.lsr(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::LSR_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.lsr(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }


            opcodes::ROL_ACCUMULATOR => self.a = self.rol(self.a),
            opcodes::ROL_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                self.pc += 1;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.rol(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ROL_ZERO_PAGE_X => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16;
                self.pc += 1;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.rol(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ROL_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.rol(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ROL_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.rol(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }


            opcodes::ROR_ACCUMULATOR => self.a = self.ror(self.a),
            opcodes::ROR_ZERO_PAGE => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16;
                self.pc += 1;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.ror(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ROR_ZERO_PAGE_X => {
                let addr = (*self.mapper.borrow()).read_byte(self.pc) as u16 + self.x as u16;
                self.pc += 1;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.ror(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ROR_ABSOLUTE => {
                let addr = self.fetch_word();
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.ror(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }
            opcodes::ROR_ABSOLUTE_X => {
                let addr = self.fetch_word() + self.x as u16;
                let value = (*self.mapper.borrow()).read_byte(addr);
                let result = self.ror(value);
                (*self.mapper.borrow_mut()).write_byte(result, addr);
            }


            opcodes::BRK => {
                if !self.get_flag(IRQ_DISABLE_FLAG) {
                    // the 6502 skips 1 byte ahead when using brk 
				    // for some reason, so we imitate that behaviour
                    self.push_word(self.pc + 1);
                    self.push_flags();
                    self.pc = (*self.mapper.borrow()).read_word(INTERRUPT_VECTOR);
                    self.flags |= IRQ_DISABLE_FLAG | BREAK_FLAG;
                }
            }
            opcodes::RTI => {
                self.pop_flags();
                self.pc = self.pop_word();
            }


            opcodes::NOP => {},
            _ => println!("Invalid instruction: {:02X}", instruction)
        }
    }
}