// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 itsakeyfut
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Audio output backend using cpal
//!
//! This module provides real-time audio playback using the cpal library,
//! handling sample buffering and output stream management.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Audio output backend
///
/// Manages audio stream and sample buffering for real-time playback.
/// Uses cpal for cross-platform audio output.
///
/// # Example
///
/// ```no_run
/// use psrx::core::audio::AudioBackend;
///
/// let mut audio = AudioBackend::new().unwrap();
/// let samples = vec![(100, 100); 100]; // 100 stereo samples
/// audio.queue_samples(&samples);
/// ```
pub struct AudioBackend {
    /// cpal audio output stream
    #[allow(dead_code)]
    stream: cpal::Stream,
    /// Queue of stereo samples (left, right) to be played
    sample_queue: Arc<Mutex<VecDeque<(i16, i16)>>>,
    /// Sample rate of the output device
    sample_rate: u32,
}

impl AudioBackend {
    /// Create a new audio backend
    ///
    /// Initializes the default audio output device and starts the playback stream.
    ///
    /// # Returns
    ///
    /// - `Ok(AudioBackend)` if initialization succeeds
    /// - `Err(Box<dyn std::error::Error>)` if device/stream creation fails
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - No audio output device is available
    /// - Failed to get device configuration
    /// - Failed to build output stream
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No audio output device available")?;

        let config = device.default_output_config()?;
        let sample_rate = config.sample_rate().0;
        let channels = config.channels();

        // Validate stereo output
        if channels != 2 {
            return Err(format!(
                "Audio backend requires stereo output (2 channels), but device '{}' default config has {} channels",
                device.name().unwrap_or_else(|_| "Unknown".to_string()),
                channels
            )
            .into());
        }

        // Warn if sample rate is not 44.1 kHz
        if sample_rate != 44_100 {
            log::warn!(
                "Audio: Device sample rate is {} Hz (expected 44100 Hz). Audio timing may drift.",
                sample_rate
            );
            log::warn!("Audio: Consider implementing resampling for accurate playback.");
        }

        log::info!(
            "Audio: Using device '{}' at {} Hz, {} channels",
            device.name().unwrap_or_else(|_| "Unknown".to_string()),
            sample_rate,
            channels
        );

        let sample_queue = Arc::new(Mutex::new(VecDeque::new()));
        let queue_clone = sample_queue.clone();

        // Build output stream with f32 samples
        let stream = device.build_output_stream(
            &config.into(),
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let mut queue = queue_clone.lock().unwrap();

                // Fill output buffer with samples from queue
                for frame in data.chunks_mut(2) {
                    if let Some((left, right)) = queue.pop_front() {
                        // Convert i16 to f32 in range [-1.0, 1.0]
                        frame[0] = left as f32 / 32768.0;
                        frame[1] = right as f32 / 32768.0;
                    } else {
                        // Output silence if queue is empty
                        frame[0] = 0.0;
                        frame[1] = 0.0;
                    }
                }
            },
            |err| eprintln!("Audio stream error: {}", err),
            None,
        )?;

        // Start playback
        stream.play()?;

        Ok(Self {
            stream,
            sample_queue,
            sample_rate,
        })
    }

    /// Queue audio samples for playback
    ///
    /// Adds stereo samples to the playback queue. Samples are consumed by the
    /// audio output stream in real-time.
    ///
    /// # Arguments
    ///
    /// * `samples` - Slice of stereo samples (left, right) in i16 format
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::audio::AudioBackend;
    ///
    /// let mut audio = AudioBackend::new().unwrap();
    ///
    /// // Queue 100 stereo samples
    /// let samples = vec![(100, 100); 100];
    /// audio.queue_samples(&samples);
    /// ```
    pub fn queue_samples(&mut self, samples: &[(i16, i16)]) {
        let mut queue = self.sample_queue.lock().unwrap();
        queue.extend(samples.iter());
    }

    /// Get current buffer level
    ///
    /// Returns the number of stereo samples currently queued for playback.
    /// This can be used to detect buffer underruns or monitor playback latency.
    ///
    /// # Returns
    ///
    /// Number of queued stereo samples
    ///
    /// # Example
    ///
    /// ```no_run
    /// use psrx::core::audio::AudioBackend;
    ///
    /// let audio = AudioBackend::new().unwrap();
    /// let level = audio.buffer_level();
    ///
    /// if level < 512 {
    ///     println!("Warning: Audio buffer running low");
    /// }
    /// ```
    pub fn buffer_level(&self) -> usize {
        self.sample_queue.lock().unwrap().len()
    }

    /// Get the sample rate of the audio output device
    ///
    /// # Returns
    ///
    /// Sample rate in Hz (typically 44100 or 48000)
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_backend_creation() {
        // This test may fail on systems without audio devices
        match AudioBackend::new() {
            Ok(audio) => {
                assert!(audio.sample_rate() > 0);
                assert_eq!(audio.buffer_level(), 0);
            }
            Err(e) => {
                println!("Audio backend creation failed (may be expected): {}", e);
                // Not failing the test as CI systems may not have audio devices
            }
        }
    }

    #[test]
    fn test_queue_samples() {
        match AudioBackend::new() {
            Ok(mut audio) => {
                let samples = vec![(100, 200); 10];
                audio.queue_samples(&samples);
                assert_eq!(audio.buffer_level(), 10);
            }
            Err(_) => {
                // Skip test if no audio device
            }
        }
    }

    #[test]
    fn test_buffer_level() {
        match AudioBackend::new() {
            Ok(mut audio) => {
                assert_eq!(audio.buffer_level(), 0);

                audio.queue_samples(&[(0, 0); 100]);
                assert_eq!(audio.buffer_level(), 100);

                audio.queue_samples(&[(0, 0); 50]);
                assert_eq!(audio.buffer_level(), 150);
            }
            Err(_) => {
                // Skip test if no audio device
            }
        }
    }

    #[test]
    #[ignore] // Run manually with: cargo test test_audio_playback -- --ignored --nocapture
    fn test_audio_playback() {
        use std::f32::consts::PI;

        let mut audio = AudioBackend::new().unwrap();

        // Generate a 440 Hz tone (A4 note) for 100ms
        let sample_rate = 44100;
        let duration = 0.1; // 100ms
        let frequency = 440.0;

        let sample_count = (sample_rate as f32 * duration) as usize;
        let samples: Vec<(i16, i16)> = (0..sample_count)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                let sample = (t * frequency * 2.0 * PI).sin() * 10000.0;
                (sample as i16, sample as i16)
            })
            .collect();

        audio.queue_samples(&samples);

        println!("Playing 440 Hz tone for 100ms...");
        std::thread::sleep(std::time::Duration::from_millis(150));
        println!("Playback complete");
    }
}
