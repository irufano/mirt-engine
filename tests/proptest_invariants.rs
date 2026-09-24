//! Fase 8 — property-based test (`proptest`) untuk invariant lintas modul
//! yang harus tetap benar untuk SEMUA input valid, bukan cuma titik-titik
//! yang ditulis manual di test unit/integrasi lain:
//!
//! 1. Probabilitas kategori satu item selalu menjumlah `1.0` (`irt`).
//! 2. Matriks informasi Fisher kumulatif selalu positive semi-definite
//!    (`irt::cumulative_information`).
//! 3. Proporsi empiris content balancing tidak pernah negatif
//!    (`content_balancing::narrow_candidates`).
//!
//! Strategi generator memakai konstruktor yang sudah memvalidasi
//! (`Dichotomous::m2pl`/`m3pl`, `GradedResponse::new`,
//! `ContentBalancingConfig::new`) supaya proptest hanya pernah menghasilkan
//! kombinasi yang valid menurut aturan `mirtCAT` sendiri.

use std::collections::HashMap;

use proptest::prelude::*;

use mirt_engine::content_balancing::{
    narrow_candidates, ContentBalancingConfig, ContentCategory,
};
use mirt_engine::irt::{cumulative_information, Dichotomous, GradedResponse, ItemResponseModel};
use mirt_engine::theta::ThetaVector;

/// Slope per dimensi, dibatasi ke rentang yang wajar secara psikometrik
/// (item ekstrem/nol tidak menambah cakupan invariant, cuma memperlambat
/// shrinking).
fn slope_strategy(nfact: usize) -> impl Strategy<Value = Vec<f64>> {
    proptest::collection::vec(0.1f64..3.0, nfact)
}

fn theta_strategy(nfact: usize) -> impl Strategy<Value = ThetaVector> {
    proptest::collection::vec(-4.0f64..4.0, nfact).prop_map(ThetaVector::from_vec)
}

/// Item dikotomus valid (M2PL atau M3PL) untuk `nfact` dimensi.
fn dichotomous_strategy(nfact: usize) -> impl Strategy<Value = Dichotomous> {
    (
        slope_strategy(nfact),
        -3.0f64..3.0,
        prop_oneof![Just(None), (0.0f64..0.4).prop_map(Some)],
    )
        .prop_map(|(a, d, g)| match g {
            None => Dichotomous::m2pl(a, d).unwrap(),
            Some(g) => Dichotomous::m3pl(a, d, g).unwrap(),
        })
}

/// Item graded response valid (3 boundary -> 4 kategori) untuk `nfact` dimensi.
fn graded_strategy(nfact: usize) -> impl Strategy<Value = GradedResponse> {
    (
        slope_strategy(nfact),
        -3.0f64..(-1.0),
        -1.0f64..1.0,
        1.0f64..3.0,
    )
        .prop_map(|(a, d3, d2, d1)| {
            // `d` harus terurut menurun (d1 > d2 > d3) -- lih. GradedResponse::new.
            GradedResponse::new(a, vec![d1, d2, d3]).unwrap()
        })
}

/// `(item, theta)` dikotomus valid, `nfact` acak 1..=4.
fn dichotomous_case_strategy() -> impl Strategy<Value = (Dichotomous, ThetaVector)> {
    (1usize..=4).prop_flat_map(|nfact| (dichotomous_strategy(nfact), theta_strategy(nfact)))
}

/// `(item, theta)` graded response valid, `nfact` acak 1..=4.
fn graded_case_strategy() -> impl Strategy<Value = (GradedResponse, ThetaVector)> {
    (1usize..=4).prop_flat_map(|nfact| (graded_strategy(nfact), theta_strategy(nfact)))
}

/// `(nfact, item dikotomus, item graded, theta)` untuk uji informasi
/// kumulatif — minimal satu item (dikotomus atau graded).
fn cumulative_info_case_strategy(
) -> impl Strategy<Value = (usize, Vec<Dichotomous>, Vec<GradedResponse>, ThetaVector)> {
    (1usize..=3, 0usize..4, 0usize..4)
        .prop_flat_map(|(nfact, n_dich, n_graded)| {
            (
                Just(nfact),
                proptest::collection::vec(dichotomous_strategy(nfact), n_dich),
                proptest::collection::vec(graded_strategy(nfact), n_graded),
                theta_strategy(nfact),
            )
        })
        .prop_filter("minimal satu item", |(_, d, g, _)| {
            !d.is_empty() || !g.is_empty()
        })
}

/// `(config, answered_items, candidate_pool)` valid untuk
/// `narrow_candidates`: kategori & target proporsi dibangun deterministik
/// dari `n_items`/`n_categories` (selalu menjumlah 1.0 persis), lalu
/// `answered_items` diacak sebagai subsequence index item, disisakan
/// minimal 1 kandidat.
fn content_balancing_case_strategy(
) -> impl Strategy<Value = (ContentBalancingConfig, Vec<usize>, Vec<usize>)> {
    (2usize..12, 1usize..4).prop_flat_map(|(n_items, n_categories)| {
        let category_names: Vec<String> = (0..n_categories).map(|i| format!("cat{i}")).collect();
        let category_per_item: Vec<ContentCategory> = (0..n_items)
            .map(|i| ContentCategory::new(category_names[i % n_categories].clone()))
            .collect();

        let mut target_prop: HashMap<ContentCategory, f64> = HashMap::new();
        let base = 1.0 / n_categories as f64;
        let mut running = 0.0;
        for (i, name) in category_names.iter().enumerate() {
            let p = if i + 1 == n_categories {
                1.0 - running
            } else {
                base
            };
            running += p;
            target_prop.insert(ContentCategory::new(name.clone()), p);
        }

        let config = ContentBalancingConfig::new(category_per_item, target_prop, true).unwrap();
        let all_indices: Vec<usize> = (0..n_items).collect();

        proptest::sample::subsequence(all_indices.clone(), 0..=n_items)
            .prop_map(move |answered_items| {
                let candidate_pool: Vec<usize> = all_indices
                    .iter()
                    .copied()
                    .filter(|i| !answered_items.contains(i))
                    .collect();
                (config.clone(), answered_items, candidate_pool)
            })
            .prop_filter("minimal 1 kandidat tersisa", |(_, _, pool)| {
                !pool.is_empty()
            })
    })
}

proptest! {
    /// Invariant 1: probabilitas kategori item dikotomus (M2PL/M3PL) selalu
    /// menjumlah `1.0`, untuk seluruh `theta` & parameter valid.
    #[test]
    fn dichotomous_probabilities_sum_to_one((item, theta) in dichotomous_case_strategy()) {
        let probs = item.probabilities(&theta).unwrap();
        let sum: f64 = probs.iter().sum();
        prop_assert!((sum - 1.0).abs() < 1e-9, "sum={sum}, probs={probs:?}");
    }

    /// Invariant 1 (lanjutan): sama untuk item graded response (politomus).
    #[test]
    fn graded_probabilities_sum_to_one((item, theta) in graded_case_strategy()) {
        let probs = item.probabilities(&theta).unwrap();
        let sum: f64 = probs.iter().sum();
        prop_assert!((sum - 1.0).abs() < 1e-9, "sum={sum}, probs={probs:?}");
    }

    /// Invariant 2: matriks informasi Fisher kumulatif dari sekumpulan item
    /// (campuran dikotomus & graded) selalu positive semi-definite --
    /// `min_eigenvalue() >= -eps` -- pada `theta` mana pun.
    #[test]
    fn cumulative_information_is_positive_semi_definite(
        (nfact, dich_items, graded_items, theta) in cumulative_info_case_strategy()
    ) {
        let refs: Vec<&dyn ItemResponseModel> = dich_items
            .iter()
            .map(|i| i as &dyn ItemResponseModel)
            .chain(graded_items.iter().map(|i| i as &dyn ItemResponseModel))
            .collect();

        let info = cumulative_information(nfact, refs, &theta).unwrap();
        let min_eig = info.min_eigenvalue();
        prop_assert!(min_eig >= -1e-8, "min_eigenvalue={min_eig}");
    }

    /// Invariant 3: proporsi empiris content balancing (`empirical_prop`)
    /// tidak pernah negatif, untuk kombinasi kategori/target/riwayat
    /// jawaban valid mana pun.
    #[test]
    fn content_balancing_empirical_proportions_never_negative(
        (config, answered_items, candidate_pool) in content_balancing_case_strategy()
    ) {
        let (_, debug) = narrow_candidates(&config, &answered_items, &candidate_pool).unwrap();
        let debug = debug.expect("debug=true di config -- narrow_candidates harus Some");

        for (cat, prop) in &debug.empirical_prop {
            prop_assert!(*prop >= 0.0, "empirical_prop[{}] = {prop}", cat.0);
        }
    }
}
