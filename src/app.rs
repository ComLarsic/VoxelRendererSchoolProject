use std::{io::Write, marker::PhantomData, time::Instant};

use pollster::block_on;

use crate::{
    render::RenderContext,
    tracer::{Camera, Tracer, Uniforms, VoxelGrid, Voxel},
    window::Window,
};

/// De main applicatie struct
/// Behandeld de control-flow van de applicatie
pub struct App {
    window: Window,
    render_context: RenderContext,
    tracer: Tracer,
    egui_platform: egui_sdl2_platform::Platform,
    // The resuling image to be drawn
    frame: egui::TextureId,
    // The amount of time it took to render the frame in milliseconds
    frame_time: f32,
    // The amount of time it took to render the screen in seconds
    delta_time: f32,

    // The voxel grid
    grid: VoxelGrid,
    // The uniforms
    uniforms: Uniforms,
    // The camera
    camera: Camera,

    // The flag for if the frame should be rendered in realrime
    realtime: bool,
    // The flag for if the app should run
    should_run: bool,
}

impl App {
    /// Construct a new [`App`]
    pub fn new() -> anyhow::Result<Self> {
        // Create the window
        let window = Window::new("Voxel Renderer", 1280, 720, true)?;
        // Create the render context.
        // Pollster is used here to execute the async method in a single-threaded context
        let mut render_context = pollster::block_on(RenderContext::new(&window))?;
        // Create the egui platform
        let egui_platform = egui_sdl2_platform::Platform::new(window.size())?;
        // Create the grid
        let grid = VoxelGrid(vec![
            Voxel::new(glam::ivec3(0, 0, 0), glam::vec3(1.0, 1.0, 1.0)),
            Voxel::new(glam::ivec3(1, 1, 0), glam::vec3(0.0, 1.0, 0.0)),
            Voxel::new(glam::ivec3(1, 2, 0), glam::vec3(1.0, 1.0, 0.0)),
            Voxel::new(glam::ivec3(-1, 1, 0), glam::vec3(0.0, 0.0, 1.0)),
            Voxel::new(glam::ivec3(0, 1, -1), glam::vec3(1.0, 0.0, 0.0)),
        ]);
        // Create the uniforms
        let uniforms = Uniforms {
            time: 0.0,
            resolution: glam::uvec2(1080, 1080),
            frames: 0,
            max_steps: 50,
            voxel_amount: grid.0.len() as u32,
            light_position: glam::vec3(0.0, 0.25, 0.0),
            background_color: glam::vec4(0.0, 0.0, 0.0, 1.0),
            floor_color: glam::vec4(0.1, 0.1, 0.1, 1.0),
            object_color: glam::vec3(1.0, 1.0, 1.0),
            sun_intensity: 1.0,
            smoothing: 0.0,
            ambient_occlusion: 20
        };
        // Create the camera
        let camera = Camera {
            position: glam::vec3(0.0, 0.0, 2.0),
            look_at: glam::vec3(0.0, 0.0, 0.0),
            zoom: 1.0,
        };
        // Create the tracer
        let tracer = Tracer::new(&mut render_context, &uniforms)?;
        // Trace the frame
        let before = Instant::now();
        let frame = tracer
            .trace(&mut render_context, uniforms, &grid, camera)
            .unwrap();
        let frame_time = (before.elapsed().as_secs_f64() * 1000.0) as f32;

        Ok(Self {
            window,
            render_context,
            tracer,
            egui_platform,
            frame,
            frame_time,
            delta_time: 0.0,
            grid,
            uniforms,
            camera,
            realtime: false,
            should_run: true,
        })
    }

    /// Run one frame
    pub fn execute(&mut self, start_time: Instant) -> anyhow::Result<()> {
        // The time since the start
        let start = Instant::now();
        // Update the time of the egui platform
        self.egui_platform
            .update_time(start_time.elapsed().as_secs_f64());
        // Get the egui context and start the egui frame
        let egui_ctx = self.egui_platform.context();

        // Render the frame if in realtime mode
        if self.realtime {
            let before = Instant::now();
            self.frame = self.tracer.trace(
                &mut self.render_context,
                self.uniforms,
                &self.grid,
                self.camera,
            )?;
            self.frame_time = (before.elapsed().as_secs_f64() * 1000.0) as f32;
        }

        // Draw the application's ui
        self.draw_ui(&egui_ctx);

        // End the egui frame and get the output
        let full_output = egui_ctx.end_frame();
        // Get the egui paint jobs
        let paint_jobs = self.egui_platform.tessellate(&full_output);

        // Render to the screen
        self.render_context
            .render(&self.window, full_output, paint_jobs)?;

        // Handle the window events
        self.window
            .handle_events(&mut self.render_context, &mut self.egui_platform);
        // Check if the app should be running
        self.should_run = !self.window.should_close();
        // Calculate the delta time
        self.delta_time = start.elapsed().as_secs_f32();

        // Update the uniforms
        self.uniforms.time = start_time.elapsed().as_secs_f32();
        self.uniforms.frames += 1;
        Ok(())
    }

    /// Draw the ui
    pub fn draw_ui(&mut self, ctx: &egui::Context) {
        // Draw the side panel
        egui::SidePanel::right("Config").show(ctx, |ui| {
            // Draw the info
            ui.label(format!("Fps: {}", 1.0 / self.delta_time));
            ui.label(format!("FrameMs: {}", self.frame_time));

            ui.separator();
            // Render the frame
            if !self.realtime {
                if ui.button("Render").clicked() {
                    let before = Instant::now();
                    self.frame = self
                        .tracer
                        .trace(
                            &mut self.render_context,
                            self.uniforms,
                            &self.grid,
                            self.camera,
                        )
                        .unwrap();
                    self.frame_time = (before.elapsed().as_secs_f64() * 1000.0) as f32;
                }
            }
            ui.checkbox(&mut self.realtime, "Realtime");
            // Save the image
            if ui.button("Save").clicked() {
                // Open file dialogue
                if let Some(path) = rfd::FileDialog::new().save_file() {
                    // Save the file
                    let save_path = path.display().to_string();
                    // Save the image to a file
                    block_on(self.tracer.frame_to_image(&save_path, &self.render_context)).unwrap();
                }
            }
            // Camera config
            ui.separator();
            ui.label("Camera: ");
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Position: ");
                ui.add(egui::DragValue::new(&mut self.camera.position[0]).speed(0.01));
                ui.add(egui::DragValue::new(&mut self.camera.position[1]).speed(0.01));
                ui.add(egui::DragValue::new(&mut self.camera.position[2]).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("Look at: ");
                ui.add(egui::DragValue::new(&mut self.camera.look_at[0]).speed(0.01));
                ui.add(egui::DragValue::new(&mut self.camera.look_at[1]).speed(0.01));
                ui.add(egui::DragValue::new(&mut self.camera.look_at[2]).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("Zoom: ");
                ui.add(egui::DragValue::new(&mut self.camera.zoom).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("MaxSteps: ");
                ui.add(egui::DragValue::new(&mut self.uniforms.max_steps).speed(1));
            });
            ui.horizontal(|ui| {
                ui.label("Smoothing: ");
                ui.add(egui::DragValue::new(&mut self.uniforms.smoothing).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("Background Color: ");
                let mut color = [
                    self.uniforms.background_color.x,
                    self.uniforms.background_color.y,
                    self.uniforms.background_color.z,
                    self.uniforms.background_color.w
                ]; 
                ui.color_edit_button_rgba_premultiplied(&mut color);
                self.uniforms.background_color = color.into();
            });
            ui.horizontal(|ui| {
                ui.label("Floor Color: ");
                let mut color = [
                    self.uniforms.floor_color.x,
                    self.uniforms.floor_color.y,
                    self.uniforms.floor_color.z,
                    self.uniforms.floor_color.w,
                ]; 
                ui.color_edit_button_rgba_premultiplied(&mut color);
                self.uniforms.floor_color = color.into();
            });
            ui.horizontal(|ui| {
                ui.label("Object Color: ");
                let mut color = [
                    self.uniforms.object_color.x,
                    self.uniforms.object_color.y,
                    self.uniforms.object_color.z,
                ]; 
                ui.color_edit_button_rgb(&mut color);
                self.uniforms.object_color = color.into();
            });
            ui.separator();
            ui.label("Lighting: ");
            ui.separator();
            // Scene config
            ui.horizontal(|ui| {
                ui.label("Sun Intensity: ");
                ui.add(egui::DragValue::new(&mut self.uniforms.sun_intensity).speed(0.01));
            });
            ui.horizontal(|ui| {
                ui.label("Ambient Occlusion");
                ui.add(egui::DragValue::new(&mut self.uniforms.ambient_occlusion).speed(1));
            });
            ui.horizontal(|ui| {
                ui.label("LightPosition: ");
                ui.add(egui::DragValue::new(&mut self.uniforms.light_position[0]).speed(0.01));
                ui.add(egui::DragValue::new(&mut self.uniforms.light_position[1]).speed(0.01));
                ui.add(egui::DragValue::new(&mut self.uniforms.light_position[2]).speed(0.01));
            });
        });

        // Draw the central panel
        egui::CentralPanel::default().show(ctx, |ui| {
            // Trace de image
            let image =
                egui::Image::new(self.frame, (ui.available_height(), ui.available_height()));
                    //.uv([egui::Pos2::new(0.0, 1.0), egui::Pos2::new(1.0, 0.0)]);
            ui.add(image);
        });
    }

    /// Start the main loop
    pub fn run(&mut self) -> anyhow::Result<()> {
        // The time before the mainloop started
        let start_time = Instant::now();
        // The mainloop
        while self.should_run {
            // Execute one frame
            self.execute(start_time)?;
        }
        Ok(())
    }

    /// Check if the app should be running
    pub fn should_run(&self) -> bool {
        self.should_run
    }
}
