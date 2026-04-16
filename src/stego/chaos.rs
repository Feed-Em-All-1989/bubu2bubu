pub fn logistic(x: f64) -> f64 {
    3.9999 * x * (1.0 - x)
}

pub fn henon(x: f64, y: f64) -> (f64, f64) {
    (1.0 - 1.4 * x * x + y, 0.3 * x)
}

pub fn tent(x: f64) -> f64 {
    if x < 0.5 { 1.9999 * x } else { 1.9999 * (1.0 - x) }
}

pub fn chaotic_sequence(seed: &[u8], length: usize, chaos_type: &str) -> Vec<f64> {
    let seed_int = u32::from_be_bytes([
        seed.first().copied().unwrap_or(0),
        seed.get(1).copied().unwrap_or(0),
        seed.get(2).copied().unwrap_or(0),
        seed.get(3).copied().unwrap_or(0),
    ]);

    let mut x = (seed_int % 1_000_000) as f64 / 1_000_000.0;
    x = if x < 0.1 {
        x + 0.1
    } else if x > 0.9 {
        x - 0.1
    } else {
        x
    };

    for _ in 0..1000 {
        x = apply_chaos(x, chaos_type);
    }

    let mut result = Vec::with_capacity(length);
    for _ in 0..length {
        x = apply_chaos(x, chaos_type);
        result.push(x);
    }
    result
}

fn apply_chaos(x: f64, chaos_type: &str) -> f64 {
    match chaos_type {
        "tent" => tent(x),
        "combined" => tent(logistic(x)),
        _ => logistic(x),
    }
}

pub fn chaotic_shuffle(data: &[u8], seed: &[u8], reverse: bool, chaos_type: &str) -> Vec<u8> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }

    let sequence = chaotic_sequence(seed, n, chaos_type);
    let mut indexed: Vec<(usize, f64)> = sequence.into_iter().enumerate().collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    let permutation: Vec<usize> = indexed.into_iter().map(|(i, _)| i).collect();

    let mut result = vec![0u8; n];
    if reverse {
        for (i, &p) in permutation.iter().enumerate() {
            result[p] = data[i];
        }
    } else {
        for (i, &p) in permutation.iter().enumerate() {
            result[i] = data[p];
        }
    }
    result
}
