//! Integration test Fase 8: jalankan `McatSession` SUNGGUHAN (bukan replay
//! seperti `tests/integration_final_result.rs`) melawan fixture
//! `tests/fixtures/session.json`, membuktikan pipa `next_item()`/
//! `submit_response()` -- item selection (`DPrule`) + content balancing (2
//! kategori) + exposure control (Sympson-Hetter) + ability estimation
//! (`MAP`) + stopping rule (`min_SEM`) digabung dalam satu sesi -- identik
//! dengan `mirtCAT()` 1.14 sungguhan.
//!
//! Fixture dihasilkan `tests/fixtures/generate_r_fixtures_session.R`
//! (`Rscript tests/fixtures/generate_r_fixtures_session.R`). Skenario
//! SENGAJA dibuat deterministik penuh (exposure Sympson-Hetter dengan
//! SELURUH nilai `1.0` -- selalu diterima di percobaan pertama, lih.
//! catatan modul skrip R) supaya `items_answered` bisa dibandingkan EKSAK
//! item demi item TANPA perlu menyamakan RNG stream Rust vs R (yang
//! berbeda generator & biasanya mustahil dicocokkan 1:1, lih. catatan
//! `PLAN.md` §Fase 5).

use std::collections::HashMap;

use mirtcat_engine::content_balancing::{ContentBalancingConfig, ContentCategory};
use mirtcat_engine::exposure_control::{ExposureControlConfig, ExposureMode};
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::item_selection::Criterion;
use mirtcat_engine::stopping_rule::StoppingConfig;
use mirtcat_engine::theta::InfoMatrix;
use mirtcat_engine::{
    EstimationConfig, GroupParams, ItemBank, ItemMeta, McatSession, Method, SelectionConfig,
    SessionConfig, ThetaVector,
};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::Value;
use std::fs;

fn load_fixture() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/session.json");
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

fn assert_slice_close(actual: &[f64], expected: &[f64], tol: f64, context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}: panjang beda");
    for i in 0..expected.len() {
        assert!(
            (actual[i] - expected[i]).abs() < tol,
            "{context}: dim {i} actual={} expected={}",
            actual[i],
            expected[i]
        );
    }
}

#[test]
fn mcat_session_full_pipeline_matches_r_mirtcat_item_sequence_and_final_result() {
    let fixture = load_fixture();
    let tol = fixture["tolerance"].as_f64().unwrap();
    let n_items = fixture["n_items"].as_u64().unwrap() as usize;

    let a1 = f64_array(&fixture["item_params"]["a1"]);
    let a2 = f64_array(&fixture["item_params"]["a2"]);
    let d = f64_array(&fixture["item_params"]["d"]);
    let mut bank = ItemBank::new();
    for i in 0..n_items {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a1[i], a2[i]], d[i]).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }

    let gcov_rows = fixture["gcov"].as_array().unwrap();
    let gcov = InfoMatrix::from_matrix(nalgebra::DMatrix::from_row_slice(
        2,
        2,
        &[
            gcov_rows[0][0].as_f64().unwrap(),
            gcov_rows[0][1].as_f64().unwrap(),
            gcov_rows[1][0].as_f64().unwrap(),
            gcov_rows[1][1].as_f64().unwrap(),
        ],
    ))
    .unwrap();
    let group = GroupParams::new(ThetaVector::zeros(2), gcov).unwrap();

    let content: Vec<ContentCategory> = fixture["content"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| ContentCategory::from(v.as_str().unwrap()))
        .collect();
    let mut target_prop = HashMap::new();
    for (key, val) in fixture["content_prop"].as_object().unwrap() {
        target_prop.insert(ContentCategory::from(key.as_str()), val.as_f64().unwrap());
    }
    let content_balancing = ContentBalancingConfig::new(content, target_prop, false).unwrap();

    let exposure = f64_array(&fixture["exposure"]);
    let exposure_control =
        ExposureControlConfig::new(ExposureMode::SympsonHetter(exposure), n_items, false).unwrap();

    let start_item = fixture["start_item"].as_u64().unwrap() as usize - 1;
    let min_items = fixture["min_items"].as_u64().unwrap() as usize;
    let max_items = fixture["max_items"].as_u64().unwrap() as usize;
    let min_sem = fixture["min_sem"].as_f64().unwrap();

    let config = SessionConfig {
        selection: SelectionConfig {
            criterion: Criterion::DRule { prior: true },
            ..Default::default()
        },
        estimation: EstimationConfig {
            method: Method::Map,
            ..Default::default()
        },
        stopping: StoppingConfig {
            min_items,
            max_items,
            min_sem: vec![min_sem],
            ..Default::default()
        },
        exposure: Some(exposure_control),
        content_balancing: Some(content_balancing),
        start_item,
        ..Default::default()
    };

    let mut session = McatSession::new(config, &bank, &group, 0).unwrap();
    // RNG diberi seed tetap murni untuk reproducibility test -- TIDAK
    // memengaruhi item yang dipilih (exposure=1.0 semua posisi selalu
    // diterima di percobaan pertama, lih. catatan modul).
    let mut rng = StdRng::seed_from_u64(0);

    let local_pattern: Vec<usize> = fixture["local_pattern"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();

    let mut items_answered_0indexed = Vec::new();
    loop {
        let item = session.next_item(&bank, &group, &mut rng).unwrap();
        items_answered_0indexed.push(item);
        let response = local_pattern[item];
        let decision = session
            .submit_response(item, response, &bank, &group, None)
            .unwrap();
        if decision.stop_now {
            break;
        }
    }

    let expected_items_answered: Vec<usize> = fixture["result"]["items_answered"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize - 1)
        .collect();
    assert_eq!(
        items_answered_0indexed, expected_items_answered,
        "urutan item McatSession Rust harus identik dengan mirtCAT() R"
    );

    assert!(
        session.last_stop_decision().terminated_early,
        "fixture sengaja dipilih agar berhenti karena presisi (bukan max_items) -- lih. \
         catatan skrip R"
    );

    let result = session.finalize();
    assert_slice_close(
        result.thetas.as_slice(),
        &f64_array(&fixture["result"]["thetas"]),
        tol,
        "thetas",
    );
    assert_slice_close(
        result.se_thetas.as_slice(),
        &f64_array(&fixture["result"]["se_thetas"]),
        tol,
        "se_thetas",
    );

    let expected_responses: Vec<Option<i32>> = expected_items_answered
        .iter()
        .map(|&idx| Some(local_pattern[idx] as i32))
        .collect();
    assert_eq!(result.responses, expected_responses);
}
