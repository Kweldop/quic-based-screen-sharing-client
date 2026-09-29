use audioadapter_buffers::direct::SequentialSliceOfVecs;
use rubato::{Async, FixedAsync, Resampler, SincInterpolationParameters, WindowFunction};

use crate::AppResult;

pub struct AudioResampler {
    resampler: Async<f32>,
    input_buf: Vec<Vec<f32>>,
    output_buf: Vec<Vec<f32>>,
    channels: usize,
}

impl AudioResampler {
    pub fn new(
        from_sample_rate: u32,
        to_sample_rate: u32,
        channels: usize,
        chunk_size: usize,
    ) -> AppResult<Self> {
        let params = SincInterpolationParameters::new(256, WindowFunction::BlackmanHarris2);
        let resampler = Async::<f32>::new_sinc(
            to_sample_rate as f64 / from_sample_rate as f64,
            1.1,
            &params,
            chunk_size,
            channels,
            FixedAsync::Input,
        )?;

        let max_out = resampler.output_frames_max();
        let input_buf = vec![vec![0.0f32; chunk_size]; channels];
        let output_buf = vec![vec![0.0f32; max_out]; channels];

        Ok(Self {
            resampler,
            input_buf,
            output_buf,
            channels,
        })
    }

    pub fn resample(&mut self, interleaved: &[f32]) -> AppResult<Vec<f32>> {
        let frames = interleaved.len() / self.channels;

        for ch in 0..self.channels {
            self.input_buf[ch].resize(frames, 0.0);
            for f in 0..frames {
                self.input_buf[ch][f] = interleaved[f * self.channels + ch];
            }
        }

        let max_out = self.resampler.output_frames_max();
        for ch in 0..self.channels {
            self.output_buf[ch].resize(max_out, 0.0);
        }

        let input = SequentialSliceOfVecs::new(&self.input_buf, self.channels, frames)?;
        let mut output =
            SequentialSliceOfVecs::new_mut(&mut self.output_buf, self.channels, max_out)?;

        let (_, output_frames) = self
            .resampler
            .process_into_buffer(&input, &mut output, None)?;

        let mut out = Vec::with_capacity(output_frames * self.channels);
        for f in 0..output_frames {
            for ch in 0..self.channels {
                out.push(self.output_buf[ch][f]);
            }
        }

        Ok(out)
    }
}
