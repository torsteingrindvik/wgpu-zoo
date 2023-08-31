use std::{path::Path, time::Duration};

use notify::{PollWatcher, Watcher};
use util::ExampleCommonState;
use wgpu::{
    Backends, Device, Features, Limits, PolygonMode, Queue, Surface, SurfaceConfiguration,
    TextureFormat,
};
use winit::{
    event::{
        DeviceEvent, ElementState, KeyboardInput, MouseScrollDelta, VirtualKeyCode, WindowEvent,
    },
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub mod util;

mod example_01;
mod example_02;
mod example_03;
mod example_04;

pub trait Example {
    // Keyboard
    fn handle_key(&mut self, _key: VirtualKeyCode) {}

    // Render!
    fn render(&mut self, data: &ExampleData);

    // Mouse scroll registered, either up or down
    fn handle_scroll(&mut self, _scroll_up: bool) {}

    // On mouse left click in clip space (-1., -1.) = bottom left to (1., 1.) = top right.
    // If `!pressed` that means released.
    fn handle_click(&mut self, _position: [f32; 2], _pressed: bool) {}

    // Used via main runner to:
    //  - increase example elapsed time
    //  - recreate shader (on file events) and mark dirty (for example to e.g. recreate pipeline)
    fn common(&mut self) -> &mut ExampleCommonState;
}

pub struct ExampleData {
    window: Window,
    device: Device,
    queue: Queue,
    surface: Surface,
    swapchain_format: TextureFormat,

    // For use in uniforms:
    mouse: [f32; 2],
    viewport: [f32; 2],
}

fn configure_surface(
    surface: &mut Surface,
    device: &Device,
    format: TextureFormat,
    window: &Window,
) -> [f32; 2] {
    let size = window.inner_size();
    let viewport = [size.width as f32, size.height as f32];

    surface.configure(
        &device,
        &SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        },
    );
    viewport
}

impl ExampleData {
    fn configure_surface(&mut self) {
        self.viewport = configure_surface(
            &mut self.surface,
            &self.device,
            self.swapchain_format,
            &self.window,
        );
    }
}

fn setup() -> (EventLoop<()>, ExampleData) {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::Window::new(&event_loop).unwrap();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: Backends::VULKAN,
        ..Default::default()
    });
    let mut surface = unsafe { instance.create_surface(&window).unwrap() };

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        compatible_surface: Some(&surface),
        ..Default::default()
    }))
    .unwrap();
    dbg!(adapter.get_info());

    let swapchain_capabilities = surface.get_capabilities(&adapter);
    dbg!(&swapchain_capabilities);
    let swapchain_formats = swapchain_capabilities.formats;
    dbg!(&swapchain_formats);
    let swapchain_format = swapchain_formats[0];

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("device-descr-setup"),
            features: Features::default()
                | Features::POLYGON_MODE_LINE
                | Features::POLYGON_MODE_POINT
                | Features::PUSH_CONSTANTS,
            limits: Limits {
                // https://docs.rs/wgpu/latest/wgpu/struct.Limits.html#structfield.max_push_constant_size
                // Seems this amount should be supported by all backends
                max_push_constant_size: 128,
                ..Default::default()
            },
        },
        Some(&Path::new("trace.txt")),
    ))
    .unwrap();

    dbg!(device.features());
    dbg!(device.limits());

    let viewport = configure_surface(&mut surface, &device, swapchain_format, &window);

    (
        event_loop,
        ExampleData {
            window,
            device,
            queue,
            surface,
            swapchain_format,
            mouse: [0., 0.],
            viewport,
        },
    )
}

fn main() {
    println!("[P]revious example\n[N]ext example");
    let (event_loop, mut example_data) = setup();

    let ex01 = example_01::Example01::new(&example_data);
    let ex02 = example_02::Example02::new(&example_data);
    let ex03 = example_03::Example03::new(&example_data);
    let ex04 = example_04::Example04::new(&example_data);

    let mut examples: Vec<Box<dyn Example>> = vec![
        Box::new(ex01),
        Box::new(ex02),
        Box::new(ex03),
        Box::new(ex04),
    ];
    let mut example_index = 2;
    let mut is_focused = true;

    let mut last_time = std::time::Instant::now();
    let mut one_second = 1.0f32;
    // let mut num_frames = 0;
    let mut num_renders_since_last_second = 0;

    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher =
        PollWatcher::new(tx, notify::Config::default().with_manual_polling()).unwrap();
    let recursive_dir = &Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    watcher
        .watch(recursive_dir, notify::RecursiveMode::Recursive)
        .unwrap();
    println!("Watching {recursive_dir:?} for file changes");

    event_loop.run(move |event, _, ctrl_flow| {
        let ex: &mut dyn Example = examples[example_index].as_mut();

        // Re-compile shaders if fs events on wgsl files happen
        watcher.poll().unwrap();
        while let Ok(res) = rx.try_recv() {
            match res {
                Ok(event) => {
                    println!("Changed: {event:?}");

                    if matches!(event.kind, notify::EventKind::Modify(_))
                        && event
                            .paths
                            .iter()
                            .any(|p| p.extension().unwrap_or_default() == "wgsl")
                    {
                        println!("wgsl changed, asking example to recompile shader");
                        let common = ex.common();
                        common.recreate_shader(&example_data.device);
                        common.dirty = true;
                    }
                }
                Err(e) => println!("Watch err: {e:?}"),
            }
        }

        // Update time, counters
        let now = std::time::Instant::now();
        let dt = now - last_time;
        // Example time update
        ex.common().increase_time(dt);
        last_time = now;

        *ctrl_flow = ControlFlow::WaitUntil(now + Duration::from_secs_f32(1. / 60.));

        // do a thing every second
        one_second -= dt.as_secs_f32();
        if one_second.is_sign_negative() {
            one_second = 1.0;
            println!("fps: {num_renders_since_last_second}");
            num_renders_since_last_second = 0;
        }

        use winit::event::Event;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *ctrl_flow = ControlFlow::Exit;
                return;
            }

            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state,
                                virtual_keycode: Some(virtual_keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                // Only act on presses, not releases
                if state == ElementState::Released {
                    return;
                }

                let common = ex.common();
                match virtual_keycode {
                    VirtualKeyCode::Up | VirtualKeyCode::W => {
                        common.polygon_mode = match common.polygon_mode {
                            PolygonMode::Fill => PolygonMode::Fill,
                            PolygonMode::Line => PolygonMode::Fill,
                            PolygonMode::Point => PolygonMode::Line,
                        };
                        common.dirty = true;
                    }
                    VirtualKeyCode::Down | VirtualKeyCode::S => {
                        common.polygon_mode = match common.polygon_mode {
                            PolygonMode::Fill => PolygonMode::Line,
                            PolygonMode::Line => PolygonMode::Point,
                            PolygonMode::Point => PolygonMode::Point,
                        };
                        common.dirty = true;
                    }

                    VirtualKeyCode::Escape => {
                        *ctrl_flow = ControlFlow::Exit;
                        return;
                    }
                    // [P]revious example
                    VirtualKeyCode::P => {
                        example_index = example_index.saturating_sub(1);
                        return;
                    }
                    // [N]ext example
                    VirtualKeyCode::N => {
                        example_index = (example_index + 1).min(examples.len() - 1);
                        return;
                    }
                    _ => {}
                }

                ex.handle_key(virtual_keycode);
            }

            Event::WindowEvent {
                event: WindowEvent::AxisMotion { .. },
                ..
            }
            | Event::WindowEvent {
                event: WindowEvent::Occluded(_),
                ..
            } => {
                // spammy
            }

            Event::WindowEvent {
                event: WindowEvent::CursorLeft { .. },
                ..
            } => {
                is_focused = false;
            }

            Event::WindowEvent {
                event: WindowEvent::CursorEntered { .. },
                ..
            } => {
                is_focused = true;
            }

            Event::WindowEvent {
                event: WindowEvent::Resized(_),
                ..
            } => {
                example_data.configure_surface();
            }

            Event::WindowEvent {
                event: WindowEvent::CursorMoved { position, .. },
                ..
            } => {
                // Set mouse position to the -1..1 range using wgpu's coordinate system,
                // i.e. origin middle of screen, top right is (1., 1.)

                let x = (position.x / example_data.window.inner_size().width as f64)
                    .max(0.0)
                    .min(1.0)
                    * 2.
                    - 1.;
                let y = (position.y / example_data.window.inner_size().height as f64)
                    .max(0.0)
                    .min(1.0)
                    * -2.
                    + 1.0;

                example_data.mouse = [x as f32, y as f32];
            }

            // Event::NewEvents(_) => todo!(),
            // Event::WindowEvent { window_id, event } => todo!(),
            // Event::DeviceEvent { device_id, event } => todo!(),
            // Event::UserEvent(_) => todo!(),
            // Event::Suspended => todo!(),
            // Event::Resumed => todo!(),
            // Event::MainEventsCleared => todo!(),
            // Event::RedrawRequested(_) => todo!(),
            // Event::LoopDestroyed => todo!(),
            Event::RedrawRequested(_) | Event::RedrawEventsCleared => {
                // Render!
                ex.render(&example_data);
                num_renders_since_last_second += 1;
            }

            Event::DeviceEvent {
                event:
                    DeviceEvent::MouseWheel {
                        delta: MouseScrollDelta::LineDelta(_horizontal, vertical),
                    },
                ..
            } => {
                if vertical > 0.5 {
                    ex.handle_scroll(true)
                } else if vertical < -0.5 {
                    ex.handle_scroll(false)
                }
            }

            Event::DeviceEvent {
                event: DeviceEvent::Button { button: 1, state },
                ..
            } => {
                ex.handle_click(example_data.mouse, state == ElementState::Pressed);
            }

            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { .. } | DeviceEvent::Motion { .. },
                ..
            }
            | Event::MainEventsCleared
            | Event::NewEvents(..) => {
                // Verbose, don't print
            }

            e => {
                let mut should_print = true;
                if !is_focused {
                    if let Event::DeviceEvent {
                        event: DeviceEvent::Key { .. },
                        ..
                    } = e
                    {
                        should_print = false;
                    }
                }

                if let Event::WindowEvent {
                    event: WindowEvent::Moved(_),
                    ..
                } = e
                {
                    should_print = false;
                };

                if should_print {
                    println!("Event: {e:?}");
                }
            }
        }
    });
}
