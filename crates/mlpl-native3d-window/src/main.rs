//! Opt-in native wgpu/winit viewer for generic line and point scenes.

use std::error::Error;
use std::io::Read;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Instant;

use mlpl_eval::{Value, run_applet_with_host};
use mlpl_native3d_scene::{Camera, LineScene, PointLimits, PointScene, Viewport};
use mlpl_native3d_window::interaction::{
    BoundedInput, FrameGate, InputError, InputEvent, Modifiers, PointerButton, PointerButtons,
};
use mlpl_native3d_window::live::{
    applet_source, close_event, input_event, key_event, life_torus_applet_source,
    model_atlas_applet_source, resize_event, tic_tac_toe_applet_source,
};
use mlpl_native3d_window::{
    GpuPointVertex, GpuVertex, line_vertices, point_vertices, text_vertices, text_vertices_colored,
};
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

const POINT_SHADER: &str = r"
struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) local: vec2<f32>,
    @location(3) stable_id: vec2<u32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.color = input.color;
    output.local = input.local;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    if dot(input.local, input.local) > 1.0 {
        discard;
    }
    return input.color;
}
";

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let arguments: Vec<_> = std::env::args().collect();
    let tic_tac_toe = arguments.iter().any(|argument| argument == "--tic-tac-toe");
    let life = arguments.iter().any(|argument| argument == "--life");
    let life_torus = arguments.iter().any(|argument| argument == "--life-torus");
    let model_atlas = arguments.iter().any(|argument| argument == "--model-atlas");
    let model_atlas_file = arguments
        .iter()
        .position(|argument| argument == "--model-atlas-file")
        .and_then(|index| arguments.get(index + 1))
        .map(std::path::PathBuf::from);
    let disk_usage = arguments
        .iter()
        .position(|argument| argument == "--disk-usage")
        .and_then(|index| arguments.get(index + 1))
        .map(std::path::PathBuf::from);
    let audio_spectrum = arguments
        .iter()
        .position(|argument| argument == "--audio-spectrum")
        .and_then(|index| arguments.get(index + 1))
        .map(std::path::PathBuf::from);
    let weight_distribution = arguments
        .iter()
        .position(|argument| argument == "--weight-distribution")
        .and_then(|index| arguments.get(index + 1))
        .map(std::path::PathBuf::from);
    let point_scene = arguments
        .iter()
        .position(|argument| argument == "--point-scene")
        .and_then(|index| arguments.get(index + 1))
        .map(|path| load_point_scene(std::path::Path::new(path)))
        .transpose()?;
    let source = if tic_tac_toe {
        tic_tac_toe_applet_source()
    } else if let Some(root) = disk_usage.as_ref() {
        let snapshot = mlpl_native3d_window::disk_usage::capture_snapshot(
            root,
            mlpl_native3d_window::disk_usage::SnapshotBudgets {
                max_entries: 256,
                max_depth: 16,
            },
        )?;
        mlpl_native3d_window::live::disk_usage_applet_source(&snapshot)
    } else if let Some(root) = audio_spectrum.as_ref() {
        let paths = mlpl_native3d_window::audio::discover_audio_paths(root, 128)?;
        mlpl_native3d_window::live::audio_spectrum_applet_source(&paths)
    } else if let Some(root) = weight_distribution.as_ref() {
        let paths = mlpl_native3d_window::model_files::discover_model_paths(root, 64)?;
        mlpl_native3d_window::live::weight_distribution_applet_source(&paths)
    } else if model_atlas_file.is_some() {
        mlpl_native3d_window::live::model_atlas_file_applet_source()
    } else if model_atlas {
        model_atlas_applet_source()
    } else if life_torus {
        life_torus_applet_source()
    } else if life {
        mlpl_native3d_window::live::life_applet_source()
    } else {
        applet_source()
    };
    println!("MLPL Native 3D — application behavior is evaluated by MLPL");
    let mut host_error = None;
    let rooted = disk_usage
        .or(model_atlas_file)
        .or(audio_spectrum)
        .or(weight_distribution);
    if let Some(root) = rooted {
        let mut application = Application::new_supervised(source, root)?;
        application.point_scene = point_scene;
        event_loop.run_app(&mut application)?;
        return Ok(());
    }
    let result = {
        run_applet_with_host(&source, |commands, events| {
            let mut application = Application::new(commands, events);
            application.point_scene = point_scene;
            if let Err(error) = event_loop.run_app(&mut application) {
                host_error = Some(error.to_string());
            }
        })
    };
    if let Some(error) = host_error {
        return Err(error.into());
    }
    result.map_err(|error| format!("MLPL applet failed: {error}"))?;
    Ok(())
}

fn load_point_scene(path: &std::path::Path) -> Result<PointScene, Box<dyn Error>> {
    const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;
    let mut source = String::new();
    std::fs::File::open(path)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_string(&mut source)?;
    if u64::try_from(source.len()).unwrap_or(u64::MAX) > MAX_FILE_BYTES {
        return Err("point scene file exceeds 64 MiB".into());
    }
    let limits = PointLimits::new(1_000_000, 44_000_000)
        .map_err(|error| format!("point limits rejected: {error:?}"))?;
    PointScene::parse(&source, limits)
        .map_err(|error| format!("point scene rejected: {error:?}").into())
}

struct Application {
    scene: Option<LineScene>,
    point_scene: Option<PointScene>,
    retained_scene: Option<mlpl_native3d_window::live::RetainedScene>,
    retained_point_scene: Option<mlpl_native3d_window::live::RetainedPointScene>,
    graphics: Option<Graphics>,
    commands: Receiver<Value>,
    events: Sender<Value>,
    angle: f32,
    rotation_speed: f32,
    camera: Camera,
    help: String,
    status: String,
    pointer_position: [f64; 2],
    pointer_buttons: PointerButtons,
    modifiers: Modifiers,
    pending_input: BoundedInput,
    frame_gate: FrameGate,
    started: Instant,
    last_frame: Instant,
    fs_root: Option<std::path::PathBuf>,
    audio: Option<mlpl_native3d_window::audio::PcmStream>,
    audio_playing: bool,
    audio_in_flight: bool,
    audio_next: Instant,
    audio_output: Option<mlpl_native3d_window::audio::PcmOutput>,
    audio_visual_pending: Option<mlpl_native3d_window::audio::PcmChunk>,
    worker_result: Option<Receiver<Result<Value, mlpl_eval::EvalError>>>,
    restart: Option<(String, std::path::PathBuf)>,
    worker_error: Option<String>,
}

impl Application {
    fn new(commands: Receiver<Value>, events: Sender<Value>) -> Self {
        Self::new_rooted(commands, events, None)
    }

    fn new_rooted(
        commands: Receiver<Value>,
        events: Sender<Value>,
        fs_root: Option<std::path::PathBuf>,
    ) -> Self {
        let now = Instant::now();
        Self {
            scene: None,
            point_scene: None,
            retained_scene: None,
            retained_point_scene: None,
            graphics: None,
            commands,
            events,
            angle: 0.0,
            rotation_speed: 0.0,
            camera: Camera::default(),
            help: String::new(),
            status: String::new(),
            pointer_position: [0.0; 2],
            pointer_buttons: PointerButtons::NONE,
            modifiers: Modifiers::NONE,
            pending_input: BoundedInput::new(64).expect("nonzero input capacity"),
            frame_gate: FrameGate::new(),
            started: now,
            last_frame: now,
            fs_root,
            audio: None,
            audio_playing: false,
            audio_in_flight: false,
            audio_next: now,
            audio_output: None,
            audio_visual_pending: None,
            worker_result: None,
            restart: None,
            worker_error: None,
        }
    }

    fn new_supervised(source: String, root: std::path::PathBuf) -> Result<Self, Box<dyn Error>> {
        let worker = mlpl_native3d_window::live::spawn_rooted_applet(&source, &root)?;
        let mut application = Self::new_rooted(worker.commands, worker.events, Some(root.clone()));
        application.worker_result = Some(worker.result);
        application.restart = Some((source, root));
        Ok(application)
    }

    fn enter_worker_error(&mut self, message: &str) {
        eprintln!("MLPL applet failed: {message}");
        self.worker_error = Some(message.to_string());
        self.help = format!(
            "MLPL WORKER ERROR — WINDOW RETAINED\nR RESTARTS A FRESH MLPL ENVIRONMENT | ESC QUITS\n\n{message}"
        );
        self.status = format!("ERROR: {message}");
        self.rotation_speed = 0.0;
        self.retained_scene = None;
        self.retained_point_scene = None;
        self.scene = LineScene::from_arrays(
            vec![
                -2.0, -2.0, 0.0, 2.0, 2.0, 0.0, -2.0, 2.0, 0.0, 2.0, -2.0, 0.0,
            ],
            vec![0, 1, 2, 3],
            0.0,
            [1.0, 0.1, 0.1, 1.0],
            5.0,
        )
        .ok();
        if let Some(graphics) = &self.graphics {
            graphics
                .window
                .set_title("MLPL worker error — R restart | Esc quit");
            graphics.window.request_redraw();
        }
    }

    fn poll_worker(&mut self) {
        let Some(result) = self.worker_result.as_ref() else {
            return;
        };
        match result.try_recv() {
            Ok(Ok(_)) => self.enter_worker_error("worker stopped without closing the window"),
            Ok(Err(error)) => self.enter_worker_error(&error.to_string()),
            Err(std::sync::mpsc::TryRecvError::Disconnected) if self.worker_error.is_none() => {
                self.enter_worker_error("worker result channel disconnected");
            }
            Err(
                std::sync::mpsc::TryRecvError::Empty | std::sync::mpsc::TryRecvError::Disconnected,
            ) => {}
        }
    }

    fn restart_worker(&mut self) {
        let Some((source, root)) = self.restart.clone() else {
            return;
        };
        match mlpl_native3d_window::live::spawn_rooted_applet(&source, &root) {
            Ok(worker) => {
                self.commands = worker.commands;
                self.events = worker.events;
                self.worker_result = Some(worker.result);
                self.worker_error = None;
                self.scene = None;
                self.retained_scene = None;
                self.retained_point_scene = None;
                self.pending_input = BoundedInput::new(64).expect("nonzero input capacity");
                self.frame_gate = FrameGate::new();
                self.help = "RESTARTING FRESH MLPL ENVIRONMENT...".into();
                self.status.clear();
            }
            Err(error) => self.enter_worker_error(&format!("restart failed: {error}")),
        }
    }

    fn drain_commands(&mut self, event_loop: &ActiveEventLoop) {
        self.poll_worker();
        if self.worker_error.is_some() {
            return;
        }
        while let Ok(value) = self.commands.try_recv() {
            match mlpl_native3d_window::live::parse_live_command(value) {
                Ok(mlpl_native3d_window::live::LiveCommand::Scene(command)) => {
                    if !self.set_line_scene(command, event_loop) {
                        return;
                    }
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
                Ok(mlpl_native3d_window::live::LiveCommand::PointScene(command)) => {
                    if !self.set_point_scene(command, event_loop) {
                        return;
                    }
                }
                Ok(mlpl_native3d_window::live::LiveCommand::PointPatch(command)) => {
                    self.patch_point_scene(&command, event_loop);
                }
                Ok(mlpl_native3d_window::live::LiveCommand::View(command)) => {
                    if let Some(retained) = self.retained_scene.as_mut() {
                        if let Err(error) =
                            retained.apply_view(command.revision, command.rotation_speed)
                        {
                            eprintln!("MLPL retained view rejected: {error}");
                            event_loop.exit();
                            return;
                        }
                        self.scene = Some(retained.scene().clone());
                    }
                    if let Some(retained) = self.retained_point_scene.as_mut()
                        && let Err(error) = retained.apply_view(command.revision)
                    {
                        eprintln!("MLPL retained point view rejected: {error}");
                        event_loop.exit();
                        return;
                    }
                    if let Some(rotation_speed) = command.rotation_speed {
                        self.rotation_speed = rotation_speed;
                    }
                    self.camera = command.camera;
                    self.help = command.help;
                    self.status = command.status;
                }
                Ok(mlpl_native3d_window::live::LiveCommand::FrameAck(_revision)) => {
                    self.frame_gate.acknowledge();
                }
                Ok(mlpl_native3d_window::live::LiveCommand::AudioOpen(path)) => {
                    self.open_audio(&path, event_loop);
                }
                Ok(mlpl_native3d_window::live::LiveCommand::AudioPlay(playing)) => {
                    self.audio_playing = playing;
                    if let Some(output) = self.audio_output.as_ref() {
                        let result = if playing {
                            output.play()
                        } else {
                            output.pause()
                        };
                        if let Err(error) = result {
                            eprintln!("audio output state change failed: {error}");
                        }
                    }
                    self.audio_next = Instant::now();
                }
                Ok(mlpl_native3d_window::live::LiveCommand::AudioSeek(seconds)) => {
                    if let Some(audio) = self.audio.as_mut()
                        && let Err(error) = audio.seek_seconds(seconds)
                    {
                        eprintln!("audio seek rejected: {error}");
                    }
                    self.audio_in_flight = false;
                    self.audio_visual_pending = None;
                    self.audio_next = Instant::now();
                    if let Some(output) = self.audio_output.as_ref() {
                        output.clear();
                    }
                }
                Ok(mlpl_native3d_window::live::LiveCommand::AudioAck) => {
                    self.audio_in_flight = false;
                }
                Err(error) => {
                    eprintln!("MLPL scene command rejected: {error}");
                    event_loop.exit();
                    return;
                }
            }
        }
    }

    fn set_line_scene(
        &mut self,
        command: mlpl_native3d_window::live::SceneCommand,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        if let Some(graphics) = &self.graphics {
            graphics.window.set_title(&scene_title(&command));
        }
        match mlpl_native3d_window::live::RetainedScene::from_scene_command(&command) {
            Ok(retained) => self.retained_scene = Some(retained),
            Err(error) => {
                eprintln!("MLPL retained scene rejected: {error}");
                event_loop.exit();
                return false;
            }
        }
        self.scene = Some(command.scene);
        self.camera = command.camera;
        self.rotation_speed = command.rotation_speed;
        self.help = command.help;
        self.status = command.status;
        true
    }

    fn set_point_scene(
        &mut self,
        command: mlpl_native3d_window::live::PointSceneCommand,
        event_loop: &ActiveEventLoop,
    ) -> bool {
        match mlpl_native3d_window::live::RetainedPointScene::from_command(&command) {
            Ok(retained) => self.retained_point_scene = Some(retained),
            Err(error) => {
                eprintln!("MLPL retained point scene rejected: {error}");
                event_loop.exit();
                return false;
            }
        }
        self.point_scene = Some(command.scene);
        self.camera = command.camera;
        self.help = command.help;
        self.status = command.status;
        true
    }

    fn patch_point_scene(
        &mut self,
        command: &mlpl_native3d_window::live::PointPatchCommand,
        event_loop: &ActiveEventLoop,
    ) {
        let Some(retained) = self.retained_point_scene.as_mut() else {
            eprintln!("MLPL point patch arrived before a complete point scene");
            event_loop.exit();
            return;
        };
        if let Err(error) = retained.apply(command) {
            eprintln!("MLPL point patch rejected: {error}");
            self.send(mlpl_native3d_window::live::resync_event(), event_loop);
            return;
        }
        self.point_scene = Some(retained.scene().clone());
    }

    fn send_point_selection(&mut self, event_loop: &ActiveEventLoop) {
        let revision = self
            .retained_point_scene
            .as_ref()
            .map_or(0, mlpl_native3d_window::live::RetainedPointScene::revision);
        let selection = self
            .point_scene
            .as_ref()
            .zip(self.graphics.as_ref().and_then(Graphics::viewport))
            .and_then(|(scene, viewport)| {
                mlpl_native3d_window::live::point_selection_event(
                    scene,
                    self.camera,
                    viewport,
                    self.angle,
                    self.pointer_position,
                    revision,
                )
                .ok()
            });
        if let Some(selection) = selection {
            self.send(selection, event_loop);
        }
    }

    fn open_audio(&mut self, path: &str, event_loop: &ActiveEventLoop) {
        let Some(root) = self.fs_root.as_ref() else {
            eprintln!("audio open requires a confined filesystem root");
            return;
        };
        let Ok(candidate) = root.join(path).canonicalize() else {
            eprintln!("audio source does not exist");
            return;
        };
        if !candidate.starts_with(root) {
            eprintln!("audio source escapes the confined root");
            return;
        }
        let limits = mlpl_native3d_window::audio::DecodeLimits {
            max_frames_per_chunk: 1024,
            max_channels: 2,
        };
        match mlpl_native3d_window::audio::PcmStream::open(&candidate, limits) {
            Ok(stream) => self.start_audio(stream),
            Err(error) => self.send(
                mlpl_native3d_window::live::audio_error_event(format!(
                    "DECODER REJECTED SOURCE: {error}"
                )),
                event_loop,
            ),
        }
    }

    fn start_audio(&mut self, stream: mlpl_native3d_window::audio::PcmStream) {
        self.audio_output =
            match mlpl_native3d_window::audio::PcmOutput::open(stream.sample_rate_hz(), 8192) {
                Ok(output) => {
                    if let Err(error) = output.play() {
                        eprintln!("audio output could not start: {error}");
                    }
                    Some(output)
                }
                Err(error) => {
                    eprintln!("audio output unavailable: {error}");
                    None
                }
            };
        self.audio = Some(stream);
        self.audio_playing = true;
        self.audio_in_flight = false;
        self.audio_visual_pending = None;
        self.audio_next = Instant::now();
    }

    fn send(&mut self, event: Value, _event_loop: &ActiveEventLoop) {
        if self.events.send(event).is_err() {
            self.poll_worker();
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
        if self.audio_playing {
            let decode_budget = if self.audio_output.is_some() { 4 } else { 1 };
            for _ in 0..decode_budget {
                let ready = self
                    .audio_output
                    .as_ref()
                    .map_or(!self.audio_in_flight && now >= self.audio_next, |output| {
                        output.queued_frames() < 4096
                    });
                if !ready {
                    break;
                }
                let Some(audio) = self.audio.as_mut() else {
                    break;
                };
                match audio.next_chunk() {
                    Ok(Some(chunk)) => {
                        if self.audio_output.is_none() {
                            let frames = u32::try_from(chunk.left.len()).unwrap_or(u32::MAX);
                            self.audio_next = now
                                + std::time::Duration::from_secs_f64(
                                    f64::from(frames) / f64::from(chunk.sample_rate_hz),
                                );
                        }
                        if let Some(output) = self.audio_output.as_ref() {
                            output.enqueue(&chunk);
                        }
                        self.audio_visual_pending = Some(chunk);
                    }
                    Ok(None) => {
                        self.audio_playing = false;
                        break;
                    }
                    Err(error) => {
                        eprintln!("audio decode failed: {error}");
                        self.audio_playing = false;
                        break;
                    }
                }
            }
            if !self.audio_in_flight
                && let Some(chunk) = self.audio_visual_pending.take()
            {
                self.audio_in_flight = true;
                self.send(
                    mlpl_native3d_window::live::audio_chunk_event(&chunk),
                    event_loop,
                );
            }
        }
        self.angle += delta.as_secs_f32() * self.rotation_speed;
        self.last_frame = now;
        let Some(graphics) = self.graphics.as_mut() else {
            return;
        };
        if self.scene.is_none() && self.point_scene.is_none() {
            graphics.window.request_redraw();
            return;
        }
        if let Err(error) = graphics.render(
            self.scene.as_ref(),
            self.point_scene.as_ref(),
            self.camera,
            self.angle,
            &self.help,
            &self.status,
        ) {
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
                    if self.worker_error.is_some() {
                        if key == "r" {
                            self.restart_worker();
                        } else if key == "escape" {
                            event_loop.exit();
                        }
                        return;
                    }
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
                    if button == PointerButton::Left && !pressed {
                        self.send_point_selection(event_loop);
                    }
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
        Key::Named(NamedKey::ArrowLeft) => Some("arrow_left"),
        Key::Named(NamedKey::ArrowRight) => Some("arrow_right"),
        Key::Named(NamedKey::Escape) => Some("escape"),
        Key::Named(NamedKey::Space) => Some("space"),
        Key::Named(NamedKey::Enter) => Some("enter"),
        Key::Named(NamedKey::Backspace | NamedKey::Delete) => Some("backspace"),
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
            "j" => Some("j"),
            "k" => Some("k"),
            "l" => Some("l"),
            "m" => Some("m"),
            "n" => Some("n"),
            "p" => Some("p"),
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
    point_pipeline: wgpu::RenderPipeline,
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
        let point_pipeline = create_point_pipeline(&device, configuration.format);
        Ok(Self {
            window,
            surface,
            device,
            queue,
            configuration,
            pipeline,
            point_pipeline,
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

    fn viewport(&self) -> Option<Viewport> {
        Viewport::new(self.configuration.width, self.configuration.height).ok()
    }

    fn render(
        &mut self,
        scene: Option<&LineScene>,
        point_scene: Option<&PointScene>,
        camera: Camera,
        angle: f32,
        help: &str,
        status: &str,
    ) -> Result<(), wgpu::SurfaceError> {
        let Ok(viewport) = Viewport::new(self.configuration.width, self.configuration.height)
        else {
            return Ok(());
        };
        let mut vertices = scene
            .and_then(|scene| scene.plan_lines(camera, viewport, angle).ok())
            .map_or_else(Vec::new, |lines| line_vertices(&lines, viewport));
        let point_vertices = point_scene
            .and_then(|scene| scene.plan_points(camera, viewport, angle).ok())
            .map_or_else(Vec::new, |plan| point_vertices(plan.points(), viewport));
        vertices.extend(text_vertices(help, viewport));
        let help_line_count = help.lines().fold(0.0_f32, |count, _| count + 1.0);
        let status_y = 14.0 + help_line_count * 18.0;
        vertices.extend(text_vertices_colored(
            status,
            viewport,
            [14.0, status_y],
            [1.0, 0.9, 0.15, 1.0],
        ));
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mlpl native3d line vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let point_buffer = (!point_vertices.is_empty()).then(|| {
            self.device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("mlpl native3d point vertices"),
                    contents: bytemuck::cast_slice(&point_vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                })
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
            if let Some(point_buffer) = point_buffer.as_ref() {
                pass.set_pipeline(&self.point_pipeline);
                pass.set_vertex_buffer(0, point_buffer.slice(..));
                pass.draw(
                    0..u32::try_from(point_vertices.len()).unwrap_or(u32::MAX),
                    0..1,
                );
            }
            pass.set_pipeline(&self.pipeline);
            pass.set_vertex_buffer(0, vertex_buffer.slice(..));
            pass.draw(0..u32::try_from(vertices.len()).unwrap_or(u32::MAX), 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

fn create_point_pipeline(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("mlpl native3d point shader"),
        source: wgpu::ShaderSource::Wgsl(POINT_SHADER.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("mlpl native3d point pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<GpuPointVertex>() as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x2,
                    1 => Float32x4,
                    2 => Float32x2,
                    3 => Uint32x2
                ],
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
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{Application, load_point_scene, normalize_button, normalize_key, normalize_wheel};
    use mlpl_native3d_window::interaction::{PointerButton, PointerButtons};
    use winit::dpi::PhysicalPosition;
    use winit::event::{MouseButton, MouseScrollDelta};
    use winit::keyboard::{Key, NamedKey};

    #[test]
    fn bundled_point_scene_loads_through_the_bounded_smoke_path() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/native3d-point-scene.json");
        let scene = load_point_scene(&path).unwrap();
        assert_eq!(scene.len(), 7);
        assert_eq!(scene.ids(), [101, 102, 103, 104, 105, 106, 107]);
    }

    #[test]
    fn normalizes_named_space_for_the_live_event_path() {
        assert_eq!(normalize_key(&Key::Named(NamedKey::Space)), Some("space"));
        assert_eq!(
            normalize_key(&Key::Named(NamedKey::Backspace)),
            Some("backspace")
        );
        assert_eq!(
            normalize_key(&Key::Named(NamedKey::Delete)),
            Some("backspace")
        );
        assert_eq!(
            normalize_key(&Key::Named(NamedKey::ArrowLeft)),
            Some("arrow_left")
        );
        assert_eq!(
            normalize_key(&Key::Named(NamedKey::ArrowRight)),
            Some("arrow_right")
        );
        for key in [
            "a", "b", "g", "h", "i", "j", "k", "l", "n", "p", "s", "t", "u",
        ] {
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

    #[test]
    fn terminal_worker_error_becomes_a_retained_restart_screen() {
        let (_command_tx, command_rx) = std::sync::mpsc::channel();
        let (event_tx, _event_rx) = std::sync::mpsc::channel();
        let mut application = Application::new(command_rx, event_tx);
        application.enter_worker_error("expected a string value");
        assert!(application.scene.is_some());
        assert!(application.help.contains("WINDOW RETAINED"));
        assert!(application.help.contains("R RESTARTS"));
        assert!(application.help.contains("ESC QUITS"));
        assert!(application.status.contains("expected a string value"));
    }
}
