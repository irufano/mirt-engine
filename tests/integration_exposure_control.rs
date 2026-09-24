//! Integration test Fase 5: membandingkan `exposure_control` Rust vs
//! fixture `tests/fixtures/exposure_control.json`.
//!
//! Fixture dihasilkan oleh
//! `tests/fixtures/generate_r_fixtures_exposure_control.R`
//! (`Rscript tests/fixtures/generate_r_fixtures_exposure_control.R`), yang
//! menjalankan `mirtCAT` 1.14 terpasang sungguhan (R 4.6.1/mirt 1.47 — lih.
//! field `source` di file JSON) tiga cara -- lih. dokumentasi di kepala
//! skrip R tsb untuk rasional lengkap tiap bagian:
//! - `mode_detection`: deteksi mode `Design$initialize()` -- perbandingan
//!   EKSAK (deterministik, tanpa RNG).
//! - `sample_replay`: satu sesi randomesque (`exposure=3` semua posisi) --
//!   diverifikasi lewat keanggotaan top-3 (Rust menghitung ULANG skor
//!   kriteria via `rank_candidates`, bukan match RNG 1:1).
//! - `sh_simulation`: 300 peserta simulasi Sympson–Hetter -- diverifikasi
//!   lewat properti statistik yang exact secara matematis (item
//!   exposure=0.0 tidak pernah muncul; `start_item` selalu muncul tepat
//!   `n_sim` kali), bukan kecocokan frekuensi R vs Rust (RNG beda stream).

use mirt_engine::ability_estimation::estimate;
use mirt_engine::exposure_control::{select_with_exposure, ExposureControlConfig, ExposureMode};
use mirt_engine::irt::Dichotomous;
use mirt_engine::theta::InfoMatrix;
use mirt_engine::{
    rank_candidates, Criterion, EstimationConfig, GroupParams, ItemBank, ItemMeta, Method,
    PersonState, SelectionConfig, ThetaVector,
};
use nalgebra::DMatrix;
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use serde_json::Value;
use std::fs;

fn load_fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/exposure_control.json"
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

fn build_group(fixture: &Value) -> GroupParams {
    let nfact = fixture["nfact"].as_u64().unwrap() as usize;
    let gmeans = ThetaVector::from_vec(f64_array(&fixture["gmeans"]));
    let gcov_rows: Vec<Vec<f64>> = fixture["gcov"]
        .as_array()
        .unwrap()
        .iter()
        .map(f64_array)
        .collect();
    let gcov_flat: Vec<f64> = gcov_rows.iter().flatten().cloned().collect();
    let gcov = InfoMatrix::from_matrix(DMatrix::from_row_slice(nfact, nfact, &gcov_flat)).unwrap();
    GroupParams::new(gmeans, gcov).unwrap()
}

#[test]
fn mode_detection_matches_r_design_initialize() {
    let fixture = load_fixture();
    for case in fixture["mode_detection"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let input = f64_array(&case["input"]);
        let expected_type = case["exposure_type"].as_str().unwrap();
        let expected_values = f64_array(&case["exposure"]);

        let mode = ExposureMode::from_values(&input)
            .unwrap_or_else(|e| panic!("[{name}] from_values gagal: {e}"));

        match (&mode, expected_type) {
            (ExposureMode::SympsonHetter(v), "SH") => {
                for (rust_v, r_v) in v.iter().zip(expected_values.iter()) {
                    assert!(
                        (rust_v - r_v).abs() < 1e-12,
                        "[{name}] SH exposure value mismatch: rust={rust_v}, r={r_v}"
                    );
                }
            }
            (ExposureMode::Randomesque(v), "sample") => {
                let rust_ints: Vec<f64> = v.iter().map(|&x| x as f64).collect();
                assert_eq!(
                    rust_ints, expected_values,
                    "[{name}] sample exposure value mismatch"
                );
            }
            _ => panic!("[{name}] mode mismatch: rust={mode:?}, r exposure_type={expected_type}"),
        }
    }
}

#[test]
fn sample_replay_chosen_item_is_always_within_top_n_by_criterion() {
    // R: exposure=3 di semua posisi, criteria="Drule", start_item=5 (1-idx).
    // Verifikasi keanggotaan (Rust menghitung skor sendiri lewat
    // rank_candidates, lalu cek item R ada di top-3 -- BUKAN mencocokkan RNG
    // stream, keduanya independen random) -- lih. `PLAN.md` §Fase 5
    // checklist integrasi ("dicek sebagai himpunan valid").
    let fixture = load_fixture();
    let bank = build_bank(&fixture);
    let group = build_group(&fixture);
    let weights = f64_array(&fixture["weights"]);
    let n_items = fixture["n_items"].as_u64().unwrap() as usize;

    let case = &fixture["sample_replay"];
    let start_item_1indexed = case["start_item"].as_u64().unwrap() as usize;
    let exposure = case["exposure"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u32)
        .collect::<Vec<_>>();
    let items_answered_r: Vec<usize> = case["items_answered"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as usize)
        .collect();
    let thetas_history: Vec<Vec<f64>> = case["thetas_history"]
        .as_array()
        .unwrap()
        .iter()
        .map(f64_array)
        .collect();
    assert_eq!(thetas_history.len(), items_answered_r.len() + 1);

    let sel_config = SelectionConfig {
        criterion: Criterion::DRule { prior: false },
        weights: Some(weights),
        debug: false,
    };

    let mut person = PersonState::new(3);
    // Langkah 0: start_item, bypass kriteria & exposure control sama sekali
    // (lih. `findNextCATItem()`: `if (all(is.na(...)) && start)
    // return(design@start_item)`) -- dicatat apa adanya.
    person.set_theta(ThetaVector::from_vec(thetas_history[0].clone()));
    assert_eq!(items_answered_r[0], start_item_1indexed);
    person.record_answer(start_item_1indexed - 1, 1);

    for (step, &expected_item_1indexed) in items_answered_r.iter().enumerate().skip(1) {
        person.set_theta(ThetaVector::from_vec(thetas_history[step].clone()));

        let candidates: Vec<usize> = (0..n_items)
            .filter(|idx| !person.answered_items().contains(idx))
            .collect();
        let scores = rank_candidates(&sel_config, &candidates, &person, &bank, &group)
            .unwrap_or_else(|e| panic!("step {step}: rank_candidates gagal: {e}"));

        let exposure_for_position = exposure[step] as usize;
        let mut sorted = scores.clone();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let top_n: Vec<usize> = sorted
            .iter()
            .take(exposure_for_position.min(sorted.len()))
            .map(|&(idx, _)| idx)
            .collect();

        let expected_item_0indexed = expected_item_1indexed - 1;
        assert!(
            top_n.contains(&expected_item_0indexed),
            "step {step}: item R ({expected_item_1indexed}, 1-idx) tidak ada di top-{exposure_for_position} \
             hasil rank_candidates Rust ({top_n:?}, 0-idx)"
        );

        person.record_answer(expected_item_0indexed, 1);
    }
}

#[test]
fn sh_simulation_zero_exposure_item_never_selected_and_start_item_always_selected() {
    // R: exposure SH campuran (item index0 1-idx -> exposure=0.0), 300
    // peserta x 6 item, criteria="Drule", start_item=5 (1-idx). Properti
    // yang diverifikasi bersifat EXACT secara matematis (bukan toleransi
    // statistik longgar): item exposure=0.0 harus TIDAK PERNAH lolos --
    // `exposure[item] >= comp` mustahil benar untuk draw kontinu (0,1) bila
    // exposure=0.0, dan forced-accept-di-kandidat-tersisa-satu tidak
    // mengubah itu (fallback hanya soal AKSEPTANSI, bukan pengecualian item
    // dari uji itu sendiri -- item exposure=0.0 yang jadi satu-satunya
    // kandidat tersisa TETAP akan lolos comp check, exposure[item]=0.0 >=
    // comp mustahil kecuali comp=0.0 tepat, P=0). Lih. `PLAN.md` §Fase 5
    // checklist integrasi.
    let fixture = load_fixture();
    let bank = build_bank(&fixture);
    let group = build_group(&fixture);
    let weights = f64_array(&fixture["weights"]);
    let n_items = fixture["n_items"].as_u64().unwrap() as usize;

    let case = &fixture["sh_simulation"];
    let start_item_1indexed = case["start_item"].as_u64().unwrap() as usize;
    let start_item = start_item_1indexed - 1;
    let exposure_values = f64_array(&case["exposure"]);
    let max_items = case["max_items"].as_u64().unwrap() as usize;
    // Simulasi lebih sedikit peserta dari R (300) -- properti yang dites di
    // sini (exposure=0.0 tidak pernah lolos) sudah exact di N kecil
    // sekalipun; N besar cuma menambah waktu test tanpa menambah kekuatan
    // bukti untuk klaim exact ini.
    let n_sim = 100usize;

    let sel_config = SelectionConfig {
        criterion: Criterion::DRule { prior: false },
        weights: Some(weights),
        debug: false,
    };
    let exp_config = ExposureControlConfig::new(
        ExposureMode::SympsonHetter(exposure_values.clone()),
        n_items,
        false,
    )
    .unwrap();
    let est_config = EstimationConfig {
        method: Method::Map,
        ..Default::default()
    };

    let mut item_counts = vec![0usize; n_items];
    let mut rng = StdRng::seed_from_u64(12345);

    for sim in 0..n_sim {
        // Pola respons tetap per peserta (mirip `local_pattern` R): respons
        // item ke-i sama terlepas dari urutan penyajian.
        let mut resp_rng = StdRng::seed_from_u64(1000 + sim as u64);
        let pattern: Vec<usize> = (0..n_items).map(|_| resp_rng.gen_range(0..2)).collect();

        let mut person = PersonState::new(3);
        let mut valid_items = vec![true; n_items];

        person.record_answer(start_item, pattern[start_item]);
        item_counts[start_item] += 1;

        for step in 1..max_items {
            let ((theta, se), _) = estimate(&est_config, &person, &bank, &group)
                .unwrap_or_else(|e| panic!("sim {sim} step {step}: estimate gagal: {e}"));
            person.set_estimate(theta, se);

            let candidates: Vec<usize> = (0..n_items)
                .filter(|idx| !person.answered_items().contains(idx) && valid_items[*idx])
                .collect();
            if candidates.is_empty() {
                break;
            }
            let scores = rank_candidates(&sel_config, &candidates, &person, &bank, &group)
                .unwrap_or_else(|e| panic!("sim {sim} step {step}: rank_candidates gagal: {e}"));
            let (chosen, _) =
                select_with_exposure(&exp_config, &scores, step, &mut valid_items, &mut rng)
                    .unwrap_or_else(|e| {
                        panic!("sim {sim} step {step}: select_with_exposure gagal: {e}")
                    });

            person.record_answer(chosen, pattern[chosen]);
            item_counts[chosen] += 1;
        }
    }

    assert_eq!(
        item_counts[0], 0,
        "item index 0 (R item1, exposure=0.0) TIDAK BOLEH pernah lolos SH -- \
         hitung Rust: {item_counts:?}"
    );
    assert_eq!(
        item_counts[start_item], n_sim,
        "start_item harus selalu terpilih tepat n_sim kali (bypass exposure control \
         sepenuhnya, sama seperti R)"
    );

    // Item-item dengan exposure=1.0 (index 1,3,4,6,8,9 0-based) harus tetap
    // bisa lolos (exposure=1.0 artinya tanpa pembatasan efektif -- selain
    // start_item, minimal SATU item exposure=1.0 lain harus pernah
    // terpilih, membuktikan SH tidak salah menahan seluruh kandidat).
    let full_exposure_indices: Vec<usize> = exposure_values
        .iter()
        .enumerate()
        .filter(|&(idx, &v)| idx != start_item && v >= 1.0)
        .map(|(idx, _)| idx)
        .collect();
    assert!(
        full_exposure_indices.iter().any(|&idx| item_counts[idx] > 0),
        "setidaknya satu item exposure=1.0 (selain start_item) harus pernah terpilih: {item_counts:?}"
    );
}
