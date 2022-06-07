use std::process::Output;
use std::sync::Arc;
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, TypedBufferAccess};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, PrimaryCommandBuffer, SubpassContents};
use vulkano::device::{physical::PhysicalDevice, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue, Device};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::image::{ImageAccess, ImageUsage, SwapchainImage};
use vulkano::image::view::ImageView;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{AcquireError, Surface, Swapchain, SwapchainCreateInfo, SwapchainCreationError};
use vulkano::{swapchain, sync};
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::{BuffersDefinition, Vertex, VertexBuffersCollection};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::sync::{FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};

#[derive(Debug)]
pub enum RenderingState {
    Stopped,
    Ready,
}
#[derive(Debug)]
pub enum RenderingError {
    NonConformingState(String)
}

pub trait Render {
    fn vertices(&self) -> Vec<super::Vertex>;
}

pub struct RenderingSystem {
    instance: Arc<Instance>,
    surface: Arc<Surface<Window>>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain<Window>>,
    render_pass: Arc<RenderPass>,
    framebuffers: Vec<Arc<Framebuffer>>,
    viewport: Viewport,
    pipeline: Arc<GraphicsPipeline>,

    recreate_swapchain: bool,
    future: Option<Box<dyn GpuFuture>>,
    image_num: Option<usize>,
    commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    state: RenderingState
}
impl RenderingSystem {
    //TODO: This can be heavily decoupled
    pub fn new(event_loop: &EventLoop<()>) -> Self {
        let instance = Instance::new(InstanceCreateInfo {
            enabled_extensions: vulkano_win::required_extensions(),
            ..Default::default()
        }).unwrap();

        let surface = WindowBuilder::new()
            .build_vk_surface(&event_loop, instance.clone())
            .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
            .filter(|&p| p.supported_extensions().is_superset_of(&device_extensions))
            .filter_map(|p| {
                p.queue_families()
                    .find(|&q| q.supports_graphics() && q.supports_surface(&surface).unwrap_or(false))
                    .map(|q| (p, q))
            })
            .min_by_key(|(p, _)| match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
            }).unwrap();

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: physical_device.required_extensions().union(&device_extensions),
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
                ..Default::default()
            }
        ).unwrap();
        let queue = queues.next().unwrap();

        let (swapchain, images) = {
            let capabilities = physical_device.surface_capabilities(&surface, Default::default()).unwrap();

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: capabilities.min_image_count,
                    image_format: Some(physical_device.surface_formats(&surface, Default::default()).unwrap()[0].0),
                    image_extent: surface.window().inner_size().into(),
                    image_usage: ImageUsage::color_attachment(),
                    composite_alpha: capabilities.supported_composite_alpha.iter().next().unwrap(),
                    ..Default::default()
                },
            ).unwrap()
        };

        let render_pass = vulkano::single_pass_renderpass!(device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap();

        let deferred_vertex = deferred_vertex::load(device.clone()).unwrap();
        let deferred_fragment = deferred_fragment::load(device.clone()).unwrap();

        let pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<super::Vertex>())
            .vertex_shader(deferred_vertex.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(deferred_fragment.entry_point("main").unwrap(), ())
            .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
            .build(device.clone())
            .unwrap();


        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let framebuffers = window_size_dependent_setup(&images, render_pass.clone(), &mut viewport);
        let future = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        RenderingSystem {
            instance,
            surface,
            device,
            queue,
            swapchain,
            render_pass,
            framebuffers,
            viewport,
            pipeline,

            recreate_swapchain: false,
            image_num: None,
            future,
            commands: None,
            state: RenderingState::Stopped
        }
    }

    pub fn start_render(&mut self) -> Result<(), RenderingError> {
        match self.state {
            RenderingState::Stopped => {
                self.state = RenderingState::Ready;
            },
            RenderingState::Ready => {
                self.state = RenderingState::Stopped;
                self.commands = None;
                return Err(RenderingError::NonConformingState(format!("Can not start a new render, one is already started")))
            }
        };

        self.future.as_mut().take().unwrap().cleanup_finished();

        if self.recreate_swapchain {
            let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
                image_extent: self.surface.window().inner_size().into(),
                ..self.swapchain.create_info()
            }) {
                Ok(r) => r,
                Err(SwapchainCreationError::ImageExtentNotSupported {..}) => return Err(RenderingError::NonConformingState(format!("Bad swapchain"))),
                Err(e) => panic!("Failed to recreate swapchain {:?}", e)
            };
            self.swapchain = new_swapchain;
            self.framebuffers = window_size_dependent_setup(&new_images, self.render_pass.clone(), &mut self.viewport);
            self.recreate_swapchain = false;
        }

        let (image, image_num, future) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok((image_num, suboptimal, future)) => {
                if suboptimal { self.recreate_swapchain = true }
                (self.framebuffers[image_num].clone(), image_num, future)
            },
            Err(AcquireError::OutOfDate) => {
                self.recreate_swapchain = true;
                return Err(RenderingError::NonConformingState(format!("No image")))
            },
            Err(e) => panic!("{:?}", e)
        };

        let mut commands = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit
        ).unwrap();
        commands
            .begin_render_pass(
                image,
                SubpassContents::Inline,
                vec!([0.0, 0.0, 1.0, 1.0].into())
            )
            .unwrap()
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.pipeline.clone());

        self.commands = Some(commands);
        self.future = Some(Box::new(future) as Box<_>);
        self.image_num = Some(image_num);

        Ok(())
    }

    pub fn draw<T: Render>(&mut self, object: &T) -> Result<(), RenderingError> {
        let vertex_buffer = CpuAccessibleBuffer::from_iter(self.device.clone(), BufferUsage::all(), false, object.vertices().iter().cloned()).unwrap();

        let mut commands = self.commands.take().unwrap();
        commands
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap();

        self.commands = Some(commands);

        Ok(())
    }

    pub fn finish_render(&mut self)  -> Result<(), RenderingError> {
        match self.state {
            RenderingState::Ready => {
                self.state = RenderingState::Stopped
            }
            RenderingState::Stopped => {
                return Err(RenderingError::NonConformingState(format!("Rendering isn't ready")))
            }
        }

        let mut commands = self.commands.take().unwrap();
        commands
            .end_render_pass()
            .unwrap();
        let command_buffer = commands.build().unwrap();

        let future = self.future
            .take()
            .unwrap()
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), self.image_num.unwrap())
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                self.future = Some(Box::new(future) as Box<_>);
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.future = Some(Box::new(sync::now(self.device.clone())) as Box<_>);
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                self.future = Some(Box::new(sync::now(self.device.clone())) as Box<_>);
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug)]
struct ViewProjection {
    view: glm::TMat4<f32>,
    projection: glm::TMat4<f32>
}
impl Default for ViewProjection {
    fn default() -> Self {
        ViewProjection {
            view: glm::identity(),
            projection: glm::identity()
        }
    }
}

/*
SHADERS
 */
mod deferred_vertex {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/graphics/shaders/deferred.vert"
    }
}
mod deferred_fragment {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/graphics/shaders/deferred.frag"
    }
}


/// Helper function taken from the Vulkano guide
fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    render_pass: Arc<RenderPass>,
    viewport: &mut Viewport,
) -> Vec<Arc<Framebuffer>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view],
                    ..Default::default()
                },
            )
                .unwrap()
        })
        .collect::<Vec<_>>()
}
