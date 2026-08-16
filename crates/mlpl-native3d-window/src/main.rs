//! Opt-in native wgpu/winit smoke viewer for a generic line-scene JSON file.

use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use mlpl_eval::{Value, run_applet_with_host};
use mlpl_native3d_scene::{Camera, LineScene, Viewport};
use mlpl_native3d_window::interaction::{
    BoundedInput, FrameGate, InputError, InputEvent, Modifiers, PointerButton, PointerButtons,
};
use mlpl_native3d_window::live::{
    applet_source, close_event, input_event, key_event, life_torus_applet_source, resize_event,
    tic_tac_toe_applet_source,
};
use mlpl_native3d_window::{GpuVertex, line_vertices, text_vertices};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

const SHADER: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
";

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let arguments: Vec<_> = std::env::args().collect();
    let tic_tac_toe = arguments.iter().any(|argument| argument == "--tic-tac-toe");
    let life = arguments.iter().any(|argument| argument == "--life");
    let life_torus = arguments.iter().any(|argument| argument == "--life-torus");
    let source = if tic_tac_toe {
        tic_tac_toe_applet_source()
    } else if life_torus {
        life_torus_applet_source()
    } else if life {
        mlpl_native3d_window::live::life_applet_source()
    } else {
        applet_source()
    };
    println!("MLPL Native 3D — application behavior is evaluated by MLPL");
    let mut host_error = None;
    let result = run_applet_with_host(&source, |commands, events| {
        let mut application = Application::new(commands, events);
        if let Err(error) = event_loop.run_app(&mut application) {
            host_error = Some(error.to_string());
        }
    });
    if let Some(error) = host_error {
        return Err(error.into());
    }
    result.map_err(|error| format!("MLPL applet failed: {error}"))?;
    Ok(())
}

struct Application {
    scene: Option<LineScene>,
    retained_scene: Option<mlpl_native3d_window::live::RetainedScene>,
    graphics: Option<Graphics>,
    commands: Receiver<Value>,
    events: Sender<Value>,
    angle: f32,
    rotation_speed: f32,
    camera: Camera,
    help: String,
    pointer_position: [f64; 2],
    pointer_buttons: PointerButtons,
    modifiers: Modifiers,
    pending_input: BoundedInput,
    frame_gate: FrameGate,
    started: Instant,
    last_frame: Instant,
}

impl Application {
    fn new(commands: Receiver<Value>, events: Sender<Value>) -> Self {
        Self {
            scene: None,
            retained_scene: None,
            graphics: None,
            commands,
            events,
            angle: 0.0,
            rotation_speed: 0.0,
            camera: Camera::default(),
            help: String::new(),
            pointer_position: [0.0; 2],
            pointer_buttons: PointerButtons::NONE,
            modifiers: Modifiers::NONE,
            pending_input: BoundedInput::new(64).expect("nonzero input capacity"),
            frame_gate: FrameGate::new(),
            started: Instant::now(),
            last_frame: Instant::now(),
        }
    }

    fn drain_commands(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(value) = self.commands.try_recv() {
            match mlpl_native3d_window::live::parse_live_command(value) {
                Ok(mlpl_native3d_window::live::LiveCommand::Scene(command)) => {
                    if let Some(graphics) = &self.graphics {
                        graphics.window.set_title(&scene_title(&command));
                    }
                    match mlpl_native3d_window::live::RetainedScene::from_scene_command(&command) {
                        Ok(retained) => self.retained_scene = Some(retained),
                        Err(error) => {
                            eprintln!("MLPL retained scene rejected: {error}");
                            event_loop.exit();
                            return;
                        }
                    }
                    self.scene = Some(command.scene);
                    self.camera = command.camera;
                    self.rotation_speed = command.rotation_speed;
                    self.help = command.help;
                }
                Ok(mlpl_native3d_window::live::LiveCommand::Patch(command)) => {
                    let Some(retained) = self.retained_scene.as_mut() else {
                        eprintln!("MLPL scene patch arrived before a complete scene");
                        event_loop.exit();
                        return;
                    };
                    if let Err(error) = retained.apply(&command) {
                        eprintln!("MLPL scene patch rejected: {error}");
                        self.send(mlpl_native3d_window::live::resync_event(), event_loop);
                        continue;
                    }
                    self.scene = Some(retained.scene().clone());
                }
                Ok(mlpl_native3d_window::live::LiveCommand::View(command)) => {
                    if let Some(retained) = self.retained_scene.as_mut()
                        && let Err(error) = retained.apply_view_revision(command.revision)
                    {
                        eprintln!("MLPL retained view rejected: {error}");
                        event_loop.exit();
                        return;
                    }
                    self.camera = command.camera;
                    self.help = command.help;
                }
                Ok(mlpl_native3d_window::live::LiveCommand::FrameAck(_revision)) => {
                    self.frame_gate.acknowledge();
                }
                Err(error) => {
                    eprintln!("MLPL scene command rejected: {error}");
                    event_loop.exit();
                    return;
                }
            }
        }
    }

    fn send(&self, event: Value, event_loop: &ActiveEventLoop) {
        if self.events.send(event).is_err() {
            event_loop.exit();
        }
    }

    fn flush_input(&mut self, event_loop: &ActiveEventLoop) {
        for event in self.pending_input.drain() {
            self.send(input_event(event), event_loop);
        }
    }

    fn queue_input(&mut self, event: InputEvent, event_loop: &ActiveEventLoop) {
        match self.pending_input.push(event) {
            Ok(()) => {}
            Err(InputError::Full) => {
                self.flush_input(event_loop);
                if self.pending_input.push(event).is_err() {
                    event_loop.exit();
                }
            }
            Err(InputError::InvalidCapacity | InputError::NonFinite) => event_loop.exit(),
        }
    }

    fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_commands(event_loop);
        let now = Instant::now();
        let elapsed = now.duration_since(self.started);
        let delta = now.duration_since(self.last_frame);
        self.flush_input(event_loop);
        if self.frame_gate.begin() {
            self.send(
                input_event(InputEvent::frame(
                    delta.as_secs_f64() * 1000.0,
                    elapsed.as_secs_f64() * 1000.0,
                )),
                event_loop,
            );
        }
        self.angle += delta.as_secs_f32() * self.rotation_speed;
        self.last_frame = now;
        let Some(graphics) = self.graphics.as_mut() else {
            return;
        };
        let Some(scene) = self.scene.as_ref() else {
            graphics.window.request_redraw();
            return;
        };
        if let Err(error) = graphics.render(scene, self.camera, self.angle, &self.help) {
            match error {
                wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => graphics.reconfigure(),
                wgpu::SurfaceError::OutOfMemory => event_loop.exit(),
                wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other => {}
            }
        }
        graphics.window.request_redraw();
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("MLPL Native 3D — interactive controls run in MLPL")
            .with_inner_size(LogicalSize::new(900, 700));
        match event_loop.create_window(attributes) {
            Ok(window) => match pollster::block_on(Graphics::new(Arc::new(window))) {
                Ok(graphics) => self.graphics = Some(graphics),
                Err(error) => {
                    eprintln!("native graphics initialization failed: {error}");
                    event_loop.exit();
                }
            },
            Err(error) => {
                eprintln!("native window creation failed: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.send(close_event(), event_loop);
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key,
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } => {
                if let Some(key) = normalize_key(&logical_key) {
                    self.send(key_event(key), event_loop);
                    if key == "escape" {
                        self.send(close_event(), event_loop);
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(graphics) = self.graphics.as_mut() {
                    graphics.resize(size.width, size.height);
                }
                self.send(resize_event(size.width, size.height), event_loop);
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer_position = [position.x, position.y];
                self.queue_input(
                    InputEvent::pointer_move(
                        self.pointer_position,
                        self.pointer_buttons,
                        self.modifiers,
                    ),
                    event_loop,
                );
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some((button, flag)) = normalize_button(button) {
                    let pressed = state == ElementState::Pressed;
                    self.pointer_buttons = self.pointer_buttons.with(flag, pressed);
                    self.queue_input(
                        InputEvent::pointer_button(
                            button,
                            pressed,
                            self.pointer_position,
                            self.modifiers,
                        ),
                        event_loop,
                    );
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.queue_input(
                    InputEvent::wheel(
                        normalize_wheel(delta),
                        self.pointer_position,
                        self.modifiers,
                    ),
                    event_loop,
                );
            }
            WindowEvent::ModifiersChanged(value) => {
                let state = value.state();
                self.modifiers = Modifiers::from_flags([
                    state.shift_key(),
                    state.control_key(),
                    state.alt_key(),
                    state.super_key(),
                ]);
            }
            WindowEvent::RedrawRequested => {
                self.redraw(event_loop);
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_commands(event_loop);
        self.flush_input(event_loop);
        if let Some(graphics) = &self.graphics {
            graphics.window.request_redraw();
        }
    }
}

fn normalize_button(button: MouseButton) -> Option<(PointerButton, PointerButtons)> {
    match button {
        MouseButton::Left => Some((PointerButton::Left, PointerButtons::LEFT)),
        MouseButton::Middle => Some((PointerButton::Middle, PointerButtons::MIDDLE)),
        MouseButton::Right => Some((PointerButton::Right, PointerButtons::RIGHT)),
        _ => None,
    }
}

fn normalize_wheel(delta: MouseScrollDelta) -> [f64; 2] {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => [f64::from(x) * 40.0, f64::from(y) * 40.0],
        MouseScrollDelta::PixelDelta(position) => [position.x, position.y],
    }
}

fn normalize_key(key: &Key) -> Option<&'static str> {
    match key {
        Key::Named(NamedKey::ArrowUp) => Some("arrow_up"),
        Key::Named(NamedKey::ArrowDown) => Some("arrow_down"),
        Key::Named(NamedKey::Escape) => Some("escape"),
        Key::Named(NamedKey::Space) => Some("space"),
        Key::Character(value) => match value.to_lowercase().as_str() {
            "w" => Some("w"),
            "s" => Some("s"),
            "a" => Some("a"),
            "d" => Some("d"),
            "=" | "+" => Some("equal"),
            "-" | "_" => Some("minus"),
            " " => Some("space"),
            "r" => Some("r"),
            "c" => Some("c"),
            "b" => Some("b"),
            "g" => Some("g"),
            "h" => Some("h"),
            "i" => Some("i"),
            "n" => Some("n"),
            "t" => Some("t"),
            "u" => Some("u"),
            "x" => Some("x"),
            "o" => Some("o"),
            "1" => Some("1"),
            "2" => Some("2"),
            "[" | "{" => Some("bracket_left"),
            "]" | "}" => Some("bracket_right"),
            _ => None,
        },
        _ => None,
    }
}

fn scene_title(command: &mlpl_native3d_window::live::SceneCommand) -> String {
    let summary = command.help.lines().next().unwrap_or("MLPL native3d");
    format!(
        "MLPL native3d • rev {} • {} lines • {}",
        command.revision,
        command.scene.edges().len(),
        summary
    )
}

struct Graphics {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    configuration: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
}

impl Graphics {
    async fn new(window: Arc<Window>) -> Result<Self, Box<dyn Error>> {
        let instance = wgpu::Instance::default();
        let surface = instance.create_surface(window.clone())?;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .ok_or("no compatible native GPU adapter")?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("mlpl native3d device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await?;
        let size = window.inner_size();
        let configuration = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface has no supported configuration")?;
        surface.configure(&device, &configuration);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mlpl native3d line shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mlpl native3d line pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x4],
                }],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: configuration.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview: None,
            cache: None,
        });
        Ok(Self {
            window,
            surface,
            device,
            queue,
            configuration,
            pipeline,
        })
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.configuration.width = width;
        self.configuration.height = height;
        self.reconfigure();
    }

    fn reconfigure(&self) {
        self.surface.configure(&self.device, &self.configuration);
    }

    fn render(
        &mut self,
        scene: &LineScene,
        camera: Camera,
        angle: f32,
        help: &str,
    ) -> Result<(), wgpu::SurfaceError> {
        let Ok(viewport) = Viewport::new(self.configuration.width, self.configuration.height)
        else {
            return Ok(());
        };
        let Ok(lines) = scene.plan_lines(camera, viewport, angle) else {
            return Ok(());
        };
        let mut vertices = line_vertices(&lines, viewport);
        vertices.extend(text_vertices(help, viewport));
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mlpl native3d line vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let frame = self.surface.get_current_texture()?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("mlpl native3d frame encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mlpl native3d frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 8.0 / 255.0,
                            g: 10.0 / 255.0,
                            b: 16.0 / 255.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..u32::try_from(vertices.len()).unwrap_or(u32::MAX), 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_button, normalize_key, normalize_wheel};
    use mlpl_native3d_window::interaction::{PointerButton, PointerButtons};
    use winit::dpi::PhysicalPosition;
    use winit::event::{MouseButton, MouseScrollDelta};
    use winit::keyboard::{Key, NamedKey};

    #[test]
    fn normalizes_named_space_for_the_live_event_path() {
        assert_eq!(normalize_key(&Key::Named(NamedKey::Space)), Some("space"));
        for key in ["b", "g", "h", "i", "n", "t", "u"] {
            assert_eq!(normalize_key(&Key::Character(key.into())), Some(key));
        }
    }

    #[test]
    fn normalizes_platform_pointer_buttons_and_wheel_units() {
        assert_eq!(
            normalize_button(MouseButton::Middle),
            Some((PointerButton::Middle, PointerButtons::MIDDLE))
        );
        let line = normalize_wheel(MouseScrollDelta::LineDelta(1.0, -2.0));
        assert!((line[0] - 40.0).abs() < f64::EPSILON);
        assert!((line[1] + 80.0).abs() < f64::EPSILON);
        let pixel = normalize_wheel(MouseScrollDelta::PixelDelta(PhysicalPosition::new(
            3.0, 4.0,
        )));
        assert!((pixel[0] - 3.0).abs() < f64::EPSILON);
        assert!((pixel[1] - 4.0).abs() < f64::EPSILON);
    }
}
