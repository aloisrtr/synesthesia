pub mod rendering_system;
pub use rendering_system::RenderingSystem;
pub use winit::event_loop::EventLoop;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Vertex2D {
    pub position: [f32; 2],
}
impl Vertex2D {
    pub fn screen_plane() -> [Vertex2D; 6] {
        [
            Vertex2D { position: [-1.0, -1.0] },
            Vertex2D { position: [-1.0, 1.0] },
            Vertex2D { position: [1.0, 1.0] },
            Vertex2D { position: [-1.0, -1.0] },
            Vertex2D { position: [1.0, 1.0] },
            Vertex2D { position: [1.0, -1.0] }
        ]
    }
}
vulkano::impl_vertex!(Vertex2D, position);

#[derive(Default, Debug, Clone)]
pub struct VP {
    pub view: glm::TMat4<f32>,
    pub projection: glm::TMat4<f32>
}

#[derive(Default, Debug, Clone)]
pub struct AmbientLight {
    pub color: [f32; 3],
    pub intensity: f32
}

#[derive(Default, Debug, Clone)]
pub struct DirectionalLight {
    pub position: [f32; 3],
    pub intensity: f32,
    pub color: [f32; 3],
}