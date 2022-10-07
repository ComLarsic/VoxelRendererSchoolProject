use crate::window::Window;

/// The wgpu context for rendering
pub struct RenderContext {
    pub instance: wgpu::Instance,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface,
    pub surface_config: wgpu::SurfaceConfiguration,

    // The egui render pass
    pub egui_pass: egui_wgpu_backend::RenderPass,
}

impl RenderContext {
    /// Construct a new [`Renderer`]
    pub async fn new(window: &Window) -> anyhow::Result<Self> {
        // Create a wgpu instance
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        // Create the surface
        let surface = window.create_surface(&instance);
        // Request the adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .ok_or(anyhow::anyhow!("Failed to request the adapter."))?;
        // Request the device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    features: wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
                    limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    label: None,
                },
                None,
            )
            .await?;

        // Get the surface format
        let surface_format = surface.get_supported_formats(&adapter)[0];
        // Create the surface config
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_DST,
            format: surface_format,
            width: window.size().0,
            height: window.size().1,
            present_mode: wgpu::PresentMode::AutoNoVsync,
        };
        surface.configure(&device, &surface_config);

        // Create the egui render pass
        let egui_pass = egui_wgpu_backend::RenderPass::new(&device, surface_format, 1);

        Ok(Self {
            instance,
            device,
            queue,
            surface,
            surface_config,
            egui_pass,
        })
    }

    /// Render to the screen
    pub fn render(
        &mut self,
        window: &Window,
        full_output: egui::FullOutput,
        paint_jobs: Vec<egui::ClippedPrimitive>,
    ) -> anyhow::Result<()> {
        // Get the output texture
        let output = self.surface.get_current_texture()?;
        // Create the output view
        let view = output.texture.create_view(&Default::default());

        // Create the command encoder
        let mut encoder = self.device.create_command_encoder(&Default::default());
        // Upload all the egui resources to the gpu
        let screen_descriptor = egui_wgpu_backend::ScreenDescriptor {
            physical_width: window.size().0,
            physical_height: window.size().1,
            scale_factor: 1.0,
        };
        // Add the textures to the render pass
        let tdelta = full_output.textures_delta;
        self.egui_pass
            .add_textures(&self.device, &self.queue, &tdelta)?;
        self.egui_pass
            .update_buffers(&self.device, &self.queue, &paint_jobs, &screen_descriptor);

        // Execute the render pass
        self.egui_pass.execute(
            &mut encoder,
            &view,
            &paint_jobs,
            &screen_descriptor,
            Some(wgpu::Color::BLACK),
        )?;

        // Submit the encoder to the queue and present the output
        self.queue.submit([encoder.finish()]);
        output.present();

        // Clean the egui pass
        self.egui_pass.remove_textures(tdelta)?;

        Ok(())
    }

    /// Resize the renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
        }
    }
}
