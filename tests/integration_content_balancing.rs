//! Integration test Fase 6: membandingkan `content_balancing` Rust vs
//! fixture `tests/fixtures/content_balancing.json`.
//!
//! Fixture dihasilkan oleh
//! `tests/fixtures/generate_r_fixtures_content_balancing.R`
//! (`Rscript tests/fixtures/generate_r_fixtures_content_balancing.R`), yang
//! menjalankan `mirtCAT` 1.14 terpasang sungguhan (R 4.6.1/mirt 1.47 — lih.
//! field `source` di file JSON) dua skenario, KEDUANYA deterministik penuh
//! (kriteria `Drule`, `exposure_type="none"`, tanpa RNG selain
//! `local_pattern` yang tetap) — beda dari fixture Fase 5 yang perlu
//! verifikasi statistik/keanggotaan karena melibatkan exposure control
//! (RNG). Di sini `items_answered` R vs Rust dicocokkan EKSAK, item demi
//! item, lewat replay `narrow_candidates()` -> `rank_candidates()` ->
//! `argmax()` di tiap langkah, dengan `theta` diinjeksi langsung dari
//! `thetas_history` fixture (bukan diestimasi ulang Rust) — sama seperti
//! pola replay `sample_replay` `tests/integration_exposure_control.rs`.
//!
//! - `main`: `content_prop` 50/50 dua kategori, item cukup di kedua
//!   kategori sepanjang sesi -- membuktikan alternasi kategori mengikuti
//!   defisit terbesar (dengan tie-break alfabetis) persis sama dengan R.
//! - `fallback`: satu kategori cuma berisi 1 item -- begitu item itu
//!   terjawab, kategori itu tetap "menang" defisit di langkah-langkah
//!   berikutnya tapi TIDAK ADA kandidat tersisa di kategori itu --
//!   membuktikan fallback ke pool penuh (bukan error) terjadi di langkah
//!   yang SAMA PERSIS seperti R.

use mirtcat_engine::content_balancing::{narrow_candidates, ContentBalancingConfig};
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::{
    select_next_item, Criterion, GroupParams, ItemBank, ItemMeta, PersonState, SelectionConfig,
    ThetaVector,
};
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

fn load_fixture() -> Value {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/content_balancing.json"
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

fn build_bank(case: &Value) -> ItemBank {
    let a1 = f64_array(&case["item_params"]["a1"]);
    let a2 = f64_array(&case["item_params"]["a2"]);
    let d = f64_array(&case["item_params"]["d"]);
    let mut bank = ItemBank::new();
    for i in 0..a1.len() {
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a1[i], a2[i]], d[i]).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

/// Replay satu skenario fixture, mengembalikan `items_answered` (0-indexed)
/// yang dipilih Rust di setiap langkah lewat
/// `narrow_candidates -> rank_candidates -> argmax`.
fn replay_case(case: &Value) -> (Vec<usize>, Vec<usize>) {
    let bank = build_bank(case);
    let group = GroupParams::standard_normal(2);

    let content: Vec<String> = case["content"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let categories = content
        .iter()
        .map(|s| s.as_str().into())
        .collect::<Vec<_>>();

    let content_prop_obj = case["content_prop"].as_object().unwrap();
    let target_prop: HashMap<_, _> = content_prop_obj
        .iter()
        .map(|(k, v)| (k.as_str().into(), v.as_f64().unwrap()))
        .collect();

    let cb_config = ContentBalancingConfig::new(categories, target_prop, false).unwrap();
    let sel_config = SelectionConfig {
        criterion: Criterion::DRule { prior: false },
        ..Default::default()
    };

    let n_items = case["n_items"].as_u64().unwrap() as usize;
    let start_item_1indexed = case["start_item"].as_u64().unwrap() as usize;
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

    let mut person = PersonState::new(2);
    let mut rust_chosen = Vec::with_capacity(items_answered_r.len());
    // `rng` tidak pernah benar-benar dipakai: `Criterion::DRule` deterministik
    // (tidak menarik dari `rng` sama sekali) -- diperlukan cuma karena
    // `select_next_item` mensyaratkannya di signature.
    let mut rng = StdRng::seed_from_u64(0);

    // Langkah 0: start_item, bypass content balancing & kriteria sepenuhnya
    // (lih. `findNextCATItem()`: `if (all(is.na(...)) && start)
    // return(design@start_item)`), dicatat apa adanya.
    person.set_theta(ThetaVector::from_vec(thetas_history[0].clone()));
    assert_eq!(items_answered_r[0], start_item_1indexed);
    person.record_answer(start_item_1indexed - 1, 1);
    rust_chosen.push(start_item_1indexed - 1);

    for (step, &expected_item_1indexed) in items_answered_r.iter().enumerate().skip(1) {
        person.set_theta(ThetaVector::from_vec(thetas_history[step].clone()));

        let candidates: Vec<usize> = (0..n_items)
            .filter(|idx| !person.answered_items().contains(idx))
            .collect();

        let (narrowed, _) = narrow_candidates(&cb_config, person.answered_items(), &candidates)
            .unwrap_or_else(|e| panic!("step {step}: narrow_candidates gagal: {e}"));
        let (chosen, _) =
            select_next_item(&sel_config, &narrowed, &person, &bank, &group, &mut rng)
                .unwrap_or_else(|e| panic!("step {step}: select_next_item gagal: {e}"));

        rust_chosen.push(chosen);
        assert_eq!(
            chosen + 1,
            expected_item_1indexed,
            "step {step}: item terpilih Rust ({}, 0-idx) != item R ({}, 1-idx)",
            chosen,
            expected_item_1indexed
        );

        person.record_answer(chosen, 1);
    }

    let expected_0indexed: Vec<usize> = items_answered_r.iter().map(|&i| i - 1).collect();
    (rust_chosen, expected_0indexed)
}

#[test]
fn main_scenario_matches_r_item_sequence_exactly() {
    let fixture = load_fixture();
    let (rust_chosen, expected) = replay_case(&fixture["main"]);
    assert_eq!(rust_chosen, expected);
}

#[test]
fn fallback_scenario_matches_r_item_sequence_exactly_and_never_errors() {
    let fixture = load_fixture();
    let (rust_chosen, expected) = replay_case(&fixture["fallback"]);
    assert_eq!(rust_chosen, expected);
}

#[test]
fn fallback_scenario_actually_exercises_the_fallback_path() {
    // Sanity check terpisah dari kecocokan urutan: buktikan skenario
    // "fallback" MEMANG memicu jalur fallback (kategori "Geometri" habis
    // sebelum sesi selesai), bukan cuma kebetulan cocok tanpa pernah
    // menyentuh jalur itu.
    let fixture = load_fixture();
    let case = &fixture["fallback"];
    let bank = build_bank(case);
    let group = GroupParams::standard_normal(2);

    let content: Vec<String> = case["content"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    let categories: Vec<_> = content.iter().map(|s| s.as_str().into()).collect();
    let content_prop_obj = case["content_prop"].as_object().unwrap();
    let target_prop: HashMap<_, _> = content_prop_obj
        .iter()
        .map(|(k, v)| (k.as_str().into(), v.as_f64().unwrap()))
        .collect();
    let cb_config = ContentBalancingConfig::new(categories, target_prop, true).unwrap();
    let sel_config = SelectionConfig {
        criterion: Criterion::DRule { prior: false },
        ..Default::default()
    };

    let n_items = case["n_items"].as_u64().unwrap() as usize;
    let start_item_1indexed = case["start_item"].as_u64().unwrap() as usize;
    let thetas_history: Vec<Vec<f64>> = case["thetas_history"]
        .as_array()
        .unwrap()
        .iter()
        .map(f64_array)
        .collect();

    let mut person = PersonState::new(2);
    person.set_theta(ThetaVector::from_vec(thetas_history[0].clone()));
    person.record_answer(start_item_1indexed - 1, 1);
    let mut rng = StdRng::seed_from_u64(0);

    let mut any_fallback = false;
    for theta in thetas_history.iter().skip(1) {
        person.set_theta(ThetaVector::from_vec(theta.clone()));
        let candidates: Vec<usize> = (0..n_items)
            .filter(|idx| !person.answered_items().contains(idx))
            .collect();
        if candidates.is_empty() {
            break;
        }
        let (narrowed, debug) =
            narrow_candidates(&cb_config, person.answered_items(), &candidates).unwrap();
        if debug.unwrap().fallback_used {
            any_fallback = true;
        }
        let (chosen, _) =
            select_next_item(&sel_config, &narrowed, &person, &bank, &group, &mut rng).unwrap();
        person.record_answer(chosen, 1);
    }

    assert!(
        any_fallback,
        "skenario fallback fixture ini seharusnya benar-benar memicu jalur fallback minimal \
         sekali"
    );
}
