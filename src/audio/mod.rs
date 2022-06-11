mod signal_processing;

use std::sync::{Arc, Mutex};
use cpal::{Device, Host, Stream};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rustfft::num_complex::Complex;
use crate::audio::signal_processing::fft;
use crate::Sound;

#[derive(Default, Clone)]
pub struct RealtimeAttributes {
    pub fft: Vec<Complex<f32>>
}

#[derive(Default)]
pub struct GeneralAttributes {}

pub struct AudioPlayer {
    device: Device,
    stream: Option<Stream>,

    realtime_attributes: Arc<Mutex<RealtimeAttributes>>,
    general_attributes: GeneralAttributes
}
impl AudioPlayer {
    pub fn new() -> Self {
        let host = cpal::default_host();
        let device = host.default_output_device().expect("no output device available");

        AudioPlayer {
            device,
            stream: None,

            realtime_attributes: Default::default(),
            general_attributes: Default::default(),
        }
    }

    pub fn play(&mut self, sound: &Sound) {
        let config = cpal::StreamConfig {
            channels: sound.channel_count() as cpal::ChannelCount,
            sample_rate: cpal::SampleRate(sound.sample_rate()),
            buffer_size: cpal::BufferSize::Default
        };

        let mut samples = sound.samples().into_iter();

        // ------------
        // Here we would calculate the general attributes,
        // tedious operations which should never be done
        // in real time
        // -------------

        self.realtime_attributes = Default::default();
        let inner_rta = self.realtime_attributes.clone();
        let stream = self.device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Fill the buffer with as much samples as we can
                let written = std::cmp::min(data.len(), samples.len());
                for sample in data[..written].iter_mut() {
                    *sample = samples.next().unwrap();
                }

                data[written..].iter_mut().for_each(|s| *s = 0.0);

                let mut rta = inner_rta.lock().unwrap();
                (*rta).fft = fft(&data[..written]);
                drop(rta)
            },
            move |e| eprintln!("audio output error: {:?}", e)
        );

        if let Ok(s) = stream {
            self.stream = Some(s);
        }

        self.resume();
    }

    pub fn get_realtime_attributes(&self) -> RealtimeAttributes {
        let rta = self.realtime_attributes.lock().unwrap();
        (*rta).clone()
    }

    pub fn resume(&mut self) {
        if let Some(s) = &self.stream {
            s.play().unwrap()
        }
    }

    pub fn pause(&mut self) {
        if let Some(s) = &self.stream {
            s.pause().unwrap()
        }
    }
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new()
    }
}