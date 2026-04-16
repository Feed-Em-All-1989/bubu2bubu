use super::chaos::henon;
use sha2::{Sha256, Digest};
use std::collections::HashSet;

pub struct PositionGenerator {
    width: usize,
    height: usize,
    seed: Vec<u8>,
}

impl PositionGenerator {
    pub fn new(width: usize, height: usize, seed: &[u8]) -> Self {
        Self {
            width,
            height,
            seed: seed.to_vec(),
        }
    }

    pub fn generate(&self, count: usize, method: &str) -> Vec<(usize, usize)> {
        match method {
            "prime" => self.prime(count),
            "spiral" => self.spiral(count),
            "random" => self.random(count),
            _ => self.henon_positions(count),
        }
    }

    fn henon_positions(&self, count: usize) -> Vec<(usize, usize)> {
        let seed_int = u32::from_be_bytes([
            self.seed.first().copied().unwrap_or(0),
            self.seed.get(1).copied().unwrap_or(0),
            self.seed.get(2).copied().unwrap_or(0),
            self.seed.get(3).copied().unwrap_or(0),
        ]);

        let mut x = (seed_int % 1000) as f64 / 1000.0 - 0.5;
        let mut y = ((seed_int >> 16) % 1000) as f64 / 1000.0 - 0.5;

        for _ in 0..500 {
            let (nx, ny) = henon(x, y);
            x = nx;
            y = ny;
        }

        let mut positions = Vec::new();
        let mut seen = HashSet::new();

        for _ in 0..count * 100 {
            let (nx, ny) = henon(x, y);
            x = nx;
            y = ny;

            let px = ((x.abs() * 1000.0) as usize) % self.width;
            let py = ((y.abs() * 1000.0) as usize) % self.height;

            if seen.insert((px, py)) {
                positions.push((px, py));
                if positions.len() >= count {
                    break;
                }
            }
        }
        positions
    }

    fn prime(&self, count: usize) -> Vec<(usize, usize)> {
        let limit = self.width * self.height;
        let primes = sieve_primes(limit);
        let seed_int = u32::from_be_bytes([
            self.seed.first().copied().unwrap_or(0),
            self.seed.get(1).copied().unwrap_or(0),
            self.seed.get(2).copied().unwrap_or(0),
            self.seed.get(3).copied().unwrap_or(0),
        ]);
        let offset = (seed_int % 1000) as usize;

        let mut positions = Vec::new();
        let mut seen = HashSet::new();

        for p in primes {
            let pos = (p + offset) % limit;
            if seen.insert(pos) {
                positions.push((pos % self.width, pos / self.width));
                if positions.len() >= count {
                    break;
                }
            }
        }
        positions
    }

    fn spiral(&self, count: usize) -> Vec<(usize, usize)> {
        let (cx, cy) = (self.width / 2, self.height / 2);
        let mut positions = vec![(cx, cy)];
        let (mut x, mut y) = (cx as isize, cy as isize);
        let (mut dx, mut dy): (isize, isize) = (1, 0);
        let mut steps = 1usize;
        let mut steps_in_dir = 0usize;
        let mut dir_changes = 0usize;

        while positions.len() < count {
            x += dx;
            y += dy;
            steps_in_dir += 1;

            if x >= 0 && x < self.width as isize && y >= 0 && y < self.height as isize {
                positions.push((x as usize, y as usize));
            }

            if steps_in_dir >= steps {
                steps_in_dir = 0;
                dir_changes += 1;
                let tmp = dx;
                dx = -dy;
                dy = tmp;
                if dir_changes % 2 == 0 {
                    steps += 1;
                }
            }

            if positions.len() > self.width * self.height {
                break;
            }
        }
        positions.truncate(count);
        positions
    }

    fn random(&self, count: usize) -> Vec<(usize, usize)> {
        let mut positions = HashSet::new();
        let mut i = 0u32;

        while positions.len() < count {
            let mut hasher = Sha256::new();
            hasher.update(&self.seed);
            hasher.update(&i.to_be_bytes());
            let hash = hasher.finalize();

            let px = u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) as usize % self.width;
            let py = u32::from_be_bytes([hash[4], hash[5], hash[6], hash[7]]) as usize % self.height;
            positions.insert((px, py));

            i += 1;
            if i > count as u32 * 100 {
                break;
            }
        }
        positions.into_iter().take(count).collect()
    }
}

fn sieve_primes(limit: usize) -> Vec<usize> {
    if limit < 2 {
        return vec![];
    }
    let mut is_prime = vec![true; limit + 1];
    is_prime[0] = false;
    is_prime[1] = false;
    let mut i = 2;
    while i * i <= limit {
        if is_prime[i] {
            let mut j = i * i;
            while j <= limit {
                is_prime[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    (0..=limit).filter(|&i| is_prime[i]).collect()
}
