use super::chaos::chaotic_shuffle;
use super::positions::PositionGenerator;
use super::embed::{get_channel, get_bit_plane, embed_bit};
use crate::crypto::aes::{encrypt_aes, xor_cipher, derive_keys};
use serde::{Serialize, Deserialize};
use rand::RngCore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StegoConfig {
    pub aes_iterations: u32,
    pub xor_iterations: u32,
    pub chaos_iterations: u32,
    pub chaos_type: String,
    pub position_method: String,
    pub channel_pattern: String,
    pub bit_plane_ratio: f64,
    pub use_xor: bool,
    pub use_shuffle: bool,
}

impl Default for StegoConfig {
    fn default() -> Self {
        Self {
            aes_iterations: 100_000,
            xor_iterations: 50_000,
            chaos_iterations: 25_000,
            chaos_type: "logistic".into(),
            position_method: "henon".into(),
            channel_pattern: "fibonacci".into(),
            bit_plane_ratio: 0.85,
            use_xor: true,
            use_shuffle: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StegoMetadata {
    pub salt: String,
    pub nonce: String,
    pub tag: String,
    pub total_bits: usize,
    pub image_dimensions: (usize, usize),
    pub config: StegoConfig,
}

fn bytes_to_bits(data: &[u8]) -> Vec<u8> {
    data.iter()
        .flat_map(|byte| (0..8).map(move |i| (byte >> (7 - i)) & 1))
        .collect()
}

pub fn generate_noise_image(seed: u64) -> (Vec<u8>, usize, usize) {
    let (w, h) = (512usize, 512usize);
    let mut pixels = vec![0u8; w * h * 3];
    let mut rng_state = seed;

    for y in 0..h {
        for x in 0..w {
            let idx = (y * w + x) * 3;
            let base_r = (128.0 + 40.0 * (x as f64 / 50.0).sin()) as i16;
            let base_g = (128.0 + 40.0 * (y as f64 / 50.0).sin()) as i16;
            let base_b = (128.0 + 40.0 * ((x + y) as f64 / 70.0).sin()) as i16;

            rng_state ^= rng_state << 13;
            rng_state ^= rng_state >> 7;
            rng_state ^= rng_state << 17;
            let noise = ((rng_state % 41) as i16) - 20;

            pixels[idx] = (base_r + noise).clamp(0, 255) as u8;
            pixels[idx + 1] = (base_g + noise).clamp(0, 255) as u8;
            pixels[idx + 2] = (base_b + noise).clamp(0, 255) as u8;
        }
    }
    (pixels, w, h)
}

pub fn encode(
    data: &[u8],
    password: &str,
    config: &StegoConfig,
) -> Result<(Vec<u8>, StegoMetadata), String> {
    let (aes_key, xor_key, chaos_seed) = derive_keys(password, config.aes_iterations);

    let (ciphertext, salt, nonce, tag) = encrypt_aes(data, &aes_key)
        .map_err(|e| format!("aes encrypt failed: {}", e))?;

    let mut processed = ciphertext;
    if config.use_xor {
        processed = xor_cipher(&processed, &xor_key);
    }
    if config.use_shuffle {
        processed = chaotic_shuffle(&processed, &chaos_seed, false, &config.chaos_type);
    }

    let bits = bytes_to_bits(&processed);
    let total_bits = bits.len();

    let mut rng = rand::thread_rng();
    let seed = rng.next_u64();
    let (mut pixels, w, h) = generate_noise_image(seed);

    let max_capacity = w * h;
    if total_bits > max_capacity {
        return Err(format!("data too large: {} bits, capacity: {}", total_bits, max_capacity));
    }

    let pos_gen = PositionGenerator::new(w, h, &chaos_seed);
    let positions = pos_gen.generate(total_bits, &config.position_method);

    for (i, &(px, py)) in positions.iter().enumerate() {
        if i >= total_bits {
            break;
        }
        let channel = get_channel(i, &config.channel_pattern);
        let plane = get_bit_plane(&chaos_seed, i, config.bit_plane_ratio, &config.chaos_type);
        let idx = (py * w + px) * 3 + channel;
        if idx < pixels.len() {
            pixels[idx] = embed_bit(pixels[idx], bits[i], plane);
        }
    }

    let png_bytes = encode_png(&pixels, w, h)?;

    let metadata = StegoMetadata {
        salt: hex::encode(salt),
        nonce: hex::encode(nonce),
        tag: hex::encode(tag),
        total_bits,
        image_dimensions: (w, h),
        config: config.clone(),
    };

    Ok((png_bytes, metadata))
}

fn encode_png(pixels: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    use image::{ImageBuffer, Rgb};
    let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
        ImageBuffer::from_raw(width as u32, height as u32, pixels.to_vec())
            .ok_or("failed to create image buffer")?;

    let mut buf = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buf);
    img.write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|e| format!("png encode failed: {}", e))?;
    Ok(buf)
}
