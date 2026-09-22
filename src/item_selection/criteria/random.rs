//! `random` — baseline exposure-neutral: pilih uniform random di antara
//! kandidat yang belum dijawab.

use rand::seq::SliceRandom;
use rand::Rng;

/// Pilih satu index kandidat secara uniform random. Panic bila `candidates`
/// kosong — caller (`select_next_item`) sudah memvalidasi non-kosong lebih
/// dulu.
pub(crate) fn select(candidates: &[usize], rng: &mut impl Rng) -> usize {
    *candidates
        .choose(rng)
        .expect("random::select: candidates tidak boleh kosong")
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn always_picks_from_candidates() {
        let mut rng = StdRng::seed_from_u64(42);
        let candidates = [2, 5, 9, 11];
        for _ in 0..50 {
            let picked = select(&candidates, &mut rng);
            assert!(candidates.contains(&picked));
        }
    }

    #[test]
    fn same_seed_is_reproducible() {
        let candidates = [2, 5, 9, 11];
        let mut rng_a = StdRng::seed_from_u64(7);
        let mut rng_b = StdRng::seed_from_u64(7);
        let seq_a: Vec<usize> = (0..10).map(|_| select(&candidates, &mut rng_a)).collect();
        let seq_b: Vec<usize> = (0..10).map(|_| select(&candidates, &mut rng_b)).collect();
        assert_eq!(seq_a, seq_b);
    }

    #[test]
    fn single_candidate_is_always_returned() {
        let mut rng = StdRng::seed_from_u64(1);
        assert_eq!(select(&[8], &mut rng), 8);
    }
}
