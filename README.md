# Voxel Renderer
A voxel renderer written in Rust using WGPU and SDL2.
It utilizes a compute shader which uses raymarching in order to draw voxels to the frame.

![screenshot](screenshot.png "Screenshot")

## Building and Running
Building and running the app can be done using cargo:
```bash
# Build the app
cargo build --release
# Run the app
cargo run --release
```
Make sure the `shaders` folder is in the same directory as the resulting executable when trying to run.

## Performance
The renderer was able to draw at an average of 60 fps with 50 steps.
It was tested using the following specs:
```
CPU: AMD Ryzen 7 3700U 2.3 GHz
RAM: 16.0GB
GPU: AMD Radeon RX Vega 10 Graphics
VRAM: 2 GB
```

## Struct Diagram
```
┌──────────────────────────┐                           ┌──────────────────────────────────────┐
│App                       │                           │Window                                │
├──────────────────────────┤                           ├──────────────────────────────────────┤
├──────────────────────────┤                           ├──────────────────────────────────────┤
│Fields:                   │                           │Fields:                               │
├──────────────────────────┤                           ├──────────────────────────────────────┤
│egui_platform: Platform   │ window                    │sdl: sdl2::Sdl                        │
├──────────────────────────┼───────────────────────────►video_subsystem: sdl2::VideoSubsystem │
│Properties:               │                           │event_pump: sdl2::EventPump           │
├──────────────────────────┤                           │sdl_window: sdl2::video::Window       │
│frame: TextureId          │                           ├──────────────────────────────────────┤
│grid: VoxelGrid           │                           │Properties:                           │
│                          │                           ├──────────────────────────────────────┤
└─┬─┬──────────────────────┘                           │should_close: bool                    │
  │ │                                                  │                                      │
  │ │                                                  └───────────▲──────────────────────────┘
  │ │                                                              │
  │ │                                                              │
  │ │                                                              │ fn render()
  │ │                                                              │
  │ │                                                              │
  │ │                                                  ┌───────────┴──────────────────────────────┐
  │ │                                                  │RenderContext                             │
  │ │                                                  ├──────────────────────────────────────────┤
  │ │                                                  ├──────────────────────────────────────────┤
  │ │                                                  │Fields:                                   │
  │ │                        render_context            └──────────────────────────────────────────┤
  │ └──────────────────────────────────────────────────►instance: wgpu::Instance                  │
  │                                                    │device: wgpu::Device                      │
  │                                                    │queue: wgpu::Queue                        │
  │                                                    │surface: wgpu::Surface                    │
  │                                                    │surface_config: wgpu::SurfaceConfiguration│
  │                                                    │egui_pass: egui_wgpu_backend::RenderPass  │
  │                                                    └───────────▲──────────────────────────────┘
  │                                                                │
  │                                                                │
  │                                                                │
  │                                                                │ fn trace()
  │                                                                │
  │                                                                │
  │                                                                │
  │                                                    ┌───────────┴───────────────────────┐
  │                                                    │Tracer                             │
  │                                                    ├───────────────────────────────────┤
  │                                                    ├───────────────────────────────────┤
  │                                                    │Fields:                            │
  │                          tracer                    └───────────────────────────────────┤
  └────────────────────────────────────────────────────►compute: wgpu::ShaderModule        │
                                                       │texture: wgpu::Texture             │
                                                       │texture_view: wgpu::TextureView    │
                                                       ├───────────────────────────────────┤
                                                       │Properties:                        │
                                                       ├───────────────────────────────────┤
                                                       │resolution: (u32, u32)             │
                                                       └───────────────────────────────────┘
```