use anyhow::{anyhow, Result};
use hound::{WavReader, SampleFormat};
use std::io::Cursor;

pub fn decode_audio(audio_data: &[u8]) -> Result<Vec<f32>> {
    let cursor = Cursor::new(audio_data);
    let mut reader = WavReader::new(cursor)
        .map_err(|e| anyhow!("Failed to read WAV file: {}", e))?;

    let spec = reader.spec();
    
    match spec.sample_format {
        SampleFormat::Float => {
            let samples: Result<Vec<f32>, _> = reader.samples::<f32>().collect();
            samples.map_err(|e| anyhow!("Failed to read float samples: {}", e))
        }
        SampleFormat::Int => {
            let samples: Result<Vec<i32>, _> = reader.samples::<i32>().collect();
            let samples = samples.map_err(|e| anyhow!("Failed to read int samples: {}", e))?;
            
            let max_value = (1 << (spec.bits_per_sample - 1)) as f32;
            Ok(samples.into_iter().map(|s| s as f32 / max_value).collect())
        }
    }
}

pub fn normalize_audio(samples: &[f32]) -> Vec<f32> {
    if samples.is_empty() {
        return Vec::new();
    }

    let max_amplitude = samples.iter()
        .map(|&s| s.abs())
        .fold(0.0f32, f32::max);

    if max_amplitude == 0.0 {
        return samples.to_vec();
    }

    samples.iter().map(|&s| s / max_amplitude).collect()
}

pub fn downsample(samples: &[f32], original_rate: u32, target_rate: u32) -> Vec<f32> {
    if original_rate <= target_rate {
        return samples.to_vec();
    }

    let ratio = original_rate as f32 / target_rate as f32;
    let new_len = (samples.len() as f32 / ratio) as usize;
    
    (0..new_len)
        .map(|i| {
            let original_index = (i as f32 * ratio) as usize;
            samples.get(original_index).copied().unwrap_or(0.0)
        })
        .collect()
}