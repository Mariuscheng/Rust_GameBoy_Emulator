use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use env_logger::Builder;
use log::{error, info};
use pixels::{Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::VirtualKeyCode;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

use gameboy_emulator::config::{AudioConfig, SystemConfig, VideoConfig};
use gameboy_emulator::{ConfigBuilder, Emulator, Error, Result};

const SCREEN_WIDTH: u32 = 160;
const SCREEN_HEIGHT: u32 = 144;

// Setup logger
fn setup_logger() -> Result<()> {
    let mut builder = Builder::from_default_env();
    builder.filter_level(log::LevelFilter::Info);

    let log_path = PathBuf::from("logs");
    if !log_path.exists() {
        std::fs::create_dir(&log_path)?;
    }

    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path.join(format!(
            "debug_{}.log",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        )))?;

    builder.target(env_logger::Target::Pipe(Box::new(log_file)));
    builder.init();

    info!("Logger initialized");
    Ok(())
}

// Setup panic hook
fn setup_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        error!("Fatal error occurred: {}", info);
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/panic.log")
            .and_then(|mut file| writeln!(file, "Panic: {}", info));
    }));
}

fn main() -> Result<()> {
    // 最小化的錯誤處理設置
    setup_logger()?;
    setup_panic_hook(); // 載入 ROM 並初始化
    let rom_path = PathBuf::from("rom/tetris.gb");
    info!("檢查 ROM 文件: {}", rom_path.display());
    if !rom_path.exists() {
        error!("ROM 文件不存在: {}", rom_path.display());
        return Err(Error::Other(format!(
            "ROM 文件不存在: {}",
            rom_path.display()
        )));
    }
    info!("ROM 文件存在，開始初始化模擬器");

    let config = ConfigBuilder::new()
        .system_config(SystemConfig {
            debug_mode: false,
            rom_path,
            ..Default::default()
        })
        .video_config(VideoConfig {
            scale: 3,
            ..Default::default()
        })
        .audio_config(AudioConfig {
            enable_sound: true,
            ..Default::default()
        })
        .build();
    info!("開始初始化視窗...");

    // 創建視窗和模擬器
    let event_loop = EventLoop::new();
    let window = {
        let size = LogicalSize::new(
            SCREEN_WIDTH as f64 * config.video.scale as f64,
            SCREEN_HEIGHT as f64 * config.video.scale as f64,
        );
        WindowBuilder::new()
            .with_title("Game Boy - Tetris")
            .with_inner_size(size)
            .with_min_inner_size(size)
            .build(&event_loop)
            .map_err(|e| Error::Other(format!("Failed to create window: {}", e)))?
    };

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let mut pixels = Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface_texture)
        .map_err(|e| Error::Other(format!("Failed to initialize display: {}", e)))?;

    // 創建並初始化模擬器
    let mut emulator = Emulator::new(config)?;
    let frame_duration = Duration::from_secs_f64(1.0 / 59.73);
    let mut last_frame = Instant::now();

    // 事件循環
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } if window_id == window.id() => {
                *control_flow = ControlFlow::Exit;
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::KeyboardInput { input, .. },
            } if window_id == window.id() => {
                let is_pressed = input.state == ElementState::Pressed;
                if let Some(virtual_code) = input.virtual_keycode {
                    // Handle ESC key first
                    if virtual_code == VirtualKeyCode::Escape && is_pressed {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    // Handle other game controls
                    match virtual_code {
                        VirtualKeyCode::Up => emulator.set_button_up(is_pressed),
                        VirtualKeyCode::Down => emulator.set_button_down(is_pressed),
                        VirtualKeyCode::Left => emulator.set_button_left(is_pressed),
                        VirtualKeyCode::Right => emulator.set_button_right(is_pressed),
                        VirtualKeyCode::Z => emulator.set_button_a(is_pressed),
                        VirtualKeyCode::X => emulator.set_button_b(is_pressed),
                        VirtualKeyCode::Space => emulator.set_button_select(is_pressed),
                        VirtualKeyCode::Return => emulator.set_button_start(is_pressed),
                        _ => (),
                    }
                }
            }
            Event::MainEventsCleared => {
                // 根據時間差來控制更新速度
                let now = Instant::now();
                if now.duration_since(last_frame) >= frame_duration {
                    last_frame = now;

                    // 更新遊戲狀態
                    if let Err(e) = emulator.step() {
                        error!("模擬器步進失敗：{}", e);
                        *control_flow = ControlFlow::Exit;
                        return;
                    }
                    info!("模擬器執行了一步");

                    // 更新畫面
                    if let Ok(frame) = emulator.get_frame() {
                        if frame.len() == (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize {
                            pixels.frame_mut().copy_from_slice(&frame);
                            if let Err(e) = pixels.render() {
                                error!("無法渲染畫面：{}", e);
                                *control_flow = ControlFlow::Exit;
                            }
                        } else {
                            error!(
                                "frame_buffer 大小不正確：預期 {} 位元組，實際 {} 位元組",
                                SCREEN_WIDTH * SCREEN_HEIGHT * 4,
                                frame.len(),
                            );
                            *control_flow = ControlFlow::Exit;
                        }
                    } else {
                        error!("無法取得下一幀");
                        *control_flow = ControlFlow::Exit;
                    }
                }
                window.request_redraw();
            }
            _ => (),
        }
    })
}
