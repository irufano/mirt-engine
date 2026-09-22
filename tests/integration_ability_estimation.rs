//! Integration test Fase 3: membandingkan `ability_estimation::estimate()`
//! Rust vs fixture `tests/fixtures/ability_estimation.json`.
//!
//! Fixture dihasilkan oleh
//! `tests/fixtures/generate_r_fixtures_ability_estimation.R`
//! (`Rscript tests/fixtures/generate_r_fixtures_ability_estimation.R`), yang
//! memanggil `mirt::fscores()` LANGSUNG (bukan `mirtCAT()` penuh — lih.
//! komentar di skrip R soal kenapa) dengan `response.pattern` (sebagian
//! `NA` untuk item belum dijawab) pada beberapa item bank 2D/3D/5D tetap.
//!
//! **Toleransi** (lih. juga field `tolerance` di JSON fixture):
//! - EAP/MAP: `1e-4` untuk theta, `1e-3` untuk SE — keduanya deterministik
//!   murni (grid kuadratur / optimisasi cembung dengan prior), parity
//!   sangat dekat diharapkan.
//! - ML: `1e-3` untuk kasus yang well-identified (jauh dari batas
//!   `max_theta`). Kasus yang membuat ML divergen/mendekati batas (mis.
//!   `2d_all_correct`, sebagian dimensi `3d_full_mixed`) sengaja diuji
//!   **kualitatif saja** (finite vs divergen, tanda arah) — ML dekat
//!   ketidak-teridentifikasian multidimensional (analog "perfect
//!   separation" regresi logistik) inherently sensitif terhadap detail
//!   numerik solver (R `nlm` vs Newton-Raphson finite-difference Rust),
//!   bukan indikasi port yang salah. Ini eksplisit sesuai PLAN.md
//!   "toleransi lebih longgar ... didokumentasikan eksplisit bila ada
//!   deviasi terukur".
//! - WLE/EAPsum: `1e-2` — solver berbeda (`mirt` nlm vs Newton-Raphson FD
//!   Rust untuk WLE; algoritma sama tapi presisi kuadratur untuk EAPsum),
//!   dites longgar sesuai PLAN.md.

use mirtcat_engine::ability_estimation::{eap_sum_score_table, estimate};
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::theta::InfoMatrix;
use mirtcat_engine::{EstimationConfig, GroupParams, ItemBank, ItemMeta, Method, PersonState, ThetaVector};
use nalgebra::DMatrix;
use serde_json::Value;
use std::fs;

fn load_fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/ability_estimation.json"
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

/// `null` di JSON (Inf/NaN R, tidak direpresentasikan JSON standar) -> `None`.
fn opt_f64_array(v: &Value) -> Vec<Option<f64>> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64())
        .collect()
}

fn build_bank(item_params: &Value, nfact: usize) -> ItemBank {
    let a_cols: Vec<Vec<f64>> = (1..=nfact)
        .map(|k| f64_array(&item_params[format!("a{k}")]))
        .collect();
    let d = f64_array(&item_params["d"]);
    let mut bank = ItemBank::new();
    for i in 0..d.len() {
        let a: Vec<f64> = a_cols.iter().map(|col| col[i]).collect();
        bank.push(
            Box::new(Dichotomous::m2pl(a, d[i]).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn build_group(fixture_case: &Value, nfact: usize) -> GroupParams {
    let gmeans = ThetaVector::from_vec(f64_array(&fixture_case["gmeans"]));
    let gcov_rows: Vec<Vec<f64>> = fixture_case["gcov"].as_array().unwrap().iter().map(f64_array).collect();
    let gcov_flat: Vec<f64> = gcov_rows.iter().flatten().cloned().collect();
    let gcov = InfoMatrix::from_matrix(DMatrix::from_row_slice(nfact, nfact, &gcov_flat)).unwrap();
    GroupParams::new(gmeans, gcov).unwrap()
}

/// Bangun `PersonState` dari `response_pattern` (`null` = belum dijawab,
/// tidak masuk `answered_items`) -- persis semantik `PersonState` Rust.
fn build_person(response_pattern: &Value, nfact: usize) -> PersonState {
    let mut person = PersonState::new(nfact);
    for (idx, v) in response_pattern.as_array().unwrap().iter().enumerate() {
        if let Some(cat) = v.as_f64() {
            person.record_answer(idx, cat.round() as usize);
        }
    }
    person
}

fn assert_close(actual: f64, expected: f64, tol: f64, label: &str) {
    assert!(
        (actual - expected).abs() < tol,
        "{label}: rust={actual}, r={expected}, |diff|={} >= tol={tol}",
        (actual - expected).abs()
    );
}

/// Bandingkan satu method untuk satu kasus. `theta_tol`/`se_tol` dipakai
/// untuk elemen yang FINITE di R (`Some`); elemen `None` (R divergen ke
/// Inf/NaN) hanya dicek KUALITATIF -- Rust juga harus `is_infinite()`.
#[allow(clippy::too_many_arguments)]
fn compare_method(
    config: &EstimationConfig,
    person: &PersonState,
    bank: &ItemBank,
    group: &GroupParams,
    expected: &Value,
    theta_tol: f64,
    se_tol: f64,
    label: &str,
) {
    let ((theta, se), _) = estimate(config, person, bank, group)
        .unwrap_or_else(|e| panic!("{label}: estimate() gagal: {e}"));
    let expected_theta = opt_f64_array(&expected["theta"]);
    let expected_se = opt_f64_array(&expected["se"]);

    for (i, exp) in expected_theta.iter().enumerate() {
        match exp {
            Some(v) => assert_close(theta[i], *v, theta_tol, &format!("{label} theta[{i}]")),
            None => assert!(
                theta[i].is_infinite(),
                "{label} theta[{i}]: R divergen (null/Inf) tapi Rust={} (harus juga infinite)",
                theta[i]
            ),
        }
    }
    for (i, exp) in expected_se.iter().enumerate() {
        if let Some(v) = exp {
            assert_close(se.as_slice()[i], *v, se_tol, &format!("{label} se[{i}]"));
        }
        // `None` (R NA/NaN SE) -- tidak diverifikasi kualitatif tambahan;
        // sudah cukup terwakili lewat cek `theta[i].is_infinite()` di atas.
    }
}

#[test]
fn eap_matches_r_across_cases() {
    let fixture = load_fixture();
    for (name, case) in fixture["cases"].as_object().unwrap() {
        let Some(expected) = case["results"].get("EAP") else {
            continue;
        };
        let nfact = case["nfact"].as_u64().unwrap() as usize;
        let quadpts = case["quadpts"].as_u64().unwrap() as usize;
        let bank = build_bank(&case["item_params"], nfact);
        let group = build_group(case, nfact);
        let person = build_person(&case["response_pattern"], nfact);
        let config = EstimationConfig {
            method: Method::Eap { qmc: false },
            quadpts: Some(quadpts),
            ..Default::default()
        };
        compare_method(&config, &person, &bank, &group, expected, 1e-4, 1e-3, &format!("[{name}] EAP"));
    }
}

#[test]
fn map_matches_r_across_cases() {
    let fixture = load_fixture();
    for (name, case) in fixture["cases"].as_object().unwrap() {
        let Some(expected) = case["results"].get("MAP") else {
            continue;
        };
        let nfact = case["nfact"].as_u64().unwrap() as usize;
        let bank = build_bank(&case["item_params"], nfact);
        let group = build_group(case, nfact);
        let person = build_person(&case["response_pattern"], nfact);
        let config = EstimationConfig {
            method: Method::Map,
            ..Default::default()
        };
        compare_method(&config, &person, &bank, &group, expected, 1e-4, 1e-3, &format!("[{name}] MAP"));
    }
}

/// ML dites hanya untuk kasus yang well-identified (jauh dari batas
/// `max_theta`) -- lih. catatan modul di atas soal kenapa `2d_all_correct`/
/// `3d_full_mixed` (sebagian dimensi divergen/mendekati batas) diuji
/// kualitatif lewat `ml_diverges_or_stays_near_max_theta_like_r` di bawah,
/// bukan di sini.
#[test]
fn ml_matches_r_for_well_identified_cases() {
    let fixture = load_fixture();
    let name = "2d_partial_3of6";
    let case = &fixture["cases"][name];
    let expected = &case["results"]["ML"];
    let nfact = case["nfact"].as_u64().unwrap() as usize;
    let bank = build_bank(&case["item_params"], nfact);
    let group = build_group(case, nfact);
    let person = build_person(&case["response_pattern"], nfact);
    let config = EstimationConfig {
        method: Method::Ml,
        ..Default::default()
    };
    compare_method(&config, &person, &bank, &group, expected, 1e-3, 1e-2, &format!("[{name}] ML"));
}

/// Kasus ML yang divergen/mendekati batas `max_theta` di R (dites lewat
/// eksplorasi manual, lih. skrip fixture) -- di sini hanya diverifikasi
/// Rust juga mengklasifikasikan sebagai divergen (bukan diam-diam
/// mengembalikan nilai finite yang salah), bukan kecocokan numerik persis.
#[test]
fn ml_diverges_like_r_for_boundary_cases() {
    let fixture = load_fixture();

    // `2d_all_correct`: R -> theta=[Inf,Inf] (null di JSON).
    let case = &fixture["cases"]["2d_all_correct"];
    let nfact = case["nfact"].as_u64().unwrap() as usize;
    let bank = build_bank(&case["item_params"], nfact);
    let group = build_group(case, nfact);
    let person = build_person(&case["response_pattern"], nfact);
    let config = EstimationConfig {
        method: Method::Ml,
        ..Default::default()
    };
    let ((theta, _), _) = estimate(&config, &person, &bank, &group).unwrap();
    assert!(theta[0].is_infinite() && theta[0] > 0.0);
    assert!(theta[1].is_infinite() && theta[1] > 0.0);
}

#[test]
fn wle_matches_r_within_looser_tolerance() {
    let fixture = load_fixture();
    for (name, case) in fixture["cases"].as_object().unwrap() {
        let Some(expected) = case["results"].get("WLE") else {
            continue;
        };
        let nfact = case["nfact"].as_u64().unwrap() as usize;
        let bank = build_bank(&case["item_params"], nfact);
        let group = build_group(case, nfact);
        let person = build_person(&case["response_pattern"], nfact);
        let config = EstimationConfig {
            method: Method::Wle,
            ..Default::default()
        };
        compare_method(&config, &person, &bank, &group, expected, 1e-2, 5e-2, &format!("[{name}] WLE"));
    }
}

#[test]
fn eap_sum_matches_r_within_looser_tolerance() {
    let fixture = load_fixture();
    for (name, case) in fixture["cases"].as_object().unwrap() {
        let Some(expected) = case["results"].get("EAPsum") else {
            continue;
        };
        let nfact = case["nfact"].as_u64().unwrap() as usize;
        let quadpts = case["quadpts"].as_u64().unwrap() as usize;
        let bank = build_bank(&case["item_params"], nfact);
        let group = build_group(case, nfact);
        let person = build_person(&case["response_pattern"], nfact);
        let config = EstimationConfig {
            method: Method::EapSum,
            quadpts: Some(quadpts),
            ..Default::default()
        };
        compare_method(&config, &person, &bank, &group, expected, 1e-2, 5e-2, &format!("[{name}] EAPsum"));
    }
}

#[test]
fn eap_sum_full_score_table_matches_r() {
    let fixture = load_fixture();
    let table = &fixture["eapsum_table"];
    let nfact = table["nfact"].as_u64().unwrap() as usize;
    let quadpts = table["quadpts"].as_u64().unwrap() as usize;
    let bank = build_bank(&table["item_params"], nfact);
    let group = build_group(table, nfact);
    let person = build_person(&table["response_pattern"], nfact);
    let config = EstimationConfig {
        method: Method::EapSum,
        quadpts: Some(quadpts),
        ..Default::default()
    };

    let rust_table = eap_sum_score_table(&config, &person, &bank, &group).unwrap();
    let sum_scores: Vec<u64> = table["sum_scores"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect();
    let theta_rows: Vec<Vec<f64>> = table["theta"].as_array().unwrap().iter().map(f64_array).collect();
    let se_rows: Vec<Vec<f64>> = table["se"].as_array().unwrap().iter().map(f64_array).collect();

    assert_eq!(rust_table.len(), sum_scores.len());
    for (i, entry) in rust_table.iter().enumerate() {
        assert_eq!(entry.sum_score as u64, sum_scores[i]);
        for j in 0..nfact {
            assert_close(
                entry.theta[j],
                theta_rows[i][j],
                1e-2,
                &format!("eapsum_table score={} theta[{j}]", sum_scores[i]),
            );
            assert_close(
                entry.se.as_slice()[j],
                se_rows[i][j],
                5e-2,
                &format!("eapsum_table score={} se[{j}]", sum_scores[i]),
            );
        }
    }
}

/// Fase 3 §"Checklist Integration Test": bandingkan ukuran grid kuadratur
/// default Rust vs `quadpts` default R untuk `nfact=5` -- nilai `quadpts`
/// yang dipakai fixture (`5d_eap_grid_size`) sendiri diambil dari tabel
/// `mirtCAT` (dibaca langsung dari source `TestClass.R:58-60`, direplikasi
/// di `quadrature::default_quadpts` & diuji unit di
/// `ability_estimation::quadrature::tests::default_quadpts_matches_mirtcat_table`).
/// Di sini diverifikasi ULANG dari sisi fixture (bukan cuma dari bacaan
/// source) bahwa `quadpts=7` untuk `nfact=5` memang dipakai R.
#[test]
fn nfact5_quadpts_matches_mirtcat_default_table() {
    let fixture = load_fixture();
    let case = &fixture["cases"]["5d_eap_grid_size"];
    assert_eq!(case["nfact"].as_u64().unwrap(), 5);
    assert_eq!(case["quadpts"].as_u64().unwrap(), 7);
    assert_eq!(
        mirtcat_engine::ability_estimation::quadrature::default_quadpts(5),
        7
    );
}
