use std::mem;
use std::sync::Arc;
use glm::{look_at, ortho, perspective, TMat4, vec3};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool, TypedBufferAccess};
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, PrimaryAutoCommandBuffer, SubpassContents};
use vulkano::device::{physical::PhysicalDevice, DeviceExtensions, DeviceCreateInfo, QueueCreateInfo, Queue, Device};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::image::{AttachmentImage, ImageAccess, ImageUsage, SwapchainImage};
use vulkano::image::view::ImageView;
use vulkano::format::Format;
use vulkano::instance::{Instance, InstanceCreateInfo};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::swapchain::{AcquireError, Surface, Swapchain, SwapchainAcquireFuture, SwapchainCreateInfo, SwapchainCreationError};
use vulkano::{swapchain, sync};
use vulkano::buffer::cpu_pool::CpuBufferPoolSubbuffer;
use vulkano::descriptor_set::{PersistentDescriptorSet, WriteDescriptorSet};
use vulkano::memory::pool::StdMemoryPool;
use vulkano::pipeline::graphics::input_assembly::InputAssemblyState;
use vulkano::pipeline::graphics::vertex_input::{BuffersDefinition, Vertex};
use vulkano::pipeline::{GraphicsPipeline, Pipeline, PipelineBindPoint};
use vulkano::pipeline::graphics::color_blend::{AttachmentBlend, BlendFactor, BlendOp, ColorBlendState};
use vulkano::pipeline::graphics::depth_stencil::DepthStencilState;
use vulkano::pipeline::graphics::rasterization::{CullMode, RasterizationState};
use vulkano::sync::{FlushError, GpuFuture};
use vulkano_win::VkSurfaceBuild;
use winit::event_loop::EventLoop;
use winit::window::{Window, WindowBuilder};
use winit::window::Fullscreen::Borderless;
use super::{AmbientLight, DirectionalLight};
use crate::graphics::{Vertex2D, VP};

#[derive(Debug)]
pub enum RenderingState {
    Stopped,
    Deferred,
    Ambient,
    Directional,
    WaitingRedraw,
}
#[derive(Debug)]
pub enum RenderingError {
    NonConformingState(String)
}

pub trait Render<T: Vertex> {
    fn vertices(&self) -> Vec<T>;
    fn model_matrices(&self) -> (TMat4<f32>, TMat4<f32>);
}

pub struct RenderingSystem {
    surface: Arc<Surface<Window>>,
    device: Arc<Device>,
    queue: Arc<Queue>,

    swapchain: Arc<Swapchain<Window>>,

    vp: VP,
    vp_descriptor_set: Arc<PersistentDescriptorSet>,
    vp_buffer: Arc<CpuAccessibleBuffer<deferred_vertex::ty::VP>>,
    model_buffer: CpuBufferPool<deferred_vertex::ty::Model>,
    ambient_buffer: CpuBufferPool<ambient_fragment::ty::AmbientLight>,
    directional_buffer: CpuBufferPool<directional_fragment::ty::DirectionalLight>,

    render_pass: Arc<RenderPass>,
    deferred_pipeline: Arc<GraphicsPipeline>,
    directional_pipeline: Arc<GraphicsPipeline>,
    ambient_pipeline: Arc<GraphicsPipeline>,
    viewport: Viewport,

    framebuffers: Vec<Arc<Framebuffer>>,
    color_buffer: Arc<ImageView<AttachmentImage>>,
    normal_buffer: Arc<ImageView<AttachmentImage>>,
    vertex2d_buffer: Arc<CpuAccessibleBuffer<[Vertex2D]>>,
    
    future: Option<SwapchainAcquireFuture<Window>>,
    image_index: Option<usize>,
    commands: Option<AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>>,
    state: RenderingState
}
impl RenderingSystem {
    //TODO: This can be heavily decoupled
    pub fn new(event_loop: &EventLoop<()>) -> (Self, Option<Box<dyn GpuFuture>>) {
        let instance = Instance::new(InstanceCreateInfo {
            enabled_extensions: vulkano_win::required_extensions(),
            ..Default::default()
        }).unwrap();

        let surface = WindowBuilder::new()
            .build_vk_surface(event_loop, instance.clone())
            .unwrap();

        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::none()
        };

        let _minimal_features = vulkano::device::Features {
            fill_mode_non_solid: true,
            .. Default::default()
        };

        let optimal_features = vulkano::device::Features {
            fill_mode_non_solid: true,
            .. Default::default()
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

        let requested_extensions = physical_device.required_extensions().union(&device_extensions);
        let requested_features = optimal_features.intersection(physical_device.supported_features());

        let (device, mut queues) = Device::new(
            physical_device,
            DeviceCreateInfo {
                enabled_extensions: requested_extensions,
                enabled_features: requested_features,
                queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
                ..Default::default()
            }
        ).unwrap();
        let queue = queues.next().unwrap();

        let mut vp = VP::default();
        let (swapchain, images) = {
            let capabilities = physical_device.surface_capabilities(&surface, Default::default()).unwrap();
            let dimensions: [u32; 2] = surface.window().inner_size().into();
            vp.projection = perspective(dimensions[0] as f32 / dimensions[1] as f32, 45.0, 0.01, 100.0);
            vp.view = look_at(&vec3(0.0, 0.0, 0.01), &vec3(0.0, 0.0, 0.0), &vec3(0.0, -1.0, 0.0));

            Swapchain::new(
                device.clone(),
                surface.clone(),
                SwapchainCreateInfo {
                    min_image_count: capabilities.min_image_count,
                    image_format: Some(physical_device.surface_formats(&surface, Default::default()).unwrap()[0].0),
                    image_extent: dimensions,
                    image_usage: ImageUsage::color_attachment(),
                    composite_alpha: capabilities.supported_composite_alpha.iter().next().unwrap(),
                    ..Default::default()
                },
            ).unwrap()
        };

        let vp_buffer = CpuAccessibleBuffer::from_data(
            device.clone(),
            BufferUsage::all(),
            false,
            deferred_vertex::ty::VP {
                view: vp.view.into(),
                projection: vp.projection.into(),
            }
        ).unwrap();
        let model_buffer = CpuBufferPool::<deferred_vertex::ty::Model>::uniform_buffer(device.clone());
        let ambient_buffer = CpuBufferPool::<ambient_fragment::ty::AmbientLight>::uniform_buffer(device.clone());
        let directional_buffer = CpuBufferPool::<directional_fragment::ty::DirectionalLight>::uniform_buffer(device.clone());

        let render_pass = vulkano::ordered_passes_renderpass!(device.clone(),
            attachments: {
                final_color: {
                    load: Clear,
                    store: Store,
                    format: swapchain.image_format(),
                    samples: 1,
                },
                vertex_color: {
                    load: Clear,
                    store: DontCare,
                    format: Format::A2B10G10R10_UNORM_PACK32,
                    samples: 1,
                },
                normals: {
                    load: Clear,
                    store: DontCare,
                    format: Format::R16G16B16A16_SFLOAT,
                    samples: 1,
                },
                depth: {
                    load: Clear,
                    store: DontCare,
                    format: Format::D16_UNORM,
                    samples: 1,
                }
            },
            passes: [
                {
                    color: [vertex_color, normals],
                    depth_stencil: {depth},
                    input: []
                },
                {
                    color: [final_color],
                    depth_stencil: {},
                    input: [vertex_color, normals]
                }
            ]
        ).unwrap();

        let deferred_pass = Subpass::from(render_pass.clone(), 0).unwrap();
        let lighting_pass = Subpass::from(render_pass.clone(), 1).unwrap();

        let deferred_vertex = deferred_vertex::load(device.clone()).unwrap();
        let deferred_fragment = deferred_fragment::load(device.clone()).unwrap();
        let ambient_vertex = ambient_vertex::load(device.clone()).unwrap();
        let ambient_fragment = ambient_fragment::load(device.clone()).unwrap();
        let directional_vertex = directional_vertex::load(device.clone()).unwrap();
        let directional_fragment = directional_fragment::load(device.clone()).unwrap();

        let deferred_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<crate::resource_pool::NormalVertex>())
            .vertex_shader(deferred_vertex.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(deferred_fragment.entry_point("main").unwrap(), ())
            .depth_stencil_state(DepthStencilState::simple_depth_test())
            .rasterization_state(
                RasterizationState::new()
                    .cull_mode(CullMode::Back)
                    /*.polygon_mode(PolygonMode::Line)*/
            )
            .render_pass(deferred_pass.clone())
            .build(device.clone())
            .unwrap();
        let directional_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex2D>())
            .vertex_shader(directional_vertex.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(directional_fragment.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Max,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One
                    }
                )
            )
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();
        let ambient_pipeline = GraphicsPipeline::start()
            .vertex_input_state(BuffersDefinition::new().vertex::<Vertex2D>())
            .vertex_shader(ambient_vertex.entry_point("main").unwrap(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
            .fragment_shader(ambient_fragment.entry_point("main").unwrap(), ())
            .color_blend_state(
                ColorBlendState::new(lighting_pass.num_color_attachments()).blend(
                    AttachmentBlend {
                        color_op: BlendOp::Add,
                        color_source: BlendFactor::One,
                        color_destination: BlendFactor::One,
                        alpha_op: BlendOp::Max,
                        alpha_source: BlendFactor::One,
                        alpha_destination: BlendFactor::One,
                    }
                ),
            )
            .render_pass(lighting_pass.clone())
            .build(device.clone())
            .unwrap();

        let vp_layout = deferred_pipeline.layout().set_layouts().get(0).unwrap();
        let vp_descriptor_set = PersistentDescriptorSet::new(
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(0, vp_buffer.clone())]
        ).unwrap();

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let vertex2d_buffer = CpuAccessibleBuffer::from_iter(
            device.clone(),
            BufferUsage::all(),
            false,
            Vertex2D::screen_plane().iter().cloned()
        ).unwrap();

        let (framebuffers, color_buffer, normal_buffer) = Self::window_size_dependent_setup(&device, &images, render_pass.clone(), &mut viewport);
        let previous_frame_end = Some(Box::new(sync::now(device.clone())) as Box<dyn GpuFuture>);

        (RenderingSystem {
            surface,
            device,
            queue,
            swapchain,
            render_pass,
            viewport,

            deferred_pipeline,
            ambient_pipeline,
            directional_pipeline,

            framebuffers,
            color_buffer,
            normal_buffer,
            vertex2d_buffer,

            vp,
            vp_descriptor_set,
            vp_buffer,
            model_buffer,
            ambient_buffer,
            directional_buffer,

            image_index: None,
            future: None,
            commands: None,
            state: RenderingState::Stopped,
        }, previous_frame_end)
    }

    pub fn start_render(&mut self) -> Result<(), RenderingError> {
        match self.state {
            RenderingState::Stopped => {
                self.state = RenderingState::Deferred;
            },
            RenderingState::WaitingRedraw => {
                self.recreate_swapchain();
                self.state = RenderingState::Stopped;
                self.commands = None;
                // We can just restart the rendering seamlessly here since we
                // did not start anything anyway
                return self.start_render()
            },
            _ => {
                self.state = RenderingState::Stopped;
                self.commands = None;
                return Err(RenderingError::NonConformingState(format!("Can not start a new render, one is already started")))
            }
        };

        let (image, image_num, future) = match swapchain::acquire_next_image(self.swapchain.clone(), None) {
            Ok((image_num, suboptimal, future)) => {
                if suboptimal { self.recreate_swapchain() }
                (self.framebuffers[image_num].clone(), image_num, future)
            },
            Err(AcquireError::OutOfDate) => {
                self.recreate_swapchain();
                return Err(RenderingError::NonConformingState(format!("No image")))
            },
            Err(e) => panic!("{:?}", e)
        };

        let clear_values = vec![
            [0.0, 0.0, 0.0, 1.0].into(),
            [0.0, 0.0, 0.0, 1.0].into(),
            [0.0, 0.0, 0.0, 1.0].into(),
            1f32.into()
        ];
        let mut commands = AutoCommandBufferBuilder::primary(
            self.device.clone(),
            self.queue.family(),
            CommandBufferUsage::OneTimeSubmit
        ).unwrap();
        commands
            .begin_render_pass(
                image,
                SubpassContents::Inline,
                clear_values
            )
            .unwrap();

        self.commands = Some(commands);
        self.future = Some(future);
        self.image_index = Some(image_num);

        Ok(())
    }

    pub fn add_model<T: Render<U>, U: Vertex>(&mut self, object: &T) -> Result<(), RenderingError> {
        match self.state {
            RenderingState::Deferred => (),
            RenderingState::WaitingRedraw => {
                self.recreate_swapchain();
                self.state = RenderingState::Stopped;
                self.commands = None;
                return Err(RenderingError::NonConformingState(String::new()))
            },
            _ => {
                self.state = RenderingState::Stopped;
                self.commands = None;
                return Err(RenderingError::NonConformingState(String::new()))
            }
        }

        let model_subbuffer = {
            let (model, normal) = object.model_matrices();
            let uniform_data = deferred_vertex::ty::Model {
                model: model.into(),
                normals: normal.into()
            };

            self.model_buffer.next(uniform_data).unwrap()
        };
        let model_layout = self.deferred_pipeline.layout().set_layouts().get(1).unwrap();
        let model_descriptor_set = PersistentDescriptorSet::new(
            model_layout.clone(),
            [
                WriteDescriptorSet::buffer(0, model_subbuffer)
            ]
        ).unwrap();

        let vertex_buffer = CpuAccessibleBuffer::from_iter(
            self.device.clone(),
            BufferUsage::all(),
            false,
            object.vertices().iter().cloned()
        ).unwrap();

        let mut commands = self.commands.take().unwrap();
        commands
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.deferred_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.deferred_pipeline.layout().clone(),
                0,
                (self.vp_descriptor_set.clone(), model_descriptor_set)
            )
            .bind_vertex_buffers(0, vertex_buffer.clone())
            .draw(vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap();

        self.commands = Some(commands);

        Ok(())
    }

    pub fn calculate_ambient_light(&mut self, light: &AmbientLight) -> Result<(), RenderingError> {
        match self.state {
            RenderingState::Deferred => self.state = RenderingState::Ambient,
            RenderingState::Ambient => return Ok(()),
            RenderingState::WaitingRedraw => {
                self.recreate_swapchain();
                self.state = RenderingState::Stopped;
                self.commands = None;
                return Err(RenderingError::NonConformingState(String::new()));
            },
            _ => {
                self.state = RenderingState::Stopped;
                self.commands = None;
                return Err(RenderingError::NonConformingState(String::new()));
            }
        }

        let ambient_buffer = CpuAccessibleBuffer::from_data(
            self.device.clone(),
            BufferUsage::all(),
            false,
            ambient_fragment::ty::AmbientLight {
                color: light.color,
                intensity: light.intensity
            }
        ).unwrap();

        let ambient_layout = self.ambient_pipeline.layout().set_layouts().get(0).unwrap();
        let ambient_descriptor_set = PersistentDescriptorSet::new(
            ambient_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.color_buffer.clone()),
                WriteDescriptorSet::buffer(1, ambient_buffer),
            ]
        ).unwrap();

        let mut commands = self.commands.take().unwrap();
        commands
            .next_subpass(SubpassContents::Inline)
            .unwrap()
            .bind_pipeline_graphics(self.ambient_pipeline.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.ambient_pipeline.layout().clone(),
                0,
                ambient_descriptor_set
            )
            .set_viewport(0, [self.viewport.clone()])
            .bind_vertex_buffers(0, self.vertex2d_buffer.clone())
            .draw(self.vertex2d_buffer.len() as u32, 1, 0,0)
            .unwrap();

        self.commands = Some(commands);

        Ok(())
    }

    pub fn calculate_directional_light(&mut self, light: &DirectionalLight) -> Result<(), RenderingError> {
        match self.state {
            RenderingState::Ambient => self.state = RenderingState::Directional,
            RenderingState::Directional => (),
            RenderingState::WaitingRedraw => {
                self.recreate_swapchain();
                self.state = RenderingState::Stopped;
                self.commands = None;
                return Err(RenderingError::NonConformingState(String::new()));
            },
            _ => {
                self.state = RenderingState::Stopped;
                self.commands = None;
                return Err(RenderingError::NonConformingState(String::new()));
            }
        }

        let directional_subbuffer = Self::new_directional_buffer(&self.directional_buffer, &light);

        let directional_layout = self.directional_pipeline.layout().set_layouts().get(0).unwrap();
        let directional_descriptor_set = PersistentDescriptorSet::new(
            directional_layout.clone(),
            [
                WriteDescriptorSet::image_view(0, self.color_buffer.clone()),
                WriteDescriptorSet::image_view(1, self.normal_buffer.clone()),
                WriteDescriptorSet::buffer(2, directional_subbuffer.clone())
            ]
        ).unwrap();

        let mut commands = self.commands.take().unwrap();
        commands
            .set_viewport(0, [self.viewport.clone()])
            .bind_pipeline_graphics(self.directional_pipeline.clone())
            .bind_vertex_buffers(0, self.vertex2d_buffer.clone())
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                self.directional_pipeline.layout().clone(),
                0,
                directional_descriptor_set.clone()
            )
            .draw(self.vertex2d_buffer.len() as u32, 1, 0, 0)
            .unwrap();
        self.commands = Some(commands);

        Ok(())
    }

    pub fn finish_render(&mut self, previous_frame_end: &mut Option<Box<dyn GpuFuture>>)  -> Result<(), RenderingError> {
        match self.state {
            RenderingState::Directional => {
                self.state = RenderingState::Stopped
            }
            RenderingState::WaitingRedraw => {
                self.recreate_swapchain();
                self.commands = None;
                self.state = RenderingState::Stopped;
            }
            _ => {
                self.commands = None;
                self.state = RenderingState::Stopped;
                return Err(RenderingError::NonConformingState(format!("The rendering can not be wrapped up yet")))
            }
        }

        let mut commands = self.commands.take().unwrap();
        commands
            .end_render_pass()
            .unwrap();
        let command_buffer = commands.build().unwrap();

        let previous_future = self.future.take().unwrap();
        let mut local_future: Option<Box<dyn GpuFuture>> = Some(Box::new(sync::now(self.device.clone())) as Box<_>);

        mem::swap(&mut local_future, previous_frame_end);

        let future = local_future.take().unwrap()
            .join(previous_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), self.image_index.unwrap())
            .then_signal_fence_and_flush();

        match future {
            Ok(future) => {
                *previous_frame_end = Some(Box::new(future) as Box<_>);
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain();
                *previous_frame_end = Some(Box::new(sync::now(self.device.clone())) as Box<_>);
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                *previous_frame_end = Some(Box::new(sync::now(self.device.clone())) as Box<_>);
            }
        }

        self.commands = None;
        Ok(())
    }

    /// Helper function taken from the Vulkano guide
    fn window_size_dependent_setup(
        device: &Arc<Device>,
        images: &[Arc<SwapchainImage<Window>>],
        render_pass: Arc<RenderPass>,
        viewport: &mut Viewport,
    ) -> (
        Vec<Arc<Framebuffer>>,
        Arc<ImageView<AttachmentImage>>,
        Arc<ImageView<AttachmentImage>>
    ) {
        let dimensions = images[0].dimensions().width_height();
        let color_buffer = ImageView::new_default(
            AttachmentImage::transient_input_attachment(
                device.clone(),
                dimensions,
                Format::A2B10G10R10_UNORM_PACK32
            ).unwrap()
        ).unwrap();
        let normal_buffer = ImageView::new_default(
            AttachmentImage::transient_input_attachment(
                device.clone(),
                dimensions,
                Format::R16G16B16A16_SFLOAT
            ).unwrap()
        ).unwrap();
        let depth_buffer = ImageView::new_default(
            AttachmentImage::transient(device.clone(), dimensions, Format::D16_UNORM).unwrap()
        ).unwrap();
        viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

        (
            images
                .iter()
                .map(|image| {
                    let view = ImageView::new_default(image.clone()).unwrap();
                    Framebuffer::new(
                        render_pass.clone(),
                        FramebufferCreateInfo {
                            attachments: vec![
                                view,
                                color_buffer.clone(),
                                normal_buffer.clone(),
                                depth_buffer.clone()
                            ],
                            ..Default::default()
                        },
                    ).unwrap()
                }).collect::<Vec<_>>(),
            color_buffer,
            normal_buffer
        )
    }

    pub fn set_view(&mut self, view: &TMat4<f32>) {
        self.vp.view = *view;
        self.vp_buffer = CpuAccessibleBuffer::from_data(
            self.device.clone(),
            BufferUsage::all(),
            false,
            deferred_vertex::ty::VP {
                view: self.vp.view.into(),
                projection: self.vp.projection.into()
            }
        ).unwrap();

        let vp_layout = self.deferred_pipeline.layout().set_layouts().get(0).unwrap();
        self.vp_descriptor_set = PersistentDescriptorSet::new(
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(0, self.vp_buffer.clone())]
        ).unwrap();

        self.state = RenderingState::WaitingRedraw;
    }

    pub fn set_projection(&mut self, projection: &TMat4<f32>) {
        self.vp.projection = *projection;
        self.vp_buffer = CpuAccessibleBuffer::from_data(
            self.device.clone(),
            BufferUsage::all(),
            false,
            deferred_vertex::ty::VP {
                view: self.vp.view.into(),
                projection: self.vp.projection.into()
            }
        ).unwrap();

        let vp_layout = self.deferred_pipeline.layout().set_layouts().get(0).unwrap();
        self.vp_descriptor_set = PersistentDescriptorSet::new(
            vp_layout.clone(),
            [WriteDescriptorSet::buffer(0, self.vp_buffer.clone())]
        ).unwrap();

        self.state = RenderingState::WaitingRedraw;
    }

    pub fn set_orthogonal_projection(&mut self) {
        let dimensions: [f32; 2] = self.surface.window().inner_size().into();
        self.set_projection(&ortho(0.0, dimensions[0], 0.0, dimensions[1], 0.01, 100.0))
    }

    pub fn recreate_swapchain(&mut self) {
        self.state = RenderingState::WaitingRedraw;
        self.commands = None;

        let dimensions: [u32; 2] = self.surface.window().inner_size().into();
        let (new_swapchain, new_images) = match self.swapchain.recreate(SwapchainCreateInfo {
            image_extent: dimensions,
            ..self.swapchain.create_info()
        }) {
            Ok(r) => r,
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain {:?}", e)
        };
        self.swapchain = new_swapchain;
        let (new_fb, new_cb, new_nb) = Self::window_size_dependent_setup(&self.device, &new_images, self.render_pass.clone(), &mut self.viewport);
        self.framebuffers = new_fb;
        self.color_buffer = new_cb;
        self.normal_buffer = new_nb;

        self.vp.projection = perspective(
            dimensions[0] as f32 / dimensions[1] as f32,
            180.0,
            0.01,
            100.0
        );
        self.vp_buffer = CpuAccessibleBuffer::from_data(
            self.device.clone(),
            BufferUsage::all(),
            false,
            deferred_vertex::ty::VP {
                view: self.vp.view.into(),
                projection: self.vp.projection.into(),
            },
        ).unwrap();
        let deferred_layout = self.deferred_pipeline.layout().set_layouts().get(0).unwrap();
        self.vp_descriptor_set = PersistentDescriptorSet::new(
            deferred_layout.clone(),
            [WriteDescriptorSet::buffer(0, self.vp_buffer.clone())]
        ).unwrap();

        self.state = RenderingState::Stopped;
    }

    fn new_directional_buffer(
        buffer_pool: &CpuBufferPool<directional_fragment::ty::DirectionalLight>,
        light: &DirectionalLight
    ) -> Arc<CpuBufferPoolSubbuffer<directional_fragment::ty::DirectionalLight, Arc<StdMemoryPool>>> {
        let uniform_data = directional_fragment::ty::DirectionalLight {
            position: light.position,
            color: light.color,
            intensity: light.intensity,
        };
        buffer_pool.next(uniform_data).unwrap()
    }

    pub fn set_fullscreen(&mut self) {
        if self.surface.window().fullscreen().is_some() {
            self.surface.window().set_fullscreen(None);
        } else {
            self.surface.window().set_fullscreen(
                Some(
                    Borderless(self.surface.window().current_monitor())
                )
            );
        }
    }
}

/*
SHADERS
 */
mod deferred_vertex {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/graphics/shaders/deferred.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}
mod deferred_fragment {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/graphics/shaders/deferred.frag",
    }
}

mod ambient_vertex {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/graphics/shaders/ambient.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}

mod ambient_fragment {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/graphics/shaders/ambient.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}

mod directional_vertex {
    vulkano_shaders::shader!{
        ty: "vertex",
        path: "src/graphics/shaders/directional.vert",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}

mod directional_fragment {
    vulkano_shaders::shader!{
        ty: "fragment",
        path: "src/graphics/shaders/directional.frag",
        types_meta: {
            use bytemuck::{Pod, Zeroable};
            #[derive(Clone, Copy, Zeroable, Pod)]
        }
    }
}