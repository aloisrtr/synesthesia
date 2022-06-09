extern crate nalgebra_glm as glm;

mod graphics;
mod resource_pool;

use std::time::Instant;
use std::default::Default;
use glm::{identity, look_at, perspective, pi, rotate_normalized_axis, translate, vec3};
use vulkano::buffer::{BufferUsage, CpuAccessibleBuffer};
use vulkano::sync::GpuFuture;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use graphics::RenderingSystem;
use crate::graphics::rendering_system::Render;
use crate::graphics::{AmbientLight, DirectionalLight, VP};
use crate::resource_pool::model_loader::Model;
use crate::resource_pool::ResourcePool;

fn main() {
    let event_loop = EventLoop::new();
    let (mut rendering_system, mut previous_frame_end) =
        RenderingSystem::new(&event_loop);
    rendering_system.set_view(&look_at(&vec3(0.0, 0.0, 0.01), &vec3(0.0, 0.0, 0.0), &vec3(0.0, -1.0, 0.0)));

    let mut model_pool: ResourcePool<Model> = ResourcePool::default();

    model_pool.load("cube", "assets/models/cube.obj");
    model_pool.load("suzanne", "assets/models/suzanne.obj");
    model_pool.load("teapot", "assets/models/teapot.obj");

    model_pool.get_mut("teapot").unwrap().translate(vec3(5.5, -3.0, -8.0));
    model_pool.get_mut("suzanne").unwrap().translate(vec3(-7.0, 5.0, -5.0));
    model_pool.get_mut("cube").unwrap().translate(vec3(0.0, 0.0, -4.0));


    let ambient_light = AmbientLight { color: [1.0, 1.0, 1.0], intensity: 0.05 };

    let directional_lights = vec![
        DirectionalLight { position: [-4.0, 0.0, -2.0], color: [1.0, 0.0, 0.0], intensity: 0.2 },
        DirectionalLight { position: [0.0, -4.0, 1.0], color: [0.0, 1.0, 0.0], intensity: 0.5 },
        DirectionalLight { position: [4.0, -2.0, -1.0], color: [0.0, 0.0, 1.0], intensity: 1.0 },
    ];

    let timer = Instant::now();
    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
            *control_flow = ControlFlow::Exit;
        },
        Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
            rendering_system.recreate_swapchain();
        },
        Event::RedrawEventsCleared => {
            previous_frame_end.as_mut().take().unwrap().cleanup_finished();
            let elapsed = timer.elapsed().as_secs() as f64 + timer.elapsed().subsec_nanos() as f64 / 1_000_000_000.0;
            let elapsed_as_radians = elapsed * pi::<f64>() / 180.0;

            model_pool.get_mut("suzanne").unwrap()
                .zero_rotation()
                .rotate(elapsed_as_radians as f32 * 50.0, vec3(0.0, 0.0, 1.0))
                .rotate(elapsed_as_radians as f32 * 30.0, vec3(0.0, 1.0, 0.0))
                .rotate(elapsed_as_radians as f32 * 20.0, vec3(1.0, 0.0, 0.0));
            model_pool.get_mut("teapot").unwrap()
                .zero_rotation()
                .rotate(elapsed_as_radians as f32 * 10.0, vec3(0.0, 0.0, 1.0))
                .rotate(elapsed_as_radians as f32 * 50.0, vec3(0.0, 1.0, 0.0))
                .rotate(elapsed_as_radians as f32 * 40.0, vec3(1.0, 0.0, 0.0));

            let teapot = model_pool.get("teapot").unwrap();
            let suzanne = model_pool.get("suzanne").unwrap();

            rendering_system.start_render().unwrap();
            rendering_system.add_model(suzanne).unwrap();
            rendering_system.add_model(teapot).unwrap();
            rendering_system.calculate_ambient_light(&ambient_light);
            for light in &directional_lights {
                rendering_system.calculate_directional_light(light).unwrap();
            }
            rendering_system.finish_render(&mut previous_frame_end).unwrap();
        },
        _ => ()
    });
}