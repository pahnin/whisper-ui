use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait};
use ringbuffer::{AllocRingBuffer, RingBuffer};
use tokio::sync::mpsc;
use tokio::sync::Mutex as AsyncMutex;

const SAMPLE_RATE: u32 = 16000;
const CHANNELS: u16 = 1;
const BUFFER_DURATION_SECS: f64 = 2.0;
const RING_BUFFER_CAPACITY: usize = 10;

pub struct AudioCapture {
    config: cpal::StreamConfig,
    ring_buffer: Arc<AsyncMutex<AllocRingBuffer<f32>>>,
    running: Arc<AtomicBool>,
    level_tx: Option<mpsc::UnboundedSender<f32>>,
    stream_handle: Option<cpal::Stream>,
}

impl AudioCapture {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let _device = host
            .default_input_device()
            .ok_or("No default input device found".to_string())?;

        let config = cpal::StreamConfig {
            channels: CHANNELS,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let ring_buffer: Arc<AsyncMutex<AllocRingBuffer<f32>>> =
            Arc::new(AsyncMutex::new(AllocRingBuffer::new(RING_BUFFER_CAPACITY * SAMPLE_RATE as usize)));

        Ok(Self {
            config,
            ring_buffer,
            running: Arc::new(AtomicBool::new(false)),
            level_tx: None,
            stream_handle: None,
        })
    }

    pub fn set_level_sender(&mut self, tx: mpsc::UnboundedSender<f32>) {
        self.level_tx = Some(tx);
    }

    pub fn start(&mut self) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);

        let ring_buffer = self.ring_buffer.clone();
        let config = self.config.clone();
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No default input device found".to_string())?;
        let level_tx = self.level_tx.clone();

        let err_fn = move |err| eprintln!("audio error: {}", err);

        let stream = device
            .build_input_stream::<f32, _, _>(
                &config,
                move |data: &[f32], _: &_| {
                    if let Some(&value) = data.first() {
                        let mut rb = ring_buffer.blocking_lock();
                        let _ = rb.push(value);
                        drop(rb);

                        if let Some(ref tx) = level_tx {
                            let _ = tx.send((value.abs() * 100.0) as f32);
                        }
                    }
                },
                err_fn,
                Some(Duration::from_secs(1)),
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        self.stream_handle = Some(stream);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.stream_handle.take();
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn get_audio_chunk(&self) -> Vec<f32> {
        let mut chunk = Vec::new();
        let samples = (SAMPLE_RATE as usize) * BUFFER_DURATION_SECS as usize;
        let mut rb = self.ring_buffer.blocking_lock();

        for _ in 0..samples {
            if let Some(value) = rb.dequeue() {
                chunk.push(value);
            }
        }

        chunk
    }

    pub fn get_audio_level(&self) -> f32 {
        let mut sum = 0.0;
        let mut count = 0u64;
        let mut temp_values = Vec::new();
        let mut rb = self.ring_buffer.blocking_lock();

        for _ in 0..100 {
            if let Some(value) = rb.dequeue() {
                sum += value.abs() as f64;
                count += 1;
                temp_values.push(value);
            }
        }

        for value in temp_values {
            let _ = rb.push(value);
        }

        if count > 0 {
            ((sum / count as f64) * 100.0) as f32
        } else {
            0.0
        }
    }
}
