# mirtcat-engine — Rencana Implementasi

> Engine Rust untuk **Multidimensional Computerized Adaptive Testing (MCAT)**,
> mengacu pada algoritma dan perilaku library **mirt** dan **mirtCAT** (R),
> dipersempit khusus ke skenario `nfact > 1` (multidimensional IRT).
> Didesain agar bisa dipakai sebagai dependency biasa (`cargo add mirtcat-engine`)
> di web API Rust (mis. Axum) maupun project Rust lain, tanpa terikat framework
> tertentu.

Status dokumen: **draft rencana**, belum ada implementasi. Update checklist
`[ ]` → `[x]` di tiap item begitu item itu benar-benar selesai dikerjakan &
lulus test — jangan tandai selesai berdasarkan asumsi.

---

## 1. Ringkasan & Tujuan

`mirtcat-engine` menyediakan **mesin administrasi tes adaptif multidimensional**
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
  dependency ke Axum/Actix/dll di crate inti. Contoh integrasi Axum disediakan
  terpisah (lihat Fase 8).
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

`mirt` dan `mirtCAT` berlisensi **GPL (>= 3)**. Agar `mirtcat-engine` bisa
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
   `dependencies`; `axum` dkk. hanya masuk sebagai `dev-dependencies` untuk
   contoh (Fase 8).
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
   bisa diekspos lewat endpoint "debug" di web API (Fase 8) untuk audit/
   transparansi keputusan adaptif. Fase 0 (fondasi IRT) dikecualikan —
   `ItemResponseModel::probabilities()`/`information()` sudah *mengembalikan*
   seluruh hasil kalkulasi secara langsung, tidak ada keputusan tersembunyi
   yang perlu di-debug.

---

## 3. Struktur Direktori (target akhir)

```
mirtcat-engine/
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
│   ├── result.rs                   # McatResult — Fase 7
│   └── session.rs                  # McatSession orchestrator — Fase 8
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
│   ├── 04_stopping_rule.rs
│   ├── 05_exposure_control.rs
│   ├── 06_content_balancing.rs
│   ├── 07_final_result.rs
│   └── 08_axum_api/                # contoh integrasi web API (Fase 8)
├── tests/
│   ├── fixtures/
│   │   ├── generate_r_fixtures.R   # skrip R — sumber ground truth
│   │   └── *.json                  # snapshot hasil R
│   └── integration_*.rs
├── benches/                        # opsional, criterion — Fase 9
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
    │   ├── 04-stopping-rule.md
    │   ├── 05-exposure-control.md
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
        ├── 04-stopping-rule.md
        ├── 05-exposure-control.md
        ├── 06-content-balancing.md
        ├── 07-hasil-akhir-theta.md
        └── 08-integrasi-axum.md
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
| — | Integrasi end-to-end (session loop) + contoh Axum | **Fase 8** | `mirtCAT/R/mirtCAT.R` (alur utama), `updateDesign.R` |
| — | Hardening, parity testing menyeluruh, rilis | **Fase 9** | — |

**Urutan pengerjaan wajib sekuensial** (Fase 0 → 9) karena tiap fase
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
| 3 | Algoritma Estimasi Kemampuan | ⬜ Belum Dimulai |
| 4 | Stopping Rule | ⬜ Belum Dimulai |
| 5 | Exposure Control | ⬜ Belum Dimulai |
| 6 | Content Balancing | ⬜ Belum Dimulai |
| 7 | Hasil Bersih Akhir untuk Theta | ⬜ Belum Dimulai |
| 8 | Integrasi End-to-End & Contoh Axum | ⬜ Belum Dimulai |
| 9 | Hardening, Validasi Silang & Rilis | ⬜ Belum Dimulai |

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
- [x] `cargo test -p mirtcat-engine initial_theta` hijau (juga `cargo test` penuh: 45 test hijau, `cargo clippy --all-targets -- -D warnings` bersih, `cargo fmt --check` bersih)
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

**Status: ⬜ Belum Dimulai**

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
- [ ] `ability_estimation::Method` (enum) & `EstimationConfig`
- [ ] `quadrature::build_grid` — grid reguler `quadpts^nfact`, dengan tabel default `quadpts` per `nfact`
- [ ] `quadrature::build_qmc_points` — titik QMC (Sobol atau Halton) sebagai alternatif untuk `nfact` besar
- [ ] `eap.rs` — EAP (grid-based & QMC-based)
- [ ] `map.rs` — MAP (optimisasi langsung dengan prior `N(gmeans, gcov)`)
- [ ] `ml.rs` — ML (optimisasi likelihood tanpa prior)
- [ ] `wle.rs` — WLE (weighted likelihood, koreksi bias Warm)
- [ ] `eap_sum.rs` — EAP berbasis summed score
- [ ] `fixed.rs` — tidak melakukan apa-apa (identity, mengembalikan theta/SE sebelumnya)
- [ ] Peringatan (log/tracing) untuk kombinasi `EAP` + `nfact > 4` tanpa QMC
- [ ] Dukungan debug trace opsional (`EstimationConfig::debug`, default `false`) → `Option<AbilityEstimationDebug>` (jumlah titik kuadratur, iterasi, jejak log-likelihood, status konvergensi)

### Checklist Unit Test
- [ ] EAP grid-based konvergen ke nilai analitik pada kasus sintetis 1D sederhana (sanity check dasar sebelum diuji multidimensional)
- [ ] MAP & ML menghasilkan estimasi berbeda saat prior informatif (`gcov` sempit) — MAP tertarik ke prior, ML tidak
- [ ] WLE menghasilkan estimasi dengan bias lebih kecil dibanding ML pada simulasi respons ekstrem (semua benar/semua salah, kasus di mana ML bisa divergen ke ±∞ sementara WLE tetap terbatas)
- [ ] `EapSum` menghasilkan estimasi berbasis kategori summed score sesuai definisi
- [ ] `Fixed` tidak mengubah theta/SE input
- [ ] `quadrature::build_grid` menghasilkan jumlah titik `quadpts^nfact` sesuai tabel default, dan bisa dioverride manual
- [ ] Peringatan ter-trigger untuk `nfact=5, method=Eap{qmc:false}` (dites via capture log/tracing, bukan panic)
- [ ] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(AbilityEstimationDebug)` terisi (mis. `n_quadrature_points` cocok dengan `build_grid`) dan tidak mengubah `(ThetaVector, SEVector)` yang dikembalikan

### Checklist Integration Test
- [ ] Fixture R: pola respons tetap pada item bank 2–3 dimensi, estimasi `fscores()` dengan tiap method (`EAP`,`MAP`,`ML`,`WLE`,`EAPsum`) → simpan theta & SE hasil R
- [ ] Bandingkan hasil Rust vs R per method (toleransi numerik, EAP/MAP/ML diharapkan sangat dekat; WLE/EAPsum ditoleransi lebih longgar karena kemungkinan detail numerik solver berbeda — didokumentasikan eksplisit jika ada deviasi terukur)
- [ ] Test `nfact=5` — bandingkan ukuran grid kuadratur default Rust vs `quadpts` default R

### Checklist Example Script
- [ ] `examples/03_ability_estimation_eap_map_ml_wle.rs` — pola respons yang sama diestimasi dengan tiap method, cetak perbandingan theta/SE & waktu komputasi relatif

### Checklist Dokumentasi
- [ ] `docs/teori/03-ability-estimation.md` — rumus EAP/MAP/ML/WLE/EAPsum multidimensional, isu integrasi dimensi tinggi & QMC, referensi akademik
- [ ] `docs/panduan/03-ability-estimation.md` — cara memilih method & `quadpts`, kapan pakai QMC

### Definition of Done
- [ ] Semua checklist di atas tercentang
- [ ] `cargo test -p mirtcat-engine ability_estimation` hijau
- [ ] Parity numerik EAP/MAP/ML dengan fixture R terverifikasi dalam toleransi yang didokumentasikan

---

## Fase 4 — Stopping Rule pada MCAT

**Status: ⬜ Belum Dimulai**

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
- [ ] `ClassifyType`, `StoppingConfig`, `StopDecision`
- [ ] `sem.rs` — evaluasi `met_SEM` per-dimensi + `all()`
- [ ] `delta_theta.rs` — evaluasi `met_delta_thetas` per-dimensi + `all()`
- [ ] `classify_ci.rs` — z-score per dimensi vs cutoff, keputusan per-dimensi, `all()` untuk stop
- [ ] `sprt.rs` — log-likelihood ratio test terhadap `theta_lower`/`theta_upper`
- [ ] `max_items_time.rs` — kondisi `max_items`/`max_time`
- [ ] Dukungan `CustomStop` (trait objek/closure)
- [ ] Guard `allow_constrain_breaks`
- [ ] Dukungan debug trace opsional (`StoppingConfig::debug`, default `false`) → `Option<StoppingRuleDebug>` (nilai SE/delta-theta mentah, rasio log-likelihood SPRT)

### Checklist Unit Test
- [ ] `met_SEM` — stop hanya ketika **semua** dimensi di bawah `min_SEM`; satu dimensi belum memenuhi → tidak stop
- [ ] `met_delta_thetas` — sama, semantik `all()`
- [ ] `Ci` classify — kasus 2 dimensi, satu terklasifikasi yakin & satu belum → belum stop; keduanya yakin → stop dengan keputusan per-dimensi yang benar
- [ ] `Sprt` — kasus respons konsisten dengan `theta_upper` → keputusan `'above cutoff'`; konsisten `theta_lower` → `'below cutoff'`; ambigu → belum stop
- [ ] `max_items` & `max_time` menghentikan tes terlepas dari precision
- [ ] `CustomStop` closure dipanggil dan hasilnya dihormati
- [ ] `allow_constrain_breaks=false` mencegah stop di tengah blok `ordered` constraint yang belum selesai
- [ ] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(StoppingRuleDebug)` terisi (mis. `se_values` cocok dengan SE aktual di `person_state`) dan tidak mengubah `StopDecision` yang dikembalikan

### Checklist Integration Test
- [ ] Fixture R: simulasi sesi penuh 3 dimensi dengan `min_SEM` tertentu → catat item ke berapa tes berhenti & keputusan akhir dari `mirtCAT`
- [ ] Bandingkan titik-berhenti & keputusan klasifikasi Rust vs R
- [ ] Fixture R terpisah untuk mode `SPRT` (dua hipotesis vektor), bandingkan keputusan (`above`/`below`/`no decision`) & item ke berapa berhenti

### Checklist Example Script
- [ ] `examples/04_stopping_rule.rs` — simulasi sesi dengan berbagai kombinasi (`min_SEM` saja, `delta_thetas`, `CI`, `SPRT`, `max_items`), cetak kapan & kenapa tes berhenti

### Checklist Dokumentasi
- [ ] `docs/teori/04-stopping-rule.md` — semantik "all dimensi", SPRT multidimensional, referensi akademik
- [ ] `docs/panduan/04-stopping-rule.md` — cara memilih kombinasi aturan berhenti yang realistis untuk `nfact` besar

### Definition of Done
- [ ] Semua checklist di atas tercentang
- [ ] `cargo test -p mirtcat-engine stopping_rule` hijau
- [ ] Parity titik-berhenti & keputusan klasifikasi dengan fixture R terverifikasi

---

## Fase 5 — Exposure Control pada MCAT

**Status: ⬜ Belum Dimulai**

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
- `exposure_control::ExposureMode` — enum `SympsonHetter(Vec<f64>)`, `Randomesque(Vec<u32>)`.
- `exposure_control::ExposureMode::from_values(values: &[f64]) -> Result<Self, MirtCatError>` — replikasi deteksi mode otomatis R.
- `exposure_control::ExposureControlConfig { mode: ExposureMode, debug: bool }` — `debug` default `false` (prinsip desain #7); membungkus `mode` supaya `select_with_exposure` punya satu parameter config yang konsisten dengan fase lain.
- `exposure_control::ExposureControlDebug { attempts: Vec<(ItemIndex, bool /* diterima */)>, forced_accept: bool }` — urutan kandidat yang dicoba & diterima/ditolak (Sympson–Hetter), atau kandidat top-*n* yang dipertimbangkan (Randomesque); `forced_accept` menandai jalur fallback "kandidat tersisa satu".
- `exposure_control::select_with_exposure(config, ranked_candidates, position_in_test, person_valid_items, rng) -> (ItemIndex, Option<ExposureControlDebug>)`.
- Validasi panjang vektor `== nitems` saat konstruksi.

### Checklist Implementasi
- [ ] `ExposureMode` (enum) + `from_values()` (deteksi otomatis gaya R)
- [ ] Validasi panjang vektor `== nitems`, validasi `Randomesque` berisi integer `≥ 1`
- [ ] `sympson_hetter.rs` — loop terima/tolak + penandaan `valid_item=false` per-sesi + fallback paksa-terima saat kandidat tersisa satu
- [ ] `randomesque.rs` — ranking kandidat + pemilihan acak dari top-*n* berdasarkan posisi urutan penyajian
- [ ] Error eksplisit saat kandidat habis (bukan panic)
- [ ] Dukungan debug trace opsional (`ExposureControlConfig::debug`, default `false`) → `Option<ExposureControlDebug>` (urutan kandidat dicoba/diterima/ditolak, status fallback paksa-terima)

### Checklist Unit Test
- [ ] `from_values()` — semua di `[0,1]` → `SympsonHetter`; ada `>1` → `Randomesque`; nilai `>1` non-integer → error
- [ ] `Randomesque` dengan `exposure=1` selalu memilih kandidat terbaik (deterministik)
- [ ] `Randomesque` dengan `exposure=3` memilih dari 3 kandidat teratas (properti statistik: distribusi pilihan mencakup ketiga kandidat dalam banyak sampel, RNG seeded untuk reprodusibilitas)
- [ ] `SympsonHetter` — item dengan `exposure[item]=0.0` tidak pernah lolos dalam banyak percobaan (RNG seeded)
- [ ] `SympsonHetter` — item dengan `exposure[item]=1.0` selalu lolos begitu jadi kandidat terbaik
- [ ] `SympsonHetter` — fallback paksa-terima saat kandidat tersisa satu meski `exposure` rendah
- [ ] Item yang ditolak `SympsonHetter` ditandai tidak valid hanya untuk sesi berjalan, tidak memengaruhi sesi/peserta lain (state terisolasi per `PersonState`)
- [ ] Error dikembalikan (bukan panic) saat seluruh kandidat tersaring habis
- [ ] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(ExposureControlDebug)` mencatat setiap kandidat yang dicoba (termasuk yang ditolak) dan tidak mengubah item yang dikembalikan

### Checklist Integration Test
- [ ] Fixture R: sesi dengan `exposure` mode `'SH'` (RNG seed tetap direplikasi setepat mungkin) → bandingkan pola penerimaan/penolakan item vs R secara statistik (distribusi kemunculan item lintas banyak simulasi peserta, bukan kecocokan RNG stream 1:1 karena RNG R vs Rust berbeda)
- [ ] Fixture R: sesi dengan mode `'sample'` (`exposure[posisi]=3`) → bandingkan bahwa item terpilih selalu berasal dari top-3 kandidat kriteria pada posisi tsb (Rust vs R, dicek sebagai himpunan valid, bukan kecocokan item persis karena keduanya random)
- [ ] Uji integrasi dengan Fase 2: exposure control mengonsumsi hasil kriteria (`crit` values) dengan benar sebagai input ranking

### Checklist Example Script
- [ ] `examples/05_exposure_control.rs` — simulasi 1000 peserta sintetis, cetak distribusi frekuensi kemunculan tiap item untuk mode `SH` vs `Randomesque` vs tanpa exposure control (bukti visual bahwa exposure control meratakan distribusi)

### Checklist Dokumentasi
- [ ] `docs/teori/05-exposure-control.md` — algoritma Sympson–Hetter & randomesque, referensi akademik
- [ ] `docs/panduan/05-exposure-control.md` — cara memilih & mengkalibrasi nilai `exposure`, peringatan risiko kehabisan item saat digabung content balancing

### Definition of Done
- [ ] Semua checklist di atas tercentang
- [ ] `cargo test -p mirtcat-engine exposure_control` hijau
- [ ] Parity statistik distribusi eksposur dengan fixture R terverifikasi

---

## Fase 6 — Content Balancing pada MCAT

**Status: ⬜ Belum Dimulai**

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
- [ ] `ContentCategory`, `ContentBalancingConfig` (+ validasi kategori & proporsi)
- [ ] Perhitungan proporsi empiris dari item terjawab (semua kategori selalu muncul walau 0)
- [ ] Logika pemilihan kategori "paling kurang jatah" (`argmax(target - empirical)`)
- [ ] `narrow_candidates()` dengan fallback ke pool penuh saat kategori target habis
- [ ] Validasi larangan kombinasi dengan `Criterion::Seq`
- [ ] Dukungan debug trace opsional (`ContentBalancingConfig::debug`, default `false`) → `Option<ContentBalancingDebug>` (proporsi empiris & defisit tiap kategori, kategori terpilih, status fallback)

### Checklist Unit Test
- [ ] Kategori yang belum pernah terjawab tetap dihitung 0 dalam proporsi empiris (bukan hilang dari perhitungan)
- [ ] Kategori dengan defisit target terbesar dipilih sebagai prioritas (kasus 3 kategori dengan defisit berbeda)
- [ ] Fallback ke pool penuh saat kategori prioritas kehabisan item kandidat
- [ ] Konstruksi `ContentBalancingConfig` bersama `Criterion::Seq` menghasilkan error
- [ ] Urutan pipeline: hasil `narrow_candidates()` dipakai sebagai input exposure control (diuji via integrasi modul, memastikan urutan panggilan benar)
- [ ] `debug=false` → elemen ke-2 `None`; `debug=true` → `Some(ContentBalancingDebug)` terisi (proporsi empiris cocok hitungan manual) dan tidak mengubah pool kandidat yang dikembalikan

### Checklist Integration Test
- [ ] Fixture R: sesi dengan `content` & `content_prop` tertentu (mis. 2 kategori, target 50/50) pada item bank 2D di mana kategori memetakan ke dimensi berbeda → catat proporsi realisasi tiap kategori sepanjang sesi dari `mirtCAT`
- [ ] Bandingkan proporsi realisasi & urutan kategori yang diprioritaskan tiap langkah, Rust vs R
- [ ] Fixture R: kasus kategori kehabisan item → verifikasi fallback (tidak error, kembali ke pool penuh) sama antara Rust & R

### Checklist Example Script
- [ ] `examples/06_content_balancing.rs` — simulasi sesi dengan target 50/50 dua kategori, cetak proporsi realisasi tiap N item untuk menunjukkan konvergensi ke target

### Checklist Dokumentasi
- [ ] `docs/teori/06-content-balancing.md` — algoritma penyeimbangan proporsi, kaitannya dengan dimensi laten pada MCAT, referensi akademik
- [ ] `docs/panduan/06-content-balancing.md` — cara mendefinisikan kategori & target proporsi, catatan larangan kombinasi dengan `seq`

### Definition of Done
- [ ] Semua checklist di atas tercentang
- [ ] `cargo test -p mirtcat-engine content_balancing` hijau
- [ ] Parity proporsi realisasi dengan fixture R terverifikasi

---

## Fase 7 — Hasil Bersih Akhir untuk Theta pada MCAT

**Status: ⬜ Belum Dimulai**

### Tujuan
Merangkum keluaran akhir sesi tes ke dalam struktur hasil yang bersih,
konsisten dengan bentuk output `mirtCAT` untuk `nfact > 1`.

### Rujukan Kode Sumber (R)
- `mirtCAT/R/mirtCAT_preamble.R:245-285` (`mirtCAT_post_internal`)

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
- `result::ClassifyDecision` — enum `AboveCutoff`, `BelowCutoff`, `NoDecision`.
- `result::McatResult { thetas: ThetaVector, se_thetas: SEVector, thetas_history: Vec<ThetaVector>, thetas_se_history: Vec<SEVector>, classification: Vec<ClassifyDecision>, column_names: Vec<String>, items_answered: Vec<usize>, responses: Vec<Option<i32>>, terminated_early: bool }` — turunan `serde::Serialize` di belakang feature `serde`.
- `result::McatResult::column_names()` — generator `Theta_1..Theta_nfact`.
- `session::McatSession::extract_info_matrix(&self) -> Option<&InfoMatrix>` — accessor opsional selama sesi masih hidup (bukan bagian `McatResult`), setara `extract.mirtCAT()` + `design_elements=TRUE`.

### Checklist Implementasi
- [ ] `ClassifyDecision` (enum) & `McatResult` (struct)
- [ ] Perakitan `McatResult` dari `PersonState` final (thetas, SE, history, classification)
- [ ] `column_names()` — `Theta_1..Theta_nfact`
- [ ] Feature flag `serde` untuk `McatResult` (dan struct terkait) agar mudah diserialisasi ke JSON
- [ ] Accessor opsional `info_thetas` terpisah dari `McatResult` (bukan bagian hasil baku)

### Checklist Unit Test
- [ ] `McatResult` terisi benar dari sesi sintetis 3 dimensi (histori 5 item) — cek dimensi matriks histori `5 × 3`
- [ ] `column_names()` menghasilkan `["Theta_1","Theta_2","Theta_3"]` untuk `nfact=3`
- [ ] `classification` default `NoDecision` sepanjang `nfact` bila `classify_type=None`
- [ ] `info_thetas` **tidak** ada di field `McatResult` (dicek secara struktural/tipe), hanya dapat diakses lewat `McatSession` sebelum sesi selesai
- [ ] Serialisasi JSON (`serde`) menghasilkan struktur field yang diharapkan (uji round-trip serialize/deserialize)

### Checklist Integration Test
- [ ] Fixture R: sesi penuh 3 dimensi hingga selesai → ekstrak `thetas`, `SE_thetas`, `thetas_history`, `thetas_SE_history`, `classification` dari objek hasil `mirtCAT`
- [ ] Bandingkan seluruh field `McatResult` Rust vs fixture R (toleransi numerik untuk theta/SE, kecocokan eksak untuk dimensi matriks & nama kolom)

### Checklist Example Script
- [ ] `examples/07_final_result.rs` — jalankan sesi sintetis end-to-end (memakai Fase 1–6), cetak `McatResult` lengkap termasuk histori, lalu serialisasi ke JSON

### Checklist Dokumentasi
- [ ] `docs/teori/07-hasil-akhir-theta.md` — interpretasi `theta`/SE akhir multidimensional, kenapa `info_thetas` sengaja tidak dikembalikan default, referensi akademik
- [ ] `docs/panduan/07-hasil-akhir-theta.md` — cara membaca `McatResult`, cara mengakses `info_thetas` opsional, contoh serialisasi ke JSON untuk API

### Definition of Done
- [ ] Semua checklist di atas tercentang
- [ ] `cargo test -p mirtcat-engine result` hijau
- [ ] Parity struktur & nilai hasil akhir dengan fixture R terverifikasi

---

## Fase 8 — Integrasi End-to-End (Session Orchestrator) & Contoh Axum

**Status: ⬜ Belum Dimulai**

> Fase ini di luar 7 spesifikasi eksplisit, tapi diperlukan untuk memenuhi
> tujuan yang dinyatakan pengguna: engine harus **bisa dipakai** di web API
> (contoh Axum) maupun project Rust lain. Menyatukan Fase 0–7 menjadi satu
> alur `McatSession` yang mudah dipanggil per-request/per-langkah.

### Tujuan
- Menyediakan `McatSession` — orkestrator siklus adaptif lengkap: mulai sesi
  → (loop: pilih item → sajikan → terima respons → estimasi ulang → cek
  stop) → hasil akhir.
- Desain API yang **stateless-friendly**: sesi bisa di-serialize/deserialize
  di antara request HTTP (server tidak wajib menyimpan state di memori
  antar-request), cocok untuk web API.
- Contoh nyata pemakaian di Axum, sebagai bukti "framework-agnostic tapi
  siap pakai".

### Rujukan Kode Sumber (R)
- `mirtCAT/R/mirtCAT.R` (alur utama fungsi `mirtCAT()`)
- `mirtCAT/R/updateDesign.R` (transisi state antar-item)

### Desain Modul Rust
- `session::McatSession` — memegang `PersonState`, `Design`-setara
  (gabungan config Fase 1–6), referensi `ItemBank`+`GroupParams`.
- `session::McatSession::new(config, bank, group) -> Self` — `config.debug: bool` (default `false`, prinsip desain #7) diteruskan ke seluruh sub-config Fase 1–6 sekaligus (satu saklar, bukan diatur per-fase secara terpisah).
- `session::McatSession::next_item(&mut self) -> Result<ItemIndex, MirtCatError>`
- `session::McatSession::submit_response(&mut self, item: ItemIndex, response: i32) -> Result<StopDecision, MirtCatError>` — internal: update info matrix (bila perlu) → estimasi ulang theta (Fase 3) → evaluasi stop (Fase 4).
- `session::McatSession::finalize(self) -> McatResult` (Fase 7).
- `session::StepDebug` — enum agregasi (varian `Selection(ItemSelectionDebug)`, `Estimation(AbilityEstimationDebug)`, `Stopping(StoppingRuleDebug)`, `Exposure(ExposureControlDebug)`, `ContentBalancing(ContentBalancingDebug)`) merekam tiap `Option<XxxDebug>` dari Fase 1–6 yang `Some` di satu langkah `next_item()`/`submit_response()`.
- `session::McatSession::debug_log(&self) -> &[StepDebug]` — accessor append-only, kosong bila `config.debug == false`; dipakai endpoint "debug" contoh Axum (`GET /session/:id/debug`).
- `McatSession` + field pendukungnya `Serialize`/`Deserialize` di balik feature `serde` (agar bisa disimpan di cookie/DB/redis antar-request HTTP).

### Checklist Implementasi
- [ ] `session::McatSession` (struct) menggabungkan config Fase 1–6
- [ ] `McatSession::new()`, `next_item()`, `submit_response()`, `finalize()`
- [ ] Urutan pipeline internal benar: kriteria (Fase 2) → content balancing (Fase 6) → exposure control (Fase 5) → (constraints, bila ada di lingkup masa depan) → item final
- [ ] `McatSession` bisa di-serialize/deserialize (feature `serde`) untuk state antar-request HTTP
- [ ] `session.debug: bool` (default `false`) diteruskan otomatis ke seluruh sub-config Fase 1–6 saat `McatSession::new()`
- [ ] `StepDebug` (enum agregasi) & `McatSession::debug_log()` — mengumpulkan `Option<XxxDebug>` tiap fase per langkah
- [ ] `examples/08_axum_api/` — mini server Axum: endpoint `POST /session`, `POST /session/:id/answer`, `GET /session/:id/result`, `GET /session/:id/debug` (kosong/404 bila `debug=false`; in-memory store untuk contoh; didokumentasikan bahwa produksi sebaiknya pakai store eksternal)

### Checklist Unit Test
- [ ] Siklus penuh sintetis (item bank kecil, 3 dimensi) dari `new()` sampai `finalize()` menghasilkan `McatResult` yang konsisten
- [ ] `submit_response()` menolak `item` yang bukan hasil `next_item()` terakhir (mencegah state korup)
- [ ] Serialize → deserialize `McatSession` di tengah sesi menghasilkan sesi yang bisa dilanjutkan identik dengan yang tidak di-serialize
- [ ] `debug=false` di level sesi → `debug_log()` selalu kosong & seluruh sub-config Fase 1–6 otomatis ikut `debug=false`; `debug=true` → `debug_log()` terisi tiap langkah tanpa mengubah `ItemIndex`/`StopDecision`/`McatResult` yang dikembalikan

### Checklist Integration Test
- [ ] Fixture R: seluruh sesi (item selection + ability estimation + stopping + exposure + content balancing digabung) dari `mirtCAT()` end-to-end pada 1 skenario representatif → bandingkan hasil akhir Rust `McatSession` vs R
- [ ] Test HTTP end-to-end pada contoh Axum (`tests/integration_axum.rs` di dalam folder example atau test terpisah): request penuh dari mulai sesi sampai hasil akhir via HTTP, memverifikasi response JSON sesuai `McatResult`

### Checklist Example Script
- [ ] `examples/08_axum_api/main.rs` — server Axum yang bisa dijalankan (`cargo run --example axum_api`), didokumentasikan cara uji manual via `curl`

### Checklist Dokumentasi
- [ ] `docs/panduan/08-integrasi-axum.md` — cara memasang `mirtcat-engine` di project Axum (atau framework lain), pola penyimpanan state sesi (in-memory vs Redis/DB), contoh request/response JSON

### Definition of Done
- [ ] Semua checklist di atas tercentang
- [ ] `cargo test -p mirtcat-engine session` hijau
- [ ] Contoh Axum bisa dijalankan dan diuji manual end-to-end
- [ ] Parity hasil akhir sesi gabungan dengan fixture R terverifikasi

---

## Fase 9 — Hardening, Validasi Silang Menyeluruh & Kesiapan Rilis

**Status: ⬜ Belum Dimulai**

### Tujuan
Menaikkan tingkat kepercayaan terhadap kebenaran & kestabilan engine sebelum
dipublikasikan sebagai crate yang bisa dipakai project lain.

### Checklist
- [ ] Audit menyeluruh: jalankan **seluruh** skrip `generate_r_fixtures.R`
      dari Fase 0–8 dalam satu pipeline CI opsional (manual/nightly, karena
      butuh R terpasang) untuk mendeteksi regresi parity
- [ ] Property-based testing (`proptest`) untuk invariant lintas modul (mis.
      probabilitas kategori selalu menjumlah 1, matriks informasi selalu
      positive semi-definite, proporsi content balancing tidak pernah
      negatif)
- [ ] Benchmark dasar (`criterion`) untuk kriteria seleksi item & estimasi
      ability pada item bank besar (mis. 500 item, 5 dimensi) — memastikan
      layak dipakai real-time di web API
- [ ] Dokumentasi API level crate (`//!` di `lib.rs`, `cargo doc` bersih
      tanpa warning `missing_docs`)
- [ ] `README.md` — ringkasan, instalasi, quickstart, tautan ke `docs/teori`
      dan `docs/panduan`, pernyataan scope/non-scope, atribusi ke `mirt`/
      `mirtCAT` sebagai rujukan konseptual
- [ ] Keputusan & penambahan file lisensi (`LICENSE-MIT`/`LICENSE-APACHE`
      atau sesuai keputusan Fase 0 §1.3)
- [ ] `CHANGELOG.md` awal (mengikuti Keep a Changelog)
- [ ] Review keamanan dasar: tidak ada `unsafe` tanpa justifikasi tertulis,
      tidak ada panic pada input pengguna yang tidak tervalidasi (semua
      jalur error via `Result`)
- [ ] Kesiapan publish `cargo publish --dry-run` bersih

### Definition of Done
- [ ] Semua checklist di atas tercentang
- [ ] Seluruh fase 0–8 berstatus ✅ Selesai
- [ ] `cargo test`, `cargo clippy -- -D warnings`, `cargo doc --no-deps` semua hijau di CI

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
