use audiopus::{Channels, coder::Decoder};

use crate::{AppResult, BUFFER_SIZE, apperror::AppError::XCustomError};

pub struct OpusDecoder {
    decoder: Decoder,
    channels: u8,
    buffer: Vec<f32>,
}

impl OpusDecoder {
    pub fn new(sample_rate: u32, channels: u8) -> AppResult<Self> {
        let channel = match channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => return Err(XCustomError("Invalid channel format")),
        };

        let sample_rate = match sample_rate {
            8_000 => audiopus::SampleRate::Hz8000,
            12_000 => audiopus::SampleRate::Hz12000,
            16_000 => audiopus::SampleRate::Hz16000,
            24_000 => audiopus::SampleRate::Hz24000,
            48_000 => audiopus::SampleRate::Hz48000,
            _ => return Err(XCustomError("Invalid sample rate")),
        };
        let decoder = Decoder::new(sample_rate, channel)?;
        Ok(Self {
            decoder: decoder,
            channels,
            buffer: vec![0f32; BUFFER_SIZE * channels as usize],
        })
    }

    pub fn decode(&mut self, data: &[u8]) -> AppResult<Vec<f32>> {
        let samples_decoded = self
            .decoder
            .decode_float(Some(data), &mut self.buffer, false)?;
        let total_samples = samples_decoded * self.channels as usize;
        Ok(self.buffer[..total_samples].to_vec())
    }
}
