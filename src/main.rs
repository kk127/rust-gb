use env_logger;
use rust_gb::cpu::Cpu;
use rust_gb::joypad;
// use sdl2::pixels::PixelFormatEnum;
use std::env;
use std::thread;
use std::time;

use log::{debug, info};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;

fn translate_keycode(key: Keycode) -> Option<joypad::Key> {
    match key {
        Keycode::Down => Some(joypad::Key::Down),
        Keycode::Up => Some(joypad::Key::Up),
        Keycode::Left => Some(joypad::Key::Left),
        Keycode::Right => Some(joypad::Key::Right),
        Keycode::Return => Some(joypad::Key::Start),
        Keycode::RShift => Some(joypad::Key::Select),
        Keycode::X => Some(joypad::Key::A),
        Keycode::Z => Some(joypad::Key::B),
        _ => None,
    }
}

/// Handles key down event.
fn handle_keydown(cpu: &mut Cpu, key: Keycode) {
    translate_keycode(key).map(|k| cpu.mmu.joypad.keydown(k));
}

/// Handles key up event.
fn handle_keyup(cpu: &mut Cpu, key: Keycode) {
    translate_keycode(key).map(|k| cpu.mmu.joypad.keyup(k));
}

fn main() {
    env::set_var("RUST_LOG", "info");
    env_logger::init();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        // .window("gbr", 960, 864)
        // .window("gbr", 160, 144)
        .window("rust-gameboy", 480, 432)
        // .window("gbr", 320, 288)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    let texture_creator = canvas.texture_creator();

    let mut texture = texture_creator
        .create_texture_streaming(sdl2::pixels::PixelFormatEnum::RGB24, 160, 144)
        .unwrap();
    let mut event_pump = sdl_context.event_pump().unwrap();

    // let mut cpu = Cpu::new("joypad.gb");
    // let mut cpu = Cpu::new("cpu_instrs.gb");
    // let mut cpu = Cpu::new("mem_timing.gb");
    // let mut cpu = Cpu::new("02-write_timing.gb");
    // let mut cpu = Cpu::new("meta_sprite.gb");
    // let mut cpu = Cpu::new("DONKEYKO.GB");
    // let mut cpu = Cpu::new("TETRIS.GB");
    // let mut cpu = Cpu::new("MARIOLAN.GB");
    // let mut cpu = Cpu::new("GAMEBOY.GB");
    // let mut cpu = Cpu::new("POKEMON.GB");
    // let mut cpu = Cpu::new("PM_CRYST.GBC");
    // let mut cpu = Cpu::new("YUGIOH.GB");
    let mut cpu = Cpu::new("POKEMON_.GB");
    // let mut cpu = Cpu::new("POKEMONRED.GB");
    // let mut cpu = Cpu::new("KIRBY'S.GB");
    // let mut cpu = Cpu::new("ZELDANA.GBC");

    let mut step_count: u64 = 0;

    'running: loop {
        // for _ in 0..1000 {
        // info!("loop");
        let now = time::Instant::now();
        let mut elapsed_tick: u32 = 0;

        // Emulate one frame
        while elapsed_tick < 456 * (144 + 10) {
            elapsed_tick += cpu.step() as u32;
            step_count += 1;
            debug!("==step_count: {}", step_count);
        }

        texture
            .with_lock(None, |buf: &mut [u8], pitch: usize| {
                let fb = cpu.mmu.ppu.get_frame();
                // println!("frame {}", fb.len());

                for y in 0..144 {
                    for x in 0..160 {
                        let offset = y * pitch + x * 3;
                        let color = fb[y * 160 + x];

                        buf[offset] = color;
                        buf[offset + 1] = color;
                        buf[offset + 2] = color;
                    }
                }
            })
            .unwrap();

        canvas.clear();
        canvas.copy(&texture, None, None).unwrap();
        canvas.present();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                } => handle_keydown(&mut cpu, keycode),
                Event::KeyUp {
                    keycode: Some(keycode),
                    ..
                } => handle_keyup(&mut cpu, keycode),
                _ => (),
            }
        }

        let wait = time::Duration::from_micros(1000000 / 60); // 1s / 59.73Hz * 10**6 = 16742.0056923 ms
        let elapsed = now.elapsed();

        if wait > elapsed {
            thread::sleep(wait - elapsed);
        }
    }
    cpu.mmu.cartridge.write_save_data();
}
