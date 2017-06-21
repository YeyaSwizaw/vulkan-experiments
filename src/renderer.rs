use std::cell::UnsafeCell;
use std::sync::Arc;
use std::time::Duration;
use std::iter::once;
use std::ptr;

use stateloop::app::Window;

use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::device::{Device, Queue, DeviceExtensions};
use vulkano::swapchain::{acquire_next_image, Swapchain, SurfaceTransform};
use vulkano::buffer::BufferUsage;
use vulkano::buffer::device_local::DeviceLocalBuffer;
use vulkano::buffer::immutable::ImmutableBuffer;
use vulkano::pipeline::{GraphicsPipeline, GraphicsPipelineParams, GraphicsPipelineAbstract};
use vulkano::pipeline::vertex::SingleBufferDefinition;
use vulkano::pipeline::input_assembly::{InputAssembly, PrimitiveTopology};
use vulkano::pipeline::viewport::{Scissor, Viewport, ViewportsState};
use vulkano::pipeline::multisample::Multisample;
use vulkano::pipeline::depth_stencil::DepthStencil;
use vulkano::pipeline::blend::Blend;
use vulkano::pipeline::raster::{Rasterization, PolygonMode};
use vulkano::descriptor::descriptor_set::DescriptorSet;
use vulkano::framebuffer::{Subpass, Framebuffer, FramebufferAbstract};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferBuilder, DynamicState};
use vulkano::sync::GpuFuture;

use sprite::Sprite;
use terrain::{TerrainMesh, TerrainVertex};
use shaders;

#[derive(Copy, Clone)]
pub struct Point {
    point: [f32; 2]
}

fn pt(x: f32, y: f32) -> Point {
    Point {
        point: [x, y]
    }
}

impl_vertex!(Point, point);

pub struct Renderer {
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,

    quad_vertex_buffer: Arc<ImmutableBuffer<[Point]>>,
    sprite_pipeline: Arc<GraphicsPipelineAbstract + Sync + Send>,
    sprite_set: Arc<DescriptorSet + Sync + Send>,

    terrain_vertex_buffer: Arc<ImmutableBuffer<[Point]>>,
    terrain_index_buffer: Arc<ImmutableBuffer<[u32]>>,
    terrain_pipeline: Arc<GraphicsPipelineAbstract + Sync + Send>,
    terrain_set: Arc<DescriptorSet + Sync + Send>,

    display_uniform_buffer: Arc<DeviceLocalBuffer<shaders::sprite::DisplayUniforms>>,

    framebuffers: Vec<Arc<FramebufferAbstract + Sync + Send>>,

    frame_future: UnsafeCell<Box<GpuFuture>>,
}

impl Renderer {
    pub fn new(instance: Arc<Instance>, window: &Window) -> Renderer {
        for device in PhysicalDevice::enumerate(&instance) {
            println!("Found device: {} (type: {:?})", device.name(), device.ty());
        }

        // Select physical device
        let physical = PhysicalDevice::enumerate(&instance)
            .next()
            .expect("No device found");

        println!("Using device: {} (type: {:?})", physical.name(), physical.ty());

        // Choose gpu queue
        let queue = physical.queue_families().find(|&queue| {
            println!("{:?}", queue);
            queue.supports_graphics() && window.surface().is_supported(queue).unwrap_or(false)
        })
            .expect("No queue family found");

        // Build vulkano device object
        let (device, mut queues) = {
            let device_ext = DeviceExtensions {
                khr_swapchain: true,
                ..DeviceExtensions::none()
            };

            Device::new(
                &physical, 
                physical.supported_features(), 
                &device_ext,
                [(queue, 0.5)].iter().cloned()
            )
                .expect("Failed to create device")
        };

        let queue = queues.next().unwrap();

        let (w, h) = window.window().get_inner_size_pixels().unwrap();

        // Create swapchain
        let (swapchain, images) = {
            let caps = window.surface().capabilities(physical)
                .expect("Failed to get surface capabilities");

            let dimensions = caps.current_extent.unwrap_or([w, h]);
            let present = caps.present_modes.iter().next().unwrap();
            let alpha = caps.supported_composite_alpha.iter().next().unwrap();
            let format = caps.supported_formats[0].0;

            Swapchain::new(
                device.clone(), 
                window.surface().clone(),
                caps.min_image_count,
                format,
                dimensions,
                1,
                caps.supported_usage_flags,
                &queue,
                SurfaceTransform::Identity,
                alpha,
                present,
                true,
                None
            )
                .expect("Failed to create swapchain")
        };

        // Create vertex buffer
        let (quad_vertex_buffer, quad_vertex_buffer_future) = ImmutableBuffer::from_iter(
            [
                 Point { point: [0.0, 0.0] },
                 Point { point: [1.0, 0.0] },
                 Point { point: [0.0, 1.0] },
                 Point { point: [1.0, 1.0] },
            ].iter().cloned(),
            BufferUsage::vertex_buffer(),
            Some(queue.family()),
            queue.clone()
        )
            .expect("Failed to create vertex buffer");

        // Create uniform buffer
        let uniform_buffer = DeviceLocalBuffer::new(
            device.clone(),
            BufferUsage::uniform_buffer_transfer_dest(),
            Some(queue.family()),
        )
            .expect("Failed to create uniform buffer");

        // Create initial terrain buffers
        let (terrain_vertex_buffer, terrain_vertex_buffer_future) = ImmutableBuffer::from_iter(
            once(pt(0f32, 0f32)),
            BufferUsage::vertex_buffer(),
            Some(queue.family()),
            queue.clone(),
        )
            .expect("Failed to create terrain vertex buffer");

        let (terrain_index_buffer, terrain_index_buffer_future) = ImmutableBuffer::from_iter(
            once(0),
            BufferUsage::index_buffer(),
            Some(queue.family()),
            queue.clone(),
        )
            .expect("Failed to create terrain index buffer");

        // Create render pass
        let render_pass = Arc::new(single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap());

        let sprite_vs = shaders::sprite::vertex::load(&device).expect("Failed to load sprite vertex shader");
        let sprite_fs = shaders::sprite::fragment::load(&device).expect("Failed to load sprite fragment shader");

        // Create sprite pipeline
        let sprite_pipeline = Arc::new(GraphicsPipeline::new(
            device.clone(),
            GraphicsPipelineParams {
                vertex_input: SingleBufferDefinition::<Point>::new(),
                vertex_shader: sprite_vs.main_entry_point(),
                input_assembly: InputAssembly {
                    topology: PrimitiveTopology::TriangleStrip,
                    primitive_restart_enable: false
                },
                tessellation: None,
                geometry_shader: None,
                viewport: ViewportsState::Fixed {
                    data: vec![(
                        Viewport {
                            origin: [0.0, 0.0],
                            depth_range: 0.0 .. 1.0,
                            dimensions: [images[0].dimensions()[0] as f32,
                                         images[0].dimensions()[1] as f32],
                        },
                        Scissor::irrelevant()
                    )],
                },
                raster: Default::default(),
                multisample: Multisample::disabled(),
                fragment_shader: sprite_fs.main_entry_point(),
                depth_stencil: DepthStencil::disabled(),
                blend: Blend::pass_through(),
                render_pass: Subpass::from(render_pass.clone(), 0).unwrap(),
            }
        ).unwrap());

        let terrain_vs = shaders::terrain::vertex::load(&device).expect("Failed to load terrain vertex shader");
        let terrain_fs = shaders::terrain::fragment::load(&device).expect("Failed to load terrain fragment shader");

        // Create terrain pipeline
        let terrain_pipeline = Arc::new(GraphicsPipeline::new(
            device.clone(),
            GraphicsPipelineParams {
                vertex_input: SingleBufferDefinition::<Point>::new(),
                vertex_shader: terrain_vs.main_entry_point(),
                input_assembly: InputAssembly {
                    topology: PrimitiveTopology::TriangleStrip,
                    primitive_restart_enable: true
                },
                tessellation: None,
                geometry_shader: None,
                viewport: ViewportsState::Fixed {
                    data: vec![(
                        Viewport {
                            origin: [0.0, 0.0],
                            depth_range: 0.0 .. 1.0,
                            dimensions: [images[0].dimensions()[0] as f32,
                                         images[0].dimensions()[1] as f32],
                        },
                        Scissor::irrelevant()
                    )],
                },
                raster: Rasterization {
                    polygon_mode: PolygonMode::Line,
                    ..Default::default()
                },
                multisample: Multisample::disabled(),
                fragment_shader: terrain_fs.main_entry_point(),
                depth_stencil: DepthStencil::disabled(),
                blend: Blend::pass_through(),
                render_pass: Subpass::from(render_pass.clone(), 0).unwrap(),
            }
        ).unwrap());

        let sprite_set = Arc::new(simple_descriptor_set!(sprite_pipeline.clone(), 0, {
            display: uniform_buffer.clone()
        }));

        let terrain_set = Arc::new(simple_descriptor_set!(terrain_pipeline.clone(), 0, {
            display: uniform_buffer.clone()
        }));

        // Create framebuffers
        let framebuffers = images.iter().map(|image| {
            Arc::new(Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()) as Arc<FramebufferAbstract + Send + Sync>
        }).collect();

        let future = quad_vertex_buffer_future
            .join(terrain_vertex_buffer_future)
            .join(terrain_index_buffer_future);

        let mut renderer = Renderer {
            device: device.clone(),
            queue: queue,
            swapchain: swapchain,

            quad_vertex_buffer: quad_vertex_buffer,
            sprite_pipeline: sprite_pipeline as Arc<GraphicsPipelineAbstract + Send + Sync>,
            sprite_set: sprite_set as Arc<DescriptorSet + Sync + Send>,

            terrain_vertex_buffer: terrain_vertex_buffer,
            terrain_index_buffer: terrain_index_buffer,
            terrain_pipeline: terrain_pipeline as Arc<GraphicsPipelineAbstract + Send + Sync>,
            terrain_set: terrain_set as Arc<DescriptorSet + Sync + Send>,

            display_uniform_buffer: uniform_buffer,

            framebuffers: framebuffers,

            frame_future: UnsafeCell::new(Box::new(future) as Box<GpuFuture>),
        };

        renderer.update_display_uniforms(w, h);
        renderer
    }

    fn with_future<T, F>(&self, f: F) where T: GpuFuture + 'static, F: FnOnce(Box<GpuFuture>) -> T {
        let frame_future = unsafe { 
            let ptr = self.frame_future.get();
            ptr::read(ptr)
        };

        let new_future = f(frame_future);

        unsafe {
            let ptr = self.frame_future.get();
            ptr::write(ptr, Box::new(new_future) as Box<_>);
        }
    }

    pub fn load_terrain(&mut self, terrain: &TerrainMesh) {
        let vertices = terrain.mesh_vertices().map(|c| pt(c.0 as f32, c.1 as f32)).collect::<Vec<_>>();
        let indices = terrain.mesh_indices(0).collect::<Vec<_>>();

        let (vertex_buffer, vertex_future) = ImmutableBuffer::from_iter(
            vertices.into_iter(),
            BufferUsage::vertex_buffer(),
            Some(self.queue.family()),
            self.queue.clone(),
        )
            .expect("Failed to create terrain vertex buffer");

        let (index_buffer, index_future) = ImmutableBuffer::from_iter(
            indices.into_iter(),
            BufferUsage::index_buffer(),
            Some(self.queue.family()),
            self.queue.clone(),
        )
            .expect("Failed to create terrain index buffer");

        self.terrain_vertex_buffer = vertex_buffer;
        self.terrain_index_buffer = index_buffer;

        self.with_future(|future| {
            vertex_future
                .join(index_future)
                .join(future)
        });
    }

    pub fn update_display_uniforms(&mut self, w: u32, h: u32) {
        self.with_future(|future| {
            let command_buffer = AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())
                .unwrap()
                .update_buffer(
                    self.display_uniform_buffer.clone(), 
                    shaders::sprite::DisplayUniforms {
                        bounds: [w, h]
                    }
                )
                .unwrap()
                .build()
                .unwrap();

            future
                .then_execute(self.queue.clone(), command_buffer)
                .unwrap()
        });
    }

    pub fn render(&self, sprites: &[Sprite]) {
        self.with_future(|mut future| {
            future.cleanup_finished();
            let (image_num, acquire_future) = acquire_next_image(
                self.swapchain.clone(),
                Duration::new(1, 0)
            ).unwrap();

            let command_buffer = {
                let render_pass = AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())
                    .unwrap()
                    .begin_render_pass(
                        self.framebuffers[image_num].clone(),
                        false,
                        vec![[0.0, 0.0, 0.0, 1.0].into()]
                    )
                    .unwrap();

                sprites.iter().fold(render_pass, |buffer, sprite| buffer
                    .draw(
                        self.sprite_pipeline.clone(),
                        DynamicState::none(),
                        vec![self.quad_vertex_buffer.clone()], 
                        self.sprite_set.clone(), 
                        shaders::sprite::SpriteUniforms::from(&sprite.rect)
                    )
                    .unwrap()
                )
                    .draw_indexed(
                        self.terrain_pipeline.clone(),
                        DynamicState::none(),
                        vec![self.terrain_vertex_buffer.clone()],
                        self.terrain_index_buffer.clone(),
                        self.terrain_set.clone(),
                        ()
                    )
                    .unwrap()
                    .end_render_pass()
                    .unwrap()
                    .build()
                    .unwrap()
            };

            future
                .join(acquire_future)
                .then_execute(
                    self.queue.clone(), 
                    command_buffer
                )
                .unwrap()
                .then_swapchain_present(
                    self.queue.clone(), 
                    self.swapchain.clone(), 
                    image_num
                )
                .then_signal_fence_and_flush()
                .unwrap()
        })
    }
}

