# Voxel Renderer
A voxel renderer written in Rust using WGPU and SDL2.
It utilizises a compute shader which uses raymarching in order to draw voxels to the frame.

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
