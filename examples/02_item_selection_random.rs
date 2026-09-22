//! Contoh Fase 2: kriteria `random` — pemilihan uniform random di antara
//! kandidat tersisa, baseline exposure-neutral. Memakai `StdRng` dengan
//! seed tetap supaya hasil reproducible.
//!
//! `debug: true` diaktifkan untuk mendemonstrasikan `ItemSelectionDebug`
//! apa adanya -- `random` tidak menghitung nilai kriteria sama sekali, jadi
//! `candidate_scores` selalu kosong dan `info_before`/`info_after` selalu
//! `None` (beda dari kriteria D/T/A/E/W-rule, lih. contoh lain).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 02_item_selection_random
//! ```

use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::{
    select_next_item, Criterion, GroupParams, ItemBank, ItemMeta, ItemSelectionDebug, PersonState,
    SelectionConfig,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

fn build_bank() -> ItemBank {
    let params: [([f64; 2], f64); 8] = [
        ([1.2, 0.3], 0.2),
        ([0.3, 1.2], -0.1),
        ([0.9, 0.9], 0.0),
        ([1.5, 0.2], 0.4),
        ([0.2, 1.5], -0.3),
        ([1.0, 1.0], 0.1),
        ([0.7, 1.3], 0.0),
        ([1.3, 0.7], -0.2),
    ];
    let mut bank = ItemBank::new();
    for (i, (a, d)) in params.into_iter().enumerate() {
        bank.push(
            Box::new(Dichotomous::m2pl(a.to_vec(), d).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

/// Cetak isi `ItemSelectionDebug` selengkapnya: nilai kriteria seluruh
/// kandidat (bukan cuma yang terpilih), dan snapshot info sebelum/sesudah
/// bila tersedia (`None` untuk `seq`/`random`/`KL`).
fn print_debug(debug: &ItemSelectionDebug) {
    if debug.candidate_scores.is_empty() {
        println!("    candidate_scores = [] (kriteria ini tidak menghitung nilai per kandidat)");
    } else {
        let mut scores = debug.candidate_scores.clone();
        scores.sort_by_key(|(idx, _)| *idx);
        for (idx, val) in &scores {
            let marker = if *idx == debug.chosen {
                "  <- dipilih"
            } else {
                ""
            };
            println!("    item ke-{}: {val:.6}{marker}", idx + 1);
        }
    }
    match (&debug.info_before, &debug.info_after) {
        (Some(before), Some(after)) => {
            println!("    info_before = {before:?}");
            println!("    info_after  = {after:?}");
        }
        _ => println!("    info_before/info_after = None"),
    }
}

fn run_with_seed(seed: u64, verbose: bool) -> Vec<usize> {
    let bank = build_bank();
    let group = GroupParams::standard_normal(2);
    let config = SelectionConfig {
        criterion: Criterion::Random,
        debug: true,
        ..Default::default()
    };
    let mut person = PersonState::new(2);
    let mut rng = StdRng::seed_from_u64(seed);

    let mut order = Vec::new();
    for step in 0..5 {
        let candidates: Vec<usize> = (0..bank.len())
            .filter(|idx| !person.answered_items().contains(idx))
            .collect();
        let (chosen, debug) =
            select_next_item(&config, &candidates, &person, &bank, &group, &mut rng).unwrap();
        if verbose {
            let debug = debug.expect("debug=true harus menghasilkan Some");
            println!("  langkah {step}: item ke-{} dipilih", chosen + 1);
            print_debug(&debug);
        }
        order.push(chosen);
        person.record_answer(chosen);
    }
    order
}

fn main() {
    println!("== random: seed tetap (42) -- reproducible, dengan debug trace ==");
    let order_a = run_with_seed(42, true);
    println!(
        "  urutan: {:?}",
        order_a.iter().map(|i| i + 1).collect::<Vec<_>>()
    );

    let order_b = run_with_seed(42, false);
    assert_eq!(order_a, order_b);
    println!("  (seed sama -> urutan identik, diverifikasi ulang)");

    println!("\n== random: seed berbeda (7) -- urutan lain ==");
    let order_c = run_with_seed(7, false);
    println!(
        "  urutan: {:?}",
        order_c.iter().map(|i| i + 1).collect::<Vec<_>>()
    );
}
