//! Fase 8 — benchmark dasar (`criterion`) untuk seleksi item & estimasi
//! kemampuan pada item bank besar (500 item, 5 dimensi), memastikan engine
//! layak dipakai real-time di web API (lih. `PLAN.md` §Fase 8 checklist).
//!
//! Bukan gate pass/fail — `cargo bench` mencetak laporan waktu yang bisa
//! diinspeksi manual/dibandingkan antar-run (`target/criterion/`).

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion as CriterionHarness};
use rand::rngs::StdRng;
use rand::SeedableRng;

use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::{
    select_next_item, Criterion as SelectionCriterion, EstimationConfig, GroupParams, ItemBank,
    ItemMeta, Method, PersonState, SelectionConfig,
};

const N_ITEMS: usize = 500;
const NFACT: usize = 5;
const N_ANSWERED: usize = 10;

/// Bank sintetis 500 item M2PL, 5 dimensi, slope & intercept deterministik
/// (seeded) -- cukup representatif untuk mengukur biaya `info_thetas`/
/// optimisasi pada bank besar tanpa bergantung pada data eksternal.
fn build_bank() -> ItemBank {
    let mut rng = StdRng::seed_from_u64(42);
    let mut bank = ItemBank::new();
    for i in 0..N_ITEMS {
        let a: Vec<f64> = (0..NFACT)
            .map(|_| rand::Rng::gen_range(&mut rng, 0.4..1.6))
            .collect();
        let d = rand::Rng::gen_range(&mut rng, -2.0..2.0);
        bank.push(Box::new(Dichotomous::m2pl(a, d).unwrap()), ItemMeta::new(i))
            .unwrap();
    }
    bank
}

/// `PersonState` dengan `N_ANSWERED` item pertama sudah dijawab (respons
/// deterministik, alternasi 0/1) -- mensimulasikan sesi CAT yang sudah
/// berjalan sebagian, supaya `info_thetas()`/log-likelihood yang dibench
/// punya biaya realistis (bukan biaya bank kosong).
fn build_partial_session(bank: &ItemBank, group: &GroupParams) -> PersonState {
    let mut person = PersonState::from_theta(mirtcat_engine::ThetaVector::zeros(NFACT));
    for i in 0..N_ANSWERED {
        person.record_answer(i, i % 2);
    }
    let config = EstimationConfig {
        method: Method::Map,
        ..Default::default()
    };
    let ((theta, se), _) = estimate(&config, &person, bank, group).unwrap();
    person.set_estimate(theta, se);
    person
}

fn bench_item_selection(c: &mut CriterionHarness) {
    let bank = build_bank();
    let group = GroupParams::standard_normal(NFACT);
    let person = build_partial_session(&bank, &group);
    let candidates: Vec<usize> = (N_ANSWERED..N_ITEMS).collect();
    let mut rng = StdRng::seed_from_u64(123);

    let criteria: Vec<(&str, SelectionCriterion)> = vec![
        ("seq", SelectionCriterion::Seq),
        ("random", SelectionCriterion::Random),
        ("kl", SelectionCriterion::Kl { delta: 0.1 }),
        ("kl_n", SelectionCriterion::KlN { delta: 0.1 }),
        ("d_rule", SelectionCriterion::DRule { prior: false }),
        ("t_rule", SelectionCriterion::TRule { prior: false }),
        ("a_rule", SelectionCriterion::ARule { prior: false }),
        ("e_rule", SelectionCriterion::ERule { prior: false }),
        ("w_rule", SelectionCriterion::WRule { prior: false }),
    ];

    let mut group_bench = c.benchmark_group("item_selection");
    for (name, criterion) in criteria {
        let config = SelectionConfig {
            criterion,
            ..Default::default()
        };
        group_bench.bench_with_input(BenchmarkId::from_parameter(name), &config, |b, config| {
            b.iter(|| {
                select_next_item(config, &candidates, &person, &bank, &group, &mut rng).unwrap()
            });
        });
    }
    group_bench.finish();
}

fn bench_ability_estimation(c: &mut CriterionHarness) {
    let bank = build_bank();
    let group = GroupParams::standard_normal(NFACT);
    let person = build_partial_session(&bank, &group);

    let methods: Vec<(&str, Method)> = vec![
        ("eap_grid", Method::Eap { qmc: false }),
        ("eap_qmc", Method::Eap { qmc: true }),
        ("map", Method::Map),
        ("ml", Method::Ml),
        ("wle", Method::Wle),
        ("eap_sum", Method::EapSum),
    ];

    let mut group_bench = c.benchmark_group("ability_estimation");
    for (name, method) in methods {
        let config = EstimationConfig {
            method,
            ..Default::default()
        };
        group_bench.bench_with_input(BenchmarkId::from_parameter(name), &config, |b, config| {
            b.iter(|| estimate(config, &person, &bank, &group).unwrap());
        });
    }
    group_bench.finish();
}

criterion_group!(benches, bench_item_selection, bench_ability_estimation);
criterion_main!(benches);
