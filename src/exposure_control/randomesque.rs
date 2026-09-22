//! Algoritma top-*n* random (`'sample'`, a.k.a. randomesque) — pilih uniform
//! random di antara `n` kandidat berperingkat teratas; `exposure` diindeks
//! per **posisi penyajian** (bukan ID item — beda dari
//! [`super::sympson_hetter`], lih. catatan modul `super`).
//!
//! Ground truth: `findNextCATItem.R`, cabang `design@exposure_type ==
//! "sample"` (diperiksa langsung dari source terpasang mirtCAT 1.14 via
//! `getFromNamespace("findNextCATItem", "mirtCAT")`):
//! ```r
//! exposure <- design@exposure[lastitem + 1L]
//! if (exposure == 1L) {
//!     item <- index[which.max(crit)][1L]
//! } else {
//!     rnk <- length(crit) - rank(crit, ties.method = "random") + 1L
//!     pick <- which(rnk %in% 1L:exposure)
//!     item <- index[sample(pick, 1L)]
//! }
//! ```
//! `exposure == 1` → deterministik, sama seperti tanpa exposure control
//! sama sekali (tie pertama menang, lih. [`crate::item_selection::argmax`]).
//! `exposure > 1` → peringkat kandidat DESCENDING dengan **tie-break
//! random** (`rank(..., ties.method="random")`, direplikasi di sini lewat
//! shuffle acak sebelum *stable sort* descending — tiap kandidat tied
//! mendapat urutan seragam-acak, persis efek `ties.method="random"` R),
//! ambil `exposure` kandidat teratas (dipangkas ke jumlah kandidat tersisa
//! bila lebih sedikit), lalu pilih SATU secara uniform random dari
//! himpunan itu — bukan berbobot peringkat.

use rand::seq::SliceRandom;
use rand::Rng;

use crate::error::MirtCatError;
use crate::item_selection::{argmax, ItemIndex};

/// Hasil satu pemanggilan [`select`].
#[derive(Debug)]
pub(crate) struct Outcome {
    pub chosen: ItemIndex,
    /// Himpunan kandidat top-*n* yang dipertimbangkan sebelum pemilihan
    /// uniform random (posisi di `Vec` TIDAK bermakna peringkat, hanya
    /// keanggotaan). Untuk `exposure_for_position <= 1`, berisi persis satu
    /// elemen (`chosen` itu sendiri, deterministik).
    pub considered: Vec<ItemIndex>,
}

/// `exposure_for_position` = `exposure[posisi penyajian saat ini]`
/// (0-based, setara `exposure[lastitem + 1]` 1-based R — lih. catatan
/// modul `super`). Error bila `candidate_scores` kosong.
pub(crate) fn select(
    exposure_for_position: u32,
    candidate_scores: &[(ItemIndex, f64)],
    rng: &mut impl Rng,
) -> Result<Outcome, MirtCatError> {
    if candidate_scores.is_empty() {
        return Err(MirtCatError::InvalidParameter(
            "randomesque::select: candidate_scores kosong -- kehabisan item untuk disajikan".into(),
        ));
    }

    if exposure_for_position <= 1 {
        let chosen = argmax(candidate_scores)?;
        return Ok(Outcome {
            chosen,
            considered: vec![chosen],
        });
    }

    let mut shuffled: Vec<(ItemIndex, f64)> = candidate_scores.to_vec();
    shuffled.shuffle(rng);
    shuffled.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let n = (exposure_for_position as usize).min(shuffled.len());
    let considered: Vec<ItemIndex> = shuffled[..n].iter().map(|&(idx, _)| idx).collect();
    let chosen = *considered
        .choose(rng)
        .expect("considered tidak boleh kosong -- n >= 1 karena candidate_scores non-kosong");

    Ok(Outcome { chosen, considered })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::collections::HashSet;

    #[test]
    fn errors_on_empty_candidates() {
        let mut rng = StdRng::seed_from_u64(0);
        let err = select(3, &[], &mut rng).unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn exposure_one_is_deterministic_best_candidate() {
        let scores = [(0usize, 1.0), (1usize, 5.0), (2usize, 3.0)];
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..50 {
            let outcome = select(1, &scores, &mut rng).unwrap();
            assert_eq!(outcome.chosen, 1);
            assert_eq!(outcome.considered, vec![1]);
        }
    }

    #[test]
    fn exposure_zero_treated_same_as_one_deterministic() {
        // Tidak seharusnya terjadi lewat ExposureMode::from_values (validasi
        // >= 1), tapi fungsi ini sendiri harus tetap aman (bukan panic) bila
        // dipanggil langsung dengan 0.
        let scores = [(0usize, 1.0), (1usize, 5.0)];
        let mut rng = StdRng::seed_from_u64(2);
        let outcome = select(0, &scores, &mut rng).unwrap();
        assert_eq!(outcome.chosen, 1);
    }

    #[test]
    fn exposure_three_only_picks_from_top_three_candidates() {
        let scores = [
            (0usize, 1.0),
            (1usize, 5.0),
            (2usize, 4.0),
            (3usize, 3.0),
            (4usize, 0.5),
        ];
        let mut rng = StdRng::seed_from_u64(3);
        let top3: HashSet<ItemIndex> = [1, 2, 3].into_iter().collect();
        for _ in 0..200 {
            let outcome = select(3, &scores, &mut rng).unwrap();
            assert!(top3.contains(&outcome.chosen));
            assert_eq!(outcome.considered.len(), 3);
            let considered_set: HashSet<ItemIndex> = outcome.considered.iter().copied().collect();
            assert_eq!(considered_set, top3);
        }
    }

    #[test]
    fn exposure_three_distribution_covers_all_three_top_candidates() {
        let scores = [
            (0usize, 1.0),
            (1usize, 5.0),
            (2usize, 4.0),
            (3usize, 3.0),
            (4usize, 0.5),
        ];
        let mut rng = StdRng::seed_from_u64(4);
        let mut seen: HashSet<ItemIndex> = HashSet::new();
        for _ in 0..300 {
            let outcome = select(3, &scores, &mut rng).unwrap();
            seen.insert(outcome.chosen);
        }
        assert_eq!(seen, [1usize, 2, 3].into_iter().collect());
    }

    #[test]
    fn exposure_larger_than_candidate_pool_covers_entire_pool() {
        let scores = [(0usize, 1.0), (1usize, 5.0), (2usize, 3.0)];
        let mut rng = StdRng::seed_from_u64(5);
        let mut seen: HashSet<ItemIndex> = HashSet::new();
        for _ in 0..200 {
            let outcome = select(10, &scores, &mut rng).unwrap();
            assert_eq!(outcome.considered.len(), 3);
            seen.insert(outcome.chosen);
        }
        assert_eq!(seen, [0usize, 1, 2].into_iter().collect());
    }

    #[test]
    fn tied_scores_break_randomly_not_always_the_same_winner() {
        // Semua skor sama -> exposure=2 harus mencakup 2 dari 4 kandidat,
        // dan lintas banyak panggilan seluruh kandidat pernah masuk
        // himpunan `considered` (tie-break acak, bukan urutan index tetap).
        let scores = [(0usize, 1.0), (1usize, 1.0), (2usize, 1.0), (3usize, 1.0)];
        let mut rng = StdRng::seed_from_u64(6);
        let mut ever_considered: HashSet<ItemIndex> = HashSet::new();
        for _ in 0..200 {
            let outcome = select(2, &scores, &mut rng).unwrap();
            assert_eq!(outcome.considered.len(), 2);
            ever_considered.extend(outcome.considered.iter().copied());
        }
        assert_eq!(ever_considered, [0usize, 1, 2, 3].into_iter().collect());
    }
}
