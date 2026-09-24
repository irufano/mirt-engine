//! Contoh Fase 9 (meta -- bukan fase baru `PLAN.md`): struktur sesi PERSIS
//! `08_mcat_session_300_items_3d.rs` (bank 300 item, 3 dimensi bernama,
//! Fase 1-7 dijahit sekaligus), TAPI keempat komponen algoritmik yang di
//! `08_...` di-hardcode (kriteria seleksi Fase 2, mode exposure control
//! Fase 5, metode estimasi Fase 3, aturan berhenti/klasifikasi Fase 4)
//! dijadikan **profil** yang bisa diganti-ganti lewat SATU baris const di
//! `main()` (bagian "GANTI DI SINI" di bawah), tanpa menyentuh logika
//! pipeline sama sekali.
//!
//! Content balancing (Fase 6) SENGAJA tetap hardcoded selalu aktif (bukan
//! sumbu ke-5 yang bisa dimatikan) -- ini satu-satunya larangan kombinasi
//! yang eksplisit didokumentasikan `mirtCAT` sendiri (`Criterion::Seq`),
//! jadi keberadaannya justru yang membuat pemeriksaan #1 di bawah punya
//! sesuatu nyata untuk didemonstrasikan. Pemanggil yang ingin sesi `Seq`
//! murni tanpa content balancing lihat `07_final_result.rs`.
//!
//! ## Validasi kombinasi
//!
//! Sebelum item pertama disajikan, `validate_combination()` menjalankan
//! pemeriksaan kompatibilitas dan mencetak laporan lolos/gagal/peringatan.
//! Kombinasi yang dilarang menghentikan program (`process::exit(1)`) dengan
//! pesan jelas, BUKAN panic/error samar di tengah sesi. Setiap pemeriksaan
//! memanggil fungsi validasi/produksi ASLI milik crate (bukan
//! reimplementasi aturan) supaya tetap benar walau aturan berubah:
//!
//! 1. **Content balancing x kriteria seleksi** -- `Criterion::Seq` dilarang
//!    mutlak (`ContentBalancingConfig::validate_with_criterion`, error
//!    `MirtCatError::ContentBalancingIncompatibleWithSeq`); kriteria lain
//!    (termasuk `Random`) didukung penuh.
//! 2. **Exposure control x kriteria seleksi** -- `Criterion::Seq`/`Random`
//!    TIDAK PERNAH menghasilkan skor kandidat (`rank_candidates` Fase 2
//!    `return Err` untuk keduanya, lih. catatan modul `exposure_control`
//!    poin 1), jadi tidak bisa digabung exposure control mode apa pun.
//!    Diperiksa dengan MEMANGGIL `rank_candidates` sungguhan (item 0
//!    sebagai kandidat probe), bukan menduplikasi aturannya.
//! 3. **Panjang vektor exposure vs jumlah item bank** -- divalidasi
//!    `ExposureControlConfig::new` saat config dibangun.
//! 4. **Panjang cutoff/hipotesis klasifikasi (`Ci`/`Sprt`) vs `NFACT`** --
//!    diperiksa lewat `ThetaVector::validate_nfact` (fungsi bawaan yang
//!    sama dipanggil `evaluate()` internal untuk `Sprt`) supaya pesan
//!    errornya identik dengan yang akan muncul di tengah sesi.
//! 5. **(peringatan, bukan larangan)** `Method::Fixed` pada sesi adaptif --
//!    theta/SE tidak pernah diperbarui (lih. `ability_estimation::fixed`),
//!    jadi target presisi (`min_SEM`/`delta_thetas`/klasifikasi) TIDAK
//!    AKAN PERNAH terpenuhi secara alami; sesi selalu berhenti di
//!    `max_items`. API tetap menerimanya (cocok untuk kuesioner
//!    non-adaptif), makanya cuma peringatan, bukan `[GAGAL]`.
//!
//! Dua kombinasi TERLARANG siap dicoba (edit bagian "GANTI DI SINI" lalu
//! jalankan ulang -- harus berhenti di pemeriksaan #1/#2, bukan lolos):
//! - `SelectionProfile::Seq` (bareng dengan apa pun -- content balancing
//!   selalu aktif) -> gagal pemeriksaan #1.
//! - `SelectionProfile::Random` + `ExposureProfile::SympsonHetter` atau
//!   `Randomesque` -> gagal pemeriksaan #2 (content balancing tetap lolos,
//!   cuma `Seq` yang dilarangnya).
//!
//! Jalankan dengan (butuh fitur `serde` untuk bagian serialisasi JSON):
//! ```sh
//! cargo run --example 09_configurable_pipeline_with_validation --features serde
//! ```

use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::content_balancing::{narrow_candidates, ContentBalancingConfig};
use mirtcat_engine::exposure_control::{select_with_exposure, ExposureControlConfig, ExposureMode};
use mirtcat_engine::initial_theta::{initialize, InitialThetaConfig};
use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::stopping_rule::{
    evaluate as evaluate_stopping, ClassifyType, StopDecision, StoppingConfig,
};
use mirtcat_engine::theta::InfoMatrix;
use mirtcat_engine::{
    rank_candidates, select_next_item, Criterion, EstimationConfig, GroupParams, ItemBank,
    ItemMeta, McatResult, MirtCatError, Method, PersonState, SelectionConfig, ThetaVector,
};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;
use std::collections::HashMap;

const NFACT: usize = 3;
const N_ITEMS: usize = 300;
const DIM_NAMES: [&str; NFACT] = ["Verbal", "Numerik", "Spatial"];

// ============================================================================
// Profil algoritma/metode -- satu enum per sumbu yang bisa diganti-ganti.
// Tiap varian di-map ke tipe config asli crate lewat `build()`, tidak ada
// logika baru di sini, murni pemetaan.
// ============================================================================

/// Profil kriteria seleksi (Fase 2). Subset representatif `Criterion` --
/// varian `*Rule` lain hanya beda `prior: bool`/`delta`, tidak menambah
/// pertanyaan kombinasi baru (lih. `02_item_selection_*.rs` per-kriteria).
// `#[allow(dead_code)]` di 4 enum profil di bawah: SENGAJA -- ini "menu"
// pilihan, hanya satu varian per enum yang aktif tiap kali dijalankan (lih.
// "GANTI DI SINI" di `main()`), sisanya memang tidak dikonstruksi pada
// kompilasi tertentu tapi tetap harus ada supaya bisa dipilih lain kali.
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum SelectionProfile {
    Seq,
    Random,
    KlN,
    DRule,
    TRule,
    ARule,
    ERule,
    WRule,
}

impl SelectionProfile {
    fn build(self) -> SelectionConfig {
        let criterion = match self {
            SelectionProfile::Seq => Criterion::Seq,
            SelectionProfile::Random => Criterion::Random,
            SelectionProfile::KlN => Criterion::KlN { delta: 0.1 },
            SelectionProfile::DRule => Criterion::DRule { prior: true },
            SelectionProfile::TRule => Criterion::TRule { prior: true },
            SelectionProfile::ARule => Criterion::ARule { prior: true },
            SelectionProfile::ERule => Criterion::ERule { prior: true },
            // Weights default (`None` -> `vec![1.0; nfact]`) -- efeknya
            // identik `TRule` di sini; lih. `02_item_selection_w_rule.rs`
            // untuk weights kustom yang membedakan keduanya.
            SelectionProfile::WRule => Criterion::WRule { prior: true },
        };
        SelectionConfig {
            criterion,
            ..Default::default()
        }
    }
}

/// Profil mode exposure control (Fase 5). `None` di sini berarti fitur
/// nonaktif -- ketiadaan `ExposureControlConfig`, bukan variasi mode,
/// konsisten dengan konvensi `ExposureMode` sendiri (tidak ada varian
/// "none").
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum ExposureProfile {
    None,
    SympsonHetter,
    Randomesque,
}

impl ExposureProfile {
    /// `a_strong` = diskriminasi dominan tiap item (dipakai `SympsonHetter`,
    /// pola sama seperti `08_...`/`05_exposure_control_sympson_hetter.rs`);
    /// diabaikan `Randomesque`/`None`.
    fn build(self, a_strong: &[f64]) -> Result<Option<ExposureControlConfig>, MirtCatError> {
        let mode = match self {
            ExposureProfile::None => return Ok(None),
            ExposureProfile::SympsonHetter => ExposureMode::SympsonHetter(
                a_strong
                    .iter()
                    .map(|&a| if a > 1.3 { 0.25 } else { 1.0 })
                    .collect(),
            ),
            // Top-3 random di SETIAP posisi (pola sama seperti
            // `05_exposure_control_randomesque.rs`) -- vektor tetap
            // panjang N_ITEMS walau makna indeksnya "posisi penyajian"
            // (`ExposureControlConfig::new` mewajibkan panjang == n_items
            // apa pun mode-nya).
            ExposureProfile::Randomesque => ExposureMode::Randomesque(vec![3; N_ITEMS]),
        };
        Ok(Some(ExposureControlConfig::new(mode, N_ITEMS, false)?))
    }
}

/// Profil metode estimasi kemampuan (Fase 3).
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum EstimationProfile {
    EapGrid,
    EapQmc,
    Map,
    Ml,
    Wle,
    EapSum,
    Fixed,
}

impl EstimationProfile {
    fn build(self) -> EstimationConfig {
        let method = match self {
            EstimationProfile::EapGrid => Method::Eap { qmc: false },
            EstimationProfile::EapQmc => Method::Eap { qmc: true },
            EstimationProfile::Map => Method::Map,
            EstimationProfile::Ml => Method::Ml,
            EstimationProfile::Wle => Method::Wle,
            EstimationProfile::EapSum => Method::EapSum,
            EstimationProfile::Fixed => Method::Fixed,
        };
        EstimationConfig {
            method,
            ..Default::default()
        }
    }
}

/// Profil aturan berhenti/klasifikasi (Fase 4).
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
enum StoppingProfile {
    MinSem,
    DeltaThetas,
    ClassifyCi,
    ClassifySprt,
}

impl StoppingProfile {
    fn build(self) -> StoppingConfig {
        match self {
            StoppingProfile::MinSem => StoppingConfig {
                min_items: 20,
                max_items: 120,
                min_sem: vec![0.35],
                ..Default::default()
            },
            StoppingProfile::DeltaThetas => StoppingConfig {
                min_items: 20,
                max_items: 120,
                // Nonaktifkan cabang min_SEM (ambang mustahil, SE tidak
                // pernah 0) supaya delta_thetas yang jadi pemicu utama,
                // padanan pola `04_stopping_rule_delta_thetas.rs`.
                min_sem: vec![0.0],
                delta_thetas: vec![0.02],
                ..Default::default()
            },
            StoppingProfile::ClassifyCi => StoppingConfig {
                min_items: 20,
                max_items: 120,
                classify: ClassifyType::Ci {
                    cutoffs: vec![0.0; NFACT],
                    alpha: 0.05,
                },
                ..Default::default()
            },
            StoppingProfile::ClassifySprt => StoppingConfig {
                min_items: 20,
                max_items: 120,
                classify: ClassifyType::Sprt {
                    lower: ThetaVector::from_vec(vec![-0.5; NFACT]),
                    upper: ThetaVector::from_vec(vec![0.5; NFACT]),
                    alpha: 0.05,
                    beta: 0.05,
                },
                ..Default::default()
            },
        }
    }
}

// ============================================================================
// Bank, group, sampling respons -- identik `08_mcat_session_300_items_3d.rs`.
// ============================================================================

/// Bank 300 item, dimensi dominan interleaved `i % NFACT`. Mengembalikan
/// juga diskriminasi dominan tiap item, dipakai profil `SympsonHetter`.
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

/// Fase 6: kategori konten dari `ItemMeta` bank, target proporsi merata
/// 1/NFACT tiap dimensi -- TIDAK divalidasi terhadap kriteria di sini
/// (itu tanggung jawab `validate_combination`, pemeriksaan #1).
fn build_content_balancing_config(bank: &ItemBank) -> ContentBalancingConfig {
    let categories = (0..N_ITEMS)
        .map(|i| bank.meta(i).unwrap().content.clone().unwrap().into())
        .collect();
    let mut target_prop = HashMap::new();
    for name in DIM_NAMES {
        target_prop.insert(name.into(), 1.0 / NFACT as f64);
    }
    ContentBalancingConfig::new(categories, target_prop, false).unwrap()
}

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

// ============================================================================
// Validasi kombinasi -- lih. docstring modul untuk daftar lengkap #1-#5.
// ============================================================================

/// Hasil satu pemeriksaan kompatibilitas.
struct CombinationCheck {
    name: &'static str,
    /// `Ok(detail)` lolos, `Err(detail)` gagal -- `fatal` menentukan apakah
    /// `Err` menghentikan sesi (`[GAGAL]`) atau cuma dicatat (`[WARN]`).
    detail: Result<String, String>,
    fatal: bool,
}

/// Jalankan seluruh pemeriksaan #1-#5 (lih. docstring modul). Setiap
/// pemeriksaan memanggil fungsi validasi/produksi ASLI crate, bukan
/// menduplikasi aturannya (Prinsip Desain #1 `PLAN.md`).
fn validate_combination(
    sel_config: &SelectionConfig,
    cb_config: &ContentBalancingConfig,
    exp_config: Option<&ExposureControlConfig>,
    est_config: &EstimationConfig,
    stop_config: &StoppingConfig,
    bank: &ItemBank,
    group: &GroupParams,
) -> Vec<CombinationCheck> {
    let mut checks = Vec::new();

    // #1: content balancing x kriteria seleksi.
    checks.push(CombinationCheck {
        name: "Content balancing x kriteria seleksi",
        detail: cb_config
            .validate_with_criterion(sel_config.criterion)
            .map(|()| format!("{:?} kompatibel dengan content balancing", sel_config.criterion))
            .map_err(|e| e.to_string()),
        fatal: true,
    });

    // #2: exposure control x kriteria seleksi -- diperiksa lewat pemanggilan
    // `rank_candidates` sungguhan (item 0 sbg kandidat probe, person nfact
    // sesuai bank -- kandidat/nilai probe tidak relevan, yang diperiksa
    // hanya apakah kriteria ini PUNYA skor sama sekali).
    checks.push(match exp_config {
        None => CombinationCheck {
            name: "Exposure control x kriteria seleksi",
            detail: Ok("exposure control nonaktif (ExposureProfile::None) -- kriteria apa pun valid".into()),
            fatal: false,
        },
        Some(_) => {
            let probe_person = PersonState::new(NFACT);
            let probe = rank_candidates(sel_config, &[0], &probe_person, bank, group);
            CombinationCheck {
                name: "Exposure control x kriteria seleksi",
                detail: probe
                    .map(|_| {
                        format!(
                            "{:?} menghasilkan skor kandidat -- kompatibel dengan exposure control",
                            sel_config.criterion
                        )
                    })
                    .map_err(|e| e.to_string()),
                fatal: true,
            }
        }
    });

    // #3: panjang vektor exposure vs jumlah item bank -- sudah divalidasi
    // `ExposureControlConfig::new` saat config dibangun (build gagal lebih
    // dulu bila tidak cocok); di sini cuma melaporkan status akhirnya.
    checks.push(CombinationCheck {
        name: "Panjang vektor exposure vs N_ITEMS",
        detail: Ok(match exp_config {
            Some(_) => format!("panjang == {N_ITEMS} (divalidasi ExposureControlConfig::new)"),
            None => "tidak relevan -- exposure control nonaktif".into(),
        }),
        fatal: false,
    });

    // #4: panjang cutoff/hipotesis klasifikasi vs NFACT.
    let classify_detail = match &stop_config.classify {
        ClassifyType::None => {
            Ok("classify nonaktif -- min_SEM yang dipakai, tidak ada dimensi hipotesis untuk diperiksa".into())
        }
        ClassifyType::Ci { cutoffs, .. } => {
            if cutoffs.len() == NFACT {
                Ok(format!("cutoffs panjang {} == NFACT", cutoffs.len()))
            } else {
                Err(format!(
                    "cutoffs panjang {} != NFACT ({NFACT}) -- evaluate() akan Err \
                     DimensionMismatch begitu min_items tercapai",
                    cutoffs.len()
                ))
            }
        }
        ClassifyType::Sprt { lower, upper, .. } => lower
            .validate_nfact(NFACT, "StoppingConfig::classify Sprt.lower")
            .and_then(|()| upper.validate_nfact(NFACT, "StoppingConfig::classify Sprt.upper"))
            .map(|()| format!("lower/upper panjang {NFACT} == NFACT"))
            .map_err(|e| e.to_string()),
    };
    checks.push(CombinationCheck {
        name: "Panjang cutoff/hipotesis klasifikasi vs NFACT",
        detail: classify_detail,
        fatal: true,
    });

    // #5 (peringatan, bukan larangan keras): Method::Fixed pada sesi adaptif.
    if est_config.method == Method::Fixed {
        checks.push(CombinationCheck {
            name: "Method::Fixed x stopping rule presisi",
            detail: Err(
                "Method::Fixed tidak pernah memperbarui theta/SE (lih. ability_estimation::fixed) \
                 -- target presisi (min_SEM/delta_thetas/classify) TIDAK AKAN PERNAH terpenuhi \
                 secara alami, sesi akan selalu berjalan sampai max_items. BUKAN kombinasi \
                 terlarang (API tetap menerimanya, cocok utk kuesioner non-adaptif), cuma \
                 kemungkinan besar bukan yang dimaksud."
                    .into(),
            ),
            fatal: false,
        });
    }

    checks
}

/// Cetak laporan `checks`, lalu `process::exit(1)` bila ADA pemeriksaan
/// `fatal` yang gagal -- sesi tidak pernah mulai berjalan dengan kombinasi
/// terlarang.
fn print_report_and_enforce(checks: &[CombinationCheck]) {
    println!("=== Validasi kombinasi algoritma/metode (sebelum sesi berjalan) ===");
    let mut any_fatal_failed = false;
    for c in checks {
        let (tag, msg) = match &c.detail {
            Ok(m) => ("OK   ", m.as_str()),
            Err(m) if c.fatal => {
                any_fatal_failed = true;
                ("GAGAL", m.as_str())
            }
            Err(m) => ("WARN ", m.as_str()),
        };
        println!("[{tag}] {:<42} -- {msg}", c.name);
    }
    println!();
    if any_fatal_failed {
        eprintln!(
            "Sesi DIBATALKAN: ada kombinasi terlarang di atas ([GAGAL]). Ganti salah satu \
             profil di bagian \"GANTI DI SINI\" (`main()`) lalu jalankan ulang."
        );
        std::process::exit(1);
    }
}

/// Jelaskan kenapa sesi berhenti di langkah terakhir, dari `StopDecision`
/// (Fase 4) + `StoppingConfig`/`PersonState` yang menghasilkannya --
/// `terminated_early` membedakan berhenti karena kriteria PRESISI
/// (`min_SEM`/`delta_thetas`/klasifikasi) vs BATAS (`max_items`/`max_time`,
/// lih. docstring `StopDecision::terminated_early`). Beberapa kriteria bisa
/// terpenuhi BERSAMAAN di langkah yang sama (mis. SEM tercapai TEPAT saat
/// `max_items` juga tercapai) -- semuanya dicantumkan, bukan cuma satu.
fn explain_stop_reason(
    decision: &StopDecision,
    stop_config: &StoppingConfig,
    person: &PersonState,
) -> String {
    let mut reasons = Vec::new();

    if decision.terminated_early {
        match &stop_config.classify {
            ClassifyType::None => {
                if !decision.met_sem.is_empty() && decision.met_sem.iter().all(|&m| m) {
                    reasons.push(format!(
                        "SEM semua dimensi sudah <= ambang min_SEM (SE aktual={:?}, ambang={:?})",
                        person.se().as_slice(),
                        stop_config.min_sem
                    ));
                }
            }
            ClassifyType::Ci { cutoffs, .. } => {
                if decision
                    .met_classify
                    .as_ref()
                    .is_some_and(|met| met.iter().all(|&m| m))
                {
                    reasons.push(format!(
                        "klasifikasi CI: semua dimensi sudah punya keputusan pasti terhadap \
                         cutoff {cutoffs:?} (hasil={:?})",
                        decision.classification
                    ));
                }
            }
            ClassifyType::Sprt { lower, upper, .. } => {
                if decision
                    .met_classify
                    .as_ref()
                    .is_some_and(|met| met.first().copied().unwrap_or(false))
                {
                    reasons.push(format!(
                        "SPRT mencapai keputusan antara hipotesis lower={:?} vs upper={:?} \
                         (hasil={:?})",
                        lower.as_slice(),
                        upper.as_slice(),
                        decision.classification
                    ));
                }
            }
        }

        if !decision.met_delta_thetas.is_empty() && decision.met_delta_thetas.iter().all(|&m| m) {
            reasons.push(format!(
                "|delta theta| semua dimensi sudah di bawah ambang delta_thetas={:?}",
                stop_config.delta_thetas
            ));
        }
    }

    if person.n_answered() >= stop_config.max_items {
        reasons.push(format!(
            "jumlah item terjawab ({}) sudah mencapai max_items ({})",
            person.n_answered(),
            stop_config.max_items
        ));
    }
    // max_time jarang relevan di contoh ini (default `StoppingProfile` di
    // atas selalu None), tapi tetap diperiksa supaya penjelasan lengkap
    // bila diedit.
    if let Some(max_time) = stop_config.max_time {
        if person.total_time() >= max_time {
            reasons.push(format!(
                "total waktu sesi sudah mencapai max_time ({max_time:?})"
            ));
        }
    }

    if reasons.is_empty() {
        // Seharusnya tidak pernah terjadi (stop_now true tapi tidak ada
        // kriteria presisi/batas yang cocok) -- fallback informatif kalau
        // logika di atas pernah tidak sinkron dengan `evaluate()`.
        "tidak diketahui -- stop_now true tapi tidak ada kriteria presisi/batas yang cocok \
         (periksa StopDecision mentah di bawah)"
            .to_string()
    } else {
        reasons.join("; DAN ")
    }
}

fn main() {
    // ========================================================================
    // GANTI DI SINI untuk mencoba kombinasi lain -- lih. docstring modul
    // untuk dua contoh kombinasi TERLARANG siap dicoba.
    // ========================================================================
    let selection_profile = SelectionProfile::DRule;
    let exposure_profile = ExposureProfile::Randomesque;
    let estimation_profile = EstimationProfile::Map;
    let stopping_profile = StoppingProfile::MinSem;
    // ========================================================================

    let mut bank_rng = StdRng::seed_from_u64(300);
    let (bank, a_strong) = build_bank(&mut bank_rng);
    let group = build_group();

    let sel_config = selection_profile.build();
    let cb_config = build_content_balancing_config(&bank);
    let exp_config = exposure_profile
        .build(&a_strong)
        .expect("build_exposure_config: vektor exposure salah panjang -- lih. pemeriksaan #3");
    let est_config = estimation_profile.build();
    let stop_config = stopping_profile.build();

    let checks = validate_combination(
        &sel_config,
        &cb_config,
        exp_config.as_ref(),
        &est_config,
        &stop_config,
        &bank,
        &group,
    );
    print_report_and_enforce(&checks);

    let theta_true = ThetaVector::from_vec(vec![0.5, -1.0, 0.9]);
    let mut session_rng = StdRng::seed_from_u64(8);

    let initial_config = InitialThetaConfig::default(); // ThetaStart::Zero
    let (theta0, se0, _weights, _) = initialize(&initial_config, &group, 0).unwrap();
    let mut person = PersonState::from_theta_and_se(theta0, se0);
    let mut valid_items = vec![true; N_ITEMS]; // state Fase 5 (SH), bertahan sepanjang sesi

    println!(
        "Sesi MCAT 3 dimensi ({DIM_NAMES:?}), bank {N_ITEMS} item (100/dimensi). Profil aktif: \
         seleksi={selection_profile:?}, exposure={exposure_profile:?}, \
         estimasi={estimation_profile:?}, stopping={stopping_profile:?}."
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

        // Exposure control HARUS lewat rank_candidates + select_with_exposure
        // (Fase 5); tanpa exposure control, select_next_item (Fase 2)
        // menangani SEMUA kriteria termasuk Seq/Random langsung -- pola
        // sama seperti `07_final_result.rs` (tanpa exposure) vs `08_...`
        // (dengan exposure).
        let chosen = match &exp_config {
            Some(cfg) => {
                let scores = rank_candidates(&sel_config, &narrowed, &person, &bank, &group).unwrap();
                select_with_exposure(cfg, &scores, person.n_answered(), &mut valid_items, &mut session_rng)
                    .unwrap()
                    .0
            }
            None => {
                select_next_item(&sel_config, &narrowed, &person, &bank, &group, &mut session_rng)
                    .unwrap()
                    .0
            }
        };

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

    println!();
    match &exp_config {
        Some(cfg) => {
            let n_high_disc = a_strong.iter().filter(|&&a| a > 1.3).count();
            match &cfg.mode {
                ExposureMode::SympsonHetter(_) => {
                    let high_disc_chosen = person
                        .answered_items()
                        .iter()
                        .filter(|&&idx| a_strong[idx] > 1.3)
                        .count();
                    println!(
                        "=== Exposure Control (Sympson-Hetter): item berdiskriminasi tinggi (>1.3) \
                         yang lolos === \n{high_disc_chosen}/{} item terjawab (dari {n_high_disc} \
                         item bank yang memenuhi >1.3 -- exposure=0.25 MEMBATASI peluang lolos \
                         tiap dicoba, bukan melarang mutlak)",
                        person.n_answered()
                    );
                }
                ExposureMode::Randomesque(_) => {
                    println!(
                        "=== Exposure Control (Randomesque, top-3 tiap posisi): aktif -- item \
                         terpilih tiap langkah diambil uniform random dari 3 kandidat \
                         berperingkat kriteria teratas (bukan selalu argmax tunggal seperti SH) ==="
                    );
                }
            }
        }
        None => println!("=== Exposure Control: nonaktif untuk kombinasi profil ini ==="),
    }

    println!();
    println!("=== Alasan Berhenti ===");
    println!(
        "Sesi berhenti di item ke-{} karena: {}.",
        person.n_answered(),
        explain_stop_reason(&stop_decision, &stop_config, &person)
    );
    println!(
        "terminated_early = {} (true = dipicu kriteria presisi min_SEM/delta_thetas/klasifikasi; \
         false = dipicu batas max_items/max_time, presisi belum tentu tercapai)",
        stop_decision.terminated_early
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
