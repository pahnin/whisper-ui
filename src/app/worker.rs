use std::panic;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crossbeam_channel::TrySendError;

use crate::app::TranscriptionResult;
use crate::inference::backend::WhisperBackend;



/// Session statistics for monitoring worker health.
pub struct SessionStats {
    pub chunks_transcribed: usize,
    pub chunks_skipped_silent: usize,
    pub chunks_failed: usize,
    pub session_start: std::time::Instant,
}

impl SessionStats {
    pub fn duration_secs(&self) -> u64 {
        self.session_start.elapsed().as_secs()
    }

    pub fn format_summary(&self) -> String {
        let secs = self.duration_secs();
        let mins = secs / 60;
        let secs_rem = secs % 60;
        format!(
            "Transcribed {} chunks | {} silent | {} errors | {}m{}s",
            self.chunks_transcribed,
            self.chunks_skipped_silent,
            self.chunks_failed,
            mins,
            secs_rem,
        )
    }

    pub fn is_healthy(&self) -> bool {
        self.chunks_failed < 3
    }
}

/// Minimum audio to accumulate before transcribing (4 seconds).
const MIN_AUDIO_SECS: u64 = 4;

/// Overlap between consecutive chunks (2 seconds).
const OVERLAP_SECS: u64 = 2;

/// Poll interval for checking ring buffer (50ms, reduced from 200ms).
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Maximum audio buffer size in seconds (20 seconds of audio).
const MAX_AUDIO_BUFFER_SECS: u64 = 20;

/// Maximum consecutive errors before stopping the worker.
const MAX_ERRORS: usize = 3;

/// RMS threshold below which audio is considered silence.
const SILENCE_THRESHOLD: f32 = 0.002;

/// Runs the transcription pipeline on a dedicated thread.
/// Accumulates audio until MIN_AUDIO_SECS has elapsed, then transcribes
/// with OVERLAP_SECS of overlap between consecutive chunks.
pub fn run_worker(
    ring_buffer: Arc<crossbeam_queue::ArrayQueue<f32>>,
    mut backend: WhisperBackend,
    result_tx: crossbeam_channel::Sender<TranscriptionResult>,
    running: Arc<AtomicBool>,
    device_sample_rate: u32,
) -> std::thread::JoinHandle<()> {
    thread::spawn(move || {
        let min_samples: usize = (device_sample_rate as usize) * (MIN_AUDIO_SECS as usize);
        let overlap_samples: usize = (device_sample_rate as usize) * (OVERLAP_SECS as usize);
        let mut consecutive_errors: usize = 0;

        let mut recent_audio: Vec<f32> = Vec::new();

        let mut stats = SessionStats {
            chunks_transcribed: 0,
            chunks_skipped_silent: 0,
            chunks_failed: 0,
            session_start: Instant::now(),
        };

        let (_wake_tx, wake_rx): (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>) = std::sync::mpsc::channel();

        loop {
            let running = running.load(Ordering::SeqCst);
            if !running {
                break;
            }

            // Drain available samples from ring buffer
            let mut drained: Vec<f32> = Vec::new();
            while let Some(value) = ring_buffer.pop() {
                drained.push(value);
            }

            if !drained.is_empty() {
                recent_audio.extend(drained);

                let max_samples = (device_sample_rate as u64 * MAX_AUDIO_BUFFER_SECS) as usize;
                if recent_audio.len() > max_samples {
                    recent_audio.drain(..recent_audio.len() - max_samples);
                }
            }

            // Check if we've accumulated enough audio for a new chunk
            if recent_audio.len() >= min_samples {
                // Grab the last (min_samples + overlap_samples) samples
                let chunk_size = min_samples.saturating_add(overlap_samples);
                let start = recent_audio.len().saturating_sub(chunk_size);
                let chunk = recent_audio[start..recent_audio.len()].to_vec();

                let rms = if chunk.is_empty() {
                    0.0
                } else {
                    (chunk.iter().map(|x| x * x).sum::<f32>() / chunk.len() as f32).sqrt()
                };

                if rms < SILENCE_THRESHOLD {
                    stats.chunks_skipped_silent += 1;
                    continue;
                }

                // Synchronous transcription with panic protection
                let result = panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    backend.transcribe_segment_sync(&chunk)
                }));

                let segment = match result {
                    Ok(Ok(s)) => s,
                    Ok(Err(e)) => {
                        eprintln!("[WORKER] Transcription error: {}", e);
                        consecutive_errors += 1;
                        stats.chunks_failed += 1;
                        if consecutive_errors >= MAX_ERRORS {
                            eprintln!("[WORKER] Max errors reached, stopping worker");
                            let _ = result_tx.try_send(TranscriptionResult::Error(format!("Max transcription errors reached")));
                            break;
                        }
                        let delay_secs = 2u64.pow(consecutive_errors as u32);
                        eprintln!("[WORKER] Retrying in {}s (error {}/{})", delay_secs, consecutive_errors, MAX_ERRORS);
                        std::thread::sleep(Duration::from_secs(delay_secs));
                        continue;
                    }
                    Err(_) => {
                        eprintln!("[WORKER] Transcription panicked");
                        if let Err(e) = backend.reset_state() {
                            eprintln!("[WORKER] Failed to reset state after panic: {}", e);
                        }
                        consecutive_errors += 1;
                        stats.chunks_failed += 1;
                        if consecutive_errors >= MAX_ERRORS {
                            eprintln!("[WORKER] Max errors reached, stopping worker");
                            let _ = result_tx.try_send(TranscriptionResult::Error(format!("Max transcription errors reached")));
                            break;
                        }
                        let delay_secs = 2u64.pow(consecutive_errors as u32);
                        eprintln!("[WORKER] Retrying in {}s (error {}/{})", delay_secs, consecutive_errors, MAX_ERRORS);
                        std::thread::sleep(Duration::from_secs(delay_secs));
                        continue;
                    }
                };

                if !segment.text.is_empty() {
                    if let Err(TrySendError::Full(_)) = result_tx.try_send(TranscriptionResult::Segment(segment.text)) {
                        continue;
                    }
                }
                stats.chunks_transcribed += 1;
                consecutive_errors = 0;

                let elapsed = stats.session_start.elapsed();
                if elapsed.as_secs() % 60 == 0 && elapsed.as_secs() > 0 {
                    eprintln!("[WORKER STATS] {}", stats.format_summary());
                }

                // Keep only the overlap portion for the next chunk
                let keep = overlap_samples.min(recent_audio.len());
                let discarded = recent_audio.len() - keep;
                recent_audio.drain(..discarded);
            }

            // Use recv_timeout for interruptible sleep — reduces shutdown latency from 200ms to 50ms
            match wake_rx.recv_timeout(POLL_INTERVAL) {
                Ok(_) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
                Err(_) => break,
            }
        }
    })
}
