//! Contoh Fase 2: kriteria `Drule`/`DPrule` — D-optimality, maksimalkan
//! determinan matriks informasi Fisher kumulatif (Segall, 1996). Varian
//! `DPrule` menambah `inverse(gcov)` sebagai informasi prior sebelum
//! dievaluasi, sehingga full-rank sejak awal (lih. `PLAN.md` §Fase 2: tanpa
//! prior, determinan 1 item dikotomus tunggal selalu 0 -- rank defisien).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 02_item_selection_d_rule
//! ```

use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::theta::InfoMatrix;
use mirtcat_engine::{
    select_next_item, Criterion, GroupParams, ItemBank, ItemMeta, ItemSelectionDebug, PersonState,
    SelectionConfig, ThetaVector,
};
use nalgebra::DMatrix;
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

fn build_group() -> GroupParams {
    let gcov = InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[1.0, 0.3, 0.3, 1.0]))
        .expect("gcov persegi");
    GroupParams::new(ThetaVector::zeros(2), gcov).unwrap()
}

/// Format `InfoMatrix` 2×2 secara ringkas untuk dicetak (lebih terbaca
/// dibanding `{:?}` bawaan, yang membocorkan detail penyimpanan `nalgebra`).
fn format_info(m: &InfoMatrix) -> String {
    format!(
        "[[{:.4}, {:.4}], [{:.4}, {:.4}]]",
        m.get(0, 0),
        m.get(0, 1),
        m.get(1, 0),
        m.get(1, 1)
    )
}

/// Cetak isi `ItemSelectionDebug` selengkapnya: nilai kriteria (determinan)
/// seluruh kandidat (bukan cuma yang terpilih), dan snapshot `info_thetas`
/// sebelum/sesudah item terpilih ditambahkan.
fn print_debug(debug: &ItemSelectionDebug) {
    let mut scores = debug.candidate_scores.clone();
    scores.sort_by_key(|(idx, _)| *idx);
    for (idx, val) in &scores {
        let marker = if *idx == debug.chosen {
            "  <- dipilih"
        } else {
            ""
        };
        println!("    item ke-{}: det={val:.6}{marker}", idx + 1);
    }
    println!(
        "    info_before = {}",
        format_info(debug.info_before.as_ref().unwrap())
    );
    println!(
        "    info_after  = {}",
        format_info(debug.info_after.as_ref().unwrap())
    );
}

fn run(prior: bool, label: &str) {
    let bank = build_bank();
    let group = build_group();
    let config = SelectionConfig {
        criterion: Criterion::DRule { prior },
        debug: true,
        ..Default::default()
    };
    let mut person = PersonState::new(2);
    let mut rng = StdRng::seed_from_u64(0);

    println!("== {label} ==");
    for step in 0..5 {
        let candidates: Vec<usize> = (0..bank.len())
            .filter(|idx| !person.answered_items().contains(idx))
            .collect();
        let (chosen, debug) =
            select_next_item(&config, &candidates, &person, &bank, &group, &mut rng).unwrap();
        let debug = debug.expect("debug=true harus menghasilkan Some");
        println!("  langkah {step}: item ke-{} dipilih", chosen + 1);
        print_debug(&debug);
        person.record_answer(chosen);
    }
}

fn main() {
    run(false, "Drule: tanpa prior");
    println!();
    run(true, "DPrule: dengan prior inverse(gcov)");
}
