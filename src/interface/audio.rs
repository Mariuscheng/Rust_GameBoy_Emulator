use cpal::{
    traits::{DeviceTrait, HostTrait},
    Sample, SizedSample, Stream, StreamConfig,
};
use std::fmt;

pub trait AudioInterface: fmt::Debug {
    fn push_sample(&mut self, sample: f32);
    fn start(&mut self);
    fn stop(&mut self);
}

// Implement special Debug to handle cpal types
#[derive(Default)]
pub struct CpalAudioOutput {
    _stream: Option<Stream>,
    device_id: String,
    config: Option<StreamConfig>,
}

impl fmt::Debug for CpalAudioOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CpalAudioOutput")
            .field("device_id", &self.device_id)
            .field("has_stream", &self._stream.is_some())
            .field("config", &self.config)
            .finish()
    }
}

impl CpalAudioOutput {
    pub fn new(_sample_rate: u32) -> Self {
        let host = cpal::default_host();
        if let Some(device) = host.default_output_device() {
            let config = device
                .default_output_config()
                .ok()
                .map(|config| config.config());

            Self {
                _stream: None,
                device_id: device.name().unwrap_or_else(|_| "Unknown".to_string()),
                config,
            }
        } else {
            Self::default()
        }
    }

    #[allow(dead_code)]
    fn build_stream<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Stream
    where
        T: Sample + SizedSample + Send + 'static + num_traits::cast::FromPrimitive,
    {
        let channels = config.channels as usize;
        let mut sample_clock = 0f32;
        let sample_rate = config.sample_rate.0 as f32;
        let mut next_value = move || {
            sample_clock = (sample_clock + 1.0) % sample_rate;
            (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()
        };

        device
            .build_output_stream(
                config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    for sample in data.chunks_mut(channels) {
                        let value = next_value();
                        for sample in sample.iter_mut() {
                            *sample = T::from_f32(value).unwrap_or(T::from_f32(0.0).unwrap());
                        }
                    }
                },
                |err| eprintln!("an error occurred on stream: {}", err),
                None,
            )
            .expect("Failed to build output stream")
    }
}

impl AudioInterface for CpalAudioOutput {
    fn push_sample(&mut self, _sample: f32) {
        // TODO: Implement audio sample pushing
    }

    fn start(&mut self) {
        // TODO: Implement audio start
    }

    fn stop(&mut self) {
        // TODO: Implement audio stop
    }
}

// Empty audio output implementation for testing or disabling sound
#[derive(Debug)]
pub struct NullAudioOutput;

impl AudioInterface for NullAudioOutput {
    fn push_sample(&mut self, _sample: f32) {}
    fn start(&mut self) {}
    fn stop(&mut self) {}
}
