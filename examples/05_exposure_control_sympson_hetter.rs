//! Contoh Fase 5: exposure control Sympson–Hetter (`SH`) — loop terima/
//! tolak per-item, exposure diindeks per **ID item** (`docs/teori/
//! 05-exposure-control-sympson-hetter.md`).
//!
//! Simulasi 1000 "peserta" sintetis menjawab 5 item lewat kriteria `Drule`
//! (Fase 2), theta TETAP di nol sepanjang sesi (murni mengisolasi mekanisme
//! exposure control itu sendiri, terpisah dari ability estimation Fase 3 --
//! pola yang sama seperti contoh Fase 2/4). Frekuensi administrasi tiap
//! item dicetak berdampingan dengan baseline TANPA exposure control, supaya
//! terlihat jelas bagaimana SH meratakan distribusi menuju target
//! `exposure[item]`, sekaligus mendemonstrasikan fallback "kandidat tersisa
//! satu" (item exposure sangat rendah TETAP bisa lolos begitu semua
//! alternatif lain sudah habis di satu sesi).
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 05_exposure_control_sympson_hetter
//! ```

use mirt_engine::exposure_control::{select_with_exposure, ExposureControlConfig, ExposureMode};
use mirt_engine::irt::Dichotomous;
use mirt_engine::theta::InfoMatrix;
use mirt_engine::{
    rank_candidates, Criterion, GroupParams, ItemBank, ItemMeta, PersonState, SelectionConfig,
    ThetaVector,
};
use nalgebra::DMatrix;
use rand::rngs::StdRng;
use rand::SeedableRng;

const N_ITEMS: usize = 10;
const N_PESERTA: usize = 1000;
const N_ITEM_PER_SESI: usize = 5;

fn build_bank() -> ItemBank {
    // Item dibuat SENGAJA mirip satu sama lain (informasi tinggi & serupa)
    // supaya, tanpa exposure control, beberapa item "favorit" mendominasi
    // hampir seluruh sesi (D-rule argmax deterministik begitu bank
    // homogen) -- kondisi yang justru paling butuh exposure control.
    let mut bank = ItemBank::new();
    for i in 0..N_ITEMS {
        let a = 1.0 + 0.05 * (i as f64);
        bank.push(
            Box::new(Dichotomous::m2pl(vec![a, a * 0.9], 0.0).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn build_group() -> GroupParams {
    let gcov = InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[1.0, 0.2, 0.2, 1.0]))
        .expect("gcov persegi");
    GroupParams::new(ThetaVector::zeros(2), gcov).unwrap()
}

fn simulate(exposure: Option<&[f64]>, rng: &mut StdRng) -> [usize; N_ITEMS] {
    let bank = build_bank();
    let group = build_group();
    let sel_config = SelectionConfig {
        criterion: Criterion::DRule { prior: false },
        ..Default::default()
    };
    let exp_config = exposure.map(|v| {
        ExposureControlConfig::new(ExposureMode::SympsonHetter(v.to_vec()), N_ITEMS, false).unwrap()
    });

    let mut counts = [0usize; N_ITEMS];
    for _ in 0..N_PESERTA {
        let mut person = PersonState::new(2);
        let mut valid_items = vec![true; N_ITEMS];
        for _ in 0..N_ITEM_PER_SESI {
            let candidates: Vec<usize> = (0..N_ITEMS)
                .filter(|idx| !person.answered_items().contains(idx) && valid_items[*idx])
                .collect();
            let scores = rank_candidates(&sel_config, &candidates, &person, &bank, &group).unwrap();

            let chosen = match &exp_config {
                Some(cfg) => {
                    select_with_exposure(cfg, &scores, 0, &mut valid_items, rng)
                        .unwrap()
                        .0
                }
                None => {
                    // Baseline: langsung argmax, tanpa exposure control.
                    scores
                        .iter()
                        .copied()
                        .fold((0, f64::NEG_INFINITY), |best, (idx, v)| {
                            if v > best.1 {
                                (idx, v)
                            } else {
                                best
                            }
                        })
                        .0
                }
            };
            counts[chosen] += 1;
            person.record_answer(chosen, 1);
        }
    }
    counts
}

fn main() {
    let exposure = [0.1, 1.0, 0.2, 1.0, 0.3, 1.0, 0.9, 1.0, 0.05, 1.0];

    let mut rng_baseline = StdRng::seed_from_u64(1);
    let baseline = simulate(None, &mut rng_baseline);

    let mut rng_sh = StdRng::seed_from_u64(1);
    let with_sh = simulate(Some(&exposure), &mut rng_sh);

    println!(
        "{N_PESERTA} peserta x {N_ITEM_PER_SESI} item, kriteria Drule, bank {N_ITEMS} item mirip \
         (tanpa exposure control, beberapa item mendominasi)."
    );
    println!();
    println!(
        "{:>6} {:>10} {:>12} {:>12}",
        "item", "exposure", "baseline", "dgn SH"
    );
    for i in 0..N_ITEMS {
        println!(
            "{:>6} {:>10.2} {:>12} {:>12}",
            i + 1,
            exposure[i],
            baseline[i],
            with_sh[i]
        );
    }
    println!();
    println!(
        "Perhatikan: item dgn exposure rendah (mis. item 9, target 0.05) tetap MUNGKIN \
         terpilih beberapa kali -- SH membatasi PELUANG lolos tiap kali dicoba sebagai \
         kandidat terbaik, bukan melarang mutlak; fallback 'kandidat tersisa satu' juga \
         bisa memaksa lolos di sesi tertentu. Item exposure=1.0 (tanpa pembatasan efektif) \
         frekuensinya mendekati baseline."
    );
}
