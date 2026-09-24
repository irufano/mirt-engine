//! Contoh Fase 3: EAP berbasis summed score (`EapSum`) — cetak pemetaan
//! kategori skor total ke theta/SE (`docs/teori/03-ability-estimation-eap-sum.md`).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 03_ability_estimation_eap_sum
//! ```

use mirt_engine::ability_estimation::{eap_sum_score_table, estimate};
use mirt_engine::irt::Dichotomous;
use mirt_engine::{EstimationConfig, GroupParams, ItemBank, ItemMeta, Method, PersonState};

fn build_bank() -> ItemBank {
    let params: [(f64, f64); 4] = [(1.2, 0.0), (0.9, 0.1), (1.0, -0.1), (0.8, 0.2)];
    let mut bank = ItemBank::new();
    for (i, (a, d)) in params.into_iter().enumerate() {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a], d).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn main() {
    let bank = build_bank();
    let group = GroupParams::standard_normal(1);

    let mut person = PersonState::new(1);
    for (idx, response) in [1, 0, 1, 1].into_iter().enumerate() {
        person.record_answer(idx, response);
    }

    println!("4 item dikotomus dijawab, pola: [1, 0, 1, 1] (skor total = 3)");
    println!();

    let config = EstimationConfig {
        method: Method::EapSum,
        ..Default::default()
    };

    println!("== Tabel LENGKAP: pemetaan skor total (0..=4) -> (theta, SE) ==");
    let table = eap_sum_score_table(&config, &person, &bank, &group).unwrap();
    for row in &table {
        println!(
            "  skor={} -> theta={:.4}, se={:.4}",
            row.sum_score,
            row.theta[0],
            row.se.as_slice()[0]
        );
    }

    println!();
    println!("== Hasil `estimate()` -- cuma skor peserta SEKARANG (3) ==");
    let ((theta, se), _) = estimate(&config, &person, &bank, &group).unwrap();
    println!("  theta={:.4}, se={:.4}", theta[0], se.as_slice()[0]);
    println!(
        "  (harus sama dengan baris skor=3 di tabel lengkap di atas -- \
         `estimate()` hanya mengambil baris yang relevan dari tabel yang sama)"
    );

    println!();
    println!(
        "Pengamatan: theta monoton naik seiring skor total naik (0 -> 4) -- \
         EapSum tidak peduli item MANA yang benar, hanya JUMLAH total yang benar \
         (beda dari EAP biasa yang memakai pola respons item-per-item)."
    );
}
