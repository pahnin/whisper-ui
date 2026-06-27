use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ringbuffer::{AllocRingBuffer, RingBuffer};
use tokio::runtime::Runtime;

use crate::app::TranscriptionResult;
use crate::inference::backend::TranscriptionBackend;
use crate::inference::backend::WhisperBackend;

pub fn run_worker(
    ring_buffer: Arc<Mutex<AllocRingBuffer<f32>>>,
    mut backend: WhisperBackend,
    result_tx: std::sync::mpsc::Sender<TranscriptionResult>,
) {
    let rt = Runtime::new().unwrap();

    thread::spawn(move || {
        rt.block_on(async {
            loop {
                let chunk = {
                    let mut rb = ring_buffer.lock().unwrap();
                    let mut chunk = Vec::new();
                    let samples = 16000 * 2;
                    for _ in 0..samples {
                        if let Some(value) = rb.dequeue() {
                            chunk.push(value);
                        }
                    }
                    chunk
                };

                if !chunk.is_empty() {
                    match backend.transcribe_segment(&chunk).await {
                        Ok(segment) => {
                            if !segment.text.is_empty() {
                                if let Err(e) = result_tx.send(TranscriptionResult::Segment(segment.text)) {
                                    eprintln!("Failed to send transcription result: {}", e);
                                    break;
                                }
                            }
                        }
                        Err(e) => {
                            if let Err(e2) = result_tx.send(TranscriptionResult::Error(format!("{}", e))) {
                                eprintln!("Failed to send error: {}", e2);
                                break;
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(2000));
            }
        });
    });
}
