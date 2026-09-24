//! Contoh Fase 8 (integrasi skala realistis): satu sesi MCAT LENGKAP yang
//! menjahit Fase 1–7 sekaligus -- termasuk dua fase yang SENGAJA dilewatkan
//! di `examples/07_final_result.rs` demi fokus (content balancing & exposure
//! control) -- di atas bank 300 item, 3 dimensi bernama: **Verbal**,
//! **Numerik**, **Spatial** (bukan `dim0`/`dim1`/`dim2` generik seperti
//! contoh fase individual, supaya terasa seperti tes psikometri sungguhan).
//! Tidak ada dokumen teori terpisah untuk contoh ini -- tiap mekanisme di
//! bawah sudah dijelaskan lengkap di dokumen fase masing-masing
//! (`docs/teori/0{1..7}-*.md`); di sini murni komposisi pipeline.
//!
//! Urutan tiap langkah (persis urutan nyata `mirtCAT::findNextCATItem()`,
//! lih. catatan modul `content_balancing`): kandidat belum terjawab & masih
//! `valid` -> **Fase 6** menyempitkan ke kategori (dimensi) berdefisit
//! terbesar dari target 1/3-1/3-1/3 -> **Fase 2** (`DPrule`) mengurutkan
//! kandidat tersempit -> **Fase 5** (Sympson-Hetter, exposure diindeks per
//! ID item) menerima/menolak sampai satu item lolos -> sampling respons ->
//! **Fase 3** (`MAP`) re-estimasi -> **Fase 4** (`min_SEM=0.3` semua
//! dimensi, `min_items=20`) dievaluasi. Begitu berhenti, dirakit jadi
//! `McatResult` (**Fase 7**). **Fase 1** (initial theta) hanya dipakai SEKALI
//! di awal sesi.
//!
//! Bank 300 item (100/dimensi, dimensi dominan interleaved `i % 3` seperti
//! `07_final_result.rs`, TAPI diskriminasi & difficulty disampel dari
//! rentang realistis via RNG berseed -- pada skala 300 item, enumerasi
//! manual seperti bank 12/18-item contoh fase individual sudah tidak
//! proporsional). Item dengan diskriminasi dominan tinggi (>1.3 -- yang
//! paling sering diargmax D-rule) diberi target exposure ketat (0.25),
//! sisanya bebas (1.0) -- pola sama seperti `05_exposure_control_sympson_hetter.rs`,
//! supaya efek Fase 5 tetap terlihat meski bank jauh lebih besar.
//!
//! Peserta simulasi `theta_benar = [0.5, -1.0, 0.9]` tetap (dipakai HANYA
//! untuk sampling respons, bukan diketahui pipeline -- sama seperti Fase 7).
//!
//! Jalankan dengan (butuh fitur `serde` untuk bagian serialisasi JSON):
//! ```sh
//! cargo run --example 08_mcat_session_300_items_3d --features serde
//! ```

use mirt_engine::ability_estimation::estimate;
use mirt_engine::content_balancing::{narrow_candidates, ContentBalancingConfig};
use mirt_engine::exposure_control::{select_with_exposure, ExposureControlConfig, ExposureMode};
use mirt_engine::initial_theta::{initialize, InitialThetaConfig};
use mirt_engine::irt::Dichotomous;
use mirt_engine::stopping_rule::{evaluate as evaluate_stopping, StoppingConfig};
use mirt_engine::theta::InfoMatrix;
use mirt_engine::{
    rank_candidates, Criterion, EstimationConfig, GroupParams, ItemBank, ItemMeta, McatResult,
    Method, PersonState, SelectionConfig, ThetaVector,
};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;

const NFACT: usize = 3;
const N_ITEMS: usize = 300;
const DIM_NAMES: [&str; NFACT] = ["Verbal", "Numerik", "Spatial"];

/// Bank 300 item, dimensi dominan interleaved `i % NFACT` (100 item persis
/// per dimensi sejak awal, padanan pola `07_final_result.rs`). Diskriminasi
/// dominan & difficulty disampel dari rentang realistis lewat `rng` yang
/// diteruskan pemanggil (RNG bank TERPISAH dari RNG sesi -- dua sumber
/// keacakan berbeda peran, sama seperti pemisahan RNG bank/sesi implisit di
/// seluruh contoh fase individual). Mengembalikan juga diskriminasi dominan
/// tiap item, dipakai membangun vektor exposure Fase 5.
fn build_bank(rng: &mut impl Rng) -> (ItemBank, Vec<f64>) {
    let mut bank = ItemBank::new();
    let mut a_strong = Vec::with_capacity(N_ITEMS);
    for i in 0..N_ITEMS {
        let dim = i % NFACT;
        let strong: f64 = rng.gen_range(0.6..=1.8);
        let d: f64 = rng.gen_range(-2.0..=2.0);
        let mut a = vec![0.15; NFACT];
        a[dim] = strong;
        bank.push(
            Box::new(Dichotomous::m2pl(a, d).unwrap()),
            ItemMeta::with_content(i, DIM_NAMES[dim]),
        )
        .unwrap();
        a_strong.push(strong);
    }
    (bank, a_strong)
}

fn build_group() -> GroupParams {
    GroupParams::new(ThetaVector::zeros(NFACT), InfoMatrix::identity(NFACT)).unwrap()
}

/// Fase 2: `SelectionConfig` dengan kriteria `DRule` (D-rule, varian
/// multidimensional dari maximum information) berprior -- didukung penuh
/// berpasangan dengan content balancing Fase 6 (lih. catatan modul
/// `content_balancing` poin 2, larangan eksplisit `mirtCAT` hanya berlaku
/// untuk `Criterion::Seq`).
fn build_selection_config() -> SelectionConfig {
    SelectionConfig {
        criterion: Criterion::DRule { prior: true },
        ..Default::default()
    }
}

/// Fase 6: bangun `ContentBalancingConfig` dengan kategori konten diambil
/// langsung dari `ItemMeta` bank (bukan array kategori terpisah seperti
/// `06_content_balancing.rs` 12-item -- pada skala 300 item, satu sumber
/// kebenaran tunggal menghindari duplikasi/divergensi antara bank &
/// kategori), target proporsi merata 1/NFACT tiap dimensi, lalu divalidasi
/// terhadap `criterion` seleksi yang akan dipakai sesi. Larangan eksplisit
/// `mirtCAT`: content balancing HANYA dilarang berpasangan dengan
/// `Criterion::Seq` -- `DRule` didukung penuh (lih. catatan modul
/// `content_balancing` poin 2).
fn build_content_balancing_config(bank: &ItemBank, criterion: Criterion) -> ContentBalancingConfig {
    let categories = (0..N_ITEMS)
        .map(|i| bank.meta(i).unwrap().content.clone().unwrap().into())
        .collect();
    let mut target_prop = HashMap::new();
    for name in DIM_NAMES {
        target_prop.insert(name.into(), 1.0 / NFACT as f64);
    }
    let cb_config = ContentBalancingConfig::new(categories, target_prop, false).unwrap();
    cb_config.validate_with_criterion(criterion).unwrap();
    cb_config
}

/// Fase 5: `ExposureControlConfig` Sympson-Hetter, exposure diindeks per ID
/// item. Item dengan diskriminasi dominan tinggi (>1.3, kandidat favorit
/// D-rule) dibatasi ketat (0.25); sisanya bebas (1.0, tanpa pembatasan
/// efektif) -- pola sama seperti `05_exposure_control_sympson_hetter.rs`.
/// Mengembalikan juga jumlah item berdiskriminasi tinggi di bank, dipakai
/// laporan ringkasan akhir sesi.
fn build_exposure_config(a_strong: &[f64]) -> (ExposureControlConfig, usize) {
    let exposure: Vec<f64> = a_strong
        .iter()
        .map(|&a| if a > 1.3 { 0.25 } else { 1.0 })
        .collect();
    let n_high_disc = a_strong.iter().filter(|&&a| a > 1.3).count();
    let exp_config =
        ExposureControlConfig::new(ExposureMode::SympsonHetter(exposure), N_ITEMS, false).unwrap();
    (exp_config, n_high_disc)
}

/// Fase 3: `EstimationConfig` dengan metode `MAP` (Bayes modal, butuh
/// `GroupParams` sebagai prior -- lih. `03_ability_estimation.rs`).
fn build_estimation_config() -> EstimationConfig {
    EstimationConfig {
        method: Method::Map,
        ..Default::default()
    }
}

/// Fase 4: `StoppingConfig` -- berhenti setelah minimal 20 item TAPI SEM
/// semua dimensi sudah <=0.35, dipaksa berhenti di 120 item apa pun SEM-nya
/// (guard supaya sesi tidak berjalan tanpa batas).
fn build_stopping_config() -> StoppingConfig {
    StoppingConfig {
        min_items: 20,
        max_items: 120,
        min_sem: vec![0.35],
        ..Default::default()
    }
}

/// Sampel respons dikotomus dari peluang model di `theta_true` (sama persis
/// `07_final_result.rs` -- bukan diketahui pipeline seleksi/estimasi).
fn sample_response(
    item: &dyn mirt_engine::irt::ItemResponseModel,
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
    let mut bank_rng = StdRng::seed_from_u64(300);
    let (bank, a_strong) = build_bank(&mut bank_rng);
    let group = build_group();

    // Selection Config
    let sel_config = build_selection_config();

    // Content Balancing Config, divalidasi langsung terhadap kriteria seleksi di atas.
    let cb_config = build_content_balancing_config(&bank, sel_config.criterion);

    // Exposure Config 
    let (exp_config, n_high_disc) = build_exposure_config(&a_strong);

    // Estimation Config (Fase 3)
    let est_config = build_estimation_config();
    // Stopping Config (Fase 4)
    let stop_config = build_stopping_config();

    let theta_true = ThetaVector::from_vec(vec![0.5, -1.0, 0.9]);
    let mut session_rng = StdRng::seed_from_u64(8);

    // Initial Theta Config -- dipakai SEKALI di awal sesi.
    let initial_config = InitialThetaConfig::default(); // ThetaStart::Zero
    let (theta0, se0, _weights, _) = initialize(&initial_config, &group, 0).unwrap();
    let mut person = PersonState::from_theta_and_se(theta0, se0);
    let mut valid_items = vec![true; N_ITEMS]; // state Fase 5, bertahan sepanjang sesi

    println!(
        "Sesi MCAT 3 dimensi ({DIM_NAMES:?}), bank {N_ITEMS} item (100/dimensi, {n_high_disc} \
         berdiskriminasi tinggi >1.3 -- exposure dibatasi 0.25)."
    );
    println!(
        "Pipeline/langkah: Fase 6 (narrow ke dimensi defisit) -> Fase 2 (DPrule) -> Fase 5 (SH) \
         -> respons -> Fase 3 (MAP) -> Fase 4 (stop min_SEM={:?}, min_items={}, max_items={}).",
        stop_config.min_sem, stop_config.min_items, stop_config.max_items
    );
    println!(
        "theta_benar (tersembunyi dari pipeline) = {:?}",
        theta_true.as_slice()
    );
    println!();
    println!(
        "{:>5} {:>6} {:>9} {:>8} {:>40} {:>80}",
        "step", "item", "dimensi", "respons", "theta", "se"
    );

    let mut dim_counts = [0usize; NFACT];
    let stop_decision = loop {
        let candidates: Vec<usize> = (0..N_ITEMS)
            .filter(|idx| !person.answered_items().contains(idx) && valid_items[*idx])
            .collect();

        let (narrowed, _) =
            narrow_candidates(&cb_config, person.answered_items(), &candidates).unwrap();
        let scores = rank_candidates(&sel_config, &narrowed, &person, &bank, &group).unwrap();
        let (chosen, _) = select_with_exposure(
            &exp_config,
            &scores,
            person.n_answered(),
            &mut valid_items,
            &mut session_rng,
        )
        .unwrap();

        let response = sample_response(bank.item(chosen).unwrap(), &theta_true, &mut session_rng);
        person.record_answer(chosen, response);

        let dim_name = bank.meta(chosen).unwrap().content.as_deref().unwrap();
        let dim_idx = DIM_NAMES.iter().position(|&n| n == dim_name).unwrap();
        dim_counts[dim_idx] += 1;

        let ((theta, se), _) = estimate(&est_config, &person, &bank, &group).unwrap();
        person.set_estimate(theta, se);

        println!(
            "{:>5} {:>6} {:>9} {:>8} {:>24?} {:>24?}",
            person.n_answered(),
            chosen,
            dim_name,
            response,
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
    println!("=== Content Balancing: proporsi konten realisasi vs target 1/3-1/3-1/3 ===");
    for (i, name) in DIM_NAMES.iter().enumerate() {
        let p = dim_counts[i] as f64 / person.n_answered() as f64;
        println!("{name:>10} : {:>3} item ({p:.3})", dim_counts[i]);
    }

    let high_disc_chosen = person
        .answered_items()
        .iter()
        .filter(|&&idx| a_strong[idx] > 1.3)
        .count();
    println!();
    println!(
        "=== Exposure Control: item berdiskriminasi tinggi (>1.3) yang lolos exposure control === \
         \n{high_disc_chosen}/{} item terjawab (dari {n_high_disc} item bank yang memenuhi >1.3 -- \
         exposure=0.25 MEMBATASI peluang lolos tiap dicoba, bukan melarang mutlak)",
        person.n_answered()
    );

    println!();
    println!("=== McatResult ===");
    println!("column_names      = {:?}", result.column_names);
    println!("thetas            = {:.3?}", result.thetas.as_slice());
    println!("se_thetas         = {:.3?}", result.se_thetas.as_slice());
    println!("items_answered    = {} item", result.items_answered.len());
    println!("terminated_early  = {}", result.terminated_early);
    println!("classification    = {:?}", result.classification);
    assert_eq!(result.thetas_history.len(), result.items_answered.len());
    assert_eq!(
        dim_counts.iter().sum::<usize>(),
        person.n_answered(),
        "setiap item terjawab wajib masuk tepat satu dimensi"
    );

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
