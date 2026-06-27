use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ringbuffer::{AllocRingBuffer, RingBuffer};

use crate::app::TranscriptionResult;
use crate::inference::backend::WhisperBackend;

/// Runs the transcription pipeline on a dedicated thread.
/// Uses std::thread::spawn (blocking thread) with synchronous whisper transcription.
/// This follows the plan's "spawn_blocking" pattern: CPU-bound transcription
/// runs off the main thread, keeping the Iced event loop responsive.
pub fn run_worker(
    ring_buffer: Arc<Mutex<AllocRingBuffer<f32>>>,
    mut backend: WhisperBackend,
    result_tx: std::sync::mpsc::Sender<TranscriptionResult>,
) {
    thread::spawn(move || {
        loop {
            // Collect 1-second chunk (16000 samples at 16kHz sample rate)
            let chunk = {
                let mut rb = ring_buffer.lock().unwrap();
                let samples = 16000; // 1 second of audio at 16kHz
                let mut chunk = Vec::with_capacity(samples);
                for _ in 0..samples {
                    if let Some(value) = rb.dequeue() {
                        chunk.push(value);
                    }
                }
                chunk
            };

            if !chunk.is_empty() {
                // Synchronous transcription — no tokio runtime needed
                // because whisper-rs pipeline (pcm_to_mel, encode, decode)
                // is purely CPU-bound with no I/O or async points.
                match backend.transcribe_segment_sync(&chunk) {
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
            // Sleep between polls to avoid busy-waiting.
            // 500ms allows the ring buffer to accumulate new samples
            // while keeping latency under ~1.5s.
            thread::sleep(Duration::from_millis(500));
        }
    });
}
