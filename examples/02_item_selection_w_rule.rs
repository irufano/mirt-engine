//! Contoh Fase 2: kriteria `Wrule`/`WPrule` — kriteria informasi berbobot
//! kustom: `weights' * total_info * weights`, bentuk kuadratik PENUH
//! (termasuk off-diagonal) -- beda dari `Trule` yang hanya menjumlahkan
//! diagonal berbobot (lih. contoh `02_item_selection_t_rule.rs`). Setara
//! informasi Fisher tentang skor komposit `weights' * theta`.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 02_item_selection_w_rule
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

/// Cetak isi `ItemSelectionDebug` selengkapnya: nilai kriteria (bentuk
/// kuadratik `weights' * I * weights`) seluruh kandidat (bukan cuma yang
/// terpilih), dan snapshot `info_thetas` sebelum/sesudah item terpilih
/// ditambahkan.
fn print_debug(debug: &ItemSelectionDebug) {
    let mut scores = debug.candidate_scores.clone();
    scores.sort_by_key(|(idx, _)| *idx);
    for (idx, val) in &scores {
        let marker = if *idx == debug.chosen {
            "  <- dipilih"
        } else {
            ""
        };
        println!(
            "    item ke-{}: weights'*I*weights={val:.4}{marker}",
            idx + 1
        );
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

fn run(criterion: Criterion, weights: Vec<f64>, label: &str) {
    let bank = build_bank();
    let group = build_group();
    let config = SelectionConfig {
        criterion,
        weights: Some(weights),
        debug: true,
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
        person.record_answer(chosen, 1);
    }
}

fn main() {
    run(
        Criterion::WRule { prior: false },
        vec![1.0, 1.0],
        "Wrule: bobot setara -- info tentang jumlah dim1+dim2",
    );
    println!();
    run(
        Criterion::WRule { prior: false },
        vec![2.0, 0.2],
        "Wrule: bobot timpang -- info tentang komposit yang didominasi dim1",
    );
    println!();
    run(
        Criterion::WRule { prior: true },
        vec![1.0, 1.0],
        "WPrule: dengan prior inverse(gcov)",
    );
}
