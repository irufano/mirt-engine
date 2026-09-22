//! Contoh Fase 5: exposure control randomesque (`'sample'`, top-*n* random)
//! — exposure diindeks per **posisi penyajian** (`docs/teori/
//! 05-exposure-control-randomesque.md`).
//!
//! Simulasi 1000 "peserta" sintetis, langkah PERTAMA saja (posisi 0) yang
//! diamati -- exposure di posisi itu dibandingkan tiga cara: tanpa exposure
//! control (baseline, selalu argmax), `exposure=1` (deterministik, secara
//! matematis identik dgn baseline), dan `exposure=3` (uniform random di
//! antara 3 kandidat kriteria teratas). Theta TETAP di nol (murni
//! mengisolasi mekanisme exposure control, sama seperti contoh SH).
//!
//! Memakai `DPrule` (bukan `Drule` polos) supaya matriks informasi full
//! rank sejak 0 item terjawab -- `Drule` TANPA prior di titik ini secara
//! matematis degenerate (informasi 1 item dikotomus tunggal selalu rank 1,
//! determinan PERSIS 0 untuk SEMUA kandidat, lih. `PLAN.md` §Fase 2), yang
//! akan membuat "top-3" jadi noise floating-point alih-alih sinyal kriteria
//! yang bermakna untuk didemonstrasikan di sini.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 05_exposure_control_randomesque
//! ```

use mirtcat_engine::exposure_control::{select_with_exposure, ExposureControlConfig, ExposureMode};
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::theta::InfoMatrix;
use mirtcat_engine::{
    rank_candidates, Criterion, GroupParams, ItemBank, ItemMeta, PersonState, SelectionConfig,
    ThetaVector,
};
use nalgebra::DMatrix;
use rand::rngs::StdRng;
use rand::SeedableRng;

const N_ITEMS: usize = 8;
const N_PESERTA: usize = 1000;

fn build_bank() -> ItemBank {
    let params: [([f64; 2], f64); N_ITEMS] = [
        ([1.5, 0.3], 0.0),
        ([1.4, 0.2], 0.1),
        ([1.3, 0.4], -0.1),
        ([0.9, 0.9], 0.0),
        ([0.7, 1.1], 0.0),
        ([0.5, 1.3], 0.2),
        ([0.3, 1.4], -0.2),
        ([1.0, 1.0], 0.0),
    ];
    let mut bank = ItemBank::new();
    for (i, (a, d)) in params.into_iter().enumerate() {
        bank.push(
            Box::new(Dichotomous::m2pl(a.to_vec(), d).unwrap()),
            ItemMeta::new(i),
        )
        .unwrap();
    }
    bank
}

fn build_group() -> GroupParams {
    let gcov = InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[1.0, 0.1, 0.1, 1.0]))
        .expect("gcov persegi");
    GroupParams::new(ThetaVector::zeros(2), gcov).unwrap()
}

/// Frekuensi item terpilih di LANGKAH PERTAMA sesi (posisi 0), lintas
/// `N_PESERTA` peserta -- `exposure_for_position` sudah tetap sepanjang
/// simulasi ini (cukup untuk mendemonstrasikan posisi tunggal; posisi lain
/// bekerja identik, cuma indeks vektor exposure-nya beda -- lih. catatan
/// modul `exposure_control` soal pengindeksan per posisi).
fn simulate_first_step(exposure_for_position: Option<u32>, rng: &mut StdRng) -> [usize; N_ITEMS] {
    let bank = build_bank();
    let group = build_group();
    let sel_config = SelectionConfig {
        criterion: Criterion::DRule { prior: true },
        ..Default::default()
    };
    let exposure_vec = exposure_for_position.map(|n| vec![n; N_ITEMS]);
    let exp_config = exposure_vec.as_ref().map(|v| {
        ExposureControlConfig::new(ExposureMode::Randomesque(v.clone()), N_ITEMS, false).unwrap()
    });

    let mut counts = [0usize; N_ITEMS];
    for _ in 0..N_PESERTA {
        let person = PersonState::new(2);
        let candidates: Vec<usize> = (0..N_ITEMS).collect();
        let scores = rank_candidates(&sel_config, &candidates, &person, &bank, &group).unwrap();

        let chosen = match &exp_config {
            Some(cfg) => {
                let mut valid_items = vec![true; N_ITEMS];
                select_with_exposure(cfg, &scores, 0, &mut valid_items, rng)
                    .unwrap()
                    .0
            }
            None => {
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
    }
    counts
}

fn print_row(label: &str, counts: &[usize; N_ITEMS]) {
    print!("{label:<22}");
    for c in counts {
        print!(" {c:>5}");
    }
    println!();
}

fn main() {
    println!(
        "{N_PESERTA} peserta, langkah pertama, kriteria DPrule -- item 1 (a=[1.5,0.3]) argmax \
         tunggal di theta=0 (DPrule dipakai, bukan Drule polos, supaya full rank sejak awal --\
         lih. catatan modul)."
    );
    println!();
    print!("{:<22}", "");
    for i in 1..=N_ITEMS {
        print!(" it.{i:>2}");
    }
    println!();

    let mut rng_baseline = StdRng::seed_from_u64(7);
    let baseline = simulate_first_step(None, &mut rng_baseline);
    print_row("baseline (tanpa EC)", &baseline);

    let mut rng_exp1 = StdRng::seed_from_u64(7);
    let exp1 = simulate_first_step(Some(1), &mut rng_exp1);
    print_row("exposure=1", &exp1);

    let mut rng_exp3 = StdRng::seed_from_u64(7);
    let exp3 = simulate_first_step(Some(3), &mut rng_exp3);
    print_row("exposure=3", &exp3);

    let top3 = {
        let mut top3: Vec<(usize, usize)> = exp3.iter().copied().enumerate().collect();
        top3.sort_by_key(|&(_, c)| std::cmp::Reverse(c));
        top3.into_iter()
            .filter(|&(_, c)| c > 0)
            .map(|(idx, _)| idx + 1)
            .collect::<Vec<_>>()
    };
    println!();
    println!(
        "baseline & exposure=1 harus IDENTIK (exposure=1 = deterministik argmax, sama seperti \
         tanpa exposure control sama sekali). exposure=3 harus menyebar HANYA ke 3 item \
         berperingkat-kriteria-teratas (di sini: item {top3:?}), BUKAN cuma item 1 seperti \
         baseline -- tapi juga TIDAK ke item lain di luar top-3 itu."
    );
    assert_eq!(baseline, exp1, "exposure=1 harus identik dgn baseline");
    assert_eq!(
        top3.len(),
        3,
        "top-3 harus persis 3 item (non-degenerate, DPrule full rank)"
    );
}
