//! Integration test Fase 4: membandingkan `stopping_rule::evaluate()` Rust
//! vs fixture `tests/fixtures/stopping_rule.json`.
//!
//! Fixture dihasilkan oleh
//! `tests/fixtures/generate_r_fixtures_stopping_rule.R`
//! (`Rscript tests/fixtures/generate_r_fixtures_stopping_rule.R`), yang
//! menjalankan `mirtCAT()` sungguhan (R 4.6.1/mirt 1.47/mirtCAT 1.14 — lih.
//! field `source` di file JSON) dengan metodologi **replay** (persis Fase 2):
//! `mirtCAT()` tidak mengekspos state internal `design@met_SEM`/dst.
//! pasca-simulasi, jadi fixture menyimpan trayektori PENUH
//! `thetas_history`/`thetas_SE_history` dari satu run baseline (tanpa
//! kriteria presisi apa pun, seluruh 30 item dijawab), plus titik-berhenti
//! SUNGGUHAN (`n_answered`) dari dua run TERPISAH yang benar-benar
//! mengaktifkan `min_SEM`/`classify` (dan karenanya benar-benar berhenti
//! lebih awal lewat formula `Update.stop_now` R yang identik). Test ini
//! me-replay trayektori baseline lewat `stopping_rule::evaluate()` dengan
//! config yang sama, dan verifikasi `stop_now` PERTAMA kali `true` persis
//! di titik yang sama dengan R.
//!
//! Skenario `sprt_loglik` BUKAN replay `mirtCAT()` (SPRT multidimensional
//! via pipeline `mirtCAT()` penuh punya bug upstream -- lih.
//! `src/stopping_rule/sprt.rs` untuk detail lengkap), melainkan
//! perbandingan langsung `LL0`/`LL1` dari `mirtCAT:::personLogLik()`
//! (dipanggil manual dengan `Theta` di-shape benar) vs rasio log-likelihood
//! yang dihasilkan `stopping_rule::evaluate()` (diekspos lewat
//! `StoppingRuleDebug::sprt_log_likelihood_ratio`, `debug: true`).

use mirt_engine::irt::Dichotomous;
use mirt_engine::{
    evaluate_stopping_rule, ClassifyDecision, ClassifyType, ItemBank, ItemMeta, PersonState,
    SEVector, StoppingConfig, ThetaVector,
};
use serde_json::Value;
use std::fs;

fn load_fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/stopping_rule.json"
    );
    let raw = fs::read_to_string(path).expect("fixture JSON harus ada (jalankan Rscript dulu)");
    serde_json::from_str(&raw).expect("fixture harus JSON valid")
}

fn f64_array(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

/// `jsonlite` meng-`auto_unbox` array panjang 1 jadi skalar -- tangani
/// keduanya (dipakai `sprt_loglik.cases[*].answered_items`/`responses`
/// untuk kasus `n_answered == 1`).
fn usize_array_or_scalar(v: &Value) -> Vec<usize> {
    match v.as_array() {
        Some(arr) => arr.iter().map(|x| x.as_u64().unwrap() as usize).collect(),
        None => vec![v.as_u64().unwrap() as usize],
    }
}

fn build_bank(fixture: &Value) -> ItemBank {
    let a1 = f64_array(&fixture["item_params"]["a1"]);
    let a2 = f64_array(&fixture["item_params"]["a2"]);
    let a3 = f64_array(&fixture["item_params"]["a3"]);
    let d = f64_array(&fixture["item_params"]["d"]);
    let mut bank = ItemBank::new();
    for i in 0..a1.len() {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a1[i], a2[i], a3[i]], d[i]).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn baseline_history(fixture: &Value, key: &str) -> Vec<Vec<f64>> {
    fixture["baseline"][key]
        .as_array()
        .unwrap()
        .iter()
        .map(f64_array)
        .collect()
}

/// Replay trayektori baseline lewat `evaluate_stopping_rule`, mengembalikan
/// step (jumlah item terjawab) pertama kali `stop_now == true`, atau `None`
/// bila tidak pernah berhenti dalam trayektori. Nilai respons yang dicatat
/// tidak relevan untuk mode `None`/`Ci` (`evaluate_stopping_rule` tidak
/// pernah membaca `person.responses()` di kedua mode ini -- hanya
/// `theta`/`se` yang dipakai), jadi dipakai placeholder `1` persis pola
/// replay Fase 2.
fn replay_first_stop_step(
    theta_history: &[Vec<f64>],
    se_history: &[Vec<f64>],
    bank: &ItemBank,
    config: &StoppingConfig,
) -> Option<usize> {
    let n_items = theta_history.len() - 1;
    let mut person = PersonState::from_theta_and_se(
        ThetaVector::from_vec(theta_history[0].clone()),
        SEVector::from_vec(se_history[0].clone()),
    );
    for k in 1..=n_items {
        person.record_answer(k - 1, 1); // index item tidak relevan untuk mode None/Ci
        person.set_estimate(
            ThetaVector::from_vec(theta_history[k].clone()),
            SEVector::from_vec(se_history[k].clone()),
        );
        let (decision, _) = evaluate_stopping_rule(config, &person, bank, false, None).unwrap();
        if decision.stop_now {
            return Some(k);
        }
    }
    None
}

#[test]
fn min_sem_session_stops_at_same_item_as_r() {
    let fixture = load_fixture();
    let bank = build_bank(&fixture);
    let n_items = fixture["n_items"].as_u64().unwrap() as usize;
    let theta_history = baseline_history(&fixture, "thetas_history");
    let se_history = baseline_history(&fixture, "thetas_SE_history");

    let min_sem = fixture["min_sem_session"]["min_sem"].as_f64().unwrap();
    let min_items = fixture["min_sem_session"]["min_items"].as_u64().unwrap() as usize;
    let expected_n_answered = fixture["min_sem_session"]["n_answered"].as_u64().unwrap() as usize;

    let config = StoppingConfig {
        min_items,
        max_items: n_items, // jangan sampai max_items memicu duluan sebelum min_SEM
        min_sem: vec![min_sem],
        ..Default::default()
    };

    let stop_step = replay_first_stop_step(&theta_history, &se_history, &bank, &config);
    assert_eq!(
        stop_step,
        Some(expected_n_answered),
        "titik berhenti Rust (min_SEM={min_sem}) harus persis sama dengan R"
    );
}

#[test]
fn ci_session_stops_at_same_item_with_matching_classification() {
    let fixture = load_fixture();
    let bank = build_bank(&fixture);
    let n_items = fixture["n_items"].as_u64().unwrap() as usize;
    let theta_history = baseline_history(&fixture, "thetas_history");
    let se_history = baseline_history(&fixture, "thetas_SE_history");

    let cutoffs = f64_array(&fixture["ci_session"]["cutoffs"]);
    let classify_ci = fixture["ci_session"]["classify_ci"].as_f64().unwrap();
    let alpha = (1.0 - classify_ci) / 2.0; // persis DesignClass.R:212
    let min_items = fixture["ci_session"]["min_items"].as_u64().unwrap() as usize;
    let expected_n_answered = fixture["ci_session"]["n_answered"].as_u64().unwrap() as usize;
    let expected_classification: Vec<String> = fixture["ci_session"]["classification"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let config = StoppingConfig {
        min_items,
        max_items: n_items,
        classify: ClassifyType::Ci { cutoffs, alpha },
        ..Default::default()
    };

    let n_items_usize = theta_history.len() - 1;
    let mut person = PersonState::from_theta_and_se(
        ThetaVector::from_vec(theta_history[0].clone()),
        SEVector::from_vec(se_history[0].clone()),
    );
    let mut stop_step = None;
    let mut final_decision = None;
    for k in 1..=n_items_usize {
        person.record_answer(k - 1, 1);
        person.set_estimate(
            ThetaVector::from_vec(theta_history[k].clone()),
            SEVector::from_vec(se_history[k].clone()),
        );
        let (decision, _) = evaluate_stopping_rule(&config, &person, &bank, false, None).unwrap();
        if decision.stop_now {
            stop_step = Some(k);
            final_decision = Some(decision);
            break;
        }
    }

    assert_eq!(
        stop_step,
        Some(expected_n_answered),
        "titik berhenti Rust (classify CI) harus persis sama dengan R"
    );
    let classification = final_decision.unwrap().classification;
    let classification_str: Vec<&str> = classification
        .iter()
        .map(|c| match c {
            ClassifyDecision::AboveCutoff => "above cutoff",
            ClassifyDecision::BelowCutoff => "below cutoff",
            ClassifyDecision::NoDecision => "no decision",
        })
        .collect();
    assert_eq!(
        classification_str, expected_classification,
        "klasifikasi akhir per-dimensi Rust harus persis sama dengan R"
    );
}

#[test]
fn sprt_log_likelihood_ratio_matches_person_log_lik_r() {
    let fixture = load_fixture();
    let bank = build_bank(&fixture);
    let tolerance = fixture["tolerance"].as_f64().unwrap();

    let lower = ThetaVector::from_vec(f64_array(&fixture["sprt_loglik"]["lower"]));
    let upper = ThetaVector::from_vec(f64_array(&fixture["sprt_loglik"]["upper"]));

    for case in fixture["sprt_loglik"]["cases"].as_array().unwrap() {
        let answered_items_1indexed = usize_array_or_scalar(&case["answered_items"]);
        let responses = usize_array_or_scalar(&case["responses"]);
        let ll0 = case["ll0"].as_f64().unwrap();
        let ll1 = case["ll1"].as_f64().unwrap();
        let expected_ratio = ll1 - ll0;

        let mut person = PersonState::new(3);
        for (idx_1based, &response) in answered_items_1indexed.iter().zip(&responses) {
            person.record_answer(idx_1based - 1, response);
        }

        let config = StoppingConfig {
            min_items: 0,
            classify: ClassifyType::Sprt {
                lower: lower.clone(),
                upper: upper.clone(),
                alpha: 0.05,
                beta: 0.05,
            },
            debug: true,
            ..Default::default()
        };
        let (_, debug) = evaluate_stopping_rule(&config, &person, &bank, false, None).unwrap();
        let ratio = debug.unwrap().sprt_log_likelihood_ratio.unwrap();

        assert!(
            (ratio - expected_ratio).abs() < tolerance,
            "n_answered={}: rasio log-likelihood Rust={ratio} vs R={expected_ratio}",
            case["n_answered"]
        );
    }
}
