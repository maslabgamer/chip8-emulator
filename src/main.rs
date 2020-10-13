mod chip8;

use chip8::Chip8;
use std::fs;
use device_query::{DeviceState, DeviceQuery};
use minifb::{Window, WindowOptions, Key, Scale, ScaleMode};

const WIDTH: usize = 64;
const HEIGHT: usize = 32;

fn main() {
    // Set up window
    let mut buffer: Vec<u32> = vec![0; WIDTH * HEIGHT];
    let mut window = Window::new(
        "Test - ESC to exit",
        WIDTH,
        HEIGHT,
        WindowOptions {
            borderless: false,
            transparency: false,
            title: true,
            resize: false,
            scale: Scale::X16,
            scale_mode: ScaleMode::Stretch,
            topmost: false,
        },
    )
        .unwrap_or_else(|e| {
            panic!("{}", e);
        });
    // Limit to 60 ticks per second
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    // Set up keyboard
    let device_state = DeviceState::new();

    // Set up render system and register input callbacks
    let mut chip8 = Chip8::new();

    // Initialize the Chip8 system and load the game into memory
    let program = load_program();
    chip8.load_program(&program);

    // Emulation loop
    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Emulate one cycle
        chip8.emulate_cycle();

        chip8.draw_to_buffer(&mut buffer);

        // Store key press state (Press and Release)
        chip8.set_keys(device_state.get_keys());
        window.update_with_buffer(&buffer, WIDTH, HEIGHT).unwrap();
    };
}

fn load_program() -> Vec<u8> {
    let program = fs::read("roms/pong.rom");
    match program {
        Ok(program_loaded) => program_loaded,
        Err(error) => panic!("Could not load program!\n{}", error)
    }
}
