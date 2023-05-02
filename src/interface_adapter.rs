#![allow(arithmetic_overflow)]

pub const MAX_ROM_SIZE: u32 = 16_777_216;

pub const KEYDOWN: u8 = 0xff;
pub const KEYUP  : u8 = 0xfe;

pub const MOUSE_LCLICK: u8 = 0xfd;
pub const MOUSE_RCLICK: u8 = 0xfc;

use std::{fs::File, io::Read};
use rand::Rng;

pub struct Adapter {
    pub port_a: u8,
    pub port_b: u8,

    pub keyb: u8,

    pub mouse_x: u8,
    pub mouse_y: u8,

    pub rom_ptr: u32,
    rom: Vec<u8>,

    pub interrupt_id: u8
}

impl Adapter {
    pub fn new() -> Self {
        return Adapter { 
            port_a: 0, port_b: 0, keyb: 0, 
            mouse_x: 0, mouse_y: 0, rom_ptr: 0, 
            rom: vec![0; MAX_ROM_SIZE as usize], interrupt_id: 0
        }
    }

    pub fn load_cartridge(&mut self, filename: &str) {
        let mut file = File::open(filename)
            .expect("Couldn't open cartridge file");
        
        let mut rom: Vec<u8> = Vec::new();
        file.read_to_end(&mut rom)
            .expect("Couldn't read cartridge file");

        while rom.len() < MAX_ROM_SIZE as usize {
            rom.push(0);
        }

        self.rom = rom;
    }

    pub fn write_byte(&mut self, value: u8, address: u16) {
        match address {
            0x0 => self.port_b  = value,
            0x1 => self.port_a  = value,
            0x2 => self.keyb    = value,
            0x3 => self.mouse_x = value,
            0x4 => self.mouse_y = value,
            0x5 => {
                self.rom_ptr &= 0x00ffff00;
                self.rom_ptr |= value as u32;
            },
            0x6 => {
                self.rom_ptr &= 0x00ff00ff;
                self.rom_ptr |= (value as u32) << 8;
            },
            0x7 => {
                self.rom_ptr &= 0x0000ffff;
                self.rom_ptr |= (value as u32) << 16;
            }
            0x8 => panic!("CPU is trying to write to adapter ROM"),
            0x9 => println!("CPU is trying to write to RNG source"),
            0xf => self.interrupt_id = value,
            _   => println!("Invalid adapter address {:04X}", address)
        }
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        return match address {
            0x0 => self.port_b,
            0x1 => self.port_a,
            0x2 => self.keyb,
            0x3 => self.mouse_x,
            0x4 => self.mouse_y,
            0x5 => (self.rom_ptr  & 0x00ff) as u8,
            0x6 => (self.rom_ptr >>      8) as u8,
            0x7 => (self.rom_ptr >>     16) as u8,
            0x8 => self.rom[self.rom_ptr as usize],
            0x9 => rand::thread_rng().gen_range(0 .. 0xff),
            0xf => self.interrupt_id,
            _   => {
                println!("Invalid adapter address {:04X}", address);
                0
            }
        };
    }

    pub fn write_word(&mut self, value: u16, address: u16) -> bool {
        match address {
            0x0 => {
                self.port_b = (value  & 0x00ff) as u8;
                self.port_a = (value >>      8) as u8;
            },
            0x1 => {
                self.port_a = (value  & 0x00ff) as u8;
                self.keyb   = (value >>      8) as u8;
            },
            0x2 => {
                self.keyb    = (value  & 0x00ff) as u8;
                self.mouse_x = (value >>      8) as u8;
            },
            0x3 => {
                self.mouse_x = (value  & 0x00ff) as u8;
                self.mouse_y = (value >>      8) as u8;
            },
            0x4 => {
                self.mouse_y = (value & 0x00ff) as u8;

                self.rom_ptr &= 0x00ffff00;
                self.rom_ptr |= (value >> 8) as u32;
            },
            0x5 => {
                self.rom_ptr &= 0x00ff0000;
                self.rom_ptr |= value as u32;
            }
            0x6 => {
                self.rom_ptr &= 0x000000ff;
                self.rom_ptr |= (value << 8) as u32;
            }
            0x7 => {
                self.rom_ptr &= 0x0000ffff;
                self.rom_ptr |= ((value & 0x00ff) << 16) as u32;

                self.interrupt_id = (value >> 8) as u8;
            }
            0x8 => println!("CPU is trying to write to adapter ROM and RNG source"),
            0x9 => println!("CPU is trying to write to RNG source and unbound memory"),
            0xf => {
                self.interrupt_id = (value & 0x00ff) as u8;
                return true;
            },
            _   => println!("Invalid adapter address {:04X}", address)
        }

        return false;
    }

    pub fn read_word(&self, address: u16) -> Option<u16> {
        return match address {
            0x0 => Some((self.port_b  as u16) | ((self.port_a  as u16) << 8)),
            0x1 => Some((self.port_a  as u16) | ((self.keyb    as u16) << 8)),
            0x2 => Some((self.keyb    as u16) | ((self.mouse_x as u16) << 8)),
            0x3 => Some((self.mouse_x as u16) | ((self.mouse_y as u16) << 8)),
            0x4 => Some((self.mouse_y as u16) | ((self.rom_ptr & 0x000000ff) << 8) as u16),
            0x5 => Some((self.rom_ptr & 0x0000ffff) as u16),
            0x6 => Some(((self.rom_ptr & 0x00ffff00) >> 8) as u16),
            0x7 => Some((self.rom_ptr >> 16) as u16 | ((self.rom[self.rom_ptr as usize] as u16) << 8)),
            0x8 => Some((self.rom[self.rom_ptr as usize] as u16) | (rand::thread_rng().gen_range(0 .. 0xff) << 8)),
            0x9 => {
                println!("CPU is trying to access unbound memory");
                Some(rand::thread_rng().gen_range(0 .. 0xff))
            },
            0xf => None,
            _   => {
                println!("Invalid adapter address {:04X}", address);
                Some(0)
            }
        };
    }
}