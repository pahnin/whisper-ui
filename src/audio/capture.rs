use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait};
use cpal::{SampleFormat, StreamConfig};

const TARGET_SAMPLE_RATE: u32 = 16000;
const TARGET_CHANNELS: u16 = 1;
const RING_BUFFER_SECONDS: usize = 10;

struct DeviceConfig {
    stream_config: StreamConfig,
    sample_format: SampleFormat,
}

fn find_supported_config(
    device: &cpal::Device,
    target_rate: u32,
    target_channels: u16,
) -> Option<DeviceConfig> {
    let supported: Vec<_> = device
        .supported_input_configs()
        .map_err(|e| eprintln!("Failed to get supported configs: {}", e))
        .ok()?
        .collect();

    if supported.is_empty() {
        return None;
    }

    let best = supported
        .iter()
        .max_by(|a, b| {
            let a_score = score_config(a, target_rate, target_channels);
            let b_score = score_config(b, target_rate, target_channels);
            a_score.cmp(&b_score)
        })?;

    let sample_rate = {
        let min_rate = best.min_sample_rate();
        let max_rate = best.max_sample_rate();
        if target_rate >= min_rate.0 && target_rate <= max_rate.0 {
            cpal::SampleRate(target_rate)
        } else if target_rate < min_rate.0 {
            min_rate
        } else {
            max_rate
        }
    };

    let channels = if best.channels() == target_channels {
        target_channels
    } else if target_channels > best.channels() {
        best.channels()
    } else {
        let supports_target = device
            .supported_input_configs()
            .map_err(|_| ())
            .ok()?
            .any(|c| c.channels() == target_channels);
        if supports_target {
            target_channels
        } else {
            best.channels()
        }
    };

    let buffer_size = supported
        .iter()
        .find(|c| !matches!(c.buffer_size(), cpal::SupportedBufferSize::Range { .. }))
        .and_then(|c| match c.buffer_size() {
            cpal::SupportedBufferSize::Range { min, .. } => Some(cpal::BufferSize::Fixed(*min)),
            cpal::SupportedBufferSize::Unknown => Some(cpal::BufferSize::Default),
        })
        .unwrap_or(cpal::BufferSize::Default);

    Some(DeviceConfig {
        stream_config: StreamConfig {
            channels,
            sample_rate,
            buffer_size,
        },
        sample_format: best.sample_format(),
    })
}

fn score_config(
    config: &cpal::SupportedStreamConfigRange,
    target_rate: u32,
    target_channels: u16,
) -> u64 {
    let sample_rate = config.min_sample_rate().0.max(
        config.max_sample_rate().0.min(target_rate),
    );
    let rate_diff = (sample_rate as i64 - target_rate as i64).unsigned_abs();
    let rate_score = 1000u64.saturating_sub(rate_diff as u64);
    let channel_score = if config.channels() == target_channels {
        100u64
    } else {
        10u64
    };
    rate_score + channel_score
}

fn build_ring_buffer_capacity(sample_rate: u32, _channels: u16, seconds: usize) -> usize {
    sample_rate as usize * seconds
}

pub struct AudioCapture {
    pub sample_rate: u32,
    device_config: DeviceConfig,
    ring_buffer: Arc<crossbeam_queue::ArrayQueue<f32>>,
    running: Arc<AtomicBool>,
    level_tx: Sender<f32>,
    stream_handle: Option<cpal::Stream>,
}

impl AudioCapture {
    pub fn new(level_tx: Sender<f32>) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No default input device found".to_string())?;

        let device_config = find_supported_config(&device, TARGET_SAMPLE_RATE, TARGET_CHANNELS)
            .ok_or_else(|| "No supported audio input config found".to_string())?;

        let capacity =
            build_ring_buffer_capacity(
                device_config.stream_config.sample_rate.0,
                device_config.stream_config.channels,
                RING_BUFFER_SECONDS,
            );
        let ring_buffer: Arc<crossbeam_queue::ArrayQueue<f32>> =
            Arc::new(crossbeam_queue::ArrayQueue::new(capacity));

        Ok(Self {
            sample_rate: device_config.stream_config.sample_rate.0,
            device_config,
            ring_buffer,
            running: Arc::new(AtomicBool::new(false)),
            level_tx,
            stream_handle: None,
        })
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        let ring_buffer = self.ring_buffer.clone();
        let config = self.device_config.stream_config.clone();
        let sample_format = self.device_config.sample_format;
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No default input device found".to_string())?;
        let level_tx = self.level_tx.clone();

        let err_fn = move |err| eprintln!("audio error: {}", err);

        match sample_format {
            SampleFormat::F32 => {
                let stream = device.build_input_stream::<f32, _, _>(
                    &config,
                    move |data: &[f32], _info: &cpal::InputCallbackInfo| {
                        if !data.is_empty() {
                            for &value in data {
                                let _ = ring_buffer.push(value);
                            }
                            let rms = (data.iter().map(|x| x * x).sum::<f32>()
                                / data.len() as f32)
                                .sqrt()
                                * 100.0;
                            let _ = level_tx.send(rms);
                        }
                    },
                    err_fn,
                    Some(Duration::from_secs(1)),
                );

                let stream = stream.map_err(|e| format!("Failed to build input stream: {}", e))?;
                self.stream_handle = Some(stream);
            }
            SampleFormat::I16 => {
                let stream = device.build_input_stream::<i16, _, _>(
                    &config,
                    move |data: &[i16], _info: &cpal::InputCallbackInfo| {
                        if !data.is_empty() {
                            for &value in data {
                                let _ = ring_buffer.push(value as f32 / 32768.0);
                            }
                            let rms = data
                                .iter()
                                .map(|x| (*x as f32 / 32768.0).powi(2))
                                .sum::<f32>()
                                / data.len() as f32;
                            let _ = level_tx.send(rms.sqrt() * 100.0);
                        }
                    },
                    err_fn,
                    Some(Duration::from_secs(1)),
                );

                let stream = stream.map_err(|e| format!("Failed to build input stream: {}", e))?;
                self.stream_handle = Some(stream);
            }
            _ => {
                return Err(format!(
                    "Unsupported sample format: {:?}",
                    sample_format
                ));
            }
        }

        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.stream_handle.take();
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn get_ring_buffer(&self) -> Arc<crossbeam_queue::ArrayQueue<f32>> {
        self.ring_buffer.clone()
    }

    pub fn get_running(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}
