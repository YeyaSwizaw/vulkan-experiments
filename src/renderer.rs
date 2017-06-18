use std::cell::UnsafeCell;
use std::sync::Arc;
use std::time::Duration;
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
use vulkano::descriptor::descriptor_set::DescriptorSet;
use vulkano::framebuffer::{Subpass, Framebuffer, FramebufferAbstract};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferBuilder, DynamicState};
use vulkano::sync::{now, GpuFuture};

use sprite::Sprite;
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
    display_uniform_buffer: Arc<DeviceLocalBuffer<shaders::sprite::DisplayUniforms>>,

    pipeline: Arc<GraphicsPipelineAbstract + Sync + Send>,
    descriptor_set: Arc<DescriptorSet + Sync + Send>,
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

        let vs = shaders::sprite::vertex::load(&device).expect("Failed to load vertex shader");
        let fs = shaders::sprite::fragment::load(&device).expect("Failed to load vertex shader");

        // Create uniform buffer
        let uniform_buffer = DeviceLocalBuffer::new(
            device.clone(),
            BufferUsage::uniform_buffer_transfer_dest(),
            Some(queue.family()),
        )
            .expect("Failed to create uniform buffer");

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

        // Create pipeline
        let pipeline = Arc::new(GraphicsPipeline::new(
            device.clone(),
            GraphicsPipelineParams {
                vertex_input: SingleBufferDefinition::<Point>::new(),
                vertex_shader: vs.main_entry_point(),
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
                fragment_shader: fs.main_entry_point(),
                depth_stencil: DepthStencil::disabled(),
                blend: Blend::pass_through(),
                render_pass: Subpass::from(render_pass.clone(), 0).unwrap(),
            }
        ).unwrap());

        let set = Arc::new(simple_descriptor_set!(pipeline.clone(), 0, {
            display: uniform_buffer.clone()
        }));

        // Create framebuffers
        let framebuffers = images.iter().map(|image| {
            Arc::new(Framebuffer::start(render_pass.clone())
                .add(image.clone()).unwrap()
                .build().unwrap()) as Arc<FramebufferAbstract + Send + Sync>
        }).collect();

        let mut renderer = Renderer {
            device: device.clone(),
            queue: queue,
            swapchain: swapchain,

            quad_vertex_buffer: quad_vertex_buffer,
            display_uniform_buffer: uniform_buffer,

            pipeline: pipeline as Arc<GraphicsPipelineAbstract + Send + Sync>,
            descriptor_set: set as Arc<DescriptorSet + Sync + Send>,
            framebuffers: framebuffers,

            frame_future: UnsafeCell::new(Box::new(quad_vertex_buffer_future) as Box<GpuFuture>),
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
                        self.pipeline.clone(),
                        DynamicState::none(),
                        vec![self.quad_vertex_buffer.clone()], 
                        self.descriptor_set.clone(), 
                        shaders::sprite::SpriteUniforms::from(&sprite.rect)
                    )
                    .unwrap()
                )
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

