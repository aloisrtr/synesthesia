extern crate nalgebra_glm as glm;

use std::f32::consts::PI;
use std::sync::Arc;
use std::time::Instant;
use glm::{scale, vec3};
use vulkano::sync::GpuFuture;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use crate::audio::{AudioPlayer, RealtimeAttributes};
use crate::graphics::{AmbientLight, DirectionalLight, RenderingSystem};
use crate::resource_pool::model_loader::Model;
use crate::resource_pool::ResourcePool;
use crate::resource_pool::sound_loader::Sound;

mod graphics;
mod resource_pool;
mod audio;

pub struct Synesthesia {
    scene: Option<Scene>,

    event_loop: EventLoop<()>,
    rendering_system: RenderingSystem,
    previous_frame_end: Option<Box<dyn GpuFuture>>,

    audio_player: AudioPlayer,

    model_pool: ResourcePool<Model>,
    sound_pool: ResourcePool<Sound>,
}
impl Synesthesia {
    pub fn init() -> Self {
        let event_loop = EventLoop::new();
        let (mut rendering_system, previous_frame_end) = RenderingSystem::new(&event_loop);
        rendering_system.set_orthogonal_projection();

        Self {
            scene: None,

            event_loop,
            rendering_system,
            previous_frame_end,

            audio_player: Default::default(),

            model_pool: Default::default(),
            sound_pool: Default::default()
        }
    }

    pub fn load_scene(&mut self, script_path: &str) {
        self.sound_pool.load("sound", script_path);
        self.audio_player.play(self.sound_pool.get("sound").unwrap());

        self.model_pool.load("cube", "assets/models/cube.obj");
        let cubes_count = 64;
        let cubes: Vec<Model> = (0..=cubes_count).map(|i| {
            self.model_pool.get_copy("cube").unwrap()
        }).collect();

        let scene = Scene {
            main: Arc::new(move |scene, dt, rta| {
                let hamming_window = |n: usize, a: f32| a - (1.0 - a) * ((2.0 * PI * n as f32) / rta.fft.len() as f32).cos();
                let normalization_factor = 1.0 / (rta.fft.len() as f32).sqrt();
                let powers: Vec<f32> = rta.fft.iter().enumerate().map(|(i, v)| {
                    (v * normalization_factor * hamming_window(i, 0.53836)).norm()
                }).collect();
                let intensity = powers.iter().fold(0.0, |acc, p| acc + (p + 1.0).log10()) / powers.len() as f32;
                let cubes_count = scene.models.len();
                let cube_width = 1.0 / cubes_count as f32;
                let smoothing_delta = 0.6;

                let mut cubes = scene.models.iter_mut();

                let background_cube = cubes.next().unwrap();
                let rotation = dt * PI / 180.0;
                background_cube.rotate(rotation * 2.0, vec3(1.0, 0.0, 0.0));
                background_cube.rotate(rotation * 3.0, vec3(0.0, 1.0, 0.0));
                background_cube.rotate(rotation * 5.0, vec3(0.0, 0.0, 1.0));
                background_cube.set_position(vec3(0.0, -0.5, -4.0));
                background_cube.reset_scaling();
                background_cube.scale(vec3(1.5, 1.5, 1.5));
                background_cube.scale(vec3(1.0 + intensity * 3.0, 1.0 + intensity * 3.0, 1.0 + intensity * 3.0));
                background_cube.set_color(vec3(1.0, 0.2, 0.0));

                for (i, cube) in cubes.enumerate() {
                    // Collect values in a gamma-corrected frequency range (using gamma = 2)
                    let values: Vec<f32> = powers
                        .iter()
                        .enumerate()
                        .filter(|(fi, _)| {
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
                    let last_iteration_scale = cube.get_scale().y;
                    scale_value = last_iteration_scale * smoothing_delta + scale_value * (1.0 - smoothing_delta);

                    cube.set_position(vec3(-1.0 + (cube_width * 2.0 * i as f32), 0.5 - scale_value, -0.5));
                    cube.reset_scaling();
                    cube.scale(vec3(cube_width, scale_value, cube_width));
                    cube.set_color(
                        vec3(
                        (scale_value * PI).sin() * 2.0,
                        0.2,
                        (scale_value * PI).cos()
                        )
                    );
                }
            }),
            models: cubes,
            ambient: AmbientLight { color: [1.0, 1.0, 1.0], intensity: 0.5 },
            directionals: vec![
                DirectionalLight { position: [-4.0, 0.0, 0.0], color: [1.0, 1.0, 1.0], intensity: 1.0 },
            ]
        };

        self.scene = Some(scene);
    }

    pub fn run(mut self) {
        let mut last_frame = Instant::now();
        self.event_loop.run(move |event, _, control_flow| match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent { event: WindowEvent::Resized(_), .. } => {
                self.rendering_system.recreate_swapchain();
            },
            Event::WindowEvent { event: WindowEvent::KeyboardInput { input: i, .. }, .. } => {
                match i.scancode {
                    87 => self.rendering_system.set_fullscreen(),
                    1 => *control_flow = ControlFlow::Exit,
                    _ => ()
                }
            },
            Event::RedrawEventsCleared => {
                self.previous_frame_end.as_mut().take().unwrap().cleanup_finished();
                if let Some(mut s) = self.scene.take() {
                    // Update our scene
                    let dt = last_frame.elapsed().as_secs_f32();
                    last_frame = Instant::now();
                    let main = s.main.clone();
                    main(&mut s, dt, self.audio_player.get_realtime_attributes());

                    // Then drawing it
                    self.rendering_system.start_render().unwrap();
                    for model in &s.models {
                        if self.rendering_system.add_model(model).is_err() { self.scene = Some(s); return; };
                    }
                    if self.rendering_system.calculate_ambient_light(&s.ambient).is_err() { self.scene = Some(s); return;};
                    for light in &s.directionals {
                        self.rendering_system.calculate_directional_light(light).unwrap();
                    }
                    if self.rendering_system.finish_render(&mut self.previous_frame_end).is_err() { self.scene = Some(s); return; }

                    self.scene = Some(s);
                }
            },
            _ => ()
        });
    }
}

/// A scene should be composed of:
/// - a main function, that will be run once every single frame
/// - zero or more entities
/// - one ambient light
/// - zero or more directional lights
struct Scene {
    pub main: Arc<dyn Fn(
        &mut Scene,
        f32,
        RealtimeAttributes
    )>,
    pub models: Vec<Model>,
    pub ambient: AmbientLight,
    pub directionals: Vec<DirectionalLight>
}