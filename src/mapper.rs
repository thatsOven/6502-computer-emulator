#![allow(arithmetic_overflow)]

use std::{fs::File, io::Read};

use crate::interface_adapter;

const RAM_SIZE: u16 = 32768;
const ROM_SIZE: u16 = 32768;

pub struct Map {
    pub fbuf_changed: bool,

    rom:     Vec<u8>,
    pub ram: Vec<u8>,

    pub int_adapter: interface_adapter::Adapter 
}

impl Map {
    pub fn new(filename: &str) -> Self {
        let mut file = File::open(filename)
            .expect("Couldn't open ROM file");
        
        let mut rom: Vec<u8> = Vec::new();
        file.read_to_end(&mut rom)
            .expect("Couldn't read ROM file");

        while rom.len() < ROM_SIZE as usize {
            rom.push(0);
        }

        return Map {
            rom, ram: vec![0; RAM_SIZE as usize], fbuf_changed: true,
            int_adapter: interface_adapter::Adapter::new()
        }
    }

    pub fn write_byte(&mut self, value: u8, address: u16) {
        if address <= 0x7fff {
            if address >= 0x6000 && address <= 0x600f {
                self.int_adapter.write_byte(value, address & 0xf);
            } else {
                if address >= 0x6010 && address <= 0x7010 {
                    self.fbuf_changed = true;
                }

                (*self.ram.get_mut(address as usize).unwrap()) = value;
            }
        } else {
            println!("CPU is trying to write to ROM!");
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        if address <= 0x7fff {
            if address >= 0x6000 && address <= 0x600f {
                return self.int_adapter.read_byte(address & 0xf);
            } else {
                return *self.ram.get(address as usize).unwrap();
            }
        } else {
            return *self.rom.get((address & 0x7fff) as usize).unwrap();
        }
    }

    pub fn write_word(&mut self, value: u16, address: u16) {
        let addr = address as usize;

        if address <= 0x7fff {
            if address >= 0x6000 && address <= 0x600f {
                if self.int_adapter.write_word(value, address & 0xf) {
                    if address + 1 >= 0x6010 && address + 1 <= 0x7010 {
                        self.fbuf_changed = true;
                    }

                    (*self.ram.get_mut(addr + 1).unwrap()) = (value >> 8) as u8;
                } 
            } else {
                if (address >= 0x6010 && address <= 0x7010) || (address + 1 >= 0x6010 && address + 1 <= 0x7010) {
                    self.fbuf_changed = true;
                }

                (*self.ram.get_mut(addr).unwrap()) = (value & 0xff) as u8;

                if address + 1 < 0x7fff {
                    (*self.ram.get_mut(addr + 1).unwrap()) = (value >> 8) as u8;
                } else {
                    println!("CPU is trying to write to ROM!");
                }
            }
        } else {
            println!("CPU is trying to write to ROM!");
        }
    }

    pub fn read_word(&self, address: u16) -> u16 {
        if address <= 0x7fff {
            if address >= 0x6000 && address <= 0x600f {
                return match self.int_adapter.read_word(address & 0xf) {
                    Some(x) => x,
                    None => (self.int_adapter.interrupt_id as u16) | ((self.ram[address as usize + 1] as u16) << 8)
                };
            }

            let addr = address as usize;
            if address + 1 <= 0x7fff {
                return (*self.ram.get(addr).unwrap() as u16) | ((*self.ram.get(addr + 1).unwrap() as u16) << 8);
            } else {
                return (*self.ram.get(addr).unwrap() as u16) | ((*self.rom.get(((address + 1) & 0x7fff) as usize).unwrap() as u16) << 8);
            }
        }
        
        let addr = (address & 0x7fff) as usize;
        let addr_plus_one = ((address + 1) & 0x7fff) as usize;
        return (*self.rom.get(addr).unwrap() as u16) | ((*self.rom.get(addr_plus_one).unwrap() as u16) << 8);
    }
}