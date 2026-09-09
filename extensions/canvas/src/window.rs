//! Blocking winit/wgpu presentation kept separate from ABI validation.

use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use crate::CanvasScene;

const SHADER: &str = r"
struct Input { @location(0) position: vec2<f32>, @location(1) color: vec4<f32> };
struct Output { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32> };
@vertex fn vs_main(input: Input) -> Output { var out: Output; out.position=vec4<f32>(input.position,0.0,1.0); out.color=input.color; return out; }
@fragment fn fs_main(input: Output) -> @location(0) vec4<f32> { return input.color; }
";

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

pub fn show(scene: CanvasScene) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|error| error.to_string())?;
    let mut app = CanvasApp {
        scene,
        graphics: None,
        error: None,
    };
    event_loop
        .run_app(&mut app)
        .map_err(|error| error.to_string())?;
    app.error.map_or(Ok(()), Err)
}

struct CanvasApp {
    scene: CanvasScene,
    graphics: Option<Graphics>,
    error: Option<String>,
}

impl CanvasApp {
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: &str) {
        self.error = Some(message.to_owned());
        event_loop.exit();
    }
}

impl ApplicationHandler for CanvasApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.graphics.is_some() {
            return;
        }
        let attributes = Window::default_attributes()
            .with_title(&self.scene.title)
            .with_inner_size(LogicalSize::new(self.scene.width, self.scene.height));
        let result = event_loop
            .create_window(attributes)
            .map(Arc::new)
            .map_err(|error| error.to_string())
            .and_then(|window| pollster::block_on(Graphics::new(window)));
        match result {
            Ok(graphics) => {
                graphics.window.request_redraw();
                self.graphics = Some(graphics);
            }
            Err(error) => self.fail(event_loop, &error),
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(graphics) = self.graphics.as_mut() {
                    graphics.resize(size.width, size.height);
                    graphics.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(graphics) = self.graphics.as_mut() {
                    if let Err(error) = graphics.render(&self.scene) {
                        match error {
                            wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => {
                                graphics.reconfigure();
                            }
                            wgpu::SurfaceError::OutOfMemory => {
                                self.fail(event_loop, "native canvas GPU memory exhausted");
                            }
                            _ => {}
                        }
                    }
                }
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
    config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
}

impl Graphics {
    async fn new(window: Arc<Window>) -> Result<Self, String> {
        let instance = wgpu::Instance::default();
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| e.to_string())?;
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
                    label: Some("MLPL canvas device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| e.to_string())?;
        let size = window.inner_size();
        let config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("surface has no supported configuration")?;
        surface.configure(&device, &config);
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MLPL canvas shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MLPL canvas pipeline"),
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x2,1=>Float32x4],
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
                    format: config.format,
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
            config,
            pipeline,
        })
    }
    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.reconfigure();
        }
    }
    fn reconfigure(&self) {
        self.surface.configure(&self.device, &self.config);
    }
    fn render(&mut self, scene: &CanvasScene) -> Result<(), wgpu::SurfaceError> {
        let vertices = vertices(scene, self.config.width, self.config.height);
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("MLPL canvas vertices"),
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
                label: Some("MLPL canvas frame"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MLPL canvas pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.025,
                            g: 0.035,
                            b: 0.065,
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
            pass.set_vertex_buffer(0, buffer.slice(..));
            pass.draw(0..u32::try_from(vertices.len()).unwrap_or(u32::MAX), 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

#[allow(
    clippy::cast_precision_loss,
    reason = "canvas dimensions are bounded to 8192 and exactly represented by f32"
)]
fn vertices(scene: &CanvasScene, width: u32, height: u32) -> Vec<Vertex> {
    let mut output = Vec::with_capacity(scene.line_count() * 6);
    let aspect = height as f32 / width as f32;
    for index in 0..scene.line_count() {
        let a = scene.points[index];
        let b = scene.points[index + 1];
        let dx = (b[0] - a[0]) / aspect;
        let dy = b[1] - a[1];
        let length = dx.hypot(dy);
        if length <= f32::EPSILON {
            continue;
        }
        let radius = scene.thicknesses[index] / height as f32;
        let offset = [-dy / length * radius * aspect, dx / length * radius];
        let color = scene.colors[index];
        let v = |point: [f32; 2]| Vertex {
            position: point,
            color,
        };
        let c = [
            v([a[0] + offset[0], a[1] + offset[1]]),
            v([a[0] - offset[0], a[1] - offset[1]]),
            v([b[0] + offset[0], b[1] + offset[1]]),
            v([b[0] - offset[0], b[1] - offset[1]]),
        ];
        output.extend_from_slice(&[c[0], c[1], c[2], c[2], c[1], c[3]]);
    }
    output
}
