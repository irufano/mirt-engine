//! Integration test Fase 0: membandingkan `ItemResponseModel` Rust vs
//! fixture `tests/fixtures/irt_probability_information.json`.
//!
//! Fixture dihasilkan oleh `tests/fixtures/generate_r_fixtures.R`
//! (`Rscript tests/fixtures/generate_r_fixtures.R`), yang membangun objek
//! `mirt` dari parameter tetap lewat `mirtCAT::generate.mirt_object()` lalu
//! mengekstrak `mirt::probtrace()`/`mirt::iteminfo(multidim_matrix=TRUE)`
//! sebagai ground truth sungguhan (divalidasi dengan R 4.6.1, mirt 1.47,
//! mirtCAT 1.14 — lih. field `source` di file JSON). Ini memenuhi syarat
//! parity numerik `1e-6` di Definition of Done Fase 0.
//!
//! Formula yang diimplementasikan Rust mengikuti Reckase (2009) & Muraki &
//! Carlson (1995), dibaca dari struktur `mirt::probtrace()`/
//! `mirt::iteminfo()` (`mirt/src/traceLinePts.cpp`,
//! `mirt/R/utils.R::ItemInfo2`) sebagai rujukan perilaku, bukan disalin
//! (clean-room reimplementation, lih. `PLAN.md` §1.3).

use mirt_engine::irt::{Dichotomous, GradedResponse, ItemResponseModel};
use mirt_engine::theta::ThetaVector;
use serde_json::Value;
use std::fs;

fn load_fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/irt_probability_information.json"
    );
    let raw = fs::read_to_string(path).expect("fixture JSON harus ada");
    serde_json::from_str(&raw).expect("fixture harus JSON valid")
}

fn assert_case_matches(
    fixture: &Value,
    case_name: &str,
    item: &dyn ItemResponseModel,
    tolerance: f64,
) {
    let case = &fixture["cases"][case_name];
    let theta_points = case["theta"].as_array().expect("theta array");
    let expected_probs = case["probabilities"]
        .as_array()
        .expect("probabilities array");
    let expected_info = case["information"].as_array().expect("information array");

    for (idx, theta_json) in theta_points.iter().enumerate() {
        let theta_vals: Vec<f64> = theta_json
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        let theta = ThetaVector::from_vec(theta_vals);

        let probs = item.probabilities(&theta).unwrap();
        let exp_probs: Vec<f64> = expected_probs[idx]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_f64().unwrap())
            .collect();
        assert_eq!(
            probs.len(),
            exp_probs.len(),
            "[{case_name}] theta idx {idx}: jumlah kategori tidak cocok"
        );
        for (k, (p, ep)) in probs.iter().zip(exp_probs.iter()).enumerate() {
            assert!(
                (p - ep).abs() < tolerance,
                "[{case_name}] theta idx {idx}, kategori {k}: rust={p}, fixture={ep}"
            );
        }

        let info = item.information(&theta).unwrap();
        let exp_info_rows = expected_info[idx].as_array().unwrap();
        for (i, exp_row_json) in exp_info_rows.iter().enumerate() {
            let exp_row: Vec<f64> = exp_row_json
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_f64().unwrap())
                .collect();
            for (j, exp) in exp_row.iter().enumerate() {
                let got = info.get(i, j);
                assert!(
                    (got - exp).abs() < tolerance,
                    "[{case_name}] theta idx {idx}, info[{i},{j}]: rust={got}, fixture={exp}"
                );
            }
        }
    }
}

#[test]
fn dichotomous_m2pl_2d_matches_fixture() {
    let fixture = load_fixture();
    let tol = fixture["tolerance"].as_f64().unwrap();
    let item = Dichotomous::m2pl(vec![1.2, 0.8], 0.3).unwrap();
    assert_case_matches(&fixture, "m2pl_2d", &item, tol);
}

#[test]
fn dichotomous_m3pl_2d_matches_fixture() {
    let fixture = load_fixture();
    let tol = fixture["tolerance"].as_f64().unwrap();
    let item = Dichotomous::m3pl(vec![1.0, 1.0], -0.5, 0.2).unwrap();
    assert_case_matches(&fixture, "m3pl_2d", &item, tol);
}

#[test]
fn graded_response_2d_matches_fixture() {
    let fixture = load_fixture();
    let tol = fixture["tolerance"].as_f64().unwrap();
    let item = GradedResponse::new(vec![1.1, 0.6], vec![1.5, 0.2, -1.2]).unwrap();
    assert_case_matches(&fixture, "graded_2d", &item, tol);
}

#[test]
fn dichotomous_m2pl_3d_matches_fixture() {
    let fixture = load_fixture();
    let tol = fixture["tolerance"].as_f64().unwrap();
    let item = Dichotomous::m2pl(vec![0.9, 0.7, 1.1], 0.0).unwrap();
    assert_case_matches(&fixture, "m2pl_3d", &item, tol);
}
