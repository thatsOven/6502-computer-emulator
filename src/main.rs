#![allow(arithmetic_overflow)]

mod cpu;
mod ppu;
mod opcodes;
mod mapper;
mod interface_adapter;

use std::cell::RefCell;
use std::cmp;
use std::rc::Rc;
use std::time::{Instant, Duration};
use std::thread::sleep;

use speedy2d::color::Color;
use speedy2d::dimen::Vector2;
use speedy2d::window::{WindowSize, WindowPosition, MouseButton};
use speedy2d::window::{WindowHandler, WindowHelper, WindowCreationOptions};
use speedy2d::{Graphics2D, Window};

use clap::Parser;

const RESOLUTION_X: u16 = ppu::INTERNAL_RESOLUTION_X * 2;
const RESOLUTION_Y: u16 = ppu::INTERNAL_RESOLUTION_Y * 2; 

const TICKS_PER_FRAME: u32 = 1;
const DEFAULT_DELAY: f32 = 0.0;
const FORCE_UPDATE_EACH: u16 = 3600;
const UPDATE_EACH_CHANGED: u16 = 1;

struct Emu {
    update_each: u16,
    update_each_changed: u16,
    changed_cnt: u16,
    do_sleep: bool,
    sleep: Duration,

    frame: u16,
    timer: Instant,

    ticks:  u32,
    mapper: Rc<RefCell<mapper::Map>>,
    cpu:    cpu::CPU,
    ppu:    ppu::PPU
}

impl Emu {
    fn draw_text(&mut self, text: &str, x: u8, y: u8, ch_color: Color) {
        for (i, ch) in text.chars().enumerate() {
            self.ppu.draw_char_at(x + i as u8, y, ch as u8, ch_color, Color::BLUE);
        }
    }

    fn memoryrow(&mut self, addr: u16, mut x: u8, y: u8) {
        self.draw_text(&(format!("{:04X}", addr) + &": ".to_string()), x, y, Color::WHITE);

        x += 6;
        for i in 0 .. 16 {
            let byte = (*self.mapper.borrow()).read_byte(addr + i as u16);
            self.draw_text(&format!("{:02X}", byte), x, y, if addr + i == self.cpu.pc { Color::GREEN } else { Color::WHITE });

            x += 3;
        }
    }
}

impl WindowHandler for Emu {
    fn on_draw(&mut self, helper: &mut WindowHelper, graphics: &mut Graphics2D) {
        let cpu_time = self.timer.elapsed().as_secs_f32();
        let mut changed = false;

        for _ in 0 .. self.ticks {
            self.cpu.tick();

            if (*self.mapper.borrow()).fbuf_changed {
                (*self.mapper.borrow_mut()).fbuf_changed = false;
                self.changed_cnt += 1;
                changed = true;
            }
        }
        
        if self.frame == self.update_each && self.update_each != 0xffff {
            self.frame = 0;
            changed = true;
        }

        if changed {
            self.ppu.tick();

            if self.changed_cnt >= self.update_each_changed {
                self.changed_cnt = 0;

                let clock_str = (1.0 / (cpu_time / self.ticks as f32)).to_string();
                let lim = cmp::min(32, clock_str.len());
                self.draw_text(("Clock: ".to_string() + &clock_str[..lim] + " Hz   ").as_str(), 4, 34, Color::WHITE);

                self.draw_text(("X:  ".to_string() + &format!("{:02X}", self.cpu.x)).as_str(), 4, 36, Color::WHITE);
                self.draw_text(("Y:  ".to_string() + &format!("{:02X}", self.cpu.y)).as_str(), 4, 37, Color::WHITE);

                self.draw_text(("A:  ".to_string() + &format!("{:02X}", self.cpu.a)).as_str(), 4, 39, Color::WHITE);
                self.draw_text(("SP: ".to_string() + &format!("{:02X}", self.cpu.sp)).as_str(), 4, 40, Color::WHITE);

                self.draw_text(("PORTA:  ".to_string() + &format!("{:02X}", (*self.mapper.borrow()).int_adapter.port_a)).as_str(), 13, 36, Color::WHITE);
                self.draw_text(("PORTB:  ".to_string() + &format!("{:02X}", (*self.mapper.borrow()).int_adapter.port_b)).as_str(), 13, 37, Color::WHITE);

                self.draw_text(("MOUSEX: ".to_string() + &format!("{:02X}", (*self.mapper.borrow()).int_adapter.mouse_x)).as_str(), 13, 39, Color::WHITE);
                self.draw_text(("MOUSEY: ".to_string() + &format!("{:02X}", (*self.mapper.borrow()).int_adapter.mouse_y)).as_str(), 13, 40, Color::WHITE);

                self.draw_text(("KEYB:   ".to_string() + &format!("{:02X}", (*self.mapper.borrow()).int_adapter.keyb)).as_str(), 25, 36, Color::WHITE);
                self.draw_text(("INTID:  ".to_string() + &format!("{:02X}", (*self.mapper.borrow()).int_adapter.interrupt_id)).as_str(), 25, 37, Color::WHITE);
                
                self.draw_text(("ROMPTR: ".to_string() + &format!("{:06X}", (*self.mapper.borrow()).int_adapter.rom_ptr)).as_str(), 25, 39, Color::WHITE);

                self.draw_text(("CF: ".to_string() + &format!("{:01X}", self.cpu.get_flag(cpu::CARRY_FLAG) as u8)).as_str(), 44, 36, Color::WHITE);
                self.draw_text(("ZF: ".to_string() + &format!("{:01X}", self.cpu.get_flag(cpu::ZERO_FLAG) as u8)).as_str(), 44, 37, Color::WHITE);
                self.draw_text(("IF: ".to_string() + &format!("{:01X}", self.cpu.get_flag(cpu::IRQ_DISABLE_FLAG) as u8)).as_str(), 44, 38, Color::WHITE);
                self.draw_text(("DF: ".to_string() + &format!("{:01X}", self.cpu.get_flag(cpu::DEC_MODE_FLAG) as u8)).as_str(), 44, 39, Color::WHITE);
                
                self.draw_text(("BF: ".to_string() + &format!("{:01X}", self.cpu.get_flag(cpu::BREAK_FLAG) as u8)).as_str(), 52, 36, Color::WHITE);
                self.draw_text(("VF: ".to_string() + &format!("{:01X}", self.cpu.get_flag(cpu::OVERFLOW_FLAG) as u8)).as_str(), 52, 37, Color::WHITE);
                self.draw_text(("NF: ".to_string() + &format!("{:01X}", self.cpu.get_flag(cpu::NEGATIVE_FLAG) as u8)).as_str(), 52, 38, Color::WHITE);

                for i in 0 .. 7 {
                    self.memoryrow((self.cpu.pc & 0xfff0) + (i * 0x10), 4, 43 + i as u8);
                }

                for y in 0 .. ppu::INTERNAL_RESOLUTION_Y {
                    for x in 0 .. ppu::INTERNAL_RESOLUTION_X {
                        let ix = (x * 2) as f32;
                        let iy = (y * 2) as f32;
        
                        graphics.draw_quad(
                            [
                                Vector2::new(ix, iy), 
                                Vector2::new(ix + 2.0, iy),
                                Vector2::new(ix + 2.0, iy + 2.0), 
                                Vector2::new(ix, iy + 2.0)
                            ], 
                            *self.ppu.frame_buf.get(y as usize).unwrap()
                                .get(x as usize).unwrap()
                        );
                    }
                }
            }
            
            (*self.mapper.borrow_mut()).fbuf_changed = false;
        }

        self.timer = Instant::now();
        self.frame += 1;

        if self.do_sleep {
            sleep(self.sleep);
        }
        
        helper.request_redraw();
    }

    #[allow(unused)]
    fn on_key_down(
            &mut self, helper: &mut WindowHelper,
            virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
            scancode: speedy2d::window::KeyScancode
    ) {
        (*self.mapper.borrow_mut()).int_adapter.keyb         = scancode as u8;
        (*self.mapper.borrow_mut()).int_adapter.interrupt_id = interface_adapter::KEYDOWN;

        self.cpu.interrupt_request();
    }

    #[allow(unused)]
    fn on_key_up(
            &mut self,helper: &mut WindowHelper,
            virtual_key_code: Option<speedy2d::window::VirtualKeyCode>,
            scancode: speedy2d::window::KeyScancode
    ) {
        (*self.mapper.borrow_mut()).int_adapter.keyb         = scancode as u8;
        (*self.mapper.borrow_mut()).int_adapter.interrupt_id = interface_adapter::KEYUP;

        self.cpu.interrupt_request();
    }

    #[allow(unused)]
    fn on_mouse_move(&mut self, helper: &mut WindowHelper, position: speedy2d::dimen::Vec2) {
        (*self.mapper.borrow_mut()).int_adapter.mouse_x = (position.x / ppu::CHAR_X as f32) as u8;
        (*self.mapper.borrow_mut()).int_adapter.mouse_y = (position.y / ppu::CHAR_Y as f32) as u8;
    }

    #[allow(unused)]
    fn on_mouse_button_down(&mut self, helper: &mut WindowHelper, button: speedy2d::window::MouseButton) {
        match button {
            MouseButton::Left  => (*self.mapper.borrow_mut()).int_adapter.interrupt_id = interface_adapter::MOUSE_LCLICK,
            MouseButton::Right => (*self.mapper.borrow_mut()).int_adapter.interrupt_id = interface_adapter::MOUSE_RCLICK,
            _ => {}
        }

        self.cpu.interrupt_request();
    }
}

#[derive(Parser, Debug)]
struct Args {
    #[arg(short, long, default_value_t = TICKS_PER_FRAME)]
    ticks: u32,

    #[arg(short, long, default_value_t = String::from("none"))]
    cartridge: String,

    #[arg(short, long, default_value_t = DEFAULT_DELAY)]
    delay: f32,

    #[arg(long, default_value_t = UPDATE_EACH_CHANGED)]
    update_each_changed: u16,

    #[arg(long, default_value_t = FORCE_UPDATE_EACH)]
    update_each: u16,

    file: String
}

fn main() {
    let args = Args::parse();

    let map = Rc::new(RefCell::new(mapper::Map::new(args.file.as_str())));

    if args.cartridge.as_str() != "none" {
        (*map.borrow_mut()).int_adapter.load_cartridge(args.cartridge.as_str());
    }

    let mut cpu = cpu::CPU::new(Rc::clone(&map));
    cpu.reset();

    let (delay, do_sleep) = if args.delay == 0.0 { 
        (Duration::from_secs_f32(0.0), false)
    } else {
        (Duration::from_secs_f32(args.delay), true)
    };

    let emu = Emu {
        mapper: Rc::clone(&map), cpu, ticks: args.ticks, update_each_changed: args.update_each_changed,
        timer: Instant::now(), frame: 0, sleep: delay, do_sleep, changed_cnt: 0, update_each: args.update_each,
        ppu: ppu::PPU::new(Rc::clone(&map), "charset.bin")
    };

    let window = Window::new_with_options("6502 computer emulator", 
        WindowCreationOptions::new_windowed(
            WindowSize::PhysicalPixels(
                    Vector2::new(RESOLUTION_X as u32, RESOLUTION_Y as u32)
                ),
                Some(WindowPosition::Center)
            )
            .with_resizable(false)
            .with_vsync(false)
    ).unwrap();

    window.run_loop(emu);
}
