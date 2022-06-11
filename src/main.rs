/*
use std::time::Instant;
use std::default::Default;
use std::f32::consts::PI;
use glm::{look_at, ortho, pi, translate, vec2, vec3};
use rustfft::num_traits::pow;
use vulkano::sync::GpuFuture;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use graphics::RenderingSystem;
use crate::audio::AudioPlayer;
use crate::graphics::rendering_system::Render;
use crate::graphics::{AmbientLight, DirectionalLight};
use crate::resource_pool::model_loader::Model;
use crate::resource_pool::sound_loader::Sound;
use crate::resource_pool::ResourcePool;
*/

use synesthesia::Synesthesia;

fn main()  {
    let mut synesthesia: Synesthesia = Synesthesia::init();
    synesthesia.load_scene(&std::env::args().nth(1).expect("please provide a sound sample"));
    synesthesia.run()

    /*
    let event_loop = EventLoop::new();
    let (mut rendering_system, mut previous_frame_end) =
        RenderingSystem::new(&event_loop);

    let mut model_pool: ResourcePool<Model> = ResourcePool::default();
    model_pool.load("cube", "assets/models/cube.obj");
    let cubes_count = 64;
    let cube_width = 2.0 / cubes_count as f32;
    let mut cubes: Vec<Model> = (0..cubes_count).map(|i| {
        model_pool.get_copy("cube").unwrap()
    }).collect();
    let mut last_iteration_height: Vec<f32> = vec!();

    let mut audio_player = AudioPlayer::new();
    let mut sound_pool: ResourcePool<Sound> = ResourcePool::default();
    let sound_file = std::env::args().nth(1).expect("please provide a path to a sound file");
    sound_pool.load("sound", &sound_file);
    audio_player.play(sound_pool.get("sound").unwrap());

    let ambient_light = AmbientLight { color: [0.0, 1.0, 1.0], intensity: 1.0 };

    let directional_lights = vec![
        DirectionalLight { position: [-4.0, 0.0, -2.0], color: [1.0, 1.0, 1.0], intensity: 0.0 },
    ];

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
            *control_flow = ControlFlow::Exit;
        },
        Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
            rendering_system.recreate_swapchain();
        },
        Event::RedrawEventsCleared => {
            let rta = audio_player.get_realtime_attributes();
            let hamming_window = |n: usize, a: f32| a - (1.0 - a) * ((2.0 * PI * n as f32) / rta.fft.len() as f32).cos();
            let normalization_factor = 1.0 / (rta.fft.len() as f32).sqrt();
            let powers: Vec<f32> = rta.fft.iter().enumerate().map(|(i, v)| {
                (v * normalization_factor * hamming_window(i, 0.53836)).norm()
            }).collect();


            for (i, cube) in cubes.iter_mut().enumerate() {
                // Collect values in a gamma-corrected frequency range (using gamma = 2)
                let values: Vec<f32> = powers
                    .iter()
                    .enumerate()
                    .filter(|(fi, f)| {
                        i == (((*fi as f32 / powers.len() as f32).sqrt()) * cubes_count as f32) as usize
                    })
                    .map(|(_, f)| *f)
                    .collect();

                let mut scale_value = if values.is_empty() {
                    0.0
                } else {
                    // Take only the logarithm of the maximum power to avoid swamping values
                    (values.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap() + 1.0).log10()
                };
                // Smooth the scale value
                let smoothing_delta = 0.25;
                scale_value = last_iteration_height.get(i).unwrap_or(&0.0) * smoothing_delta + scale_value * (1.0 - smoothing_delta);
                last_iteration_height.insert(i, scale_value);
                cube.set_position(vec3(-1.0 + (cube_width * i as f32), 0.5 - scale_value, -0.5));
                cube.reset_scaling();
                cube.scale(vec3(cube_width, scale_value, cube_width));
            }

            previous_frame_end.as_mut().take().unwrap().cleanup_finished();

            rendering_system.start_render().unwrap();
            for cube in &cubes {
                rendering_system.add_model(cube).unwrap()
            }
            rendering_system.calculate_ambient_light(&ambient_light);
            for light in &directional_lights {
                rendering_system.calculate_directional_light(light).unwrap();
            }
            rendering_system.finish_render(&mut previous_frame_end).unwrap();
        },
        _ => ()
    });
     */
}