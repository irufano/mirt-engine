//! Contoh Fase 6: content balancing — menjaga proporsi kategori konten item
//! yang disajikan mendekati target 50/50 dua kategori (`docs/teori/
//! 06-content-balancing.md`).
//!
//! Item bank 2D, 12 item: 6 kategori "Aljabar" (kuat di dim1), 6 kategori
//! "Geometri" (kuat di dim2) — pola sama seperti bank fixture
//! `tests/fixtures/generate_r_fixtures_content_balancing.R`. Sesi 10 item
//! (< 12, jadi TIDAK ada kategori yang habis di sini — demonstrasi
//! konvergensi murni, bukan jalur fallback; lih.
//! `tests/integration_content_balancing.rs` skenario "fallback" untuk
//! demonstrasi jalur itu).
//!
//! Theta TETAP di nol sepanjang sesi (murni mengisolasi mekanisme content
//! balancing dari ability estimation, pola yang sama seperti contoh
//! Fase 5). Memakai `DPrule` (prior) supaya matriks informasi full rank
//! sejak 0 item terjawab (lih. catatan `examples/05_exposure_control_*.rs`
//! soal `Drule` polos degenerate di titik ini).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 06_content_balancing
//! ```

use mirt_engine::content_balancing::{narrow_candidates, ContentBalancingConfig};
use mirt_engine::irt::Dichotomous;
use mirt_engine::theta::InfoMatrix;
use mirt_engine::{
    select_next_item, Criterion, GroupParams, ItemBank, ItemMeta, PersonState, SelectionConfig,
    ThetaVector,
};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::HashMap;

const N_ITEMS: usize = 12;
const SESSION_LENGTH: usize = 10;

fn build_bank() -> ItemBank {
    let params: [([f64; 2], f64); N_ITEMS] = [
        ([1.6, 0.1], 0.2),
        ([1.4, 0.2], -0.1),
        ([1.5, 0.1], 0.0),
        ([1.3, 0.3], 0.3),
        ([1.7, 0.2], -0.2),
        ([1.2, 0.1], 0.1),
        ([0.1, 1.6], 0.1),
        ([0.2, 1.4], 0.3),
        ([0.1, 1.5], -0.2),
        ([0.3, 1.3], 0.0),
        ([0.2, 1.7], 0.2),
        ([0.1, 1.2], -0.1),
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
    GroupParams::new(ThetaVector::zeros(2), InfoMatrix::identity(2)).unwrap()
}

fn main() {
    let bank = build_bank();
    let group = build_group();

    // item 0..=5 -> "Aljabar", item 6..=11 -> "Geometri".
    let categories: Vec<_> = (0..N_ITEMS)
        .map(|i| if i < 6 { "Aljabar" } else { "Geometri" }.into())
        .collect();
    let mut target_prop = HashMap::new();
    target_prop.insert("Aljabar".into(), 0.5);
    target_prop.insert("Geometri".into(), 0.5);
    let cb_config = ContentBalancingConfig::new(categories, target_prop, false).unwrap();

    let sel_config = SelectionConfig {
        criterion: Criterion::DRule { prior: true },
        ..Default::default()
    };

    let mut person = PersonState::new(2);
    let mut rng = StdRng::seed_from_u64(0);
    let mut aljabar_count = 0usize;
    let mut geometri_count = 0usize;

    println!(
        "Target: Aljabar=0.5, Geometri=0.5 -- item 0..5 Aljabar, item 6..11 Geometri, sesi {SESSION_LENGTH} item."
    );
    println!();
    println!(
        "{:>5} {:>8} {:>10} {:>10}",
        "step", "item", "p_aljabar", "p_geometri"
    );

    for step in 1..=SESSION_LENGTH {
        let candidates: Vec<usize> = (0..N_ITEMS)
            .filter(|idx| !person.answered_items().contains(idx))
            .collect();
        let (narrowed, _) =
            narrow_candidates(&cb_config, person.answered_items(), &candidates).unwrap();
        let (chosen, _) =
            select_next_item(&sel_config, &narrowed, &person, &bank, &group, &mut rng).unwrap();

        person.record_answer(chosen, 1);
        if chosen < 6 {
            aljabar_count += 1;
        } else {
            geometri_count += 1;
        }

        let p_aljabar = aljabar_count as f64 / step as f64;
        let p_geometri = geometri_count as f64 / step as f64;
        println!("{step:>5} {chosen:>8} {p_aljabar:>10.3} {p_geometri:>10.3}");
    }

    println!();
    println!(
        "Proporsi realisasi akhir konvergen ke target 0.5/0.5 -- content balancing menyempitkan \
         kandidat ke kategori dengan defisit (target - empirical) terbesar di tiap langkah."
    );
    assert_eq!(aljabar_count + geometri_count, SESSION_LENGTH);
}
