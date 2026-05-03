use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

pub fn make_rng(seed: Option<u64>) -> (ChaCha8Rng, u64) {
    let seed = seed.unwrap_or_else(rand::random);
    (ChaCha8Rng::seed_from_u64(seed), seed)
}
