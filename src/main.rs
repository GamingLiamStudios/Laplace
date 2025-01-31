use std::sync::Arc;

use tracing::{
    event,
    info,
    instrument,
};
use wgpu::{
    Backends,
    Instance,
    InstanceDescriptor,
};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    platform::wayland::WindowAttributesExtWayland,
    window::{
        Window,
        WindowAttributes,
    },
};

mod config;

struct AppSurface {
    device: wgpu::Device,
    queue:  wgpu::Queue,

    config: wgpu::SurfaceConfiguration,

    // Drop these LAST to prevent segfault :3
    surface: wgpu::Surface<'static>,
    window:  Arc<Window>,
}

const WINDOW_WIDTH: u32 = 800;
const WINDOW_HEIGHT: u32 = 600;

impl AppSurface {
    pub fn new(
        instance: &wgpu::Instance,
        window: Window,
    ) -> Self {
        pollster::block_on(async {
            let window = Arc::new(window);
            let surface = instance.create_surface(window.clone()).expect("shitface");

            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    // TODO: Allow User Config
                    power_preference:       wgpu::PowerPreference::LowPower,
                    force_fallback_adapter: false,
                    compatible_surface:     Some(&surface),
                })
                .await
                .expect("shitface");
            let (device, queue) = adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label:             Some("AppSurfaceDevice"),
                        memory_hints:      wgpu::MemoryHints::MemoryUsage,
                        required_features: wgpu::Features::default(),
                        required_limits:   wgpu::Limits::default(), // TODO: WebGL2
                    },
                    None,
                )
                .await
                .expect("shitface");

            // TODO: Handle alternative color spaces
            let surface_capabilities = surface.get_capabilities(&adapter);
            let surface_format = surface_capabilities
                .formats
                .iter()
                .find(|format| format.is_srgb())
                .copied()
                .unwrap_or(surface_capabilities.formats[0]);

            Self {
                window,
                surface,
                device,
                queue,
                config: wgpu::SurfaceConfiguration {
                    usage:                         wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format:                        surface_format,
                    width:                         WINDOW_WIDTH,
                    height:                        WINDOW_HEIGHT,
                    present_mode:                  wgpu::PresentMode::Fifo, // VSync
                    alpha_mode:                    surface_capabilities.alpha_modes[0],
                    view_formats:                  vec![],
                    desired_maximum_frame_latency: 2,
                },
            }
        })
    }
}

struct App {
    instance: wgpu::Instance,

    surface: Option<AppSurface>,
}

impl App {
    pub fn new() -> Self {
        let instance = Instance::new(&InstanceDescriptor {
            backends: Backends::PRIMARY,
            ..Default::default()
        });

        Self {
            instance,
            surface: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
    ) {
        if self.surface.is_none() {
            let window = event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title("Laplace")
                        .with_name("floating", "floating")
                        .with_inner_size(PhysicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT)),
                )
                .expect("shitface");
            self.surface = Some(AppSurface::new(&self.instance, window));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(size) => {
                if let Some(window) = &mut self.surface {
                    window.config.width = size.width;
                    window.config.height = size.height;
                    window.surface.configure(&window.device, &window.config);
                }
            },
            WindowEvent::CloseRequested => {
                event_loop.exit();
            },
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.surface {
                    let output = window.surface.get_current_texture().expect("shitface");
                    let view = output
                        .texture
                        .create_view(&wgpu::TextureViewDescriptor::default());
                    let mut encoder =
                        window
                            .device
                            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: Some("Render Encoder"),
                            });

                    {
                        let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label:                    Some("Render Pass"),
                            color_attachments:        &[Some(wgpu::RenderPassColorAttachment {
                                view:           &view,
                                resolve_target: None,
                                ops:            wgpu::Operations {
                                    load:  wgpu::LoadOp::Clear(wgpu::Color {
                                        r: 0.1,
                                        g: 0.2,
                                        b: 0.3,
                                        a: 1.0,
                                    }),
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: None,
                            occlusion_query_set:      None,
                            timestamp_writes:         None,
                        });
                    }

                    // submit will accept anything that implements IntoIter
                    window.queue.submit(std::iter::once(encoder.finish()));
                    output.present();

                    window.window.request_redraw();
                }
            },
            _ => {
                // Ignore what we're not using
            },
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fmt_subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(fmt_subscriber)?;

    info!("Hello world!");

    // Ensure config is Loaded + Valid
    config::GLOBAL_CONFIG.write();

    let event_loop = winit::event_loop::EventLoop::new()?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll); // TODO: Investigate
    event_loop.run_app(&mut App::new())?;

    Ok(())
}
