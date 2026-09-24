//! Integration test Fase 1: membandingkan `initial_theta::initialize()` Rust
//! vs fixture `tests/fixtures/initial_theta.json`.
//!
//! Fixture dihasilkan oleh
//! `tests/fixtures/generate_r_fixtures_initial_theta.R`
//! (`Rscript tests/fixtures/generate_r_fixtures_initial_theta.R`), yang
//! menjalankan pipeline `mirtCAT()` off-line sungguhan (mode
//! `local_pattern`) lalu membaca baris pertama `thetas_history`/
//! `thetas_SE_history` dari objek `Person` yang benar-benar dipakai
//! `run_local()` (divalidasi dengan R 4.6.1, mirt 1.47, mirtCAT 1.14 — lih.
//! field `source` di file JSON).

use mirt_engine::theta::InfoMatrix;
use mirt_engine::{GroupParams, InitialThetaConfig, ThetaStart, ThetaVector};
use nalgebra::DMatrix;
use serde_json::Value;
use std::fs;

fn load_fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/initial_theta.json"
    );
    let raw = fs::read_to_string(path).expect("fixture JSON harus ada");
    serde_json::from_str(&raw).expect("fixture harus JSON valid")
}

fn f64_array(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

fn assert_case_matches(fixture: &Value, case_name: &str) {
    let case = &fixture["cases"][case_name];
    let tol = fixture["tolerance"].as_f64().unwrap();
    let nfact = case["nfact"].as_u64().unwrap() as usize;

    let gmeans = ThetaVector::from_vec(f64_array(&case["gmeans"]));
    let gcov_rows: Vec<Vec<f64>> = case["gcov"]
        .as_array()
        .unwrap()
        .iter()
        .map(f64_array)
        .collect();
    let gcov_flat: Vec<f64> = gcov_rows.iter().flatten().cloned().collect();
    let gcov = InfoMatrix::from_matrix(DMatrix::from_row_slice(nfact, nfact, &gcov_flat)).unwrap();
    let group = GroupParams::new(gmeans, gcov).unwrap();

    let start = match case.get("thetas_start") {
        Some(Value::Null) | None => ThetaStart::Zero,
        Some(v) => ThetaStart::Fixed(ThetaVector::from_vec(f64_array(v))),
    };
    let config = InitialThetaConfig {
        start,
        ..Default::default()
    };

    let (theta0, se0, _, _) = mirt_engine::initial_theta::initialize(&config, &group, 0)
        .expect("konfigurasi fixture harus valid");

    let expected_theta0 = f64_array(&case["theta0"]);
    let expected_se0 = f64_array(&case["se0"]);

    for i in 0..nfact {
        assert!(
            (theta0[i] - expected_theta0[i]).abs() < tol,
            "[{case_name}] theta0[{i}]: rust={}, fixture={}",
            theta0[i],
            expected_theta0[i]
        );
        assert!(
            (se0[i] - expected_se0[i]).abs() < tol,
            "[{case_name}] se0[{i}]: rust={}, fixture={}",
            se0[i],
            expected_se0[i]
        );
    }
}

#[test]
fn dim2_default_matches_fixture() {
    let fixture = load_fixture();
    assert_case_matches(&fixture, "dim2_default");
}

#[test]
fn dim2_override_matches_fixture() {
    let fixture = load_fixture();
    assert_case_matches(&fixture, "dim2_override");
}

#[test]
fn dim3_default_matches_fixture() {
    let fixture = load_fixture();
    assert_case_matches(&fixture, "dim3_default");
}

#[test]
fn dim3_override_matches_fixture() {
    let fixture = load_fixture();
    assert_case_matches(&fixture, "dim3_override");
}
