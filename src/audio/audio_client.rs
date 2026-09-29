use cpal::{
    Device, Stream, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, SyncSender},
};

use crate::{
    AppResult, BUFFER_SIZE, apperror::AppError::XCustomError, audio::resampler::AudioResampler,
};
use std::sync::mpsc;

pub struct CpalClient {
    device: Device,
    config: StreamConfig,
    sample_tx: SyncSender<f32>,
    sample_rx: Option<Receiver<f32>>, // taken by stream()
    resampler: Option<Arc<Mutex<AudioResampler>>>,
}

impl CpalClient {
    pub fn new(sample_rate: u32) -> AppResult<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or(XCustomError("no output device"))?;

        let supported = device
            .supported_output_configs()?
            .find(|c| c.channels() == 2)
            .or_else(|| device.supported_output_configs().ok()?.next())
            .ok_or(XCustomError("no supported config"))?;

        let min = supported.min_sample_rate();
        let max = supported.max_sample_rate();
        let actual_rate = sample_rate.clamp(min, max);

        println!(
            "device rate: {}Hz  source rate: {}Hz",
            actual_rate, sample_rate
        );

        let config = supported.with_sample_rate(actual_rate).config();

        // capacity = 8 frames — enough to absorb bursts without delay
        let capacity = BUFFER_SIZE * config.channels as usize * 8;
        let (tx, rx) = mpsc::sync_channel::<f32>(capacity);

        let resampler = if actual_rate != sample_rate {
            println!("resampling {}Hz → {}Hz", sample_rate, actual_rate);
            let r = AudioResampler::new(
                sample_rate,
                actual_rate,
                config.channels as usize,
                BUFFER_SIZE,
            )?;
            Some(Arc::new(Mutex::new(r)))
        } else {
            None
        };

        Ok(Self {
            device,
            config,
            sample_tx: tx,
            sample_rx: Some(rx),
            resampler,
        })
    }

    pub fn stream(&mut self) -> AppResult<Stream> {
        // take rx out of Option — can only call stream() once
        let rx = self
            .sample_rx
            .take()
            .ok_or(XCustomError("stream already started"))?;

        let stream = self.device.build_output_stream(
            self.config,
            move |output: &mut [f32], _| {
                for sample in output.iter_mut() {
                    // try_recv — never blocks, returns silence if empty
                    *sample = rx.try_recv().unwrap_or(0.0);
                }
            },
            |e| eprintln!("cpal error: {}", e),
            None,
        )?;

        stream.play()?;
        Ok(stream)
    }

    pub fn push_samples(&self, samples: &[f32]) {
        let to_send = if let Some(ref resampler) = self.resampler {
            let mut r = resampler.lock().unwrap();
            match r.resample(samples) {
                Ok(out) => out,
                Err(e) => {
                    eprintln!("resample: {}", e);
                    return;
                }
            }
        } else {
            samples.to_vec()
        };

        // send each sample — try_send drops if channel full
        // better than blocking the decode thread
        for sample in to_send {
            let _ = self.sample_tx.try_send(sample);
        }
    }
}
