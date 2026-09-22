//! Algoritma Sympson–Hetter (`SH`) — loop terima/tolak per-item; `exposure`
//! diindeks per **ID item** (bukan posisi penyajian — beda dari
//! [`super::randomesque`], lih. catatan modul `super`).
//!
//! Ground truth: `findNextCATItem.R`, cabang `design@exposure_type == "SH"`
//! (diperiksa langsung dari source terpasang mirtCAT 1.14 via
//! `getFromNamespace("findNextCATItem", "mirtCAT")`):
//! ```r
//! while (TRUE) {
//!     item <- index[which.max(crit)][1L]
//!     comp <- runif(1, 0, 1)
//!     if (design@exposure[item] >= comp && person$valid_item[item])
//!         break
//!     if (length(crit) == 1L)
//!         break
//!     person$valid_item[item] <- FALSE
//!     pick <- index != item
//!     index <- index[pick]
//!     crit <- crit[pick]
//! }
//! ```
//! Item yang ditolak ditandai `valid_items[item] = false` — **persisten**
//! untuk sisa sesi peserta ini (pemanggil meneruskan slice yang sama di
//! langkah-langkah berikutnya), bukan cuma satu langkah, karena `person`
//! adalah objek reference class R5 yang dimutasi in-place. Jika kandidat
//! tersisa tepat satu, item itu **dipaksa diterima tanpa dites** dan
//! **tidak** ditandai invalid — forced-accept di R terjadi sebelum baris
//! `person$valid_item[item] <- FALSE`.
//!
//! **Reuse eksplisit** (Prinsip Desain #1 `PLAN.md`): pemilihan kandidat
//! terbaik tiap iterasi memakai [`crate::item_selection::argmax`] (Fase 2,
//! sudah teruji) — tie-break "pertama muncul" yang sama persis dengan
//! `which.max(crit)[1L]` R, bukan diimplementasikan ulang di sini.

use rand::Rng;

use crate::error::MirtCatError;
use crate::item_selection::{argmax, ItemIndex};

/// Hasil satu pemanggilan [`select`].
#[derive(Debug)]
pub(crate) struct Outcome {
    pub chosen: ItemIndex,
    /// `(item, diterima)` tiap kandidat yang dicoba, urut dari yang pertama
    /// dicoba (termasuk yang akhirnya diterima/dipaksa-diterima sebagai
    /// entri terakhir). Kosong bila `collect_debug == false` — tidak ada
    /// alokasi tambahan (prinsip desain #7 `PLAN.md`).
    pub attempts: Vec<(ItemIndex, bool)>,
    /// `true` bila `chosen` diterima lewat fallback "kandidat tersisa
    /// satu", bukan lolos draw `runif` normal.
    pub forced_accept: bool,
}

/// `exposure` diindeks per ID item (panjang `n_items`, sama seperti
/// `valid_items`); `valid_items` dimutasi in-place untuk item yang ditolak
/// (bukan sekadar dibaca — lih. catatan modul di atas soal persistensi
/// antar-langkah). Error bila `candidate_scores` kosong (padanan "Ran out
/// of items to administer" R — SH sendiri tidak pernah bisa exhaust di
/// tengah loop karena forced-accept saat tersisa satu, jadi kondisi ini
/// hanya bisa terjadi sebelum loop dimulai sama sekali).
pub(crate) fn select(
    exposure: &[f64],
    candidate_scores: &[(ItemIndex, f64)],
    valid_items: &mut [bool],
    rng: &mut impl Rng,
    collect_debug: bool,
) -> Result<Outcome, MirtCatError> {
    if candidate_scores.is_empty() {
        return Err(MirtCatError::InvalidParameter(
            "sympson_hetter::select: candidate_scores kosong -- kehabisan item untuk disajikan"
                .into(),
        ));
    }

    let mut pool: Vec<(ItemIndex, f64)> = candidate_scores.to_vec();
    let mut attempts = Vec::new();

    loop {
        let item = argmax(&pool)?;
        let comp: f64 = rng.gen_range(0.0..1.0);
        let accepted = exposure[item] >= comp;

        if accepted {
            if collect_debug {
                attempts.push((item, true));
            }
            return Ok(Outcome {
                chosen: item,
                attempts,
                forced_accept: false,
            });
        }

        if pool.len() == 1 {
            if collect_debug {
                attempts.push((item, true));
            }
            return Ok(Outcome {
                chosen: item,
                attempts,
                forced_accept: true,
            });
        }

        if collect_debug {
            attempts.push((item, false));
        }
        valid_items[item] = false;
        pool.retain(|&(idx, _)| idx != item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    #[test]
    fn errors_on_empty_candidates() {
        let mut valid = vec![true; 3];
        let mut rng = StdRng::seed_from_u64(0);
        let err = select(&[0.5, 0.5, 0.5], &[], &mut valid, &mut rng, false).unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn zero_exposure_item_never_passes_across_many_trials() {
        // exposure[0] = 0.0 -> `exposure[item] >= comp` hanya benar bila
        // comp == 0.0 tepat -- P=0 untuk RNG kontinu, jadi item 0 harus
        // selalu ditolak & jatuh ke kandidat berikut (di sini item 1,
        // exposure=1.0, selalu lolos begitu jadi giliran dicoba).
        let exposure = vec![0.0, 1.0];
        let scores = [(0usize, 10.0), (1usize, 1.0)];
        let mut rng = StdRng::seed_from_u64(1);
        for _ in 0..200 {
            let mut valid = vec![true; 2];
            let outcome = select(&exposure, &scores, &mut valid, &mut rng, false).unwrap();
            assert_eq!(outcome.chosen, 1);
        }
    }

    #[test]
    fn full_exposure_item_always_passes_immediately() {
        let exposure = vec![1.0, 1.0];
        let scores = [(0usize, 10.0), (1usize, 1.0)];
        let mut rng = StdRng::seed_from_u64(2);
        for _ in 0..50 {
            let mut valid = vec![true; 2];
            let outcome = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
            assert_eq!(outcome.chosen, 0);
            assert!(!outcome.forced_accept);
            assert_eq!(outcome.attempts, vec![(0, true)]);
        }
    }

    #[test]
    fn forced_accept_when_only_one_candidate_remains() {
        // Kandidat tunggal, exposure=0.0 -> draw normal selalu gagal, tapi
        // forced-accept harus tetap memilihnya (bukan error).
        let exposure = vec![0.0];
        let scores = [(0usize, 5.0)];
        let mut rng = StdRng::seed_from_u64(3);
        for _ in 0..50 {
            let mut valid = vec![true; 1];
            let outcome = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
            assert_eq!(outcome.chosen, 0);
            assert!(outcome.forced_accept);
            // Forced-accept tidak menandai item invalid (lih. catatan modul).
            assert!(valid[0]);
        }
    }

    #[test]
    fn rejected_items_are_marked_invalid_and_never_chosen_again_this_call() {
        // item0 exposure=0.0 (selalu ditolak jika masih ada kandidat lain),
        // item1 exposure=1.0 (selalu lolos). Karena item0 selalu skor
        // tertinggi, ia akan dicoba duluan, ditolak, ditandai invalid, lalu
        // item1 dicoba & lolos.
        let exposure = vec![0.0, 1.0];
        let scores = [(0usize, 100.0), (1usize, 1.0)];
        let mut valid = vec![true; 2];
        let mut rng = StdRng::seed_from_u64(4);
        let outcome = select(&exposure, &scores, &mut valid, &mut rng, true).unwrap();
        assert_eq!(outcome.chosen, 1);
        assert!(!valid[0], "item yang ditolak harus ditandai invalid");
        assert!(valid[1]);
        assert_eq!(outcome.attempts, vec![(0, false), (1, true)]);
    }

    #[test]
    fn invalidation_persists_only_in_the_given_slice_not_globally() {
        // Dua "sesi" (slice valid_items terpisah) dipakai bergantian --
        // penolakan di satu sesi tidak boleh memengaruhi sesi lain, karena
        // isolasi state sepenuhnya tanggung jawab slice yang diberikan
        // pemanggil (bukan struct global).
        let exposure = vec![0.0, 1.0];
        let scores = [(0usize, 100.0), (1usize, 1.0)];
        let mut valid_session_a = vec![true; 2];
        let valid_session_b = [true; 2];
        let mut rng = StdRng::seed_from_u64(5);

        select(&exposure, &scores, &mut valid_session_a, &mut rng, false).unwrap();
        assert!(!valid_session_a[0]);
        assert!(valid_session_b[0], "sesi lain tidak boleh ikut termutasi");
    }

    #[test]
    fn debug_false_collects_no_attempts() {
        let exposure = vec![0.0, 1.0];
        let scores = [(0usize, 100.0), (1usize, 1.0)];
        let mut valid = vec![true; 2];
        let mut rng = StdRng::seed_from_u64(6);
        let outcome = select(&exposure, &scores, &mut valid, &mut rng, false).unwrap();
        assert!(outcome.attempts.is_empty());
    }
}
