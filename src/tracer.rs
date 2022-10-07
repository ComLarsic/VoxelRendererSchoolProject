use crate::render::RenderContext;
use encase::{ShaderType, UniformBuffer, StorageBuffer};
use pollster::block_on;
use std::{num::{NonZeroU32, NonZeroU64}, path::Path};
use wgpu::util::DeviceExt;

const WORKGROUP_SIZE: u32 = 16;

/// Represents the uniforms for the shader
#[derive(Debug, ShaderType, Clone, Copy)]
pub struct Uniforms {
    pub time: f32,
    pub frames: u32,
    pub max_steps: u32,
    pub voxel_amount: u32,
    pub resolution: glam::UVec2,
    pub background_color: glam::Vec4,
    pub floor_color: glam::Vec4,
    pub object_color: glam::Vec3,
    pub light_position: glam::Vec3,
    pub sun_intensity: f32,
    pub smoothing: f32,
    pub ambient_occlusion: i32,
}

/// Represents the camera
#[derive(Debug, ShaderType, Clone, Copy)]
pub struct Camera {
    pub position: glam::Vec3,
    pub look_at: glam::Vec3,
    pub zoom: f32,
}

/// Represents a voxel
#[derive(Debug, Clone, ShaderType)]
pub struct Voxel {
    position: glam::IVec3,
    color: glam::Vec3,
}

impl Voxel {
    /// Construct a new [`Voxel`]
    pub fn new(
        position: glam::IVec3,
        color: glam::Vec3,
    ) -> Self {
        Self  {
            position,
            color
        }
    }
}

/// Represents the voxel grid
#[derive(Debug, Clone)]
pub struct VoxelGrid(pub Vec<Voxel>);

/// Handles executing the compute shader
pub struct Tracer {
    compute: wgpu::ShaderModule,

    // The resulting frame
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,

    // The resolution for the buffer
    resolution: (u32, u32),
}

impl Tracer {
    /// Construct a new [`Tracer`]
    pub fn new(ctx: &mut RenderContext, uniforms: &Uniforms) -> anyhow::Result<Self> {
        // Load the shader source
        let source = std::fs::read_to_string("shaders/voxel.wgsl")?;

        // Compile the shader
        let compute = ctx
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(source.into()),
            });

        // Create the texture buffer
        let texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: uniforms.resolution[0],
                height: uniforms.resolution[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
        });
        // Create a texture view
        let texture_view = texture.create_view(&Default::default());

        Ok(Self {
            compute,
            resolution: (uniforms.resolution[0], uniforms.resolution[1]),
            texture,
            texture_view,
        })
    }

    /// Trace the texture
    pub fn trace(
        &self,
        ctx: &mut RenderContext,
        uniforms: Uniforms,
        grid: &VoxelGrid,
        camera: Camera,
    ) -> anyhow::Result<egui::TextureId> {
        // Create the uniform buffer
        let mut buffer = UniformBuffer::new(vec![]);
        buffer.write(&uniforms)?;
        let uniform_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &buffer.into_inner(),
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::UNIFORM,
            });
        // Create the camera buffer
        let mut buffer = UniformBuffer::new(vec![]);
        buffer.write(&camera)?;
        let camera_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &buffer.into_inner(),
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::UNIFORM,
            });

        // Create the grid buffer
        let mut buffer = StorageBuffer::new(vec![]);
        buffer.write(&grid.0)?;
        let grid_buffer = ctx
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: &buffer.into_inner(),
                usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::STORAGE,
            });

        // Create the bind group layout
        let bind_group_layout =
            ctx.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: None,
                    entries: &[
                        // THe uniforms
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None, //NonZeroU64::new(
                                                        //     std::mem::size_of::<Uniforms>() as u64
                                                        //),
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None, //NonZeroU64::new(
                                                        //    std::mem::size_of::<Camera>() as u64
                                                        //),
                            },
                            count: None,
                        },
                        // The grid buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,//NonZeroU64::new(
                                    //std::mem::size_of_val(&*grid.0) as u64
                                //),
                            },
                            count: None,
                        },
                        // The texture buffer
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::StorageTexture {
                                access: wgpu::StorageTextureAccess::WriteOnly,
                                format: wgpu::TextureFormat::Rgba8Unorm,
                                view_dimension: wgpu::TextureViewDimension::D2,
                            },
                            count: None,
                        },
                    ],
                });

        // Create the bind group
        let bind_group = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: camera_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: grid_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&self.texture_view),
                },
            ],
        });

        // Create the compute pipeline
        let compute_pipeline_layout =
            ctx.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });
        let compute_pipeline =
            ctx.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: None,
                    layout: Some(&compute_pipeline_layout),
                    module: &self.compute,
                    entry_point: "main",
                });

        // Create the command encoder
        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        // Execute the compute shader
        {
            let mut compute_pass = encoder.begin_compute_pass(&Default::default());
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(self.resolution.0 / WORKGROUP_SIZE, self.resolution.1 / WORKGROUP_SIZE, 1);
        }

        // Submut the encoder to the queue
        ctx.queue.submit([encoder.finish()]);

        // Return the texture as an egui image
        let image = ctx.egui_pass.egui_texture_from_wgpu_texture(
            &ctx.device,
            &self.texture_view,
            wgpu::FilterMode::Nearest,
        );
        Ok(image)
    }

    /// Get the frame as image data
    pub async fn frame_to_image(&self, path: impl AsRef<Path>, ctx: &RenderContext) -> anyhow::Result<()> {
        // Pad the bytes per row
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = (((self.resolution.0 * 4) + align - 1) / align) * align;
        println!("{}", padded_bytes_per_row);

        let buffer = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: padded_bytes_per_row as u64 * self.resolution.1 as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = ctx.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: NonZeroU32::new(padded_bytes_per_row),
                    rows_per_image: NonZeroU32::new(self.resolution.1),
                },
            },
            wgpu::Extent3d {
                width: self.resolution.0,
                height: self.resolution.1,
                depth_or_array_layers: 1,
            },
        );

        ctx.queue.submit([encoder.finish()]);

        // Read the data from the buffer
        let buffer_slice = buffer.slice(..);
        let (sender, receiver) = futures_intrusive::channel::shared::oneshot_channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |v| sender.send(v).unwrap());

        ctx.device.poll(wgpu::Maintain::Wait);

        match receiver.receive().await.unwrap() {
            Ok(()) => {
                let data = buffer_slice.get_mapped_range();
                let result: Vec<u8> = bytemuck::cast_slice(&data).to_vec();

                image::save_buffer(
                    &path,
                    &result,
                    padded_bytes_per_row / 4,
                    self.resolution.1,
                    image::ColorType::Rgba8,
                )?;

                drop(data);
                buffer.unmap();
                return Ok(())
            }
            Err(e) => Err(anyhow::anyhow!(e)),
        }
    }
}
