use super::chaos::chaotic_shuffle;
use super::positions::PositionGenerator;
use super::embed::{get_channel, get_bit_plane, extract_bit};
use super::encoder::StegoMetadata;
use crate::crypto::aes::{decrypt_aes, xor_cipher, derive_keys};

fn bits_to_bytes(bits: &[u8]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            chunk.iter().enumerate().fold(0u8, |acc, (i, &bit)| {
                acc | (bit << (7 - i))
            })
        })
        .collect()
}

pub fn decode(
    png_bytes: &[u8],
    password: &str,
    metadata: &StegoMetadata,
) -> Result<Vec<u8>, String> {
    let (aes_key, xor_key, chaos_seed) = derive_keys(password, metadata.config.aes_iterations);

    let pixels = decode_png(png_bytes)?;
    let (w, h) = metadata.image_dimensions;

    let pos_gen = PositionGenerator::new(w, h, &chaos_seed);
    let positions = pos_gen.generate(metadata.total_bits, &metadata.config.position_method);

    let mut bits = Vec::with_capacity(metadata.total_bits);
    for (i, &(px, py)) in positions.iter().enumerate() {
        if i >= metadata.total_bits {
            break;
        }
        let channel = get_channel(i, &metadata.config.channel_pattern);
        let plane = get_bit_plane(&chaos_seed, i, metadata.config.bit_plane_ratio, &metadata.config.chaos_type);
        let idx = (py * w + px) * 3 + channel;
        if idx < pixels.len() {
            bits.push(extract_bit(pixels[idx], plane));
        }
    }

    let mut processed = bits_to_bytes(&bits);

    if metadata.config.use_shuffle {
        processed = chaotic_shuffle(&processed, &chaos_seed, true, &metadata.config.chaos_type);
    }
    if metadata.config.use_xor {
        processed = xor_cipher(&processed, &xor_key);
    }

    let salt = hex::decode(&metadata.salt).map_err(|_| "invalid salt hex")?;
    let nonce = hex::decode(&metadata.nonce).map_err(|_| "invalid nonce hex")?;
    let tag = hex::decode(&metadata.tag).map_err(|_| "invalid tag hex")?;

    decrypt_aes(&processed, &aes_key, &salt, &nonce, &tag)
        .map_err(|e| format!("aes decrypt failed: {}", e))
}

fn decode_png(png_bytes: &[u8]) -> Result<Vec<u8>, String> {
    use image::ImageReader;
    let cursor = std::io::Cursor::new(png_bytes);
    let reader = ImageReader::with_format(cursor, image::ImageFormat::Png);
    let img = reader.decode().map_err(|e| format!("png decode failed: {}", e))?;
    let rgb = img.to_rgb8();
    Ok(rgb.into_raw())
}
