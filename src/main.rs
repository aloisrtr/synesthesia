extern crate nalgebra_glm as glm;

mod graphics;

use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use graphics::RenderingSystem;
use crate::graphics::rendering_system::Render;
use crate::graphics::Vertex;

struct Triangle {
    pub vertices: [Vertex; 3]
}
impl Render for Triangle {
    fn vertices(&self) -> Vec<Vertex> {
        Vec::from(self.vertices)
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let mut rendering_system = RenderingSystem::new(&event_loop);

    let triangle = Triangle {vertices: [
        Vertex { position: [0.5, 0.5, 0.0] },
        Vertex { position: [-0.5, 0.5, 0.0] },
        Vertex { position: [0.0, -0.5, 0.0] }
    ]};

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
            *control_flow = ControlFlow::Exit;
        },
        Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {

        },
        Event::RedrawEventsCleared => {
            rendering_system.start_render().unwrap();
            rendering_system.draw(&triangle).unwrap();
            rendering_system.finish_render().unwrap();
        },
        _ => ()
    })
}