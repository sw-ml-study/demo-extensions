//! Opt-in native wgpu/winit smoke viewer for a generic line-scene JSON file.

use std::error::Error;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use mlpl_eval::{Value, run_applet_with_host};
use mlpl_native3d_scene::{Camera, LineScene, Viewport};
use mlpl_native3d_window::live::{
    applet_source, close_event, key_event, parse_scene_command, resize_event,
};
use mlpl_native3d_window::{GpuVertex, line_vertices, text_vertices};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, WindowEvent},
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
    println!("MLPL Native 3D — live controls are evaluated by controls.mlpl");
    println!("W/S width • arrows height • A/D length • +/- speed • Space pause");
    println!("C color • [/] thickness • R reset • Escape closes");
    let mut host_error = None;
    let result = run_applet_with_host(&applet_source(), |commands, events| {
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
    graphics: Option<Graphics>,
    commands: Receiver<Value>,
    events: Sender<Value>,
    angle: f32,
    rotation_speed: f32,
    help: String,
    last_frame: Instant,
}

impl Application {
    fn new(commands: Receiver<Value>, events: Sender<Value>) -> Self {
        Self {
            scene: None,
            graphics: None,
            commands,
            events,
            angle: 0.0,
            rotation_speed: 0.0,
            help: String::new(),
            last_frame: Instant::now(),
        }
    }

    fn drain_commands(&mut self, event_loop: &ActiveEventLoop) {
        while let Ok(value) = self.commands.try_recv() {
            match parse_scene_command(value) {
                Ok(command) => {
                    if let Some(graphics) = &self.graphics {
                        graphics.window.set_title(&scene_title(&command));
                    }
                    self.scene = Some(command.scene);
                    self.rotation_speed = command.rotation_speed;
                    self.help = command.help;
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
            WindowEvent::RedrawRequested => {
                self.drain_commands(event_loop);
                let now = Instant::now();
                self.angle +=
                    now.duration_since(self.last_frame).as_secs_f32() * self.rotation_speed;
                self.last_frame = now;
                let Some(graphics) = self.graphics.as_mut() else {
                    return;
                };
                let Some(scene) = self.scene.as_ref() else {
                    graphics.window.request_redraw();
                    return;
                };
                if let Err(error) = graphics.render(scene, self.angle, &self.help) {
                    match error {
                        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                            graphics.reconfigure();
                        }
                        wgpu::SurfaceError::OutOfMemory => event_loop.exit(),
                        wgpu::SurfaceError::Timeout | wgpu::SurfaceError::Other => {}
                    }
                }
                graphics.window.request_redraw();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.drain_commands(event_loop);
        if let Some(graphics) = &self.graphics {
            graphics.window.request_redraw();
        }
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
            "[" | "{" => Some("bracket_left"),
            "]" | "}" => Some("bracket_right"),
            _ => None,
        },
        _ => None,
    }
}

fn scene_title(command: &mlpl_native3d_window::live::SceneCommand) -> String {
    let positions = command.scene.positions().values();
    let width = positions[0].abs() * 2.0;
    let height = positions[1].abs() * 2.0;
    let length = positions[2].abs() * 2.0;
    format!(
        "MLPL live • rev {} • W {:.2} H {:.2} L {:.2} • speed {:.1} • Space/C/[/]/R",
        command.revision, width, height, length, command.rotation_speed
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
        angle: f32,
        help: &str,
    ) -> Result<(), wgpu::SurfaceError> {
        let Ok(viewport) = Viewport::new(self.configuration.width, self.configuration.height)
        else {
            return Ok(());
        };
        let Ok(lines) = scene.plan_lines(Camera::default(), viewport, angle) else {
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
    use super::normalize_key;
    use winit::keyboard::{Key, NamedKey};

    #[test]
    fn normalizes_named_space_for_the_live_event_path() {
        assert_eq!(normalize_key(&Key::Named(NamedKey::Space)), Some("space"));
    }
}
