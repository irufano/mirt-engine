# mirt-engine — Rencana Implementasi

> Engine Rust untuk **Multidimensional Computerized Adaptive Testing (MCAT)**,
> mengacu pada algoritma dan perilaku library **mirt** dan **mirtCAT** (R),
> dipersempit khusus ke skenario `nfact > 1` (multidimensional IRT).
> Didesain agar bisa dipakai sebagai dependency biasa (`cargo add mirt-engine`)
> di web API Rust (mis. Axum) maupun project Rust lain, tanpa terikat framework
> tertentu.

Status dokumen: **draft rencana**, belum ada implementasi. Update checklist
`[ ]` → `[x]` di tiap item begitu item itu benar-benar selesai dikerjakan &
lulus test — jangan tandai selesai berdasarkan asumsi.

---

## 1. Ringkasan & Tujuan

`mirt-engine` menyediakan **mesin administrasi tes adaptif multidimensional**
(item selection, ability estimation, stopping rule, exposure control, content
balancing) sebagai library murni (`no_std`-friendly di level algoritma inti,
walau realistisnya akan pakai `std` untuk kesederhanaan awal). Library ini
**tidak** melakukan kalibrasi model IRT dari data respons mentah (itu wilayah
`mirt`, algoritma EM/MHRM) — engine ini **menerima parameter item yang sudah
terkalibrasi** (slope/discrimination per-dimensi, intercept/difficulty,
guessing, dst.) sebagai input, lalu menjalankan siklus adaptif di atasnya.

### 1.1 Tujuan

- Port **perilaku dan hasil numerik** MCAT dari `mirt` + `mirtCAT` ke Rust,
  seakurat mungkin (divalidasi silang dengan fixture yang dihasilkan dari R).
- API murni Rust (trait + struct), **framework-agnostic**: tidak ada
  dependency ke Axum/Actix/dll di crate inti.
- Tiap algoritma dapat diuji, diverifikasi, dan didemonstrasikan secara
  independen (unit test, integration test, example script).
- Landasan teori tiap algoritma didokumentasikan dengan rujukan akademik,
  bukan hanya rujukan ke source code R.

### 1.2 Non-Tujuan (Out of Scope)

- **Kalibrasi/estimasi parameter item** (EM, MH-RM, marginal maximum
  likelihood atas data respons) — itu domain `mirt`, bukan `mirtCAT`/engine
  ini. Engine hanya *mengonsumsi* parameter yang sudah dikalibrasi.
- **Shadow testing** / optimal test design berbasis `lpSolve` (fitur
  tambahan `mirtCAT` untuk constraint kompleks) — di luar 7 spesifikasi yang
  diminta, dipertimbangkan sebagai fase lanjutan di luar dokumen ini.
- Antarmuka HTML/Shiny (`createShinyGUI`, `ui.R`, `server.R`) — di luar
  scope; posisi library ini setara `mirtCAT::Design`/`TestClass`/
  `PersonClass` + `findNextCATItem()`/`updateDesign()`, bukan lapisan GUI.
- Model IRT unidimensional non-MCAT (CAT 1 dimensi) tidak menjadi target
  utama, meski fondasi IRT (Fase 0) dibuat cukup umum sehingga kasus
  `nfact == 1` otomatis valid sebagai kasus khusus.

### 1.3 Catatan Lisensi (penting, perlu keputusan pengguna)

`mirt` dan `mirtCAT` berlisensi **GPL (>= 3)**. Agar `mirt-engine` bisa
dipakai bebas di project Rust lain (mis. dengan lisensi MIT/Apache-2.0),
implementasi **harus berupa clean-room reimplementation** dari rumus/algoritma
yang dipublikasikan di paper akademik (Segall 1996, Chalmers 2012/2016, dst.),
**bukan** menerjemahkan baris demi baris source `.R`/`.cpp` milik `mirt`/
`mirtCAT`. Source R dipakai di rencana ini semata sebagai **rujukan perilaku
yang harus disamai** (nama kriteria, urutan pipeline, semantik parameter),
bukan sebagai kode yang disalin.

- [ ] Keputusan lisensi crate dikonfirmasi pengguna (rekomendasi: dual
      `MIT OR Apache-2.0`, standar ekosistem Rust).

---

## 2. Prinsip Desain

1. **Faithful port, idiomatic Rust** — nama/semantik kriteria & parameter
   mengikuti `mirtCAT` (`Drule`, `TPrule`, `KLn`, `min_SEM`, dst.) supaya
   pengguna yang familiar dengan `mirtCAT` merasa "di rumah", tapi API-nya
   type-safe (enum, bukan string bebas; struct config, bukan `list()`).
2. **Framework-agnostic core** — tanpa dependency web framework di
   `dependencies` maupun `dev-dependencies`.
3. **Aljabar linear via `nalgebra`** — dipakai untuk matriks informasi
   Fisher `nfact × nfact` (determinan, trace, invers, eigenvalue) — setara
   fungsi `det()`/`solve()`/`eigen()` yang dipakai `ComputeCriteria` di sisi
   C++ `mirtCAT`.
4. **Serialisasi opsional** — `serde` di belakang feature flag `serde`, agar
   struct hasil (Fase 7) mudah diserialisasi jadi JSON di web API tanpa
   memaksa dependency ke pengguna yang tidak butuh.
5. **Validasi ketat & error eksplisit** — batasan yang di R berupa
   `stop(...)` (mis. kriteria tidak valid untuk `nfact > 1`, content
   balancing tidak kompatibel dengan `seq`) diterjemahkan jadi varian
   `enum MirtCatError` yang dicek saat konstruksi konfigurasi, bukan panic.
6. **Parity testing dengan R sebagai ground truth** — tiap fase yang punya
   perhitungan numerik wajib punya *fixture* hasil dari R (`mirt`/`mirtCAT`,
   seed & item bank tetap) yang disimpan di `tests/fixtures/`, dan
   integration test membandingkan output Rust vs R dengan toleransi numerik
   kecil (mis. `1e-6`). Ini cara konkret memenuhi syarat "harus sesuai
   dengan mirt dan mirtCAT".
7. **Debug/trace kalkulasi opsional, default nonaktif** — tiap fungsi
   algoritma pengambil-keputusan (item selection, ability estimation,
   stopping rule, exposure control, content balancing; Fase 1–6) menerima
   field `debug: bool` pada config-nya (default `false` lewat `Default`),
   dan mengembalikan nilai utama **plus** `Option<XxxDebug>` — `None` saat
   `debug=false` (tidak ada alokasi/komputasi tambahan di jalur ini, murni
   penambahan satu varian enum kosong), `Some(XxxDebug)` berisi nilai
   antara yang relevan (mis. nilai kriteria tiap kandidat item, titik
   kuadratur & log-likelihood tiap iterasi, evaluasi per-dimensi stopping
   rule) saat `debug=true`. Aktivasi debug **tidak boleh** mengubah nilai
   utama yang dikembalikan — hanya menambah observability. Struct
   `XxxDebug` per-fase ikut aturan serialisasi opsional (prinsip #4) supaya
   bisa diekspos oleh pemanggil eksternal (mis. lewat endpoint "debug" di
   web API) untuk audit/transparansi keputusan adaptif. Fase 0 (fondasi
   IRT) dikecualikan —
   `ItemResponseModel::probabilities()`/`information()` sudah *mengembalikan*
   seluruh hasil kalkulasi secara langsung, tidak ada keputusan tersembunyi
   yang perlu di-debug.

---

## 3. Struktur Direktori (target akhir)

```
mirt-engine/
├── Cargo.toml
├── PLAN.md                        # dokumen ini
├── README.md
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── theta.rs                   # ThetaVector, SEVector, InfoMatrix (newtypes atas nalgebra)
│   ├── irt/                       # Fase 0 — fondasi model respons item
│   │   ├── mod.rs
│   │   ├── model.rs                # trait ItemResponseModel
│   │   ├── dichotomous.rs          # M2PL/M3PL/M4PL multidimensional
│   │   ├── graded.rs               # graded response model (polytomous)
│   │   ├── probability.rs          # padanan probtrace()
│   │   └── information.rs          # Fisher information per-item & kumulatif
│   ├── bank.rs                     # ItemBank, ItemMeta (content, exposure), GroupParams
│   ├── initial_theta/mod.rs        # Fase 1
│   ├── item_selection/
│   │   ├── mod.rs                  # Fase 2 — trait SelectionCriterion + validasi nfact>1
│   │   └── criteria/
│   │       ├── seq.rs
│   │       ├── random.rs
│   │       ├── kl.rs                # KL & KLn
│   │       ├── d_rule.rs            # Drule / DPrule
│   │       ├── t_rule.rs            # Trule / TPrule
│   │       ├── a_rule.rs            # Arule / APrule
│   │       ├── e_rule.rs            # Erule / EPrule
│   │       └── w_rule.rs            # Wrule / WPrule
│   ├── ability_estimation/
│   │   ├── mod.rs                  # Fase 3
│   │   ├── quadrature.rs           # grid kuadratur + Quasi-Monte Carlo
│   │   ├── eap.rs
│   │   ├── map.rs
│   │   ├── ml.rs
│   │   ├── wle.rs
│   │   └── eap_sum.rs
│   ├── stopping_rule/
│   │   ├── mod.rs                  # Fase 4
│   │   ├── sem.rs
│   │   ├── delta_theta.rs
│   │   ├── classify_ci.rs
│   │   ├── sprt.rs
│   │   └── max_items_time.rs
│   ├── exposure_control/
│   │   ├── mod.rs                  # Fase 5
│   │   ├── sympson_hetter.rs
│   │   └── randomesque.rs          # mode 'sample' (top-n random)
│   ├── content_balancing/mod.rs    # Fase 6
│   └── result.rs                   # McatResult — Fase 7
├── examples/
│   ├── 00_irt_probability_and_information.rs
│   ├── 01_initial_theta.rs
│   ├── 02_item_selection_seq.rs
│   ├── 02_item_selection_random.rs
│   ├── 02_item_selection_kl.rs      # KL & KLn (satu modul `criteria/kl.rs`, satu contoh)
│   ├── 02_item_selection_d_rule.rs
│   ├── 02_item_selection_t_rule.rs
│   ├── 02_item_selection_a_rule.rs
│   ├── 02_item_selection_e_rule.rs
│   ├── 02_item_selection_w_rule.rs
│   ├── 03_ability_estimation_eap_map_ml_wle.rs
│   ├── 04_stopping_rule_min_sem.rs
│   ├── 04_stopping_rule_delta_thetas.rs
│   ├── 04_stopping_rule_ci.rs
│   ├── 04_stopping_rule_sprt.rs
│   ├── 04_stopping_rule_max_items_time.rs
│   ├── 05_exposure_control_sympson_hetter.rs
│   ├── 05_exposure_control_randomesque.rs
│   ├── 06_content_balancing.rs
│   └── 07_final_result.rs
├── tests/
│   ├── fixtures/
│   │   ├── generate_r_fixtures.R   # skrip R — sumber ground truth
│   │   └── *.json                  # snapshot hasil R
│   └── integration_*.rs
├── benches/                        # opsional, criterion — Fase 8
└── docs/
    ├── teori/                      # landasan teori + referensi akademik
    │   ├── 00-fondasi-irt-multidimensional.md
    │   ├── 01-theta-awal.md
    │   ├── 02-item-selection-overview.md   # kenapa nfact>1 membatasi kriteria, criteria_estimator
    │   ├── 02-item-selection-seq.md
    │   ├── 02-item-selection-random.md
    │   ├── 02-item-selection-kl.md          # KL & KLn (satu family teori, Chang & Ying 1996)
    │   ├── 02-item-selection-d-rule.md
    │   ├── 02-item-selection-t-rule.md
    │   ├── 02-item-selection-a-rule.md
    │   ├── 02-item-selection-e-rule.md
    │   ├── 02-item-selection-w-rule.md
    │   ├── 03-ability-estimation.md
    │   ├── 04-stopping-rule-overview.md
    │   ├── 04-stopping-rule-min-sem.md
    │   ├── 04-stopping-rule-delta-thetas.md
    │   ├── 04-stopping-rule-ci.md
    │   ├── 04-stopping-rule-sprt.md
    │   ├── 04-stopping-rule-max-items-time.md
    │   ├── 05-exposure-control-overview.md
    │   ├── 05-exposure-control-sympson-hetter.md
    │   ├── 05-exposure-control-randomesque.md
    │   ├── 06-content-balancing.md
    │   └── 07-hasil-akhir-theta.md
    └── panduan/                    # cara pakai (how-to)
        ├── 00-instalasi-dan-quickstart.md
        ├── 01-theta-awal.md
        ├── 02-item-selection-overview.md   # cara memilih Criterion, kapan weights/*Prule
        ├── 02-item-selection-seq.md
        ├── 02-item-selection-random.md
        ├── 02-item-selection-kl.md
        ├── 02-item-selection-d-rule.md
        ├── 02-item-selection-t-rule.md
        ├── 02-item-selection-a-rule.md
        ├── 02-item-selection-e-rule.md
        ├── 02-item-selection-w-rule.md
        ├── 03-ability-estimation.md
        ├── 04-stopping-rule-overview.md
        ├── 04-stopping-rule-min-sem.md
        ├── 04-stopping-rule-delta-thetas.md
        ├── 04-stopping-rule-ci.md
        ├── 04-stopping-rule-sprt.md
        ├── 04-stopping-rule-max-items-time.md
        ├── 05-exposure-control-overview.md
        ├── 05-exposure-control-sympson-hetter.md
        ├── 05-exposure-control-randomesque.md
        ├── 06-content-balancing.md
        └── 07-hasil-akhir-theta.md
```

---

## 4. Pemetaan Spesifikasi → Fase

| # | Spesifikasi (permintaan pengguna) | Fase | Rujukan kode R utama |
|---|---|---|---|
| — | Fondasi model IRT & struktur data (prasyarat) | **Fase 0** | `mirt/R/probtrace.R`, `mirt/R/iteminfo.R`, `mirt/src/traceLinePts.cpp` |
| 1 | Theta Awal (Initial Theta) | **Fase 1** | `mirtCAT/R/PersonClass.R:26-52`, `TestClass.R:64,73,76` |
| 2 | Algoritma Pemilihan Item (Item Selection) | **Fase 2** | `mirtCAT/R/findNextCATItem.R`, `selection_criteria.R`, `DesignClass.R:113-117` |
| 3 | Algoritma Estimasi Kemampuan (Ability Estimation) | **Fase 3** | `mirtCAT/R/DesignClass.R:57-94`, `mirtCAT_preamble.R:184-191` |
| 4 | Stopping Rule | **Fase 4** | `mirtCAT/R/DesignClass.R:312-381` (`Update.stop_now`) |
| 5 | Exposure Control | **Fase 5** | `mirtCAT/R/DesignClass.R:214-227`, `findNextCATItem.R:361-383` |
| 6 | Content Balancing | **Fase 6** | `mirtCAT/R/DesignClass.R:157-166`, `findNextCATItem.R:346-360` |
| 7 | Hasil Bersih Akhir untuk Theta | **Fase 7** | `mirtCAT/R/mirtCAT_preamble.R:245-285` (`mirtCAT_post_internal`) |
| — | Hardening, parity testing menyeluruh, rilis | **Fase 8** | — |

**Urutan pengerjaan wajib sekuensial** (Fase 0 → 8) karena tiap fase
bergantung pada tipe/trait dari fase sebelumnya. Tidak semua fase dikerjakan
sekaligus — checklist tiap fase harus 100% (termasuk test & dokumentasi)
sebelum fase berikutnya dimulai.

---

## 5. Legenda Status

- `⬜ Belum Dimulai` — belum ada pekerjaan.
- `🟡 Sedang Dikerjakan` — sebagian checklist tercentang.
- `✅ Selesai` — seluruh checklist (implementasi, unit test, integration
  test, example, dokumentasi, Definition of Done) tercentang dan
  `cargo test` + `cargo clippy` hijau.

| Fase | Nama | Status |
|---|---|---|
| 0 | Fondasi Proyek & Model IRT Dasar | ✅ Selesai |
| 1 | Theta Awal | ✅ Selesai |
| 2 | Algoritma Pemilihan Item | ✅ Selesai |
| 3 | Algoritma Estimasi Kemampuan | ✅ Selesai |
| 4 | Stopping Rule | ✅ Selesai |
| 5 | Exposure Control | ✅ Selesai |
| 6 | Content Balancing | ✅ Selesai |
| 7 | Hasil Bersih Akhir untuk Theta | ✅ Selesai |
| 8 | Hardening, Validasi Silang & Rilis | ✅ Selesai |

> Update tabel ini setiap kali status sebuah fase berubah.

---

## Fase 0 — Fondasi Proyek & Model IRT Dasar (Prasyarat)

**Status: ✅ Selesai**

> **Update (2026-09-22)**: `R` (4.6.1) + paket `mirt` (1.47) & `mirtCAT`
> (1.14) sudah terpasang (`brew install r`, lalu
> `install.packages(c("jsonlite","mirt","mirtCAT"))`) dan
> `tests/fixtures/generate_r_fixtures.R` sudah dijalankan sungguhan —
> fixture `tests/fixtures/irt_probability_information.json` sekarang
> adalah ground truth `mirt::probtrace()`/
> `mirt::iteminfo(multidim_matrix=TRUE)` yang sesungguhnya, dan
> `cargo test` (termasuk `tests/integration_irt.rs`) hijau terhadapnya
> (kecocokan sampai belasan digit signifikan, jauh di bawah toleransi
> `1e-6`). Detail teknis (kenapa objek 1-item butuh item "pengisi"/filler
> supaya lolos pengecekan derajat-kebebasan `mirt`) ada di komentar
> `generate_r_fixtures.R`.

### Tujuan
Menyediakan tipe data inti (`ThetaVector`, `InfoMatrix`), trait model respons
item, dan implementasi model IRT multidimensional (dikotomus & politomus)
yang menjadi fondasi bagi Fase 1–7. Tanpa ini, tidak ada perhitungan
probabilitas/informasi yang bisa dilakukan oleh kriteria seleksi item maupun
estimator kemampuan.

### Rujukan Kode Sumber (R)
- `mirt/R/probtrace.R`, `mirt/R/iteminfo.R` — antarmuka probabilitas &
  informasi per-item.
- `mirt/src/traceLinePts.cpp`, `mirt/src/dpars.cpp` — perhitungan trace line
  & turunan untuk model dikotomus/politomus multidimensional.
- `mirt/R/MDIFF.R`, `mirt/R/MDISC.R` — parameterisasi discrimination
  multidimensional (arah & jarak vektor `a`).

### Landasan Teori & Referensi Akademik
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*. Springer.
  (bab 2–4: model M2PL/M3PL, informasi Fisher multidimensional)
- Reckase, M. D. (1985). The difficulty of test items that measure more
  than one ability. *Applied Psychological Measurement*, 9(4), 401–412.
- Muraki, E., & Carlson, J. E. (1995). Full-information factor analysis for
  polytomous item responses. *Applied Psychological Measurement*, 19(1),
  73–90. (graded response multidimensional)
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29. doi:10.18637/jss.v048.i06

### Desain Modul Rust
- `theta::ThetaVector`, `theta::SEVector` — newtype atas `nalgebra::DVector<f64>`.
- `theta::InfoMatrix` — newtype atas `nalgebra::DMatrix<f64>` (persegi, `nfact × nfact`), dengan method `determinant()`, `trace()`, `trace_weighted(&[f64])`, `inverse()`, `min_eigenvalue()`.
- `irt::model::ItemResponseModel` — trait: `n_categories() -> usize`, `probabilities(&ThetaVector) -> Vec<f64>`, `information(&ThetaVector) -> InfoMatrix`.
- `irt::dichotomous::Dichotomous` — parameter `a: Vec<f64>` (slope per dimensi), `d: f64`, `g: f64` (guessing), `u: f64` (upper asymptote) → mencakup M2PL/M3PL/M4PL sebagai kasus khusus.
- `irt::graded::GradedResponse` — parameter `a: Vec<f64>`, `d: Vec<f64>` (boundary per kategori).
- `bank::ItemBank` — kumpulan `Box<dyn ItemResponseModel>` + metadata per item (`ItemMeta { content: Option<String>, id: usize }`).
- `bank::GroupParams` — `gmeans: ThetaVector`, `gcov: InfoMatrix` (matriks kovarians populasi, dari model terkalibrasi).
- `error::MirtCatError` — enum error terpusat dipakai lintas fase.

### Checklist Implementasi
- [x] `theta.rs`: `ThetaVector`, `SEVector`, `InfoMatrix` + operasi dasar (det, trace, trace berbobot, invers, eigenvalue minimum)
- [x] `irt/model.rs`: trait `ItemResponseModel`
- [x] `irt/probability.rs`: fungsi probabilitas kategori (padanan `probtrace()`)
- [x] `irt/dichotomous.rs`: model M2PL/M3PL/M4PL multidimensional
- [x] `irt/graded.rs`: model graded response multidimensional
- [x] `irt/information.rs`: informasi Fisher per-item (matriks `nfact × nfact`) & agregasi kumulatif
- [x] `bank.rs`: `ItemBank`, `ItemMeta`, `GroupParams`
- [x] `error.rs`: `MirtCatError` (variant awal: `DimensionMismatch`, `InvalidParameter`)
- [x] Setup `Cargo.toml` (deps: `nalgebra`, `rand`, `thiserror`; dev-deps: `approx`, `serde_json` untuk fixture)
- [x] Setup CI dasar (GitHub Actions: `cargo fmt --check`, `cargo clippy`, `cargo test`) — `.github/workflows/ci.yml`

### Checklist Unit Test
- [x] Probabilitas kategori model dikotomus (2PL/3PL/4PL) menjumlah ke 1
- [x] Probabilitas kategori graded response menjumlah ke 1 & monoton sesuai boundary
- [x] Informasi Fisher item dikotomus konsisten dengan turunan analitik (uji terhadap rumus tertutup untuk kasus 1 dimensi sebagai sanity check)
- [x] Operasi `InfoMatrix` (det, trace, trace berbobot, invers, eigenvalue minimum) terhadap matriks known-value (mis. matriks identitas, diagonal)
- [x] `ThetaVector`/`SEVector` — konstruksi, indexing, validasi dimensi (`nfact` mismatch → error)

### Checklist Integration Test
- [x] Fixture R: skrip `tests/fixtures/generate_r_fixtures.R` ditulis (memakai `mirtCAT::generate.mirt_object()` + `mirt::probtrace()`/`mirt::iteminfo()`) dan **sudah dijalankan sungguhan** (R 4.6.1, mirt 1.47, mirtCAT 1.14) — hasilnya disimpan di `tests/fixtures/irt_probability_information.json`
- [x] Integration test Rust membaca fixture JSON tsb dan membandingkan hasil `ItemResponseModel` Rust vs R sungguhan (toleransi `1e-6`) — `tests/integration_irt.rs`, hijau

### Checklist Example Script
- [x] `examples/00_irt_probability_and_information.rs` — mendefinisikan beberapa item 2D/3D, mencetak probabilitas & matriks informasi pada beberapa titik theta

### Checklist Dokumentasi
- [x] `docs/teori/00-fondasi-irt-multidimensional.md` — model M2PL/M3PL/graded multidimensional, definisi informasi Fisher, referensi akademik
- [x] `docs/panduan/00-instalasi-dan-quickstart.md` — instalasi crate, definisi item bank pertama, quickstart singkat

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] `cargo test` & `cargo clippy --all-targets -- -D warnings` hijau
- [x] Fixture R tervalidasi manual — dibandingkan ke output `mirt`/`mirtCAT` sungguhan (R 4.6.1/mirt 1.47/mirtCAT 1.14), kecocokan sampai belasan digit signifikan, dan silang-cocok dengan kalkulator Python independen

---

## Fase 1 — Theta Awal (Initial Theta)

**Status: ✅ Selesai**

> **Update (2026-09-22)**: perilaku diverifikasi langsung terhadap source
> `mirtCAT` 1.14 terpasang (`getRefClass("Person")$methods("initialize")`,
> `getFromNamespace("run_local", "mirtCAT")`) — `theta0` literal nol,
> `SE0 = sqrt(diag(gcov))` dihitung independen dari override. Fixture
> `tests/fixtures/generate_r_fixtures_initial_theta.R` menjalankan pipeline
> `mirtCAT()` off-line sungguhan (bukan hanya pembacaan source) pada model
> 2D & 3D dengan `gcov` non-diagonal, kasus default & override, disimpan di
> `tests/fixtures/initial_theta.json`; `tests/integration_initial_theta.rs`
> hijau terhadapnya (toleransi `1e-6`, kecocokan jauh lebih ketat pada
> praktiknya karena nilainya eksak/rasional). `cargo test` (45 test) &
> `cargo clippy --all-targets -- -D warnings` hijau.

### Tujuan
Menentukan titik awal vektor kemampuan (`theta₀`) dan SE awal sebelum item
pertama disajikan, sesuai perilaku `mirtCAT` untuk `nfact > 1`.

### Rujukan Kode Sumber (R)
- `mirtCAT/R/PersonClass.R:26-52` (`Person$initialize`)
- `mirtCAT/R/TestClass.R:64,73,76` (ekstraksi `gp$gmeans`, `gp$gcov` dari model terkalibrasi)

### Perilaku yang Harus Disamai
- Default `theta₀ = vektor nol` sepanjang `nfact` — **bukan** diasumsikan
  independen; prior yang mendasarinya adalah **normal multivariat**
  `N(gmeans, gcov)` hasil ekstraksi model terkalibrasi (bukan diasumsikan
  `N(0, I)`).
- Dapat dioverride via `thetas_start`: vektor panjang `nfact` (satu peserta)
  atau matriks `N × nfact` (simulasi offline banyak peserta).
- SE awal = `sqrt(diag(gcov))` — hanya diagonal, bukan seluruh matriks
  kovarians.
- `weights` (default `[1.0; nfact]`) diinisialisasi bersamaan, dipakai
  belakangan oleh `Trule`/`TPrule`, `Arule`/`APrule`, `Wrule`/`WPrule` (Fase 2).

### Landasan Teori & Referensi Akademik
- Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
  61(2), 331–354. (prior populasi multivariat sebagai titik berangkat MCAT)
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 3
  (distribusi populasi laten multivariat).

### Desain Modul Rust
- `initial_theta::ThetaStart` — enum `Zero`, `Fixed(ThetaVector)`,
  `PerPersonMatrix(Vec<ThetaVector>)` (untuk simulasi offline).
- `initial_theta::InitialThetaConfig { start: ThetaStart, weights: Option<Vec<f64>>, debug: bool }` — `debug` default `false` (prinsip desain #7).
- `initial_theta::InitialThetaDebug { source: ThetaStart, gmeans_used: ThetaVector, gcov_diag_used: Vec<f64> }` — jalur yang dipakai (`Zero`/`Fixed`/`PerPersonMatrix`) & nilai `gmeans`/`gcov` sumber SE awal.
- `initial_theta::initialize(config: &InitialThetaConfig, group: &GroupParams, person_index: usize) -> Result<(ThetaVector, SEVector, Vec<f64> /* weights */, Option<InitialThetaDebug>), MirtCatError>` — elemen terakhir `Some(_)` hanya bila `config.debug == true`. Parameter `person_index` ditambahkan di atas sketsa awal supaya `ThetaStart::PerPersonMatrix` (simulasi offline banyak peserta) punya cara memilih baris per-peserta yang eksplisit (padanan indeks `n` di `thetas.start_in[n, ]` R); diabaikan untuk `Zero`/`Fixed`. Return dibungkus `Result` (bukan tuple polos) supaya konsisten dengan pola error eksplisit yang sudah dipakai `ItemResponseModel`/`ItemBank` (Fase 0) — juga mengakomodasi item checklist "validasi error dimensi" yang sketsa awal implikasikan tapi tidak tertulis di signature.

### Checklist Implementasi
- [x] `initial_theta::ThetaStart` (enum) & `InitialThetaConfig`
- [x] Fungsi `initialize()`: theta nol default, override via `Fixed`/`PerPersonMatrix`
- [x] SE awal dari `sqrt(diag(gcov))`
- [x] Default `weights = vec![1.0; nfact]`, validasi panjang `weights == nfact` jika diberikan manual
- [x] Dukungan debug trace opsional (`InitialThetaConfig::debug`, default `false`) → `Option<InitialThetaDebug>`

### Checklist Unit Test
- [x] Default (tanpa override) menghasilkan theta nol sepanjang `nfact`
- [x] Override `Fixed(theta)` dipakai apa adanya
- [x] SE awal sama dengan akar diagonal `gcov` untuk beberapa matriks kovarians contoh (termasuk kovarians non-diagonal, memastikan hanya diagonal yang diambil)
- [x] Validasi error saat dimensi `thetas_start`/`weights` tidak cocok dengan `nfact`
- [x] `debug=false` → elemen ke-4 `None`; `debug=true` → `Some(InitialThetaDebug)` terisi dan tidak mengubah `theta`/SE/`weights` yang dikembalikan

### Checklist Integration Test
- [x] Fixture R: `mirtCAT` diinisialisasi dengan model 2–3 dimensi terkalibrasi (tanpa override) → ekstrak `theta₀`, `SE₀` dari objek `Person`/`Test`, simpan JSON — `tests/fixtures/generate_r_fixtures_initial_theta.R`, dijalankan sungguhan (R 4.6.1/mirtCAT 1.14) via pipeline `mirtCAT()` off-line sesungguhnya (`thetas_history[1,]`/`thetas_SE_history[1,]` dari objek `Person`), bukan hanya ekstraksi `test@gp`; mencakup kasus default & override, model 2D & 3D dengan `gcov` non-diagonal
- [x] Bandingkan hasil `initial_theta::initialize()` Rust vs fixture R (toleransi `1e-6`) — `tests/integration_initial_theta.rs`, 4 test, hijau

### Checklist Example Script
- [x] `examples/01_initial_theta.rs` — inisialisasi 1 peserta default vs override manual, cetak `theta₀`/`SE₀`/`weights` (juga mendemonstrasikan `PerPersonMatrix` & debug trace)

### Checklist Dokumentasi
- [x] `docs/teori/01-theta-awal.md` — landasan teori prior multivariat + referensi akademik
- [x] `docs/panduan/01-theta-awal.md` — cara set `ThetaStart`, kapan override manual masuk akal

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] `cargo test -p mirt-engine initial_theta` hijau (juga `cargo test` penuh: 45 test hijau, `cargo clippy --all-targets -- -D warnings` bersih, `cargo fmt --check` bersih)
- [x] Parity numerik dengan fixture R terverifikasi

---

## Fase 2 — Algoritma Pemilihan Item (Item Selection) Khusus MCAT

**Status: ✅ Selesai**

> **Update (2026-09-22)**: formula tiap kriteria diverifikasi numerik
> langsung terhadap `mirtCAT` terpasang (R 4.6.1/mirtCAT 1.14) lewat
> `computeCriteria()` pada item bank sintetis (bukan hanya pembacaan
> source) — dua temuan penting yang **tidak** tersirat eksplisit di
> rencana awal fase ini:
> 1. **`Arule`/`APrule` butuh pseudo-invers (SVD), bukan invers biasa** —
>    info Fisher satu item dikotomus tunggal di `nfact > 1` selalu rank 1,
>    umum singular sebelum item pertama dijawab; `mirtCAT` menanganinya
>    lewat rutin aljabar linear yang robust terhadap singularitas, hasil
>    numeriknya cocok presisi tinggi dengan `InfoMatrix::pseudo_inverse`
>    (nalgebra, via SVD). Lih. `docs/teori/02-item-selection-a-rule.md`.
> 2. **`info_thetas` dihitung ULANG dari nol tiap dipanggil, bukan
>    diakumulasi bertahap** — dikonfirmasi dari source `Person$Update.info_mats`
>    (`mirtCAT`): info tiap item terjawab selalu dievaluasi di `theta`
>    **terbaru**, bukan "dibekukan" di `theta` saat item itu dijawab.
>    `PersonState` di Rust karena itu menyimpan daftar index item terjawab
>    (`answered_items`), bukan matriks akumulasi sebagai field — metode
>    `info_thetas(&bank)` menghitung ulang lewat `irt::cumulative_information`
>    (Fase 0) tiap dipanggil. Ini deviasi dari sketsa "Desain Modul Rust"
>    awal (`update_info_matrix()` inkremental) — lih. §"Desain Modul Rust"
>    di bawah untuk desain final.
>
> Deviasi desain lain (dengan alasan, lih. juga commit/PR terkait):
> tidak ada `MirtCatError::InvalidCriterionForMultidimensional` (enum
> `Criterion` tertutup sudah mencegah konstruksi kriteria tidak valid di
> compile time — validasi runtime jadi tidak pernah bisa gagal, jadi tidak
> ditambahkan); `select_next_item()` menerima parameter tambahan `group:
> &GroupParams` (untuk `*Prule`) dan `rng: &mut impl rand::Rng` (untuk
> `Random`, supaya testable/reproducible) di atas sketsa awal.
>
> `cargo test` (86 test hijau: 76 unit + 4 integration_irt + 4
> integration_initial_theta + 2 integration_item_selection) & `cargo
> clippy --all-targets -- -D warnings` hijau. Fixture R
> (`tests/fixtures/item_selection.json`) dihasilkan sungguhan lewat
> `Rscript tests/fixtures/generate_r_fixtures_item_selection.R`.

### Tujuan
Mengimplementasikan **hanya** kriteria seleksi item yang valid untuk
`nfact > 1` di `mirtCAT`, dengan validasi keras terhadap kriteria yang
tidak didukung (`MI`, `MEPV`, `MLWI`, `MPWI`, `MEI`, seluruh `IKL*`).

### Rujukan Kode Sumber (R)
- `mirtCAT/R/DesignClass.R:113-117` — validasi kriteria valid untuk `nfact > 1`
- `mirtCAT/R/TestClass.R:65-72` — alasan `ThetaGrid`/`density` tidak dibangun untuk `nfact > 1`
- `mirtCAT/R/selection_criteria.R:64-114` — implementasi `Drule`/`Trule`/`Arule`/`Erule`/`Wrule`/`KL`
- `mirtCAT/R/findNextCATItem.R` — pipeline pemanggilan kriteria
- `mirtCAT/R/DesignClass.R:101-106` — `criteria_estimator` otomatis `ML` vs `MAP`

### Perilaku yang Harus Disamai
- **Kriteria valid untuk `nfact > 1`, HANYA**: `seq`, `random`, `KL`, `KLn`,
  `Drule`/`DPrule`, `Trule`/`TPrule`, `Arule`/`APrule`, `Erule`/`EPrule`,
  `Wrule`/`WPrule`. Kriteria lain → error saat konfigurasi, **bukan** saat
  runtime.
- Kelima kriteria D/T/A/E/W bersandar pada **matriks informasi Fisher
  kumulatif `nfact × nfact`** (`info_thetas`), diperbarui tiap item dijawab.
  - `Drule`: determinan matriks informasi (maksimalkan).
  - `Trule`: trace matriks informasi, opsional berbobot `weights` per dimensi.
  - `Arule`: **negatif** trace invers matriks informasi (minimalkan varians
    asimtotik ≈ maksimalkan negatifnya).
  - `Erule`: eigenvalue **minimum** matriks informasi (mencegah satu dimensi
    terabaikan).
  - `Wrule`: kombinasi berbobot kustom (`weights`) — bentuk kuadratik
    penuh `weights' * info * weights` (termasuk off-diagonal), **beda**
    dari `Trule` yang cuma trace diagonal berbobot; dikonfirmasi numerik
    terhadap `mirtCAT::computeCriteria`.
  - Varian `*Prule` menambahkan **informasi prior** (`inverse(gcov)`) ke
    matriks informasi sebelum dievaluasi (pendekatan Bayesian).
  - `weights` dipakai oleh `Trule`/`Arule`/`Wrule` (+varian P), **tidak**
    oleh `Drule`/`Erule`.
- `KL`: pointwise Kullback-Leibler, dihitung dari `probtrace(theta - delta)`
  vs `probtrace(theta + delta)`, **tanpa** memerlukan grid kuadratur.
- `KLn`: sama seperti `KL`, tapi `delta` diskalakan
  `delta * sqrt(n_item_terjawab)` — interval melebar seiring tes berjalan
  (mendekati pendekatan informasi global Chang & Ying).
- `criteria_estimator` otomatis: `ML` untuk `{Drule,Trule,Erule,Wrule,Arule}`,
  `MAP` untuk varian `*Prule` — dipakai Fase 3 saat estimasi ulang tidak
  eksplisit diminta pengguna.

### Landasan Teori & Referensi Akademik
- Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
  61(2), 331–354. (D-optimality sebagai fondasi seleksi item MCAT)
- van der Linden, W. J. (1999). Multidimensional adaptive testing with a
  minimum error-variance criterion. *Journal of Educational and Behavioral
  Statistics*, 24(4), 398–412. (A-rule)
- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection. *Psychometrika*,
  74(2), 273–296. (formalisasi D/A/E/T-rule multidimensional)
- Chang, H.-H., & Ying, Z. (1996). A global information approach to
  computerized adaptive testing. *Applied Psychological Measurement*,
  20(3), 213–229. (dasar kriteria KL & interval `delta` yang membesar)
- Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
  Interfaces for Multidimensional Item Response Theory Applications.
  *Journal of Statistical Software*, 71(5), 1–39. doi:10.18637/jss.v071.i05

### Desain Modul Rust

> Diperbarui dari sketsa awal sesuai implementasi final — lih. catatan
> "Update" di atas untuk alasan tiap deviasi.

- `item_selection::Criterion` — enum tertutup: `Seq`, `Random`, `Kl { delta: f64 }`, `KlN { delta: f64 }`, `DRule { prior: bool }`, `TRule { prior: bool }`, `ARule { prior: bool }`, `ERule { prior: bool }`, `WRule { prior: bool }`. `Default` → `Seq` (mengikuti default `criteria = "seq"` `mirtCAT`).
- `item_selection::SelectionConfig { criterion: Criterion, weights: Option<Vec<f64>>, debug: bool }` — `SelectionConfig::new(criterion, weights, nfact, debug) -> Result<Self, MirtCatError>` memvalidasi panjang `weights` (bila diberikan) terhadap `nfact` (`DimensionMismatch` bila tidak cocok). Validasi "kriteria tidak valid untuk `nfact > 1`" terjadi di **compile time** lewat enum tertutup, bukan lewat error runtime terpisah. `debug` default `false` (prinsip desain #7).
- `item_selection::PersonState { theta: ThetaVector, answered_items: Vec<ItemIndex> }` — **tidak** menyimpan matriks akumulasi sebagai field. `record_answer(item_index)` cuma mencatat index; `info_thetas(&ItemBank) -> Result<InfoMatrix, MirtCatError>` menghitung ulang dari `answered_items` di `theta` saat ini lewat `irt::cumulative_information` (Fase 0) tiap dipanggil — meniru `Person$Update.info_mats` `mirtCAT` (lih. catatan "Update").
- `item_selection::ItemSelectionDebug { candidate_scores: Vec<(ItemIndex, f64)>, chosen: ItemIndex, info_before: Option<InfoMatrix>, info_after: Option<InfoMatrix> }` — nilai kriteria mentah tiap kandidat, dan snapshot `info_thetas` sebelum/sesudah untuk kriteria D/T/A/E/W-rule (`None` untuk `seq`/`random`/`KL` yang tidak memakainya).
- `item_selection::select_next_item(config: &SelectionConfig, candidates: &[ItemIndex], person: &PersonState, bank: &ItemBank, group: &GroupParams, rng: &mut impl Rng) -> Result<(ItemIndex, Option<ItemSelectionDebug>), MirtCatError>` — `group` dipakai varian `*Prule` (`inverse(gcov)`), `rng` dipakai `Random`; keduanya parameter tambahan di atas sketsa awal (lih. catatan "Update").

### Checklist Implementasi
- [x] `item_selection::Criterion` (enum) — validasi "kriteria tidak valid untuk `nfact > 1`" terjadi lewat enum tertutup (compile time), bukan validasi runtime terpisah (lih. catatan "Update")
- [x] `PersonState::info_thetas(&ItemBank)` (dihitung ulang dari `answered_items`, bukan matriks kumulatif field — lih. catatan "Update")
- [x] `criteria/seq.rs`, `criteria/random.rs`
- [x] `criteria/kl.rs` (KL & KLn, termasuk skala `delta * sqrt(n_answered)`)
- [x] `criteria/d_rule.rs`, `t_rule.rs`, `a_rule.rs`, `e_rule.rs`, `w_rule.rs` (masing-masing + varian `prior: true` untuk `*Prule`)
- [x] `select_next_item()` — dispatcher yang memanggil kriteria terpilih atas kandidat item yang belum dijawab
- [x] Dukungan debug trace opsional (`SelectionConfig::debug`, default `false`) → `Option<ItemSelectionDebug>` berisi nilai kriteria tiap kandidat + snapshot `info_thetas`

### Checklist Unit Test
- [x] Validasi konstruksi kriteria tidak valid untuk `nfact > 1`: dijamin oleh sistem tipe (enum `Criterion` tertutup, `MI`/`MEPV`/`MLWI`/`MPWI`/`MEI`/`IKL*` mustahil dikonstruksi) — tidak ada test runtime untuk sesuatu yang mustahil terjadi; `SelectionConfig::new` diuji untuk validasi nyata yang memang bisa gagal (`weights.len() != nfact`)
- [x] `Drule` — determinan matriks informasi meningkat monoton saat item dengan informasi tinggi ditambahkan (kasus sintetis 2D)
- [x] `Trule` tanpa/dengan bobot — hasil berbeda sesuai bobot yang diberikan
- [x] `Arule` — nilai kriteria = negatif trace **pseudo-invers** (bukan invers biasa — lih. catatan "Update") matriks informasi, diverifikasi terhadap matriks full-rank yang inversnya diketahui manual + kasus rank-defisien (nilai referensi dari verifikasi numerik terhadap `mirtCAT`)
- [x] `Erule` — memilih item yang memaksimalkan eigenvalue minimum pada kasus kandidat dengan info matrix kontras (satu "kuat" di dimensi 1, satu di dimensi 2), termasuk pembuktian info kumulatif (bukan cuma info kandidat sendiri) yang dipakai
- [x] `Wrule` — kombinasi berbobot sesuai `weights` custom, dibuktikan beda dari `Trule` (bentuk kuadratik vs trace berbobot) untuk matriks ber-off-diagonal
- [x] `KL`/`KLn` — nilai `KL(delta=0) = 0` & `KL >= 0` (Gibbs' inequality; **tidak** simetris terhadap tanda `delta` secara umum — direvisi dari asumsi awal setelah ketahuan salah lewat test yang gagal), dan `KLn` menghasilkan interval lebih lebar seiring `n_answered` bertambah
- [x] Varian `*Prule` menghasilkan nilai berbeda dari non-prior ketika `gcov` non-identitas
- [x] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(ItemSelectionDebug)` memuat nilai kriteria seluruh kandidat (bukan cuma yang terpilih) dan tidak mengubah item yang dikembalikan

### Checklist Integration Test
- [x] Fixture R: item bank 3D tetap (10 item, `gcov` non-diagonal), untuk tiap kriteria (`Drule`,`Trule`,`Arule`,`Erule`,`Wrule`,`KL`,`KLn` + varian `P`) — dua jenis data: (1) nilai kriteria mentah seluruh kandidat di 0 item terjawab (`computeCriteria()`), (2) urutan item teradministrasi + `thetas_history` dari simulasi `mirtCAT()` end-to-end 6 langkah (`tests/fixtures/generate_r_fixtures_item_selection.R`, dijalankan sungguhan)
- [x] Bandingkan nilai kriteria mentah Rust vs R (toleransi numerik) & argmax-nya — kecuali `Drule`/`Erule` tanpa prior di 0 item terjawab, yang secara matematis degenerate (info 1 item dikotomus selalu rank 1 < `nfact`, det/eigenvalue-min persis 0 untuk semua kandidat — argmax jadi noise floating-point, bukan sinyal; nilainya sendiri tetap dibandingkan & cocok). `DPrule`/`EPrule` (full rank berkat prior) tidak kena masalah ini
- [x] Replay urutan item lengkap: inject `theta` R persis tiap langkah (estimasi ulang Fase 3 belum ada), verifikasi item terpilih Rust identik dengan R di tiap langkah (kecuali langkah pertama, yang di `mirtCAT` sungguhan selalu `start_item` paksa, bukan hasil kriteria — ditemukan lewat inspeksi `findNextCATItem()`, bukan asumsi) — ini sekaligus membuktikan `info_thetas` terakumulasi/dihitung ulang dengan benar antar-item

### Checklist Example Script

> Dipisah 1 file per algoritma (mengikuti 1:1 pemetaan modul
> `item_selection::criteria::*` di §3/"Desain Modul Rust" di atas) supaya
> tiap contoh fokus pada satu kriteria, bukan menggabungkan beberapa
> algoritma sekaligus dalam satu file. `KL`/`KLn` tetap satu file (satu
> modul `criteria/kl.rs`, satu keluarga rumus — bukan digabung demi
> kepraktisan, tapi karena memang satu algoritma dengan skala `delta`
> berbeda).

- [x] `examples/02_item_selection_seq.rs` — urutan `seq` (item ke-n = item bank ke-n), tanpa kriteria informasi
- [x] `examples/02_item_selection_random.rs` — pemilihan `random` di antara kandidat tersisa (seed tetap untuk reproducibility)
- [x] `examples/02_item_selection_kl.rs` — demonstrasi KL vs KLn, cetak bagaimana `delta` efektif melebar seiring item bertambah
- [x] `examples/02_item_selection_d_rule.rs` — simulasi 5 item pertama dipilih via `Drule` (determinan matriks informasi) pada item bank 2D, + varian `DPrule`
- [x] `examples/02_item_selection_t_rule.rs` — `Trule` tanpa/dengan `weights`, + varian `TPrule`
- [x] `examples/02_item_selection_a_rule.rs` — `Arule` (negatif trace pseudo-invers), + varian `APrule`
- [x] `examples/02_item_selection_e_rule.rs` — `Erule` (eigenvalue minimum), + varian `EPrule`
- [x] `examples/02_item_selection_w_rule.rs` — `Wrule` dengan `weights` custom, + varian `WPrule`

### Checklist Dokumentasi

> Sama seperti Example Script: dipisah 1 pasang teori+panduan per
> algoritma supaya fokus, plus satu dokumen overview bersama untuk
> konteks lintas-kriteria (validasi `nfact > 1`, `criteria_estimator`
> otomatis) yang tidak spesifik ke satu algoritma.

- [x] `docs/teori/02-item-selection-overview.md` — kenapa `nfact > 1` membatasi pilihan kriteria (curse of dimensionality grid kuadratur), daftar kriteria valid, `criteria_estimator` otomatis (`ML` vs `MAP`), peta ke dokumen per-algoritma di bawah
- [x] `docs/teori/02-item-selection-seq.md` — `seq` sebagai baseline non-adaptif/kuesioner terurut
- [x] `docs/teori/02-item-selection-random.md` — `random` sebagai baseline exposure-neutral
- [x] `docs/teori/02-item-selection-kl.md` — rumus KL & KLn, dasar Chang & Ying (1996)
- [x] `docs/teori/02-item-selection-d-rule.md` — rumus `Drule`/`DPrule`, D-optimality (Segall, 1996)
- [x] `docs/teori/02-item-selection-t-rule.md` — rumus `Trule`/`TPrule`, trace berbobot
- [x] `docs/teori/02-item-selection-a-rule.md` — rumus `Arule`/`APrule`, minimum error-variance (van der Linden, 1999), termasuk temuan pseudo-invers
- [x] `docs/teori/02-item-selection-e-rule.md` — rumus `Erule`/`EPrule`, eigenvalue minimum
- [x] `docs/teori/02-item-selection-w-rule.md` — rumus `Wrule`/`WPrule`, kombinasi berbobot kustom
- [x] `docs/panduan/02-item-selection-overview.md` — cara memilih `Criterion` secara umum, kapan pakai `weights`, kapan pakai varian `*Prule`, peta ke panduan per-algoritma
- [x] `docs/panduan/02-item-selection-seq.md`
- [x] `docs/panduan/02-item-selection-random.md`
- [x] `docs/panduan/02-item-selection-kl.md`
- [x] `docs/panduan/02-item-selection-d-rule.md`
- [x] `docs/panduan/02-item-selection-t-rule.md`
- [x] `docs/panduan/02-item-selection-a-rule.md`
- [x] `docs/panduan/02-item-selection-e-rule.md`
- [x] `docs/panduan/02-item-selection-w-rule.md`

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] `cargo test` hijau (86 test: 76 unit `item_selection`/`theta` + 2 integration `integration_item_selection` + fixture lain tetap hijau), `cargo clippy --all-targets -- -D warnings` & `cargo fmt --check` bersih
- [x] Parity urutan item & nilai kriteria dengan fixture R terverifikasi untuk seluruh kriteria yang didukung (dengan pengecualian yang didokumentasikan & dijustifikasi: argmax `Drule`/`Erule` non-prior di 0 item terjawab, dan langkah pertama urutan penuh yang di R selalu `start_item` paksa — keduanya bukan kegagalan implementasi, lih. Checklist Integration Test)

---

## Fase 3 — Algoritma Estimasi Kemampuan (Ability Estimation) pada MCAT

**Status: ✅ Selesai**

> **Update (2026-09-22)**: Diimplementasikan setelah menelusuri source R
> **asli** `mirt_1.47`/`mirtCAT_1.14` (diunduh dari CRAN, dibaca langsung —
> bukan cuma dokumentasi) untuk memastikan port setia. Temuan kunci:
> `mirtCAT::Design$Update.thetas` (`DesignClass.R:57-94`) ternyata **hanya
> memanggil `mirt::fscores()`** langsung, jadi ground truth algoritma
> sesungguhnya ada di `mirt/R/fscores.internal.R`. Keputusan desain
> penting (detail lengkap di `docs/teori/03-ability-estimation-*.md`):
> - `Ml`/`Map`/`Wle` dioptimasi lewat SATU optimizer Newton-Raphson generik
>   dengan gradien & Hessian **finite-difference**
>   (`ability_estimation::optim`, privat) — `nlm()` R yang dipakai
>   `mirt::MAP.mirt`/`WLE.mirt` juga tidak diberi gradien analitik, jadi
>   ini setia di level algoritma tanpa perlu mengubah trait
>   `ItemResponseModel`.
> - EAP QMC-based memakai leaped-Halton sequence (bukan Sobol) + invers
>   CDF normal hand-roll (aproksimasi Acklam) — persis algoritma
>   `mirt::QMC_quad`/`QUnif`, tanpa dependency statistik baru.
> - `EapSum` beroperasi atas item **terjawab saja** (`PersonState`), bukan
>   seluruh bank + `NA` — diverifikasi EMPIRIS (menjalankan R sungguhan
>   dua cara berbeda) bahwa keduanya identik.
> - `PersonState` (Fase 2) diperluas (breaking change disengaja): field
>   `se: SEVector` baru, `record_answer` sekarang butuh kategori respons
>   (`record_answer(item_index, response)`), constructor
>   `from_theta_and_se` & setter `set_estimate` baru. 9 file existing
>   (test + example Fase 2) diupdate mengikuti.
> - Fixture R baru: `tests/fixtures/generate_r_fixtures_ability_estimation.R`
>   memanggil `mirt::fscores()` langsung (bukan `mirtCAT()` penuh) — 8
>   test integrasi, parity EAP/MAP dalam `1e-4`, ML `1e-3` (kasus
>   well-identified; kasus quasi-separated diuji kualitatif, didokumentasikan
>   eksplisit di `tests/integration_ability_estimation.rs`), WLE/EAPsum
>   `1e-2`–`5e-2`.
> - `cargo test`/`cargo clippy --all-targets --all-features -- -D warnings`
>   hijau di seluruh workspace (122 unit test lib + 8 integration test
>   Fase 3 + seluruh test Fase 0-2 yang disentuh ulang).

### Tujuan
Mengestimasi ulang vektor `theta` (seluruh dimensi sekaligus) setiap kali
respons baru masuk, dengan metode yang sama seperti `mirtCAT`
(`EAP`/`MAP`/`ML`/`WLE`/`EAPsum`/`fixed`), termasuk pertimbangan numerik
khusus dimensi tinggi.

### Rujukan Kode Sumber (R)
- `mirtCAT/R/DesignClass.R:57-94` — pemanggilan `fscores()` via `Update.thetas`
- `mirtCAT/R/mirtCAT_preamble.R:184-191` — peringatan `EAP` untuk `nfact > 4`
- `mirtCAT/R/TestClass.R:58-60,65-72` — ukuran `quadpts` mengecil seiring `nfact`

### Perilaku yang Harus Disamai
- Semua metode memperbarui **seluruh vektor** dimensi dalam satu langkah
  (bukan dimensi-per-dimensi).
- Default method: `MAP`.
- `quadpts` default per `nfact`: `1→61, 2→31, 3→15, 4→9, 5→7, >5→3` (bisa
  dioverride manual) — mencegah ledakan kombinatorial grid (`quadpts^nfact`).
- Untuk `nfact > 4`: jika method `EAP` dipakai **tanpa** Quasi-Monte Carlo
  (QMC), tampilkan peringatan & sarankan `MAP` (EAP grid-based tidak stabil
  di dimensi tinggi tanpa QMC).
- `fixed`: tidak melakukan estimasi ulang (dipakai untuk kuesioner
  non-adaptif) — theta tetap seperti sebelumnya.

### Landasan Teori & Referensi Akademik
- Bock, R. D., & Aitkin, M. (1981). Marginal maximum likelihood estimation
  of item parameters: Application of an EM algorithm. *Psychometrika*,
  46(4), 443–459. (dasar EAP)
- Warm, T. A. (1989). Weighted likelihood estimation of ability in item
  response theory. *Psychometrika*, 54(3), 427–450. (WLE)
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 5
  (estimasi MAP/EAP/ML multidimensional).
- Cai, L. (2010). High-dimensional exploratory item factor analysis by a
  Metropolis–Hastings Robbins–Monro algorithm. *Psychometrika*, 75(1),
  33–57. (motivasi Quasi-Monte Carlo untuk integrasi dimensi tinggi)
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *JSS*, 48(6), 1–29.

### Desain Modul Rust
- `ability_estimation::Method` — enum `Eap { qmc: bool }`, `Map`, `Ml`, `Wle`, `EapSum`, `Fixed`.
- `ability_estimation::EstimationConfig { method: Method, quadpts: Option<usize>, debug: bool }` — default `quadpts` dihitung dari `nfact` bila `None`. `debug` default `false` (prinsip desain #7).
- `ability_estimation::quadrature::{build_grid, build_qmc_points}` — grid kuadratur reguler + titik Quasi-Monte Carlo (mis. Sobol/Halton sequence).
- `ability_estimation::AbilityEstimationDebug { method: Method, n_quadrature_points: Option<usize>, iterations: Option<usize>, log_likelihood_trace: Option<Vec<f64>>, converged: Option<bool> }` — jejak iterasi optimisasi/kuadratur; field bernilai `None` untuk method yang tidak relevan (mis. `iterations` untuk `Fixed`).
- `ability_estimation::estimate(config, person_state, bank, group) -> ((ThetaVector, SEVector), Option<AbilityEstimationDebug>)`.
- Validasi: `nfact > 4 && method == Eap { qmc: false }` → hasilkan `Warning`/log (bukan error keras, sesuai perilaku R yang hanya `warning()`).

### Checklist Implementasi
- [x] `ability_estimation::Method` (enum) & `EstimationConfig`
- [x] `quadrature::build_grid` — grid reguler `quadpts^nfact`, dengan tabel default `quadpts` per `nfact`
- [x] `quadrature::build_qmc_points` — titik QMC (leaped-Halton) sebagai alternatif untuk `nfact` besar
- [x] `eap.rs` — EAP (grid-based & QMC-based)
- [x] `map.rs` — MAP (optimisasi langsung dengan prior `N(gmeans, gcov)`)
- [x] `ml.rs` — ML (optimisasi likelihood tanpa prior)
- [x] `wle.rs` — WLE (weighted likelihood, koreksi bias Warm)
- [x] `eap_sum.rs` — EAP berbasis summed score
- [x] `fixed.rs` — tidak melakukan apa-apa (identity, mengembalikan theta/SE sebelumnya)
- [x] Peringatan (crate `log`) untuk kombinasi `EAP` + `nfact > 4` tanpa QMC
- [x] Dukungan debug trace opsional (`EstimationConfig::debug`, default `false`) → `Option<AbilityEstimationDebug>` (jumlah titik kuadratur, iterasi, jejak log-likelihood, status konvergensi)

### Checklist Unit Test
- [x] EAP grid-based konvergen ke nilai analitik pada kasus sintetis 1D sederhana (sanity check dasar sebelum diuji multidimensional)
- [x] MAP & ML menghasilkan estimasi berbeda saat prior informatif (`gcov` sempit) — MAP tertarik ke prior, ML tidak
- [x] WLE menghasilkan estimasi dengan bias lebih kecil dibanding ML pada simulasi respons ekstrem (semua benar/semua salah, kasus di mana ML bisa divergen ke ±∞ sementara WLE tetap terbatas)
- [x] `EapSum` menghasilkan estimasi berbasis kategori summed score sesuai definisi
- [x] `Fixed` tidak mengubah theta/SE input
- [x] `quadrature::build_grid` menghasilkan jumlah titik `quadpts^nfact` sesuai tabel default, dan bisa dioverride manual
- [x] Peringatan ter-trigger untuk `nfact=5, method=Eap{qmc:false}` (dites via capturing logger custom, bukan panic)
- [x] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(AbilityEstimationDebug)` terisi (mis. `n_quadrature_points` cocok dengan `build_grid`) dan tidak mengubah `(ThetaVector, SEVector)` yang dikembalikan

### Checklist Integration Test
- [x] Fixture R: pola respons tetap pada item bank 2–3 dimensi, estimasi `fscores()` dengan tiap method (`EAP`,`MAP`,`ML`,`WLE`,`EAPsum`) → simpan theta & SE hasil R
- [x] Bandingkan hasil Rust vs R per method (toleransi numerik, EAP/MAP/ML diharapkan sangat dekat; WLE/EAPsum ditoleransi lebih longgar karena kemungkinan detail numerik solver berbeda — didokumentasikan eksplisit jika ada deviasi terukur)
- [x] Test `nfact=5` — bandingkan ukuran grid kuadratur default Rust vs `quadpts` default R

### Checklist Example Script

> Dipisah 1 file per algoritma (mengikuti 1:1 pemetaan modul
> `ability_estimation::*` di §"Desain Modul Rust" di atas — `eap.rs`,
> `map.rs`, `ml.rs`, `wle.rs`, `eap_sum.rs`, `fixed.rs`) supaya tiap contoh
> fokus pada satu method, bukan menggabungkan beberapa method sekaligus
> dalam satu file. EAP grid-based & QMC-based tetap satu file (satu modul
> `eap.rs`, satu algoritma dengan mode integrasi berbeda — sama seperti
> `KL`/`KLn` di Fase 2).

- [x] `examples/03_ability_estimation_eap.rs` — EAP grid-based vs QMC-based pada pola respons yang sama, cetak perbandingan theta/SE & waktu komputasi relatif antar mode integrasi
- [x] `examples/03_ability_estimation_map.rs` — MAP dengan prior `N(gmeans, gcov)`, tunjukkan efek prior sempit vs lebar terhadap penarikan theta ke arah mean
- [x] `examples/03_ability_estimation_ml.rs` — ML tanpa prior, termasuk kasus respons ekstrem (semua benar/semua salah) untuk menunjukkan potensi divergensi ke ±∞
- [x] `examples/03_ability_estimation_wle.rs` — WLE dengan koreksi bias Warm, dibandingkan dengan ML pada kasus respons ekstrem yang sama untuk menunjukkan bias lebih kecil
- [x] `examples/03_ability_estimation_eap_sum.rs` — EAP berbasis summed score, cetak pemetaan kategori skor ke theta/SE
- [x] `examples/03_ability_estimation_fixed.rs` — `Fixed` sebagai identity (theta/SE input tidak berubah), untuk kuesioner non-adaptif

### Checklist Dokumentasi

> Sama seperti Checklist Example Script: dipisah 1 pasang teori+panduan per
> method, plus satu dokumen overview bersama untuk konteks lintas-method
> (integrasi dimensi tinggi, tabel `quadpts` default per `nfact`, kapan
> pakai QMC, peringatan `EAP`+`nfact>4`) yang tidak spesifik ke satu method.

- [x] `docs/teori/03-ability-estimation-overview.md` — isu integrasi dimensi tinggi (curse of dimensionality grid kuadratur `quadpts^nfact`), tabel `quadpts` default per `nfact`, motivasi Quasi-Monte Carlo (Cai, 2010), peringatan `EAP`+`nfact>4` tanpa QMC, peta ke dokumen per-method di bawah
- [x] `docs/teori/03-ability-estimation-eap.md` — rumus EAP multidimensional grid-based & QMC-based (Bock & Aitkin, 1981)
- [x] `docs/teori/03-ability-estimation-map.md` — rumus MAP, prior `N(gmeans, gcov)` (Reckase, 2009 bab 5)
- [x] `docs/teori/03-ability-estimation-ml.md` — rumus ML tanpa prior, potensi divergensi pada respons ekstrem
- [x] `docs/teori/03-ability-estimation-wle.md` — rumus WLE, koreksi bias Warm (Warm, 1989)
- [x] `docs/teori/03-ability-estimation-eap-sum.md` — rumus EAP berbasis summed score
- [x] `docs/teori/03-ability-estimation-fixed.md` — semantik `Fixed` (identity, tanpa estimasi ulang) untuk kuesioner non-adaptif
- [x] `docs/panduan/03-ability-estimation-overview.md` — cara memilih method & `quadpts` secara umum, kapan pakai QMC, peta ke panduan per-method
- [x] `docs/panduan/03-ability-estimation-eap.md`
- [x] `docs/panduan/03-ability-estimation-map.md`
- [x] `docs/panduan/03-ability-estimation-ml.md`
- [x] `docs/panduan/03-ability-estimation-wle.md`
- [x] `docs/panduan/03-ability-estimation-eap-sum.md`
- [x] `docs/panduan/03-ability-estimation-fixed.md`

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] `cargo test -p mirt-engine ability_estimation` hijau
- [x] Parity numerik EAP/MAP/ML dengan fixture R terverifikasi dalam toleransi yang didokumentasikan

---

## Fase 4 — Stopping Rule pada MCAT

**Status: ✅ Selesai**

### Tujuan
Menentukan kapan sesi tes berhenti, dengan semantik **"ALL dimensi"** untuk
kriteria presisi (`min_SEM`, `delta_thetas`, klasifikasi `CI`) — konsekuensi
langsung dari `theta` berupa vektor pada MCAT.

### Rujukan Kode Sumber (R)
- `mirtCAT/R/DesignClass.R:312-381` (`Update.stop_now`)

### Perilaku yang Harus Disamai
- `met_SEM = SE_terkini < min_SEM` dievaluasi **per-dimensi**; tes berhenti
  karena presisi **hanya jika SEMUA dimensi** memenuhi (`all()`), bukan salah
  satu.
- `met_delta_thetas` — sama, **semua** dimensi harus punya
  `|Δtheta| < delta_thetas` antar-langkah terakhir.
- Mode klasifikasi `'CI'` — **semua** dimensi harus bisa diklasifikasi
  (relatif ke cutoff per-dimensi via z-score dan `classify_alpha`) sebelum
  keputusan final diambil; hasil klasifikasi tetap **per-dimensi**
  (`'above cutoff'`/`'below cutoff'`/`'no decision'`).
- Mode klasifikasi `'SPRT'` — **berbeda dari `CI`**: bukan "all dimensi",
  melainkan uji rasio log-likelihood sekuensial (Wald) antara hipotesis
  `theta_lower` vs `theta_upper` (bisa berupa vektor multidimensional),
  dibandingkan terhadap batas `[a, b]` yang diturunkan dari `alpha`/`beta`.
- Kondisi berhenti tambahan yang selalu dicek independen dari precision:
  `max_items` (jumlah item terjawab), `max_time` (total waktu), hook
  `custom_stop` (closure kustom pengguna).
- `allow_constrain_breaks = false` — mencegah berhenti di tengah blok
  `ordered`/`unordered` constraint yang belum selesai.
- **Implikasi eksplisit yang perlu didokumentasikan**: makin banyak `nfact`,
  makin sulit syarat "all" terpenuhi secara statistik → `max_items`/
  `max_time` sering jadi kondisi berhenti praktis untuk model dimensi tinggi.

### Landasan Teori & Referensi Akademik
- Wald, A. (1945). Sequential tests of statistical hypotheses. *Annals of
  Mathematical Statistics*, 16(2), 117–186. (dasar SPRT)
- Weiss, D. J., & Kingsbury, G. G. (1984). Application of computerized
  adaptive testing to educational problems. *Journal of Educational
  Measurement*, 21(4), 361–375. (stopping rule berbasis SEM)
- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*. Springer. (bab stopping rule multidimensional)
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 7.

### Desain Modul Rust
- `stopping_rule::ClassifyType` — enum `None`, `Ci { cutoffs: Vec<f64>, alpha: f64 }`, `Sprt { lower: ThetaVector, upper: ThetaVector, alpha: f64, beta: f64 }`.
- `stopping_rule::StoppingConfig { min_items: usize, max_items: usize, max_time: Option<Duration>, min_sem: Vec<f64>, delta_thetas: Vec<f64>, classify: ClassifyType, allow_constrain_breaks: bool, debug: bool }` — `debug` default `false` (prinsip desain #7).
- `stopping_rule::StopDecision { stop_now: bool, met_sem: Vec<bool>, met_delta_thetas: Vec<bool>, met_classify: Option<Vec<bool>>, classification: Vec<ClassifyDecision>, terminated_early: bool }`.
- `stopping_rule::StoppingRuleDebug { se_values: SEVector, delta_theta_values: Vec<f64>, sprt_log_likelihood_ratio: Option<f64>, sprt_bounds: Option<(f64, f64)> }` — nilai mentah di balik `met_sem`/`met_delta_thetas`/`met_classify` (mis. SE aktual, bukan cuma boolean lolos/tidak), dan detail rasio log-likelihood/batas `[a,b]` untuk mode `Sprt`.
- `stopping_rule::evaluate(config, person_state, test_state) -> (StopDecision, Option<StoppingRuleDebug>)`.
- `stopping_rule::CustomStop` — trait objek (`Fn(&PersonState) -> bool`) untuk hook kustom pengguna.

### Checklist Implementasi
- [x] `ClassifyType`, `StoppingConfig`, `StopDecision`
- [x] `sem.rs` — evaluasi `met_SEM` per-dimensi + `all()`
- [x] `delta_theta.rs` — evaluasi `met_delta_thetas` per-dimensi + `all()`
- [x] `classify_ci.rs` — z-score per dimensi vs cutoff, keputusan per-dimensi, `all()` untuk stop
- [x] `sprt.rs` — log-likelihood ratio test terhadap `theta_lower`/`theta_upper`
- [x] `max_items_time.rs` — kondisi `max_items`/`max_time`
- [x] Dukungan `CustomStop` (trait objek/closure)
- [x] Guard `allow_constrain_breaks`
- [x] Dukungan debug trace opsional (`StoppingConfig::debug`, default `false`) → `Option<StoppingRuleDebug>` (nilai SE/delta-theta mentah, rasio log-likelihood SPRT)

### Checklist Unit Test
- [x] `met_SEM` — stop hanya ketika **semua** dimensi di bawah `min_SEM`; satu dimensi belum memenuhi → tidak stop
- [x] `met_delta_thetas` — sama, semantik `all()`
- [x] `Ci` classify — kasus 2 dimensi, satu terklasifikasi yakin & satu belum → belum stop; keduanya yakin → stop dengan keputusan per-dimensi yang benar
- [x] `Sprt` — kasus respons konsisten dengan `theta_upper` → keputusan `'above cutoff'`; konsisten `theta_lower` → `'below cutoff'`; ambigu → belum stop
- [x] `max_items` & `max_time` menghentikan tes terlepas dari precision
- [x] `CustomStop` closure dipanggil dan hasilnya dihormati
- [x] `allow_constrain_breaks=false` mencegah stop di tengah blok `ordered` constraint yang belum selesai
- [x] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(StoppingRuleDebug)` terisi (mis. `se_values` cocok dengan SE aktual di `person_state`) dan tidak mengubah `StopDecision` yang dikembalikan

### Checklist Integration Test
- [x] Fixture R: simulasi sesi penuh 3 dimensi dengan `min_SEM` tertentu → catat item ke berapa tes berhenti & keputusan akhir dari `mirtCAT`
- [x] Bandingkan titik-berhenti & keputusan klasifikasi Rust vs R
- [x] Fixture R terpisah untuk mode `SPRT` (dua hipotesis vektor), bandingkan keputusan (`above`/`below`/`no decision`) & item ke berapa berhenti

### Checklist Example Script

> Dipisah 1 file per mekanisme berhenti (mengikuti 1:1 pemetaan modul
> `stopping_rule::*` di §"Desain Modul Rust" di atas — `sem.rs`,
> `delta_theta.rs`, `classify_ci.rs`, `sprt.rs`, `max_items_time.rs`) supaya
> tiap contoh fokus pada satu aturan berhenti, bukan menggabungkan semua
> kombinasi sekaligus dalam satu file — sama seperti pola yang dipakai Fase
> 2 (item selection) & Fase 3 (ability estimation).

- [x] `examples/04_stopping_rule_min_sem.rs` — sesi berhenti karena `min_SEM` terpenuhi di **semua** dimensi (`all()`); tunjukkan satu dimensi belum presisi menahan tes tetap lanjut
- [x] `examples/04_stopping_rule_delta_thetas.rs` — sesi berhenti karena `|Δtheta|` antar-langkah di bawah `delta_thetas` di semua dimensi; cetak deret delta per-item hingga stabil
- [x] `examples/04_stopping_rule_ci.rs` — klasifikasi `CI` 2+ dimensi (z-score per-dimensi vs cutoff via `classify_alpha`); tunjukkan status `'no decision'` bertahan sampai semua dimensi terklasifikasi
- [x] `examples/04_stopping_rule_sprt.rs` — dua hipotesis vektor `theta_lower`/`theta_upper`; cetak rasio log-likelihood & batas `[a, b]` per-langkah hingga keputusan `'above cutoff'`/`'below cutoff'`/`'no decision'` tercapai
- [x] `examples/04_stopping_rule_max_items_time.rs` — `max_items`/`max_time` menghentikan sesi meski presisi belum tercapai; simulasikan `nfact` besar di mana syarat "all" secara statistik jarang terpenuhi, sehingga batas ini jadi kondisi berhenti praktis

### Checklist Dokumentasi

> Sama seperti Checklist Example Script: dipisah 1 pasang teori+panduan per
> mekanisme berhenti, plus satu dokumen overview bersama untuk konteks
> lintas-mekanisme (semantik "all dimensi", `CustomStop`,
> `allow_constrain_breaks`, debug trace, implikasi `nfact` besar) yang tidak
> spesifik ke satu mekanisme.

- [x] `docs/teori/04-stopping-rule-overview.md` — semantik "all dimensi" untuk `min_SEM`/`delta_thetas`/`CI`, kondisi independen (`max_items`/`max_time`/`CustomStop`), guard `allow_constrain_breaks`, implikasi `nfact` besar (Weiss & Kingsbury, 1984; van der Linden & Glas, 2010), peta ke dokumen per-mekanisme di bawah
- [x] `docs/teori/04-stopping-rule-min-sem.md` — rumus `met_SEM`, semantik `all()` per-dimensi
- [x] `docs/teori/04-stopping-rule-delta-thetas.md` — rumus `met_delta_thetas`, semantik `all()` per-dimensi
- [x] `docs/teori/04-stopping-rule-ci.md` — klasifikasi `CI`, z-score per-dimensi vs cutoff, `classify_alpha`
- [x] `docs/teori/04-stopping-rule-sprt.md` — uji rasio log-likelihood sekuensial Wald, turunan batas `[a, b]` dari `alpha`/`beta`, referensi Wald (1945)
- [x] `docs/teori/04-stopping-rule-max-items-time.md` — semantik `max_items`/`max_time` sebagai batas keras independen dari presisi
- [x] `docs/panduan/04-stopping-rule-overview.md` — cara memilih kombinasi aturan berhenti yang realistis untuk `nfact` besar, kapan `max_items` jadi kondisi berhenti praktis, peta ke panduan per-mekanisme
- [x] `docs/panduan/04-stopping-rule-min-sem.md`
- [x] `docs/panduan/04-stopping-rule-delta-thetas.md`
- [x] `docs/panduan/04-stopping-rule-ci.md`
- [x] `docs/panduan/04-stopping-rule-sprt.md`
- [x] `docs/panduan/04-stopping-rule-max-items-time.md`

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] `cargo test -p mirt-engine stopping_rule` hijau
- [x] Parity titik-berhenti & keputusan klasifikasi dengan fixture R terverifikasi

---

## Fase 5 — Exposure Control pada MCAT

**Status: ✅ Selesai**

> **Update (2026-09-22)**: perilaku diverifikasi langsung terhadap source
> `mirtCAT` 1.14 terpasang (`getFromNamespace("findNextCATItem", "mirtCAT")`,
> `selectMethod("initialize", "Design")`) — bukan hanya dibaca dari sketsa
> `PLAN.md` di bawah. Tiga temuan penting:
> 1. **`seq`/`random` tidak pernah masuk exposure control** — keduanya
>    `return()` di `findNextCATItem()` SEBELUM `crit` dihitung sama sekali;
>    exposure control hanya berlaku untuk kriteria berbasis skor. Direplikasi
>    lewat `item_selection::rank_candidates()` (fungsi baru, diekstrak dari
>    `select_next_item` tanpa mengubah perilaku/nilai numeriknya — 181 test
>    Fase 0-4 tetap hijau setelah refactor) yang mengembalikan `Err` untuk
>    `Seq`/`Random`.
> 2. **Status "item tidak valid untuk sesi ini" (`person$valid_item` R)
>    sengaja TIDAK ditambahkan sebagai field `PersonState`** — mengikuti
>    preseden Fase 4 (`allow_constrain_breaks` jadi parameter, bukan field,
>    untuk tanggung jawab lintas-fase yang belum ada orchestrator-nya).
>    `select_with_exposure()` menerima `valid_items: &mut [bool]` sebagai
>    parameter eksplisit, dimiliki & disiapkan pemanggil (nantinya
>    orkestrator sesi eksternal) — deviasi dari sketsa awal §"Desain Modul
>    Rust" di bawah, yang semula menaruhnya sebagai parameter
>    `person_valid_items` tanpa merinci relasinya dengan `PersonState`.
> 3. **Deteksi mode & mekanisme SH/randomesque** (`Design$initialize`,
>    `findNextCATItem`) terverifikasi persis sesuai sketsa awal di bawah,
>    termasuk detail non-obvious: SH pengindeksan per ID-item vs randomesque
>    per posisi-penyajian, forced-accept SH TIDAK menandai item invalid,
>    `as.integer()` R men-TRUNCATE (bukan membulatkan) saat validasi mode
>    "sample".
>
> Fixture R (`tests/fixtures/exposure_control.json`, dihasilkan sungguhan
> lewat `Rscript tests/fixtures/generate_r_fixtures_exposure_control.R`)
> mencakup tiga bagian: `mode_detection` (deteksi otomatis EKSAK, tanpa RNG),
> `sample_replay` (satu sesi randomesque, diverifikasi lewat keanggotaan
> top-*n*), `sh_simulation` (300 peserta SH, diverifikasi lewat properti
> statistik exact — item exposure=0.0 TIDAK PERNAH lolos, dibuktikan 0/1800
> di R & 0/N independen di Rust). `cargo test` (205 test: 181 unit + 24
> integrasi lintas 6 file, termasuk 3 test baru
> `tests/integration_exposure_control.rs`) & `cargo clippy --all-targets --
> -D warnings` hijau.

### Tujuan
Membatasi frekuensi kemunculan item populer di antara peserta, dengan dua
mode yang didukung `mirtCAT`: Sympson–Hetter (`'SH'`) dan top-*n* random
(`'sample'`/randomesque).

### Rujukan Kode Sumber (R)
- `mirtCAT/R/DesignClass.R:214-227` — deteksi mode & validasi
- `mirtCAT/R/findNextCATItem.R:361-383` — eksekusi tiap mode

### Perilaku yang Harus Disamai
- Mode ditentukan dari rentang nilai vektor `exposure` (di R: otomatis
  dideteksi; di Rust: **API utama eksplisit via enum**, namun sediakan
  konstruktor `from_values()` yang mereplikasi deteksi otomatis R untuk
  kemudahan migrasi):
  - Semua nilai `∈ [0, 1]` → **`SympsonHetter`**.
  - Ada nilai `> 1` → **`Randomesque`** (top-*n* random); semua nilai wajib
    bilangan bulat `≥ 1`.
  - Panjang vektor **wajib** sama dengan jumlah total item di bank.
- **Pengindeksan berbeda antar mode** (poin kritis, mudah salah):
  - `Randomesque`: diindeks oleh **posisi urutan penyajian**
    (`exposure[lastitem + 1]`) — "untuk giliran ke-n, ambil acak dari
    berapa kandidat teratas?".
  - `SympsonHetter`: diindeks oleh **ID item itu sendiri** — "berapa
    peluang item spesifik ini boleh muncul, di posisi manapun?".
- `Randomesque` dengan nilai `1` = tanpa exposure control efektif (selalu
  ambil kandidat terbaik).
- `SympsonHetter`: loop terima/tolak berdasarkan `runif(0,1) ≤ exposure[item]`;
  item yang ditolak ditandai tidak valid **khusus sesi peserta ini** (bukan
  permanen di bank), lalu coba kandidat terbaik berikutnya; jika kandidat
  tinggal satu, **dipaksa diterima** (mencegah macet).
- Jika seluruh kandidat habis tersaring → error eksplisit ("kehabisan item
  untuk disajikan"), bukan panic diam-diam.

### Landasan Teori & Referensi Akademik
- Sympson, J. B., & Hetter, R. D. (1985). Controlling item-exposure rates
  in computerized adaptive testing. *Proceedings of the 27th Annual Meeting
  of the Military Testing Association*, 973–977.
- McBride, J. R., & Martin, J. T. (1983). Reliability and validity of
  adaptive ability tests in a military setting. In *New Horizons in
  Testing* (pp. 223–236). Academic Press. (dasar randomesque/top-*n*)
- Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
  item exposure control strategies for computerized adaptive testing
  developed on the item response theory framework. *International Journal
  on E-Learning*, 6(1). (tinjauan strategi exposure control)

### Desain Modul Rust

> Diperbarui dari sketsa awal sesuai implementasi final (pola sama seperti
> Fase 2/4 — lih. catatan "Update" di atas untuk alasan tiap deviasi).

- `exposure_control::ExposureMode` — enum `SympsonHetter(Vec<f64>)`, `Randomesque(Vec<u32>)`.
- `exposure_control::ExposureMode::from_values(values: &[f64]) -> Result<Self, MirtCatError>` — replikasi deteksi mode otomatis R.
- `exposure_control::ExposureControlConfig { mode: ExposureMode, debug: bool }` — `debug` default `false` (prinsip desain #7); `ExposureControlConfig::new(mode, n_items, debug)` memvalidasi panjang vektor `== n_items`.
- `exposure_control::ExposureControlDebug { attempts: Vec<(ItemIndex, bool /* diterima */)>, forced_accept: bool }` — urutan kandidat yang dicoba & diterima/ditolak (Sympson–Hetter), atau kandidat top-*n* yang dipertimbangkan (Randomesque, seluruhnya ditandai `true`); `forced_accept` menandai jalur fallback "kandidat tersisa satu" (selalu `false` untuk `Randomesque`).
- `exposure_control::select_with_exposure(config: &ExposureControlConfig, candidate_scores: &[(ItemIndex, f64)], position_in_test: usize, valid_items: &mut [bool], rng: &mut impl Rng) -> Result<(ItemIndex, Option<ExposureControlDebug>), MirtCatError>` — parameter final `valid_items: &mut [bool]` (bukan `person_valid_items` generik di sketsa awal): dimiliki & disiapkan pemanggil, BUKAN field `PersonState` (lih. catatan "Update" poin 2). `candidate_scores` diperoleh dari `item_selection::rank_candidates()` (fungsi baru Fase 2, diekstrak dari `select_next_item` tanpa mengubah perilakunya).
- Validasi panjang vektor `== nitems` saat konstruksi `ExposureControlConfig::new`.

### Checklist Implementasi
- [x] `ExposureMode` (enum) + `from_values()` (deteksi otomatis gaya R)
- [x] Validasi panjang vektor `== nitems`, validasi `Randomesque` berisi integer `≥ 1`
- [x] `sympson_hetter.rs` — loop terima/tolak + penandaan `valid_item=false` per-sesi + fallback paksa-terima saat kandidat tersisa satu
- [x] `randomesque.rs` — ranking kandidat + pemilihan acak dari top-*n* berdasarkan posisi urutan penyajian
- [x] Error eksplisit saat kandidat habis (bukan panic)
- [x] Dukungan debug trace opsional (`ExposureControlConfig::debug`, default `false`) → `Option<ExposureControlDebug>` (urutan kandidat dicoba/diterima/ditolak, status fallback paksa-terima)

### Checklist Unit Test
- [x] `from_values()` — semua di `[0,1]` → `SympsonHetter`; ada `>1` → `Randomesque`; nilai `>1` non-integer → error
- [x] `Randomesque` dengan `exposure=1` selalu memilih kandidat terbaik (deterministik)
- [x] `Randomesque` dengan `exposure=3` memilih dari 3 kandidat teratas (properti statistik: distribusi pilihan mencakup ketiga kandidat dalam banyak sampel, RNG seeded untuk reprodusibilitas)
- [x] `SympsonHetter` — item dengan `exposure[item]=0.0` tidak pernah lolos dalam banyak percobaan (RNG seeded)
- [x] `SympsonHetter` — item dengan `exposure[item]=1.0` selalu lolos begitu jadi kandidat terbaik
- [x] `SympsonHetter` — fallback paksa-terima saat kandidat tersisa satu meski `exposure` rendah
- [x] Item yang ditolak `SympsonHetter` ditandai tidak valid hanya untuk sesi berjalan, tidak memengaruhi sesi/peserta lain (state terisolasi per slice `valid_items` yang diberikan pemanggil, bukan `PersonState` -- lih. catatan "Update" poin 2)
- [x] Error dikembalikan (bukan panic) saat seluruh kandidat tersaring habis
- [x] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(ExposureControlDebug)` mencatat setiap kandidat yang dicoba (termasuk yang ditolak) dan tidak mengubah item yang dikembalikan

### Checklist Integration Test
- [x] Fixture R: sesi dengan `exposure` mode `'SH'` (300 peserta sungguhan, `tests/fixtures/generate_r_fixtures_exposure_control.R`) → bandingkan properti statistik yang EXACT secara matematis (item `exposure=0.0` tidak pernah lolos: 0/1800 di R, 0/N independen di Rust; `start_item` selalu terpilih tepat `n_sim` kali), bukan kecocokan RNG stream 1:1 (RNG R vs Rust berbeda) -- `tests/integration_exposure_control.rs::sh_simulation_zero_exposure_item_never_selected_and_start_item_always_selected`
- [x] Fixture R: sesi dengan mode `'sample'` (`exposure[posisi]=3`, satu sesi replay penuh) → bandingkan bahwa item terpilih R selalu berasal dari top-3 kandidat kriteria Rust (dihitung ulang via `rank_candidates` pada `theta` yang sama persis dgn R di tiap langkah), dicek sebagai himpunan valid, bukan kecocokan item persis -- `tests/integration_exposure_control.rs::sample_replay_chosen_item_is_always_within_top_n_by_criterion`
- [x] Uji integrasi dengan Fase 2: exposure control mengonsumsi hasil kriteria (`crit` values) dengan benar sebagai input ranking -- dibuktikan lewat kedua test di atas (keduanya memanggil `rank_candidates()` lalu `select_with_exposure()` end-to-end), plus deteksi mode dibandingkan EKSAK terhadap `Design$initialize()` R sungguhan (`mode_detection_matches_r_design_initialize`)

### Checklist Example Script

> Dipisah 1 file per mode (mengikuti 1:1 pemetaan modul
> `exposure_control::*` di §"Desain Modul Rust" di atas — `sympson_hetter.rs`,
> `randomesque.rs`) supaya tiap contoh fokus pada satu mode, bukan
> menggabungkan keduanya sekaligus dalam satu file — sama seperti pola yang
> dipakai Fase 2/3/4.

- [x] `examples/05_exposure_control_sympson_hetter.rs` — simulasi 1000 peserta sintetis dengan mode `SympsonHetter`, cetak distribusi frekuensi kemunculan tiap item vs baseline tanpa exposure control (bukti visual distribusi meratakan)
- [x] `examples/05_exposure_control_randomesque.rs` — simulasi 1000 peserta sintetis dengan mode `Randomesque`, bandingkan `exposure=1` (deterministik, selalu kandidat terbaik, identik baseline) vs `exposure=3` (distribusi pilihan mencakup 3 kandidat teratas) vs baseline tanpa exposure control

### Checklist Dokumentasi

> Sama seperti Checklist Example Script: dipisah 1 pasang teori+panduan per
> mode, plus satu dokumen overview bersama untuk konteks lintas-mode
> (deteksi otomatis `from_values()`, perbedaan pengindeksan posisi vs ID
> item, error kandidat habis, debug trace, interaksi dengan content
> balancing) yang tidak spesifik ke satu mode.

- [x] `docs/teori/05-exposure-control-overview.md` — deteksi mode otomatis via `from_values()` (rentang nilai `[0,1]` vs `>1`), perbedaan pengindeksan kritis (`Randomesque` per posisi penyajian vs `SympsonHetter` per ID item), `seq`/`random` bypass exposure control, error eksplisit saat kandidat habis, referensi tinjauan Georgiadou et al. (2007), peta ke dokumen per-mode di bawah
- [x] `docs/teori/05-exposure-control-sympson-hetter.md` — algoritma loop terima/tolak Sympson–Hetter, fallback paksa-terima, referensi Sympson & Hetter (1985)
- [x] `docs/teori/05-exposure-control-randomesque.md` — algoritma top-*n* random (randomesque), referensi McBride & Martin (1983)
- [x] `docs/panduan/05-exposure-control-overview.md` — cara memilih mode & mengkalibrasi nilai `exposure` secara umum, peringatan risiko kehabisan item saat digabung content balancing, peta ke panduan per-mode
- [x] `docs/panduan/05-exposure-control-sympson-hetter.md`
- [x] `docs/panduan/05-exposure-control-randomesque.md`

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] `cargo test` (205 test, termasuk `exposure_control`) & `cargo clippy --all-targets -- -D warnings` hijau; `cargo fmt --check` bersih untuk seluruh file yang disentuh Fase 5 (`src/exposure_control/*`, `src/item_selection/mod.rs`, `src/lib.rs`, `tests/integration_exposure_control.rs`, `examples/05_*.rs`) -- sisa diff `cargo fmt --check` di file Fase 3/4 lama sudah ada SEBELUM Fase 5, di luar scope perubahan ini
- [x] Parity statistik distribusi eksposur dengan fixture R terverifikasi (lih. Checklist Integration Test)

---

## Fase 6 — Content Balancing pada MCAT

**Status: ✅ Selesai**

> **Update (2026-09-22)**: perilaku diverifikasi langsung terhadap source
> `mirtCAT` 1.14 terpasang (`getMethod("initialize", "Design")`,
> `getFromNamespace("findNextCATItem", "mirtCAT")`) — bukan hanya dibaca
> dari sketsa `PLAN.md` di bawah. Enam temuan penting:
> 1. **`content_prop` WAJIB menjumlah persis 1** (toleransi `1e-6`) —
>    `if (!closeEnough(sum(design$content_prop)-1,-1e-6,1e-6))
>    stop("content_prop does not sum to 1")`. Sketsa awal di bawah menduga
>    ini fleksibel ("tidak wajib persis 1.0") — **keliru**, dikoreksi:
>    `ContentBalancingConfig::new` memvalidasi sum dengan toleransi yang
>    sama.
> 2. **Hanya `Criterion::Seq` yang dilarang**, BUKAN `Criterion::Random` —
>    `Design$initialize()`: `if (use_content && criteria == "seq")
>    stop("content designs are not supported for seq criteria")`.
>    `findNextCATItem()` punya cabang content-balancing terpisah (duplikat
>    kode, bukan bug) di dalam blok `criteria == "random"`, fungsional
>    setara "sempitkan ke kategori prioritas, fallback ke pool penuh bila
>    kategori itu habis" — sama seperti kriteria berskor. Direplikasi lewat
>    `ContentBalancingConfig::validate_with_criterion()`.
> 3. **Urutan kerja nyata `findNextCATItem()`**: `crit`/`index` (skor
>    SELURUH kandidat belum dijawab) dihitung dulu, BARU disempitkan
>    berdasar kategori, BARU exposure control jalan di atas hasil sempitan.
>    Karena skor tiap kandidat independen terhadap kandidat lain,
>    menyempitkan pool `ItemIndex` SEBELUM ranking (urutan
>    `narrow_candidates()` di modul ini) menghasilkan nilai numerik identik
>    dengan menyempitkan SESUDAH ranking (urutan R) — dibuktikan lewat
>    replay fixture R (kecocokan EKSAK item demi item, bukan cuma
>    keanggotaan set seperti Fase 5, karena skenario fixture deterministik
>    penuh: kriteria `Drule` tanpa exposure control).
> 4. **Kategori direindeks ke urutan `factor()` (alfabetis)**, tie-break
>    `which.max(dif)` R = "kategori pertama" pada urutan itu — sama persis
>    pola `argmax` yang sudah ada di `item_selection::argmax`.
> 5. **`ItemMeta.content: Option<String>` (`src/bank.rs`) sudah disiapkan
>    sebelumnya untuk fase ini**, tapi `ContentBalancingConfig` SENGAJA
>    tetap decoupled dari `ItemBank` (pola sama seperti
>    `exposure_control::ExposureControlConfig`) — `category_per_item`
>    dikonstruksi eksplisit oleh pemanggil (nanti orkestrator sesi
>    eksternal, dari `bank.meta(i).content`).
> 6. **Kategori tanpa entri `target_prop` → error saat konstruksi**
>    (deviasi sengaja dari opsi "(atau default 0)" di sketsa awal —
>    mencegah typo nama kategori bocor jadi target 0 yang tidak disengaja).
>
> Fixture R (`tests/fixtures/content_balancing.json`, dihasilkan via
> `Rscript tests/fixtures/generate_r_fixtures_content_balancing.R`)
> mencakup dua skenario deterministik penuh (kriteria `Drule`,
> `exposure_type="none"`, tanpa RNG selain `local_pattern`): `main`
> (content_prop 50/50, kedua kategori cukup item) dan `fallback` (satu
> kategori cuma 1 item, kehabisan di tengah sesi). Keduanya dicocokkan
> EKSAK item demi item lewat replay `narrow_candidates -> select_next_item`
> di `tests/integration_content_balancing.rs` (3 test). `cargo test` (220
> test: 193 unit + 27 integrasi lintas 7 file, termasuk 12 test unit baru
> `content_balancing`) & `cargo clippy --all-targets -- -D warnings`
> hijau.

### Tujuan
Menjaga proporsi kategori konten item yang disajikan mendekati target yang
ditentukan pengguna, beroperasi **sebelum** exposure control dalam pipeline
seleksi item.

### Rujukan Kode Sumber (R)
- `mirtCAT/R/DesignClass.R:157-166` — setup (`content` sebagai factor, `content_prop`, `content_prop_empirical`)
- `mirtCAT/R/findNextCATItem.R:346-360` — eksekusi tiap pemilihan item
- `mirtCAT/R/DesignClass.R:273-274` — larangan kombinasi dengan `criteria='seq'`

### Perilaku yang Harus Disamai
- Setiap item punya kategori konten (mis. `"Aljabar"`, `"Geometri"`) — di
  Rust dimodelkan sebagai kategori tetap (bukan string bebas per-panggilan)
  supaya kategori yang belum pernah terjawab tetap **selalu muncul** dalam
  penghitungan proporsi empiris dengan nilai 0 (padanan `factor()` + `table()`
  di R yang selalu mengembalikan semua level).
- `content_prop` (target, pengguna) direindeks agar sejajar urutan kategori
  yang konsisten; `content_prop_empirical` mulai dari nol dengan urutan yang
  sama.
- Tiap kali memilih item: hitung ulang proporsi empiris dari item yang
  **sudah terjawab**, cari kategori dengan `target - empirical` **terbesar**
  (paling kurang jatah), persempit kandidat ke kategori itu **hanya jika**
  masih ada item tersisa di kategori tersebut — jika bank kategori itu habis,
  seleksi otomatis kembali ke seluruh kandidat tanpa pembatasan (tidak error).
- **Larangan eksplisit**: content balancing tidak kompatibel dengan kriteria
  `seq` — divalidasi saat konfigurasi (error saat konstruksi, bukan runtime).
- Beroperasi **sebelum** exposure control dalam urutan pipeline: content
  balancing mempersempit pool kandidat dulu, baru exposure control memilih
  1 item dari pool yang sudah dipersempit.

### Landasan Teori & Referensi Akademik
- Kingsbury, G. G., & Zara, A. R. (1989). Procedures for selecting items
  for computerized adaptive tests. *Applied Measurement in Education*,
  2(4), 359–375. (dasar content balancing dalam CAT)
- Veldkamp, B. P., & van der Linden, W. J. (2002). Multidimensional
  adaptive testing with constraints on test content. *Psychometrika*,
  67(4), 575–588. (content balancing khusus konteks MCAT)
- Chalmers, R. P. (2016). *JSS*, 71(5), 1–39. (implementasi referensi
  `mirtCAT`)

### Desain Modul Rust
- `content_balancing::ContentCategory` — newtype (interned string / enum kecil) sebagai kunci kategori.
- `content_balancing::ContentBalancingConfig { category_per_item: Vec<ContentCategory>, target_prop: HashMap<ContentCategory, f64>, debug: bool }` — validasi: seluruh kategori di `category_per_item` harus punya entri di `target_prop` (atau default 0), dan proporsi target menjumlah masuk akal (tidak wajib persis 1.0, mengikuti fleksibilitas R, tapi divalidasi non-negatif). `debug` default `false` (prinsip desain #7).
- `content_balancing::ContentBalancingDebug { empirical_prop: HashMap<ContentCategory, f64>, deficits: HashMap<ContentCategory, f64>, chosen_category: Option<ContentCategory>, fallback_used: bool }` — proporsi empiris & defisit tiap kategori saat keputusan diambil, kategori yang dipilih (`None` bila tidak ada pembatasan), dan penanda jalur fallback pool-penuh.
- `content_balancing::narrow_candidates(config, answered_items, candidate_pool) -> (Vec<ItemIndex>, Option<ContentBalancingDebug>)` — mempersempit pool sesuai kategori paling kurang jatah, fallback ke pool penuh bila kategori target habis.
- Validasi silang dengan `item_selection::Criterion::Seq` → error `MirtCatError::ContentBalancingIncompatibleWithSeq` saat kedua config digabung.

### Checklist Implementasi
- [x] `ContentCategory`, `ContentBalancingConfig` (+ validasi kategori & proporsi)
- [x] Perhitungan proporsi empiris dari item terjawab (semua kategori selalu muncul walau 0)
- [x] Logika pemilihan kategori "paling kurang jatah" (`argmax(target - empirical)`)
- [x] `narrow_candidates()` dengan fallback ke pool penuh saat kategori target habis
- [x] Validasi larangan kombinasi dengan `Criterion::Seq`
- [x] Dukungan debug trace opsional (`ContentBalancingConfig::debug`, default `false`) → `Option<ContentBalancingDebug>` (proporsi empiris & defisit tiap kategori, kategori terpilih, status fallback)

### Checklist Unit Test
- [x] Kategori yang belum pernah terjawab tetap dihitung 0 dalam proporsi empiris (bukan hilang dari perhitungan)
- [x] Kategori dengan defisit target terbesar dipilih sebagai prioritas (kasus 3 kategori dengan defisit berbeda)
- [x] Fallback ke pool penuh saat kategori prioritas kehabisan item kandidat
- [x] Konstruksi `ContentBalancingConfig` bersama `Criterion::Seq` menghasilkan error
- [x] Urutan pipeline: hasil `narrow_candidates()` dipakai sebagai input exposure control (diuji via integrasi modul, memastikan urutan panggilan benar) — `narrowed_pool_composes_directly_with_rank_candidates`
- [x] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(ContentBalancingDebug)` terisi (proporsi empiris cocok hitungan manual) dan tidak mengubah pool kandidat yang dikembalikan

### Checklist Integration Test
- [x] Fixture R: sesi dengan `content` & `content_prop` tertentu (2 kategori, target 50/50) pada item bank 2D di mana kategori memetakan ke dimensi berbeda → skenario `main`, `tests/fixtures/generate_r_fixtures_content_balancing.R`
- [x] Bandingkan urutan item & kategori yang diprioritaskan tiap langkah, Rust vs R — deterministik penuh (kriteria `Drule`, tanpa exposure control), jadi dicocokkan EKSAK item demi item (lebih kuat dari sekadar proporsi realisasi) — `main_scenario_matches_r_item_sequence_exactly`
- [x] Fixture R: kasus kategori kehabisan item → verifikasi fallback (tidak error, kembali ke pool penuh) sama antara Rust & R — skenario `fallback`, `fallback_scenario_matches_r_item_sequence_exactly_and_never_errors` + `fallback_scenario_actually_exercises_the_fallback_path`

### Checklist Example Script
- [x] `examples/06_content_balancing.rs` — simulasi sesi dengan target 50/50 dua kategori, cetak proporsi realisasi tiap N item untuk menunjukkan konvergensi ke target

### Checklist Dokumentasi
- [x] `docs/teori/06-content-balancing.md` — algoritma penyeimbangan proporsi, kaitannya dengan dimensi laten pada MCAT, referensi akademik
- [x] `docs/panduan/06-content-balancing.md` — cara mendefinisikan kategori & target proporsi, catatan larangan kombinasi dengan `seq`

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] `cargo test` (220 test: 193 unit termasuk 12 `content_balancing`, 27 integrasi termasuk 3 `integration_content_balancing`) & `cargo clippy --all-targets -- -D warnings` hijau; `cargo fmt --check` bersih untuk seluruh file yang disentuh Fase 6 (`src/content_balancing/mod.rs`, `src/lib.rs`, `src/error.rs`, `tests/integration_content_balancing.rs`, `examples/06_content_balancing.rs`) -- sisa diff `cargo fmt --check` di file Fase 3/4 lama sudah ada SEBELUM Fase 6, di luar scope perubahan ini
- [x] Parity urutan item & fallback dengan fixture R terverifikasi EKSAK (lih. Checklist Integration Test)

---

## Fase 7 — Hasil Bersih Akhir untuk Theta pada MCAT

**Status: ✅ Selesai**

> **Update (2026-09-22)**: perilaku diverifikasi langsung terhadap source
> `mirtCAT` 1.14 terpasang (`getFromNamespace("mirtCAT_post_internal",
> "mirtCAT")`, `getMethod("Update.stop_now", "Design")`) -- bukan hanya
> dibaca dari sketsa `PLAN.md`/checkout source di bawah. Temuan penting:
> 1. **Checkout `mirtCAT/R/DesignClass.R`/`mirtCAT_preamble.R` BERBEDA dari
>    yang sungguh ter-compile di package terpasang** (pola sama seperti
>    Fase 2/3/6): field `terminated_early` yang dijanjikan checkout source
>    TIDAK PERNAH ditulis oleh `Update.stop_now` versi terpasang (logika
>    `stop_now`/`classify_decision`-nya sendiri IDENTIK, hanya 2 baris
>    `person$terminated_early <- TRUE` yang hilang), dan
>    `mirtCAT_post_internal()` terpasang mengembalikan field
>    `terminated_sucessfully` (GUI-only, selalu `FALSE` untuk sesi
>    `local_pattern` offline) -- BUKAN `terminated_early`. Jadi tidak ada
>    field R literal untuk memparitas-ujikan `McatResult::terminated_early`;
>    field itu tetap diteruskan dari `StopDecision::terminated_early` Rust
>    (Fase 4, logikanya sendiri sudah diverifikasi benar terpisah) --
>    didokumentasikan sebagai keterbatasan yang diketahui, bukan diabaikan
>    diam-diam. Lih. `docs/teori/07-hasil-akhir-theta.md` §1.
> 2. **`result::ClassifyDecision` (sketsa awal) ternyata duplikat** --
>    `stopping_rule::ClassifyDecision` (Fase 4) sudah persis enum yang
>    dibutuhkan, dipakai ulang langsung (Prinsip Desain #1).
> 3. **Accessor `info_thetas` "terpisah" (sketsa awal) ternyata sudah ada**
>    -- `PersonState::info_thetas(&self, bank)` (Fase 2) sudah persis
>    accessor yang dimaksud ("padanan `design_elements=TRUE` +
>    `extract.mirtCAT()`"); tidak ada kode baru yang perlu ditulis untuk
>    poin desain ini, hanya didokumentasikan cara memakainya.
> 4. **`thetas_history`/`thetas_se_history` R punya baris pra-tes ekstra**
>    (`nrow = N_item + 1`, dikonfirmasi lewat fixture R nyata) -- sengaja
>    TIDAK direplikasi: `PersonState::thetas_history()`/`thetas_se_history()`
>    (field baru di `PersonState`, Fase 2) panjangnya persis `N_item`,
>    sejalan 1:1 dengan `items_answered`/`responses`. Lih.
>    `docs/teori/07-hasil-akhir-theta.md` §3 untuk alasan lengkap.
> 5. **Parameter R `classify_CI` BUKAN langsung `classify_alpha`** yang
>    dipakai `qnorm()` -- `Design$initialize()` terpasang mentransformasi
>    `classify_alpha <- (1 - classify_CI) / 2` sebelum dipakai (temuan yang
>    SAMA yang sudah didokumentasikan `tests/integration_stopping_rule.rs`
>    Fase 4, dipakai ulang di fixture generator/test integrasi fase ini).
> 6. **`extract_info_matrix()` pada orkestrator sesi (sketsa awal) dihapus
>    dari scope fase ini** -- itu bagian orkestrasi sesi eksternal (belum
>    diimplementasikan), bukan `McatResult`/`PersonState`. Poin desain
>    "`info_thetas` opsional" (temuan #3) sudah tercapai lewat
>    `PersonState::info_thetas()` yang sudah ada, tanpa perlu orkestrator
>    sesi itu.
>
> Fixture R (`tests/fixtures/final_result.json`, dihasilkan via
> `Rscript tests/fixtures/generate_r_fixtures_final_result.R`) menjalankan
> sesi `mirtCAT()` 3 dimensi end-to-end (skenario identik `ci_session`
> Fase 4: classify CI, `min_items=3`) sampai BENAR-BENAR berhenti lebih
> awal (item ke-4 dari 30) dengan klasifikasi 3-dimensi CAMPURAN.
> Dicocokkan lewat replay (theta/SE diinjeksi dari fixture, pola sama
> Fase 5/6) + `stopping_rule::evaluate()` (Fase 4) sungguhan di langkah
> terakhir -> `McatResult::from_person()` (Fase 7), seluruh field
> dibandingkan terhadap fixture (`tests/integration_final_result.rs`).
> `cargo test --features serde` (233 test: 205 unit + 28 integrasi lintas
> 8 file, termasuk 10 test unit baru `result` + 1 `theta` serde roundtrip
> + 1 `item_selection` history, dan 1 test integrasi baru
> `integration_final_result`) & `cargo clippy --all-targets --features
> serde -- -D warnings` hijau, baik dengan maupun tanpa `--features serde`.

### Tujuan
Merangkum keluaran akhir sesi tes ke dalam struktur hasil yang bersih,
konsisten dengan bentuk output `mirtCAT` untuk `nfact > 1`.

### Rujukan Kode Sumber (R)
- `mirtCAT/R/mirtCAT_preamble.R:245-285` (`mirtCAT_post_internal`, checkout
  -- lih. catatan "Update" di atas soal perbedaan dengan versi terpasang)

### Perilaku yang Harus Disamai
- `thetas`: vektor `1 × nfact` (estimasi akhir tiap dimensi).
- `se_thetas`: vektor `1 × nfact` (baris terakhir dari `thetas_SE_history`).
- `thetas_history`: matriks `N_item × nfact` (riwayat seluruh dimensi
  bersamaan, satu baris per item terjawab).
- `thetas_se_history`: matriks `N_item × nfact`, sejalan dengan di atas.
- `classification`: vektor karakter panjang `nfact` — keputusan klasifikasi
  **per-dimensi** (bukan satu keputusan global), default `'no decision'`.
- Nama kolom konsisten: `Theta_1, Theta_2, ..., Theta_nfact`.
- **Matriks informasi kumulatif `info_thetas` (`nfact × nfact`) TIDAK ikut**
  dalam hasil akhir baku — hanya tersedia secara internal selama sesi
  berjalan. Disediakan lewat accessor **opsional** (padanan
  `design_elements=TRUE` + `extract.mirtCAT()` di R) untuk pengguna yang
  butuh kovarians estimasi secara manual.
- Hasil juga membawa metadata administrasi: item yang terjawab
  (`items_answered`), respons mentah, status `terminated_early`.

### Landasan Teori & Referensi Akademik
- Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
  61(2), 331–354. (interpretasi vektor `theta` akhir & kovariansnya)
- Chalmers, R. P. (2016). *JSS*, 71(5), 1–39. (struktur output referensi
  `mirtCAT`)

### Desain Modul Rust

> Diperbarui dari sketsa awal sesuai implementasi final (pola sama seperti
> Fase 2/4/6 — lih. catatan "Update" di atas untuk alasan tiap deviasi).

- `result::McatResult { thetas: ThetaVector, se_thetas: SEVector, thetas_history: Vec<ThetaVector>, thetas_se_history: Vec<SEVector>, classification: Vec<stopping_rule::ClassifyDecision>, column_names: Vec<String>, items_answered: Vec<usize>, responses: Vec<Option<i32>>, terminated_early: bool }` — turunan `serde::Serialize`/`Deserialize` di belakang feature `serde` (opt-in, bukan default). `ClassifyDecision` dipakai ulang dari `stopping_rule` (Fase 4), BUKAN enum baru (lih. "Update" #2).
- `result::McatResult::column_names(nfact) -> Vec<String>` — generator `Theta_1..Theta_nfact`.
- `result::McatResult::from_person(person: &PersonState, stop_decision: &StopDecision) -> Self` — perakitan dari `PersonState` final (Fase 2) + `StopDecision` terakhir (Fase 4); `classification` dinormalisasi selalu panjang `nfact` (kosong -> `NoDecision` broadcast, `Sprt` panjang 1 -> broadcast, `Ci` diteruskan apa adanya).
- `PersonState::thetas_history()`/`thetas_se_history() -> &[ThetaVector]`/`&[SEVector]` (field + accessor BARU di `item_selection::PersonState`, Fase 2 -- bukan modul `result`) — riwayat penuh tiap re-estimasi, di-push di `set_theta`/`set_estimate`, panjang `N_item` (deviasi sengaja dari R, lih. "Update" #4).
- `ThetaVector`/`SEVector` (`src/theta.rs`) mendapat `impl Serialize/Deserialize` manual (delegasi ke `Vec<f64>` polos lewat `as_slice()`/`from_vec()` yang sudah ada, BUKAN derive di atas `nalgebra::DVector` -- menghindari dependency tambahan `nalgebra/serde-serialize`) di belakang `#[cfg(feature = "serde")]`, dipakai ulang lewat macro `impl_vector_newtype!` yang sudah ada.
- Accessor `info_thetas` opsional: **bukan kode baru** -- `PersonState::info_thetas(&self, bank: &ItemBank)` (Fase 2, sudah ada) dipakai ulang apa adanya (lih. "Update" #3, #6). `extract_info_matrix()` pada orkestrator sesi (sketsa awal) dihapus dari scope fase ini.

### Checklist Implementasi
- [x] `McatResult` (struct) -- `ClassifyDecision` dipakai ulang dari Fase 4, bukan didefinisikan ulang
- [x] Perakitan `McatResult` dari `PersonState` final (thetas, SE, history, classification) — `McatResult::from_person()`
- [x] `column_names()` — `Theta_1..Theta_nfact`
- [x] Feature flag `serde` untuk `McatResult` (dan struct terkait: `ThetaVector`, `SEVector`, `ClassifyDecision`) agar mudah diserialisasi ke JSON
- [x] Accessor opsional `info_thetas` terpisah dari `McatResult` (bukan bagian hasil baku) — `PersonState::info_thetas()` (Fase 2, reuse)
- [x] `PersonState::thetas_history()`/`thetas_se_history()` — riwayat penuh, prasyarat perakitan `McatResult` yang belum ada sebelum fase ini

### Checklist Unit Test
- [x] `McatResult` terisi benar dari sesi sintetis 3 dimensi (histori 5 item) — cek dimensi matriks histori `5 × 3` — `from_person_fills_thetas_history_matrix_with_n_item_rows`
- [x] `column_names()` menghasilkan `["Theta_1","Theta_2","Theta_3"]` untuk `nfact=3` — `column_names_generates_theta_1_through_nfact`
- [x] `classification` default `NoDecision` sepanjang `nfact` bila `classify_type=None` — `classification_defaults_to_no_decision_for_every_dimension_when_classify_type_none` (+ `Ci`/`Sprt`: `classification_passes_through_per_dimension_decisions_for_ci_mode`, `classification_broadcasts_single_sprt_decision_to_every_dimension`)
- [x] `info_thetas` **tidak** ada di field `McatResult` (dicek secara struktural/tipe lewat exhaustive destructuring), hanya dapat diakses lewat `PersonState::info_thetas()` (bukan orkestrator sesi eksternal -- itu belum diimplementasikan, lih. "Update" #6) — `info_thetas_is_not_a_field_of_mcat_result_structurally`
- [x] Serialisasi JSON (`serde`) menghasilkan struktur field yang diharapkan (uji round-trip serialize/deserialize) — `mcat_result_serializes_and_deserializes_round_trip` (+ `theta_vector_and_se_vector_serde_roundtrip_as_plain_json_array` di `theta.rs`)
- [x] `PersonState::thetas_history()` panjang `N_item` (bukan `N_item+1`), baris terakhir sejalan `theta()`/`se()` saat ini — `person_state_thetas_history_has_one_row_per_reestimation_not_nfact_plus_one`

### Checklist Integration Test
- [x] Fixture R: sesi penuh 3 dimensi hingga selesai (berhenti lebih awal via classify CI, item ke-4 dari 30) → ekstrak `thetas`, `SE_thetas`, `thetas_history`, `thetas_SE_history`, `classification` dari objek hasil `mirtCAT` sungguhan — `tests/fixtures/generate_r_fixtures_final_result.R`
- [x] Bandingkan seluruh field `McatResult` Rust vs fixture R (toleransi numerik untuk theta/SE, kecocokan eksak untuk dimensi matriks & nama kolom) — `tests/integration_final_result.rs::mcat_result_matches_r_final_result_fixture_after_ci_early_stop` (replay + `stopping_rule::evaluate()` Fase 4 sungguhan di langkah terakhir, membuktikan pipa Fase 4 -> Fase 7 benar)

### Checklist Example Script
- [x] `examples/07_final_result.rs` — jalankan sesi sintetis end-to-end (Fase 1–4: initial theta, item selection, ability estimation, stopping rule), cetak `McatResult` lengkap termasuk histori, lalu serialisasi ke JSON (`cargo run --example 07_final_result --features serde`)

### Checklist Dokumentasi
- [x] `docs/teori/07-hasil-akhir-theta.md` — interpretasi `theta`/SE akhir multidimensional, kenapa `info_thetas` sengaja tidak dikembalikan default, temuan checkout-vs-terpasang (`terminated_early`), deviasi `thetas_history`/`classification`, referensi akademik
- [x] `docs/panduan/07-hasil-akhir-theta.md` — cara membaca `McatResult`, cara mengakses `info_thetas` opsional, contoh serialisasi ke JSON untuk API

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] `cargo test --features serde` (233 test: 205 unit termasuk 12 baru [`result` 9 + `theta` 1 + `item_selection` 1 + `classify_type_default_still_produces_no_decision_vector`], 28 integrasi termasuk 1 `integration_final_result`) & `cargo clippy --all-targets --features serde -- -D warnings` hijau; sama-sama hijau TANPA `--features serde` (203 unit + 28 integrasi, 2 test serde di-skip lewat `#[cfg(feature = "serde")]`); `cargo fmt --check` bersih untuk seluruh file yang disentuh Fase 7 (`src/result.rs`, `src/theta.rs` [+baris serde], `src/lib.rs` [+2 baris], `src/stopping_rule/mod.rs` [+1 baris derive], `src/item_selection/mod.rs` [+field/method/test histori], `tests/integration_final_result.rs`, `examples/07_final_result.rs`, `Cargo.toml`) -- sisa diff `cargo fmt --check` di file Fase 3/4 lama (`src/ability_estimation/*.rs`, sisa `src/stopping_rule/mod.rs` di luar baris yang disentuh fase ini) sudah ada SEBELUM Fase 7, di luar scope perubahan ini
- [x] Parity struktur & nilai hasil akhir dengan fixture R terverifikasi (lih. Checklist Integration Test); keterbatasan `terminated_early` (tidak ada field R literal di package terpasang) didokumentasikan eksplisit, bukan diabaikan diam-diam (lih. catatan "Update" #1)

---

## Fase 8 — Hardening, Validasi Silang Menyeluruh & Kesiapan Rilis

**Status: ✅ Selesai**

### Tujuan
Menaikkan tingkat kepercayaan terhadap kebenaran & kestabilan engine sebelum
dipublikasikan sebagai crate yang bisa dipakai project lain.

### Checklist
- [x] Audit menyeluruh: jalankan **seluruh** skrip `generate_r_fixtures.R`
      dari Fase 0–7 dalam satu pipeline CI opsional (manual/nightly, karena
      butuh R terpasang) untuk mendeteksi regresi parity —
      `.github/workflows/r-fixtures-audit.yml` (`workflow_dispatch` +
      nightly cron), meregenerasi seluruh `tests/fixtures/generate_r_fixtures*.R`
      lalu `cargo test --all-targets --features serde` terhadap hasilnya;
      butuh R + `mirt`/`mirtCAT` terpasang jadi tidak dijalankan di sandbox
      implementasi ini (sama seperti keterbatasan fixture generation
      Fase 0–7 sebelumnya), diverifikasi lewat inspeksi (skrip lengkap,
      working directory benar, trigger benar)
- [x] Property-based testing (`proptest`) untuk invariant lintas modul (mis.
      probabilitas kategori selalu menjumlah 1, matriks informasi selalu
      positive semi-definite, proporsi content balancing tidak pernah
      negatif) — `tests/proptest_invariants.rs` (4 property test, seluruh
      lolos)
- [x] Benchmark dasar (`criterion`) untuk kriteria seleksi item & estimasi
      ability pada item bank besar (mis. 500 item, 5 dimensi) — memastikan
      layak dipakai real-time di web API — `benches/mcat_engine.rs`
      (`cargo bench`), seluruh kriteria D/T/A/E/W-rule/KL/KLn/seq/random &
      method EAP(grid/QMC)/MAP/ML/WLE/EapSum, dikonfirmasi jalan end-to-end
      (mis. `d_rule` ~150µs, `eap_qmc` ~4µs pada bank 500 item/5 dimensi —
      jauh di bawah ambang real-time; `eap_grid`/`eap_sum` ~17-26ms,
      selaras dengan peringatan bawaan `Method::Eap { qmc: false }` untuk
      `nfact > 4`)
- [x] Dokumentasi API level crate (`//!` di `lib.rs`, `cargo doc` bersih
      tanpa warning `missing_docs`) — `#![deny(missing_docs)]` di
      `src/lib.rs`, seluruh 74 item publik yang sebelumnya tanpa dokumentasi
      dilengkapi, 26 broken intra-doc link ke item privat diperbaiki;
      `cargo doc --no-deps` (default & `--features serde`) nol warning
- [x] `README.md` — ringkasan, instalasi, quickstart, tautan ke `docs/teori`
      dan `docs/panduan`, pernyataan scope/non-scope, atribusi ke `mirt`/
      `mirtCAT` sebagai rujukan konseptual
- [x] Keputusan & penambahan file lisensi (`LICENSE-MIT`/`LICENSE-APACHE`
      atau sesuai keputusan Fase 0 §1.3) — dual `MIT OR Apache-2.0` (sesuai
      rekomendasi §1.3, sudah tercermin di `Cargo.toml` sejak Fase 0)
- [x] `CHANGELOG.md` awal (mengikuti Keep a Changelog)
- [x] Review keamanan dasar: tidak ada `unsafe` tanpa justifikasi tertulis,
      tidak ada panic pada input pengguna yang tidak tervalidasi (semua
      jalur error via `Result`) — dikonfirmasi: nol `unsafe` di `src/`;
      seluruh `.expect()`/`unreachable!()` non-test di `src/` (10 titik:
      `ability_estimation/{wle,map,ml}.rs`, `content_balancing/mod.rs`,
      `exposure_control/randomesque.rs`, `item_selection/criteria/{random,seq}.rs`,
      `item_selection/mod.rs`) diverifikasi menjaga invariant yang SUDAH
      divalidasi sebelumnya (`validate_answers`, exhaustive match, cek
      non-kosong), bukan reachable dari input eksternal tak tervalidasi —
      lih. `CHANGELOG.md` §Fase 8 untuk rincian
- [x] Kesiapan publish `cargo publish --dry-run` bersih

### Definition of Done
- [x] Semua checklist di atas tercentang
- [x] Seluruh fase 0–7 berstatus ✅ Selesai
- [x] `cargo test`, `cargo clippy -- -D warnings`, `cargo doc --no-deps` semua hijau di CI

---

## Lampiran — Daftar Pustaka Lengkap

1. Bock, R. D., & Aitkin, M. (1981). Marginal maximum likelihood estimation
   of item parameters: Application of an EM algorithm. *Psychometrika*,
   46(4), 443–459.
2. Cai, L. (2010). High-dimensional exploratory item factor analysis by a
   Metropolis–Hastings Robbins–Monro algorithm. *Psychometrika*, 75(1),
   33–57.
3. Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
   Package for the R Environment. *Journal of Statistical Software*, 48(6),
   1–29. doi:10.18637/jss.v048.i06
4. Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
   Interfaces for Multidimensional Item Response Theory Applications.
   *Journal of Statistical Software*, 71(5), 1–39. doi:10.18637/jss.v071.i05
5. Chang, H.-H., & Ying, Z. (1996). A global information approach to
   computerized adaptive testing. *Applied Psychological Measurement*,
   20(3), 213–229.
6. Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
   item exposure control strategies for computerized adaptive testing
   developed on the item response theory framework. *International Journal
   on E-Learning*, 6(1).
7. Kingsbury, G. G., & Zara, A. R. (1989). Procedures for selecting items
   for computerized adaptive tests. *Applied Measurement in Education*,
   2(4), 359–375.
8. McBride, J. R., & Martin, J. T. (1983). Reliability and validity of
   adaptive ability tests in a military setting. In *New Horizons in
   Testing* (pp. 223–236). Academic Press.
9. Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
   testing with optimal design criteria for item selection. *Psychometrika*,
   74(2), 273–296.
10. Muraki, E., & Carlson, J. E. (1995). Full-information factor analysis
    for polytomous item responses. *Applied Psychological Measurement*,
    19(1), 73–90.
11. Reckase, M. D. (1985). The difficulty of test items that measure more
    than one ability. *Applied Psychological Measurement*, 9(4), 401–412.
12. Reckase, M. D. (2009). *Multidimensional Item Response Theory*.
    Springer.
13. Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
    61(2), 331–354.
14. Sympson, J. B., & Hetter, R. D. (1985). Controlling item-exposure rates
    in computerized adaptive testing. *Proceedings of the 27th Annual
    Meeting of the Military Testing Association*, 973–977.
15. van der Linden, W. J. (1999). Multidimensional adaptive testing with a
    minimum error-variance criterion. *Journal of Educational and
    Behavioral Statistics*, 24(4), 398–412.
16. van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
    Adaptive Testing*. Springer.
17. Veldkamp, B. P., & van der Linden, W. J. (2002). Multidimensional
    adaptive testing with constraints on test content. *Psychometrika*,
    67(4), 575–588.
18. Wald, A. (1945). Sequential tests of statistical hypotheses. *Annals of
    Mathematical Statistics*, 16(2), 117–186.
19. Warm, T. A. (1989). Weighted likelihood estimation of ability in item
    response theory. *Psychometrika*, 54(3), 427–450.

---

## Cara Memperbarui Dokumen Ini

1. Sebelum mulai bekerja pada suatu fase, ubah status fase itu di §5 dari
   `⬜ Belum Dimulai` → `🟡 Sedang Dikerjakan`.
2. Centang `[ ]` → `[x]` **hanya** setelah item benar-benar selesai *dan*
   lulus test terkait (implementasi tanpa test yang lulus belum dianggap
   selesai).
3. Setelah seluruh checklist + Definition of Done suatu fase tercentang,
   ubah status fase itu → `✅ Selesai` di §5 dan di header fase
   bersangkutan.
4. Jangan mulai fase berikutnya sebelum fase sebelumnya berstatus
   `✅ Selesai` (dependency tipe/trait lintas fase).
