use std::fs::File;
use std::path::Path;
use symphonia::core::audio::{SampleBuffer};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::probe::Hint;
use crate::ResourcePool;

impl ResourcePool<Sound> {
    pub fn load(&mut self, resource_id: &str, file_path: &str) -> Result<(), String> {
        let sound = Sound::load(file_path);
        self.0.insert(String::from(resource_id), sound);
        Ok(())
    }
}

#[derive(Clone)]
pub struct Sound {
    samples: Vec<f32>,
    sample_rate: u32,
    channels: usize
}
impl Sound {
    pub fn load(file_path: &str) -> Self {
        // Opens the file and create a MediaSourceStream from it
        let path = Path::new(file_path);
        let file = Box::new(File::open(path).unwrap());
        let media_source = MediaSourceStream::new(file, Default::default());

        // Help the library to infer the decoder needed for the file
        let mut format_hint = Hint::new();
        if let Some(extension) = path.extension() {
            if let Some(extension_str) = extension.to_str() {
                format_hint.with_extension(extension_str);
            }
        }

        // Probe our MSS for a format
        let probed = symphonia::default::get_probe().format(
            &format_hint,
            media_source,
            &Default::default(),
            &Default::default(),
        ).unwrap();
        let mut format = probed.format;

        // Find a default track to read from
        let track = format.default_track().unwrap().clone();

        // Our decoder
        let mut decoder =
            symphonia::default::get_codecs().make(&track.codec_params, &Default::default()).unwrap();

        let mut sound = Sound {
            samples: vec!(),
            sample_rate: track.codec_params.sample_rate.unwrap(),
            channels: track.codec_params.channels.unwrap().count()
        };

        // The sample buffer needs information that we get after decoding at least
        // one packet, so we instantiate it later
        let mut sample_buffer = None;

        let mut samples_read = 0;
        while let Ok(packet) = format.next_packet() {
            // Skip the packet if it does not belong to our track
            if packet.track_id() != track.id { continue }

            // Actual decoding
            if let Ok(audio_buffer) = decoder.decode(&packet) {
                // cpal (our audio playback backend) needs the samples in an interleaved
                // format, which is why we need to copy the contents of the buffer to
                // a sample buffer.
                if sample_buffer.is_none() {
                    let specification = *audio_buffer.spec();
                    let duration = audio_buffer.capacity() as u64;
                    sample_buffer = Some(SampleBuffer::<f32>::new(duration, specification));
                }

                if let Some(buffer) = &mut sample_buffer {
                    buffer.copy_interleaved_ref(audio_buffer);
                    let samples: Vec<f32> = buffer
                        .samples()
                        .to_vec();
                    samples_read += samples.len();
                    sound.samples.extend_from_slice(&samples);
                }
            }
        }

        sound
    }

    pub fn samples(&self) -> Vec<f32> { self.samples.clone() }
    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn channel_count(&self) -> usize { self.channels }
}