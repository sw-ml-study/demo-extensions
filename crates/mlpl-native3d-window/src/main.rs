//! Opt-in native wgpu/winit smoke viewer for a generic line-scene JSON file.

use std::{error::Error, fs, sync::Arc, time::Instant};

use mlpl_native3d_scene::{Camera, LineScene, Viewport};
use mlpl_native3d_window::{GpuVertex, line_vertices};
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
    let scene_path = std::env::args()
        .nth(1)
        .ok_or("usage: mlpl-native3d-window SCENE.json")?;
    let scene = LineScene::parse(&fs::read_to_string(scene_path)?)
        .map_err(|error| format!("invalid line scene: {error:?}"))?;
    println!("MLPL Native 3D PoC: generic line scene; Escape or window close exits.");
    println!("Cube-specific controls remain MLPL-owned and require the upstream live API.");

    let event_loop = EventLoop::new()?;
    let mut application = Application::new(scene);
    event_loop.run_app(&mut application)?;
    Ok(())
}

struct Application {
    scene: LineScene,
    graphics: Option<Graphics>,
    started: Instant,
}

impl Application {
    fn new(scene: LineScene) -> Self {
        Self {
            scene,
            graphics: None,
            started: Instant::now(),
        }
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title("MLPL Native 3D PoC — MLPL scene • Esc closes")
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
        let Some(graphics) = self.graphics.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested
            | WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: Key::Named(NamedKey::Escape),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => event_loop.exit(),
            WindowEvent::Resized(size) => graphics.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let elapsed = self.started.elapsed().as_secs_f32();
                let angle = elapsed * self.scene.controls().rotation_speed();
                if let Err(error) = graphics.render(&self.scene, angle) {
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

    fn render(&mut self, scene: &LineScene, angle: f32) -> Result<(), wgpu::SurfaceError> {
        let Ok(viewport) = Viewport::new(self.configuration.width, self.configuration.height)
        else {
            return Ok(());
        };
        let Ok(lines) = scene.plan_lines(Camera::default(), viewport, angle) else {
            return Ok(());
        };
        let vertices = line_vertices(&lines, viewport);
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
