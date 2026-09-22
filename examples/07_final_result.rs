//! Contoh Fase 7: sesi CAT sintetis end-to-end (Fase 1–6: initial theta,
//! item selection, ability estimation, stopping rule, exposure control
//! diabaikan/tanpa content balancing di sini demi fokus, lih. catatan di
//! bawah) sampai berhenti, lalu dirakit jadi `McatResult` (Fase 7) dan
//! diserialisasi ke JSON (`docs/panduan/07-hasil-akhir-theta.md`).
//!
//! Bank sintetis 3 dimensi (18 item, dispersi vektor diskriminasi
//! bergantian antar-dimensi supaya D-rule punya insentif menjelajah ketiga
//! dimensi), peserta simulasi dengan `theta_benar = [0.8, -0.5, 1.2]` tetap
//! (dipakai HANYA untuk menyampling respons via kategori peluang model
//! `Dichotomous`, bukan diketahui pipeline) -- pola sesi nyata: seleksi
//! item (`DPrule`, prior) -> sampling respons -> estimasi ulang (`MAP`) ->
//! evaluasi stopping rule (`min_SEM=0.55` semua dimensi) tiap langkah.
//!
//! Content balancing (Fase 6) & exposure control (Fase 5) sengaja
//! DILEWATKAN di sini -- keduanya sudah punya contoh dedicated
//! (`examples/05_*.rs`, `examples/06_content_balancing.rs`); menambahkannya
//! di sini hanya menambah parameter tanpa memperjelas fokus fase ini
//! (perakitan `McatResult`).
//!
//! Jalankan dengan (butuh fitur `serde` untuk bagian serialisasi JSON):
//! ```sh
//! cargo run --example 07_final_result --features serde
//! ```

use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::initial_theta::{initialize, InitialThetaConfig};
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::stopping_rule::{evaluate as evaluate_stopping, StoppingConfig};
use mirtcat_engine::theta::InfoMatrix;
use mirtcat_engine::{
    select_next_item, Criterion, EstimationConfig, GroupParams, ItemBank, ItemMeta, McatResult,
    Method, PersonState, SelectionConfig, ThetaVector,
};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

const NFACT: usize = 3;
const N_ITEMS: usize = 18;

fn build_bank() -> ItemBank {
    // Diskriminasi bergantian antar-dimensi (mirip `bank_2d` fase-fase
    // sebelumnya, diperluas ke 3 dimensi) -- tiap dimensi punya 6 item kuat.
    let mut bank = ItemBank::new();
    for i in 0..N_ITEMS {
        let dim_strong = i % 3;
        let mut a = vec![0.2; NFACT];
        a[dim_strong] = 1.4 + 0.1 * (i / 3) as f64;
        let d = 0.3 - 0.1 * (i as f64 % 5.0);
        bank.push(Box::new(Dichotomous::m2pl(a, d).unwrap()), ItemMeta::new(i))
            .unwrap();
    }
    bank
}

fn build_group() -> GroupParams {
    GroupParams::new(ThetaVector::zeros(NFACT), InfoMatrix::identity(NFACT)).unwrap()
}

/// Sampel respons dikotomus dari peluang model di `theta_true` (padanan
/// `local_pattern` sintetis, bukan diketahui pipeline seleksi/estimasi).
fn sample_response(
    item: &dyn mirtcat_engine::irt::ItemResponseModel,
    theta_true: &ThetaVector,
    rng: &mut impl Rng,
) -> usize {
    let probs = item.probabilities(theta_true).unwrap();
    let u: f64 = rng.gen();
    let mut cumulative = 0.0;
    for (category, &p) in probs.iter().enumerate() {
        cumulative += p;
        if u < cumulative {
            return category;
        }
    }
    probs.len() - 1
}

fn main() {
    let bank = build_bank();
    let group = build_group();
    let theta_true = ThetaVector::from_vec(vec![0.8, -0.5, 1.2]);
    let mut rng = StdRng::seed_from_u64(7);

    let initial_config = InitialThetaConfig::default(); // ThetaStart::Zero
    let (theta0, se0, _weights, _) = initialize(&initial_config, &group, 0).unwrap();
    let mut person = PersonState::from_theta_and_se(theta0, se0);

    let sel_config = SelectionConfig {
        criterion: Criterion::DRule { prior: true },
        ..Default::default()
    };
    let est_config = EstimationConfig {
        method: Method::Map,
        ..Default::default()
    };
    let stop_config = StoppingConfig {
        min_items: 5,
        max_items: N_ITEMS,
        min_sem: vec![0.55],
        ..Default::default()
    };

    println!("Sesi CAT sintetis 3 dimensi -- DPrule + MAP + min_SEM=0.55 (semua dimensi).");
    println!(
        "theta_benar (tersembunyi dari pipeline) = {:?}",
        theta_true.as_slice()
    );
    println!();

    let stop_decision = loop {
        let candidates: Vec<usize> = (0..N_ITEMS)
            .filter(|idx| !person.answered_items().contains(idx))
            .collect();
        let (chosen, _) =
            select_next_item(&sel_config, &candidates, &person, &bank, &group, &mut rng).unwrap();

        let response = sample_response(bank.item(chosen).unwrap(), &theta_true, &mut rng);
        person.record_answer(chosen, response);

        let ((theta, se), _) = estimate(&est_config, &person, &bank, &group).unwrap();
        person.set_estimate(theta, se);

        println!(
            "item ke-{:>2} (idx={chosen:>2}, respons={response}): theta={:?} se={:?}",
            person.n_answered(),
            person.theta().as_slice(),
            person.se().as_slice(),
        );

        let (decision, _) = evaluate_stopping(&stop_config, &person, &bank, false, None).unwrap();
        if decision.stop_now {
            break decision;
        }
    };

    let result = McatResult::from_person(&person, &stop_decision);

    println!();
    println!("=== McatResult ===");
    println!("column_names      = {:?}", result.column_names);
    println!("thetas            = {:?}", result.thetas.as_slice());
    println!("se_thetas         = {:?}", result.se_thetas.as_slice());
    println!("items_answered    = {:?}", result.items_answered);
    println!("responses         = {:?}", result.responses);
    println!("terminated_early  = {}", result.terminated_early);
    println!("classification    = {:?}", result.classification);
    println!(
        "thetas_history    = {} baris (1 per item terjawab, TANPA baris pra-tes -- lih. \
         `docs/teori/07-hasil-akhir-theta.md`)",
        result.thetas_history.len()
    );
    assert_eq!(result.thetas_history.len(), result.items_answered.len());

    #[cfg(feature = "serde")]
    {
        let json = serde_json::to_string_pretty(&result).unwrap();
        println!();
        println!("=== JSON (serde) ===");
        println!("{json}");
    }
    #[cfg(not(feature = "serde"))]
    {
        println!();
        println!(
            "(Jalankan ulang dengan `--features serde` untuk melihat serialisasi JSON penuh.)"
        );
    }
}
