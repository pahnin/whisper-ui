use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use ringbuffer::{AllocRingBuffer, RingBuffer};

use crate::app::TranscriptionResult;
use crate::inference::backend::WhisperBackend;

/// Audio sample rate used by cpal capture.
const SAMPLE_RATE: u32 = 16_000;

/// Minimum audio to accumulate before transcribing (4 seconds).
const MIN_AUDIO_SECS: u64 = 4;

/// Overlap between consecutive chunks (2 seconds).
const OVERLAP_SECS: u64 = 2;

/// Poll interval for checking ring buffer (50ms, reduced from 200ms).
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Runs the transcription pipeline on a dedicated thread.
/// Accumulates audio until MIN_AUDIO_SECS has elapsed, then transcribes
/// with OVERLAP_SECS of overlap between consecutive chunks.
pub fn run_worker(
    ring_buffer: Arc<Mutex<AllocRingBuffer<f32>>>,
    mut backend: WhisperBackend,
    result_tx: std::sync::mpsc::Sender<TranscriptionResult>,
    running: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    thread::spawn(move || {
        let min_samples: usize = (SAMPLE_RATE as usize) * (MIN_AUDIO_SECS as usize);
        let overlap_samples: usize = (SAMPLE_RATE as usize) * (OVERLAP_SECS as usize);

        let mut recent_audio: Vec<f32> = Vec::new();
        let mut flush_offset: usize = 0;
        let mut next_flush_at: usize = min_samples;

        let (_wake_tx, wake_rx): (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) = std::sync::mpsc::channel();

        loop {
            let running = running.load(Ordering::SeqCst);
            if !running {
                break;
            }

            // Drain available samples from ring buffer
            let mut drained: Vec<f32> = Vec::new();
            {
                let mut rb = match ring_buffer.lock() {
                    Ok(guard) => guard,
                    Err(e) => {
                        eprintln!("[WORKER] Ring buffer lock poisoned, exiting worker: {}", e);
                        break;
                    }
                };
                while let Some(value) = rb.dequeue() {
                    drained.push(value);
                }
            }

            if !drained.is_empty() {
                recent_audio.extend(drained);
            }

            // Check if we've reached the next flush threshold
            if recent_audio.len() >= next_flush_at {
                // Build chunk with overlap
                let start_idx = if flush_offset + overlap_samples < recent_audio.len() {
                    recent_audio.len().saturating_sub(next_flush_at.saturating_sub(flush_offset)).saturating_sub(overlap_samples)
                } else {
                    0
                };
                let chunk_end = recent_audio.len();
                let chunk = recent_audio[start_idx..chunk_end].to_vec();

                // Synchronous transcription
                match backend.transcribe_segment_sync(&chunk) {
                    Ok(segment) => {
                        if !segment.text.is_empty() {
                            if let Err(e) = result_tx.send(TranscriptionResult::Segment(segment.text)) {
                                eprintln!("[WORKER] Failed to send transcription result: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("[WORKER] Transcription error: {}", e);
                        if let Err(e2) = result_tx.send(TranscriptionResult::Error(format!("{}", e))) {
                            eprintln!("[WORKER] Failed to send error: {}", e2);
                            break;
                        }
                    }
                }

                // Advance flush window
                flush_offset = chunk_end - overlap_samples;
                next_flush_at = flush_offset + min_samples;

                // Trim old audio from buffer
                if flush_offset > overlap_samples * 2 {
                    let trim_amount = flush_offset - overlap_samples;
                    recent_audio.drain(..trim_amount);
                    flush_offset -= trim_amount;
                    next_flush_at -= trim_amount;
                }
            }

            // Use recv_timeout for interruptible sleep — reduces shutdown latency from 200ms to 50ms
            match wake_rx.recv_timeout(POLL_INTERVAL) {
                Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
        }
    })
}
