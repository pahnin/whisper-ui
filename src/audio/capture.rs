use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait};
use ringbuffer::{AllocRingBuffer, RingBuffer};

const SAMPLE_RATE: u32 = 16000;
const CHANNELS: u16 = 1;
const BUFFER_DURATION_SECS: f64 = 2.0;
const RING_BUFFER_CAPACITY: usize = 10;

pub struct AudioCapture {
    config: cpal::StreamConfig,
    ring_buffer: Arc<Mutex<AllocRingBuffer<f32>>>,
    running: Arc<AtomicBool>,
    level_tx: Sender<f32>,
    stream_handle: Option<cpal::Stream>,
}

impl AudioCapture {
    pub fn new(level_tx: Sender<f32>) -> Result<Self, String> {
        let host = cpal::default_host();
        let _device = host
            .default_input_device()
            .ok_or("No default input device found".to_string())?;

        let config = cpal::StreamConfig {
            channels: CHANNELS,
            sample_rate: cpal::SampleRate(SAMPLE_RATE),
            buffer_size: cpal::BufferSize::Default,
        };

        let ring_buffer: Arc<Mutex<AllocRingBuffer<f32>>> =
            Arc::new(Mutex::new(AllocRingBuffer::new(RING_BUFFER_CAPACITY * SAMPLE_RATE as usize)));

        Ok(Self {
            config,
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
                    if !data.is_empty() {
                        if let Ok(mut rb) = ring_buffer.lock() {
                            let available = rb.capacity() - rb.len();
                            if available > 0 {
                                let to_push = data.len().min(available);
                                for &value in &data[..to_push] {
                                    let _ = rb.push(value);
                                }
                            }
                        }
                        let rms = (data.iter().map(|x| x * x).sum::<f32>() / data.len() as f32).sqrt() * 100.0;
                        let _ = level_tx.send(rms);
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

    pub fn get_ring_buffer(&self) -> Arc<Mutex<AllocRingBuffer<f32>>> {
        self.ring_buffer.clone()
    }

    pub fn get_running(&self) -> Arc<AtomicBool> {
        self.running.clone()
    }
}
