use chrono;
use gameboy_emulator::{
    error::{Error, HardwareError, Result},
    interface::{audio::AudioInterface, input::simple_joypad::SimpleJoypad, video::PixelsDisplay},
    GameBoy,
};
use std::fs::{self, File};
use std::io::Read;
use std::time::{Duration, Instant};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

const TARGET_FPS: u64 = 1;
const FRAME_TIME: Duration = Duration::from_nanos(1_000_000_000 / TARGET_FPS);

// Simple audio interface implementation
#[derive(Debug)]
struct DummyAudio;

impl AudioInterface for DummyAudio {
    fn push_sample(&mut self, _sample: f32) {}
    fn start(&mut self) {}
    fn stop(&mut self) {}
}

fn main() -> Result<()> {
    // Initialize logs
    initialize_logs()?;

    // Get ROM path from command line arguments or use default
    let args: Vec<String> = std::env::args().collect();
    let rom_path = if args.len() > 1 {
        &args[1]
    } else {
        "rom/tetris.gb"
    };

    // Ensure ROM file exists
    if !std::path::Path::new(rom_path).exists() {
        eprintln!("ROM file not found: {}", rom_path);
        eprintln!("Usage: {} [rom_file]", args[0]);
        eprintln!("Example: {} rom/tetris.gb", args[0]);
        return Err(Error::Hardware(HardwareError::Custom(
            "ROM file not found".to_string(),
        )));
    }

    println!("Loading ROM: {}", rom_path);

    // Load ROM
    let mut rom_file = File::open(rom_path).map_err(|e| {
        Error::Hardware(HardwareError::Custom(format!("Failed to open ROM: {}", e)))
    })?;

    let mut rom_data = Vec::new();
    rom_file.read_to_end(&mut rom_data).map_err(|e| {
        Error::Hardware(HardwareError::Custom(format!("Failed to read ROM: {}", e)))
    })?;

    // Validate ROM size
    if rom_data.len() < 0x8000 {
        eprintln!("ROM file too small: {} bytes", rom_data.len());
        return Err(Error::Hardware(HardwareError::Custom(
            "ROM file too small".to_string(),
        )));
    } // Print ROM header information
    println!(
        "ROM Title: {}",
        String::from_utf8_lossy(&rom_data[0x134..0x144])
    );
    println!("ROM Size: {} bytes", rom_data.len());
    println!("Cartridge Type: 0x{:02X}", rom_data[0x147]);

    // Initialize window and display
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Game Boy Emulator")
        .with_inner_size(LogicalSize::new(160.0 * 3.0, 144.0 * 3.0))
        .with_resizable(true)
        .build(&event_loop)
        .map_err(|e| Error::Hardware(HardwareError::Display(e.to_string())))?;

    let video = PixelsDisplay::new(&window)
        .map_err(|e| Error::Hardware(HardwareError::Display(e.to_string())))?;

    // Initialize Game Boy
    let mut gameboy = GameBoy::new(Box::new(video), Some(Box::new(DummyAudio)))?;

    // Load ROM and start simulation
    println!("Loading ROM...");
    gameboy.load_rom(rom_data)?;
    println!("ROM loaded successfully");

    let joypad = SimpleJoypad::new();
    let mut last_frame = Instant::now();
    let mut fps_timer = Instant::now();
    let mut frames = 0;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(new_size),
                ..
            } => {
                // Handle window resizing
                gameboy
                    .get_video_mut()
                    .resize(new_size.width, new_size.height)
                    .ok();
            }
            Event::MainEventsCleared => {
                let now = Instant::now();
                let elapsed = now.duration_since(last_frame);

                // FPS calculation
                frames += 1;
                if fps_timer.elapsed() >= Duration::from_secs(1) {
                    window.set_title(&format!("Game Boy Emulator - {} FPS", frames));
                    frames = 0;
                    fps_timer = now;
                }

                // Frame timing
                if elapsed >= FRAME_TIME {
                    // Update joypad state
                    if let Err(e) = gameboy.update_joypad_state(&joypad) {
                        eprintln!("Failed to update joypad: {}", e);
                    }

                    // Run emulation
                    if let Err(e) = gameboy.step() {
                        eprintln!("Error during emulation: {}", e);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    // Render frame
                    if let Err(e) = gameboy.render() {
                        eprintln!("Error during render: {}", e);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    window.request_redraw();
                    last_frame = now;
                } else {
                    // Wait for next frame
                    *control_flow = ControlFlow::WaitUntil(last_frame + FRAME_TIME);
                }
            }
            _ => (),
        }
    });

    #[allow(unreachable_code)]
    Ok(())
}

fn initialize_logs() -> Result<()> {
    println!("Creating log directory and files...");
    let _ = fs::create_dir_all("logs");

    // Initialize log files
    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("logs/emulator.log")
    {
        writeln!(file, "[{}] Emulator started", chrono::Local::now())?;
    }

    Ok(())
}
