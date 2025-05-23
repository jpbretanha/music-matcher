use anyhow::{anyhow, Result};
use ndarray::Array2;
use rustfft::{FftPlanner, num_complex::Complex};

const SAMPLE_RATE: u32 = 11025;
const WINDOW_SIZE: usize = 1024;
const HOP_SIZE: usize = 512;
const FREQ_BINS: usize = 512;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AudioFingerprint {
    pub hashes: Vec<u32>,
    pub duration: f64,
}

pub fn generate_fingerprint(samples: &[f32]) -> Result<AudioFingerprint> {
    if samples.is_empty() {
        return Err(anyhow!("Empty audio samples"));
    }

    let normalized = crate::audio::normalize_audio(samples);
    let downsampled = crate::audio::downsample(&normalized, 44100, SAMPLE_RATE);
    
    let spectrogram = compute_spectrogram(&downsampled)?;
    let peaks = find_spectral_peaks(&spectrogram);
    let hashes = generate_hashes(&peaks);
    
    let duration = samples.len() as f64 / 44100.0;
    
    Ok(AudioFingerprint { hashes, duration })
}

fn compute_spectrogram(samples: &[f32]) -> Result<Array2<f64>> {
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(WINDOW_SIZE);
    
    let num_frames = (samples.len().saturating_sub(WINDOW_SIZE)) / HOP_SIZE + 1;
    let mut spectrogram = Array2::zeros((FREQ_BINS, num_frames));
    
    for (frame_idx, start) in (0..samples.len().saturating_sub(WINDOW_SIZE))
        .step_by(HOP_SIZE)
        .enumerate()
    {
        if frame_idx >= num_frames {
            break;
        }
        
        let mut buffer: Vec<Complex<f64>> = samples[start..start + WINDOW_SIZE]
            .iter()
            .map(|&x| Complex::new(x as f64, 0.0))
            .collect();
        
        apply_hann_window(&mut buffer);
        fft.process(&mut buffer);
        
        for (freq_idx, &complex) in buffer.iter().take(FREQ_BINS).enumerate() {
            let magnitude = complex.norm();
            spectrogram[[freq_idx, frame_idx]] = magnitude;
        }
    }
    
    Ok(spectrogram)
}

fn apply_hann_window(buffer: &mut [Complex<f64>]) {
    let buffer_len = buffer.len();
    for (i, sample) in buffer.iter_mut().enumerate() {
        let window_val = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * i as f64 / (buffer_len - 1) as f64).cos());
        *sample *= window_val;
    }
}

#[derive(Debug, Clone, Copy)]
struct SpectralPeak {
    freq_bin: usize,
    time_frame: usize,
    magnitude: f64,
}

fn find_spectral_peaks(spectrogram: &Array2<f64>) -> Vec<SpectralPeak> {
    let mut peaks = Vec::new();
    let (freq_bins, time_frames) = spectrogram.dim();
    
    for t in 1..time_frames - 1 {
        for f in 1..freq_bins - 1 {
            let current = spectrogram[[f, t]];
            
            if current > 0.1 && 
               current > spectrogram[[f-1, t]] &&
               current > spectrogram[[f+1, t]] &&
               current > spectrogram[[f, t-1]] &&
               current > spectrogram[[f, t+1]] {
                peaks.push(SpectralPeak {
                    freq_bin: f,
                    time_frame: t,
                    magnitude: current,
                });
            }
        }
    }
    
    peaks.sort_by(|a, b| b.magnitude.partial_cmp(&a.magnitude).unwrap());
    peaks.truncate(200);
    
    peaks
}

fn generate_hashes(peaks: &[SpectralPeak]) -> Vec<u32> {
    let mut hashes = Vec::new();
    
    for (i, &peak1) in peaks.iter().enumerate() {
        for &peak2 in peaks.iter().skip(i + 1).take(5) {
            if peak2.time_frame <= peak1.time_frame + 10 {
                let freq1 = peak1.freq_bin as u32;
                let freq2 = peak2.freq_bin as u32;
                let time_diff = (peak2.time_frame - peak1.time_frame) as u32;
                
                let hash = (freq1 << 16) | (freq2 << 8) | time_diff;
                hashes.push(hash);
            }
        }
    }
    
    hashes
}

pub fn calculate_similarity(fingerprint1: &AudioFingerprint, fingerprint2: &AudioFingerprint) -> f64 {
    if fingerprint1.hashes.is_empty() || fingerprint2.hashes.is_empty() {
        return 0.0;
    }
    
    let set1: std::collections::HashSet<_> = fingerprint1.hashes.iter().collect();
    let set2: std::collections::HashSet<_> = fingerprint2.hashes.iter().collect();
    
    let intersection = set1.intersection(&set2).count();
    let union = set1.union(&set2).count();
    
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}