//! Integration test Fase 2: membandingkan `item_selection::select_next_item()`
//! Rust vs fixture `tests/fixtures/item_selection.json`.
//!
//! Fixture dihasilkan oleh
//! `tests/fixtures/generate_r_fixtures_item_selection.R`
//! (`Rscript tests/fixtures/generate_r_fixtures_item_selection.R`), yang
//! menjalankan `mirtCAT()` sungguhan (R 4.6.1/mirt 1.47/mirtCAT 1.14 — lih.
//! field `source` di file JSON) dua cara:
//! - `step0`: nilai kriteria mentah seluruh kandidat sebelum item pertama
//!   dijawab (via `computeCriteria()`).
//! - `sequence`: urutan lengkap item yang diadministrasikan + `thetas_history`
//!   dari simulasi end-to-end 6 langkah, dipakai untuk "replay": inject
//!   theta R persis di tiap langkah (Fase 3/estimasi ulang belum ada di
//!   Rust), lalu verifikasi item yang dipilih Rust identik dengan R.

use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::theta::InfoMatrix;
use mirtcat_engine::{
    select_next_item, Criterion, GroupParams, ItemBank, ItemMeta, PersonState, SelectionConfig,
    ThetaVector,
};
use nalgebra::DMatrix;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::Value;
use std::fs;

fn load_fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/item_selection.json"
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

fn criterion_for(name: &str, kl_delta: f64) -> Criterion {
    match name {
        "Drule" => Criterion::DRule { prior: false },
        "DPrule" => Criterion::DRule { prior: true },
        "Trule" => Criterion::TRule { prior: false },
        "TPrule" => Criterion::TRule { prior: true },
        "Arule" => Criterion::ARule { prior: false },
        "APrule" => Criterion::ARule { prior: true },
        "Erule" => Criterion::ERule { prior: false },
        "EPrule" => Criterion::ERule { prior: true },
        "Wrule" => Criterion::WRule { prior: false },
        "WPrule" => Criterion::WRule { prior: true },
        "KL" => Criterion::Kl { delta: kl_delta },
        "KLn" => Criterion::KlN { delta: kl_delta },
        "seq" => Criterion::Seq,
        other => panic!("kriteria fixture tidak dikenal: {other}"),
    }
}

const STEP0_CRITERIA: &[&str] = &[
    "Drule", "DPrule", "Trule", "TPrule", "Arule", "APrule", "Erule", "EPrule", "Wrule", "WPrule",
    "KL", "KLn",
];

#[test]
fn step0_candidate_scores_match_r_for_all_criteria() {
    let fixture = load_fixture();
    let tol = fixture["tolerance"].as_f64().unwrap();
    let bank = build_bank(&fixture);
    let group = build_group(&fixture);
    let weights = f64_array(&fixture["weights"]);
    let kl_delta = fixture["kl_delta"].as_f64().unwrap();
    let n_items = fixture["n_items"].as_u64().unwrap() as usize;

    for &crit_name in STEP0_CRITERIA {
        let case = &fixture["step0"][crit_name];
        let expected_values = f64_array(&case["candidate_values"]);
        let expected_items = case["candidate_items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect::<Vec<_>>();
        assert_eq!(expected_items, (1..=n_items).collect::<Vec<_>>());

        let person = PersonState::new(3);
        let config = SelectionConfig {
            criterion: criterion_for(crit_name, kl_delta),
            weights: Some(weights.clone()),
            debug: true,
        };
        let candidates: Vec<usize> = (0..n_items).collect();
        let mut rng = StdRng::seed_from_u64(0);
        let (chosen, debug) =
            select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)
                .unwrap_or_else(|e| panic!("[{crit_name}] select_next_item gagal: {e}"));
        let debug = debug.expect("debug=true harus Some");

        let mut scores_by_index = vec![0.0_f64; n_items];
        for (idx, val) in &debug.candidate_scores {
            scores_by_index[*idx] = *val;
        }

        for i in 0..n_items {
            let rust_val = scores_by_index[i];
            let r_val = expected_values[i];
            assert!(
                (rust_val - r_val).abs() < tol.max(1e-9),
                "[{crit_name}] candidate item {} (0-idx {i}): rust={rust_val}, r={r_val}",
                i + 1
            );
        }

        // `Drule`/`Erule` TANPA prior, di 0 item terjawab, secara matematis
        // degenerate untuk nfact=3: info Fisher 1 kandidat dikotomus selalu
        // rank 1 < 3, sehingga det() dan eigenvalue minimum PERSIS 0 untuk
        // SEMUA kandidat -- argmax di antara noise floating-point ~1e-16..
        // 1e-34 (arah beda antara R vs nalgebra) bukan perilaku yang
        // well-defined untuk dibandingkan; nilai mentahnya (dicek di atas,
        // sama-sama ~0) sudah membuktikan formulanya benar. `DPrule`/
        // `EPrule` (yang menambah `inverse(gcov)` -- selalu full rank) TIDAK
        // kena masalah ini, jadi argmax-nya tetap dicek.
        if crit_name == "Drule" || crit_name == "Erule" {
            continue;
        }

        // argmax Rust harus sama dengan argmax nilai R (index dengan nilai
        // maksimum, tie-break ke yang pertama -- sama seperti `which.max` R).
        let expected_chosen0 = expected_values
            .iter()
            .enumerate()
            .fold((0usize, f64::NEG_INFINITY), |best, (i, &v)| {
                if v > best.1 {
                    (i, v)
                } else {
                    best
                }
            })
            .0;
        assert_eq!(
            chosen, expected_chosen0,
            "[{crit_name}] item terpilih beda dari argmax fixture R"
        );
    }
}

// `Drule`/`Erule` (tanpa prior) sengaja tidak diikutkan di sini: langkah
// pertamanya degenerate secara matematis (lih. komentar di
// `step0_candidate_scores_match_r_for_all_criteria`), jadi item pertama
// yang "menang" adalah noise floating-point, bukan sinyal -- begitu item
// pertama beda, seluruh urutan berikutnya ikut beda walau formulanya
// benar. `DPrule`/`EPrule` (full rank dari awal berkat prior) mewakili
// keluarga D/E-rule di sini.
const SEQUENCE_CRITERIA: &[&str] = &[
    "DPrule", "Trule", "TPrule", "APrule", "Arule", "EPrule", "Wrule", "WPrule", "KL", "KLn", "seq",
];

#[test]
fn replayed_sequence_matches_r_administered_item_order() {
    let fixture = load_fixture();
    let bank = build_bank(&fixture);
    let group = build_group(&fixture);
    let weights = f64_array(&fixture["weights"]);
    let kl_delta = fixture["kl_delta"].as_f64().unwrap();
    let n_items = fixture["n_items"].as_u64().unwrap() as usize;

    for &crit_name in SEQUENCE_CRITERIA {
        let case = &fixture["sequence"][crit_name];
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

        let config = SelectionConfig {
            criterion: criterion_for(crit_name, kl_delta),
            weights: Some(weights.clone()),
            debug: false,
        };
        let mut person = PersonState::new(3);
        let mut rng = StdRng::seed_from_u64(0);

        // Langkah 0 (item pertama) SELALU `design@start_item` di mirtCAT
        // sungguhan, bukan hasil kriteria -- `findNextCATItem()` memotong ke
        // `design@start_item` lewat argumen `start=TRUE` saat belum ada
        // respons sama sekali (beda dengan `computeCriteria()`, yang
        // memanggil `start=FALSE` -- itu sebabnya `step0` di atas justru
        // bisa mengetes kriteria murni, sedangkan urutan penuh di sini
        // TIDAK). Jadi item pertama langsung dicatat apa adanya (bukan
        // hasil `select_next_item`), verifikasi kriteria dimulai dari
        // langkah kedua, tempat R sungguhan mulai memakai kriteria.
        person.set_theta(ThetaVector::from_vec(thetas_history[0].clone()));
        person.record_answer(items_answered_r[0] - 1);

        for (step, &expected_item_1indexed) in items_answered_r.iter().enumerate().skip(1) {
            // Inject theta R yang sesungguhnya dipakai R di langkah ini
            // (menggantikan estimasi ulang Fase 3 yang belum ada).
            person.set_theta(ThetaVector::from_vec(thetas_history[step].clone()));

            let candidates: Vec<usize> = (0..n_items)
                .filter(|idx| !person.answered_items().contains(idx))
                .collect();

            let (chosen, _) =
                select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)
                    .unwrap_or_else(|e| panic!("[{crit_name}] step {step}: select gagal: {e}"));

            assert_eq!(
                chosen + 1,
                expected_item_1indexed,
                "[{crit_name}] step {step}: rust memilih item {} (1-idx), R memilih {}",
                chosen + 1,
                expected_item_1indexed
            );

            person.record_answer(chosen);
        }
    }
}
