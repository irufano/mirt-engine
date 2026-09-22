# Changelog

Seluruh perubahan penting pada `mirtcat-engine` dicatat di file ini.

Format mengikuti [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
dan proyek ini mengikuti [Semantic Versioning](https://semver.org/).

## [Unreleased]

Belum ada rilis `crates.io` — seluruh Fase 0–9 (`PLAN.md`) selesai di
riwayat commit ini, dikelompokkan di bawah sebagai satu entri "Added"
sebelum rilis pertama.

### Added

- **Fase 0 — Fondasi IRT**: trait `ItemResponseModel`, model item
  `Dichotomous` (M2PL/M3PL/M4PL) & `GradedResponse`, tipe `ThetaVector`/
  `SEVector`/`InfoMatrix`, `ItemBank`/`GroupParams`.
- **Fase 1 — Theta Awal**: inisialisasi `theta`/SE dari populasi atau
  langsung (`initial_theta`).
- **Fase 2 — Seleksi Item**: kriteria D/T/A/E/W-rule (+ varian `*Prule`),
  KL/KLn, `seq`, `random` (`item_selection`).
- **Fase 3 — Estimasi Kemampuan**: EAP (grid & QMC), MAP, ML, WLE, EAP
  berbasis summed score, `Fixed` (`ability_estimation`).
- **Fase 4 — Stopping Rule**: `min_SEM`, `delta_thetas`, klasifikasi `CI`
  & `SPRT`, `max_items`/`max_time`, hook `CustomStop` (`stopping_rule`).
- **Fase 5 — Exposure Control**: mode Sympson–Hetter & randomesque
  (`exposure_control`).
- **Fase 6 — Content Balancing**: penyempitan kandidat berbasis defisit
  proporsi kategori konten (`content_balancing`).
- **Fase 7 — Hasil Akhir**: `McatResult` — ringkasan bersih `theta`/SE/
  riwayat untuk konsumen di luar engine (`result`).
- **Fase 8 — Orkestrasi Sesi**: `McatSession` menyatukan Fase 1–7 jadi satu
  siklus adaptif per-langkah (`next_item`/`submit_response`/`finalize`),
  serialisasi state sesi lewat fitur `serde`, contoh integrasi Axum
  (`examples/08_axum_api`).
- **Fase 9 — Hardening & Kesiapan Rilis**:
  - Property-based test (`proptest`) untuk invariant lintas modul:
    probabilitas kategori selalu menjumlah 1, matriks informasi Fisher
    kumulatif selalu positive semi-definite, proporsi empiris content
    balancing tidak pernah negatif (`tests/proptest_invariants.rs`).
  - Benchmark `criterion` untuk seluruh kriteria seleksi item & metode
    estimasi kemampuan pada item bank 500 item/5 dimensi
    (`benches/mcat_engine.rs`).
  - Dokumentasi API level crate lengkap — `cargo doc --no-deps` bersih
    tanpa warning (`#![deny(missing_docs)]`, seluruh item publik
    terdokumentasi, tautan intra-doc ke item privat diperbaiki).
  - `README.md`, `LICENSE-MIT`/`LICENSE-APACHE` (dual MIT/Apache-2.0, lih.
    `PLAN.md` §Fase 0 1.3), `CHANGELOG.md`.
  - Audit keamanan dasar: tidak ada `unsafe` di `src/`; seluruh
    `.expect()`/`unreachable!()` non-test di `src/` diverifikasi hanya
    menjaga invariant yang sudah divalidasi sebelumnya (konstruktor,
    exhaustive match), bukan reachable dari input eksternal tak
    tervalidasi — semua batasan pengguna tetap lewat `Result<_,
    MirtCatError>`.
  - CI: langkah `cargo doc --no-deps` (default & fitur `serde`) ditambah
    ke `check` job; workflow terpisah (`workflow_dispatch` + nightly)
    menjalankan seluruh skrip `generate_r_fixtures*.R` lalu `cargo test`
    untuk mendeteksi regresi parity terhadap `mirt`/`mirtCAT`.
  - `cargo publish --dry-run` bersih.
