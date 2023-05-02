#![allow(arithmetic_overflow)]

use crate::mapper;
use std::cell::RefCell;
use std::fs::File;
use std::io::Read;
use std::rc::Rc;
use speedy2d::color::Color;

// character set: https://opengameart.org/content/ascii-bitmap-font-oldschool

pub const INTERNAL_RESOLUTION_X: u16 = 448;
pub const INTERNAL_RESOLUTION_Y: u16 = 470; // base is 288 + ui

pub const CHAR_X: u16 = 7;
pub const CHAR_Y: u16 = 9;

const RESOLUTION_X: u8 = 64;
const RESOLUTION_Y: u8 = 32;

const DOUBLE_RESOLUTION_X: u16 = (2 * RESOLUTION_X) as u16;

const FRAMEBUFFER_START: u16 = 0x6010; // ends at 0x7010

const COLOR_PALETTE: [[f32; 3]; 16] = [
    [ 0.0,  0.0,  0.0],
    [ 0.0,  0.0,  0.5],
    [ 0.0,  0.5,  0.0],
    [ 0.0,  0.5,  0.5],
    [ 0.5,  0.0,  0.0],
    [ 0.5,  0.0,  0.5],
    [ 0.5,  0.5,  0.0],
    [ 0.5,  0.5,  0.5],
    [0.25, 0.25, 0.25],
    [ 0.0,  0.0,  1.0],
    [ 0.0,  1.0,  0.0],
    [ 0.0,  1.0,  1.0],
    [ 1.0,  0.0,  0.0],
    [ 1.0,  0.0,  1.0],
    [ 1.0,  1.0,  0.0],
    [ 1.0,  1.0,  1.0]
];

pub struct PPU {
    chars        : Vec<Vec<u8>>,
    pub frame_buf: Vec<Vec<Color>>,

    mapper: Rc<RefCell<mapper::Map>>
}

impl PPU {
    pub fn new(mapper: Rc<RefCell<mapper::Map>>, charset: &str) -> Self {
        let mut chars: Vec<Vec<u8>> = Vec::new();
        let mut file = File::open(charset)
            .expect("Couldn't open charset file");

        loop {
            let mut chunk: Vec<u8> = Vec::with_capacity(CHAR_Y as usize);
            let i = file.by_ref().take(CHAR_Y as u64)
                .read_to_end(&mut chunk).unwrap();

            if i == 0 {
                break;
            }

            chars.push(chunk);

            if i < CHAR_Y as usize {
                break;
            }
        }

        return PPU {
            mapper, chars,
            frame_buf: vec![vec![Color::BLUE; INTERNAL_RESOLUTION_X as usize]; INTERNAL_RESOLUTION_Y as usize]
        }
    }

    pub fn draw_char_at(&mut self, x: u8, y: u8, chr: u8, ch_color: Color, bg_color: Color) {
        let lx = ((x as u16) * CHAR_X) as usize;
        let ly = ((y as u16) * CHAR_Y) as usize;
        let ch = self.chars.get((chr & 0x7f) as usize).unwrap();

        for ccy in 0 .. CHAR_Y {
            let line = ch.get(ccy as usize).unwrap();

            for ccx in 0 .. CHAR_X {
                if line & (1 << ccx) != 0 {
                    *(self.frame_buf.get_mut(ly + ccy as usize).unwrap()
                        .get_mut(lx + ccx as usize).unwrap()) = ch_color.clone();
                } else {
                    *(self.frame_buf.get_mut(ly + ccy as usize).unwrap()
                        .get_mut(lx + ccx as usize).unwrap()) = bg_color.clone();
                }
            }
        }
    }

    pub fn tick(&mut self) {
        for y in 0 .. RESOLUTION_Y {
            let mut cx: u16 = 0;
            for x in 0 .. RESOLUTION_X {
                let data = (*self.mapper.borrow()).read_word(
                    FRAMEBUFFER_START + cx + (y as u16 * DOUBLE_RESOLUTION_X)
                );

                cx += 2;

                let ch = COLOR_PALETTE[((data >> 8) & 0x0f) as usize];
                let bg = COLOR_PALETTE[(data >> 12) as usize];

                self.draw_char_at(
                    x, y, 
                    (data & 0x00ff) as u8, 
                    Color::from_rgb(
                        ch[0], ch[1], ch[2]
                    ), 
                    Color::from_rgb(
                        bg[0], bg[1], bg[2]
                    )
                );
            }
        }
    }
}