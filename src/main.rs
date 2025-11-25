// SANDBOX
//
// Different particles
// each particle can have one of a color contained in a given set
// each color has different propertys like mass, speed and relation to other colors
// like attract, push, delete, create

use std::{any, error::Error, sync::Arc};

use wgpu::{RequestAdapterError, RequestDeviceError, wgc::instance};
use winit::{application::ApplicationHandler, error::EventLoopError, event::{KeyEvent, WindowEvent}, event_loop::{self, ActiveEventLoop, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::{self, Window}};

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: wgpu::RenderPipeline,
    window: Arc<Window>
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            }).await?; 

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            }).await?;

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/vertex_shader.wgsl").into()),
        });

        let render_pipeline_layout = device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
                label: Some("Render Pipeline Layout"), 
                bind_group_layouts: &[], 
                push_constant_ranges: &[] 
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor { 
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout), 
            vertex: wgpu::VertexState { 
                module: &shader, 
                entry_point: Some("vs_main"), 
                compilation_options: wgpu::PipelineCompilationOptions::default(), 
                buffers: &[] 
            }, 
            fragment: Some(wgpu::FragmentState { 
                module: &shader, 
                entry_point: Some("fs_main"), 
                compilation_options: wgpu::PipelineCompilationOptions::default(), 
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState { 
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None, 
                front_face: wgpu::FrontFace::Ccw, 
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false, 
                polygon_mode: wgpu::PolygonMode::Fill, 
                conservative:  false
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { 
                count: 1, 
                mask: !0, 
                alpha_to_coverage_enabled: false 
            },
            multiview: None,
            cache: None, 
        });

        Ok(Self { 
            surface, 
            device, 
            queue, 
            config, 
            is_surface_configured: false, 
            render_pipeline,
            window 
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;

            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(())
        }

        let output = self.surface.get_current_texture()?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::wgt::CommandEncoderDescriptor { 
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())

    }

    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    fn update(&mut self) {}
}

pub struct App {
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>
}

impl App {
    pub fn new(event_loop: &EventLoop<State>) -> Self {
        let proxy = Some(event_loop.create_proxy());
    
        Self { 
            proxy, 
            state: None 
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &event_loop::ActiveEventLoop) {
        let window_attributes = Window::default_attributes();
        
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        self.state = Some(pollster::block_on(State::new(window)).unwrap());
    }

    fn window_event(&mut self, event_loop: &event_loop::ActiveEventLoop, window_id: window::WindowId, event: winit::event::WindowEvent) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {},
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        let size = state.window.inner_size();
                        state.resize(size.width, size.height);
                    },
                    Err(e) => {
                        log::error!("Unable to render {}", e);
                    }
                }
            },

            // please dont loose this
            // SEE ME!!!!
            // dont want to say how many times i lost this one line....
            // maybe put it into its own function... hmm..... no :D
            WindowEvent::KeyboardInput { 
                event: KeyEvent {
                    physical_key: PhysicalKey::Code(code),
                    state: key_state,
                    ..
                },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }
}


fn main() {
    env_logger::init();

    let event_loop = EventLoop::with_user_event().build().unwrap();

    let mut app = App::new(&event_loop);

    event_loop.run_app(&mut app).unwrap();
}

