use crate::render::RenderContext;
use sdl2::event::{Event, WindowEvent};

/// A wrapper around an sdl2 window
pub struct Window {
    sdl: sdl2::Sdl,
    video_subsystem: sdl2::VideoSubsystem,
    event_pump: sdl2::EventPump,

    // The sdl2 window handle
    sdl_window: sdl2::video::Window,
    // The flag for if the window should be closed
    should_close: bool,
}

impl Window {
    /// Construct a [`Window`]
    pub fn new(
        title: impl Into<String>,
        width: u32,
        height: u32,
        resizable: bool,
    ) -> anyhow::Result<Self> {
        // Initialize sdl2
        let sdl = sdl2::init().map_err(|e| anyhow::anyhow!(e))?;
        // Get the video subsystem
        let video_subsystem = sdl.video().map_err(|e| anyhow::anyhow!(e))?;
        // Create the window
        let mut window_builder = &mut video_subsystem.window(&title.into(), width, height);
        if resizable {
            window_builder = window_builder.resizable();
        }
        let sdl_window = window_builder.build()?;
        // Get the event pump
        let event_pump = sdl.event_pump().map_err(|e| anyhow::anyhow!(e))?;

        Ok(Self {
            sdl,
            video_subsystem,
            sdl_window,
            event_pump,
            should_close: false,
        })
    }

    /// Handle the sdl2 events
    pub fn handle_events(
        &mut self,
        render_context: &mut RenderContext,
        egui_platform: &mut egui_sdl2_platform::Platform,
    ) {
        // Poll the events
        for event in self.event_pump.poll_iter() {
            // Let the egui platform handle the event
            egui_platform.handle_event(&event, &self.sdl, &self.video_subsystem);

            match event {
                Event::Quit { .. } => self.should_close = true,
                Event::Window { win_event, .. } => match win_event {
                    WindowEvent::Resized(w, h) | WindowEvent::SizeChanged(w, h) => {
                        render_context.resize(w as u32, h as u32);
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    /// Check if the window should be closed
    pub fn should_close(&self) -> bool {
        self.should_close
    }

    /// Create a wgpu surface
    pub fn create_surface(&self, instance: &wgpu::Instance) -> wgpu::Surface {
        // Becomes safe since it's garunteed that our window is a valid object
        unsafe { instance.create_surface(&self.sdl_window) }
    }

    // Get the window size
    pub fn size(&self) -> (u32, u32) {
        self.sdl_window.size()
    }
}
