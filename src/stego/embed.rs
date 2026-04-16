use super::chaos::chaotic_sequence;

pub fn fibonacci_table(n: usize) -> Vec<usize> {
    let mut fib = vec![1usize, 1];
    while fib.len() < n {
        let next = fib[fib.len() - 1] + fib[fib.len() - 2];
        fib.push(next);
    }
    fib
}

pub fn get_channel(index: usize, pattern: &str) -> usize {
    match pattern {
        "sequential" => index % 3,
        "reverse" => 2 - (index % 3),
        "random" => {
            use sha2::{Sha256, Digest};
            let mut h = Sha256::new();
            h.update(index.to_string().as_bytes());
            h.finalize()[0] as usize % 3
        }
        _ => {
            let fib = fibonacci_table(50);
            fib[index % 50] % 3
        }
    }
}

pub fn get_bit_plane(seed: &[u8], index: usize, ratio: f64, chaos_type: &str) -> u8 {
    let mut extended = seed.to_vec();
    extended.extend_from_slice(&(index as u32).to_be_bytes());
    let val = chaotic_sequence(&extended, 1, chaos_type)[0];
    if val < ratio { 0 } else { 1 }
}

pub fn embed_bit(pixel_value: u8, bit: u8, plane: u8) -> u8 {
    (pixel_value & !(1 << plane)) | (bit << plane)
}

pub fn extract_bit(pixel_value: u8, plane: u8) -> u8 {
    (pixel_value >> plane) & 1
}
