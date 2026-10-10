use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

/// The random number generator used by every scrambler.
///
/// ChaCha8 has a fixed, documented output for a given seed, so a seed reproduces the same
/// stimulus on every platform and across releases.
pub type Rng = ChaCha8Rng;

pub fn rng(seed: u64) -> Rng {
    ChaCha8Rng::seed_from_u64(seed)
}

/// Returns `seed`, or a fresh random seed when none is given.
pub fn resolve(seed: Option<u64>) -> u64 {
    seed.unwrap_or_else(rand::random)
}

/// Derives an independent seed for a sub-stream (a face region, a batch file, a video frame).
pub fn derive(seed: u64, stream: u64) -> u64 {
    // SplitMix64 finaliser.
    let mut z = seed ^ stream.wrapping_add(1).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}
