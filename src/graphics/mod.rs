pub mod rendering_system;
pub use rendering_system::RenderingSystem;
pub use winit::event_loop::EventLoop;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3]
}
vulkano::impl_vertex!(Vertex, position);