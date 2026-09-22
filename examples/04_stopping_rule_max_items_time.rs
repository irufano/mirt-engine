//! Contoh Fase 4: `max_items`/`max_time` — batas keras, dicek **independen**
//! dari kriteria presisi (`docs/teori/04-stopping-rule-max-items-time.md`).
//! Skenario 1 memakai `nfact=6` untuk menunjukkan implikasi eksplisit
//! `PLAN.md`: makin banyak dimensi, makin sulit syarat "all" (`min_SEM`)
//! terpenuhi secara statistik -- `max_items` sering jadi kondisi berhenti
//! PRAKTIS untuk model dimensi tinggi, bukan cuma jaring pengaman teoretis.
//! Skenario 2 menunjukkan `max_time` menghentikan sesi walau presisi sudah
//! nyaris tercapai.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 04_stopping_rule_max_items_time
//! ```

use std::time::Duration;

use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::stopping_rule::{evaluate, StoppingConfig};
use mirtcat_engine::{ItemBank, ItemMeta, PersonState, SEVector, ThetaVector};

fn build_bank(n: usize) -> ItemBank {
    let mut bank = ItemBank::new();
    for i in 0..n {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0, 1.0, 1.0, 1.0, 1.0, 1.0], 0.0).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn scenario_max_items_high_nfact() {
    println!("== Skenario 1: max_items sebagai batas praktis pada nfact=6 ==");
    let nfact = 6;
    let n_items = 25;
    let bank = build_bank(n_items);
    let config = StoppingConfig {
        min_items: 3,
        max_items: n_items,
        min_sem: vec![0.3; nfact], // ambang "wajar" untuk nfact rendah -- sengaja TIDAK dilonggarkan
        debug: true,
        ..Default::default()
    };
    let mut person = PersonState::new(nfact);

    for k in 1..=n_items {
        person.record_answer(k - 1, 1);
        // SE menurun lambat & merata di 6 dimensi sekaligus (setiap item
        // hanya membagi informasi tipis ke tiap dimensi) -- mensimulasikan
        // realita nfact tinggi: informasi per-dimensi per-item jauh lebih
        // encer dibanding kasus nfact rendah (`04_stopping_rule_min_sem`).
        let se: Vec<f64> = (0..nfact)
            .map(|_| 0.4 + 0.6 * 0.94_f64.powi(k as i32))
            .collect();
        let theta = vec![0.5; nfact];
        person.set_estimate(ThetaVector::from_vec(theta), SEVector::from_vec(se.clone()));

        let (decision, debug) = evaluate(&config, &person, &bank, false, None).unwrap();
        let debug = debug.unwrap();
        let n_met = decision.met_sem.iter().filter(|&&m| m).count();
        println!(
            "item ke-{k:>2}: se (semua dim sama)={:.3}  {n_met}/{nfact} dimensi memenuhi min_SEM=0.3  stop_now={}",
            debug.se_values[0],
            decision.stop_now,
        );
        if decision.stop_now {
            println!();
            if decision.terminated_early {
                println!(
                    "Presisi tercapai sebelum item habis (tidak diharapkan pada skenario ini)."
                );
            } else {
                println!(
                    "Berhenti karena max_items={n_items} TERCAPAI -- min_SEM belum terpenuhi di \
                     seluruh {nfact} dimensi (terlihat dari kolom \"dimensi memenuhi\" di atas yang \
                     tidak pernah mencapai {nfact}/{nfact}). Ini PERSIS implikasi PLAN.md: makin \
                     banyak nfact, makin sulit semantik all() terpenuhi secara statistik, sehingga \
                     max_items menjadi kondisi berhenti PRAKTIS, bukan cuma jaring pengaman."
                );
            }
            break;
        }
    }
    println!();
}

fn scenario_max_time() {
    println!("== Skenario 2: max_time menghentikan sesi walau presisi nyaris tercapai ==");
    let bank = build_bank(30);
    let config = StoppingConfig {
        min_items: 3,
        max_items: bank.len(),
        max_time: Some(Duration::from_secs(120)),
        min_sem: vec![0.1, 0.1], // sengaja SANGAT ketat -- presisi TIDAK akan tercapai duluan
        debug: true,
        ..Default::default()
    };
    let mut person = PersonState::new(2);

    for k in 1..=bank.len() {
        person.record_answer(k - 1, 1);
        person.record_item_time(Duration::from_secs(12)); // 12 detik/item
        let se = vec![0.5 * 0.9_f64.powi(k as i32); 2];
        person.set_estimate(
            ThetaVector::from_vec(vec![0.3, -0.2]),
            SEVector::from_vec(se),
        );

        let (decision, _) = evaluate(&config, &person, &bank, false, None).unwrap();
        println!(
            "item ke-{k:>2}: total_time={:>4}s  se={:.3}  stop_now={}",
            person.total_time().as_secs(),
            person.se()[0],
            decision.stop_now,
        );
        if decision.stop_now {
            println!();
            assert!(
                !decision.terminated_early,
                "harus berhenti karena max_time, bukan presisi"
            );
            println!(
                "Berhenti karena max_time=120s TERCAPAI (total_time={}s) -- min_SEM=0.1 SENGAJA \
                 dibuat sangat ketat supaya presisi tidak pernah tercapai duluan, menunjukkan \
                 max_time murni sebagai batas waktu independen.",
                person.total_time().as_secs()
            );
            break;
        }
    }
}

fn main() {
    scenario_max_items_high_nfact();
    scenario_max_time();
}
