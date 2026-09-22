# mirtcat-engine

Engine Rust untuk **Multidimensional Computerized Adaptive Testing (MCAT)**
— mengacu pada algoritma dan perilaku library R **mirt** dan **mirtCAT**,
dipersempit ke skenario `nfact > 1` (multidimensional IRT). Didesain untuk
dipakai sebagai dependency biasa di web API Rust (mis. Axum) atau project
Rust lain, tanpa terikat framework tertentu.

`mirtcat-engine` **tidak** melakukan kalibrasi model IRT dari data respons
mentah (EM/MH-RM, marginal maximum likelihood — itu domain `mirt`). Engine
ini menerima parameter item yang **sudah terkalibrasi** (slope, intercept,
guessing, dst.) sebagai input, lalu menjalankan siklus adaptif di atasnya:
theta awal → seleksi item → estimasi kemampuan → stopping rule → exposure
control → content balancing → hasil akhir.

## Status

Fase 0–9 selesai — fondasi IRT, siklus adaptif penuh, orkestrasi sesi
(`McatSession`), contoh integrasi Axum, dan hardening rilis (property test,
benchmark, audit keamanan, dokumentasi API lengkap). Lihat [`PLAN.md`](PLAN.md)
untuk peta fase & status detail, [`CHANGELOG.md`](CHANGELOG.md) untuk
ringkasan per fase.

## Instalasi

Belum dipublikasikan ke crates.io. Untuk memakainya dari project Rust lain:

```toml
[dependencies]
mirtcat-engine = { git = "https://github.com/irufano/mirtcat-engine" }
# atau, dari checkout lokal:
# mirtcat-engine = { path = "../mirtcat-engine" }
# atau, setelah dipublikasikan:
# mirtcat-engine = "0.1"
```

Fitur opsional `serde` mengaktifkan `Serialize`/`Deserialize` untuk
`McatSession`/`McatResult` dan tipe pendukungnya (dibutuhkan untuk
menyimpan/memulihkan state sesi antar-request HTTP):

```toml
mirtcat-engine = { version = "0.1", features = ["serde"] }
```

Dependency inti hanya [`nalgebra`](https://crates.io/crates/nalgebra)
(aljabar linear), [`rand`](https://crates.io/crates/rand),
[`thiserror`](https://crates.io/crates/thiserror), dan
[`log`](https://crates.io/crates/log) — tidak ada dependency ke framework
web apa pun.

## Quickstart

Menghitung probabilitas & informasi Fisher satu item (fondasi IRT, Fase 0):

```rust
use mirtcat_engine::irt::{Dichotomous, ItemResponseModel};
use mirtcat_engine::theta::ThetaVector;

let item = Dichotomous::m2pl(vec![1.2, 0.8], 0.3).unwrap();
let theta = ThetaVector::from_vec(vec![0.5, -0.2]);

let p = item.probabilities(&theta).unwrap(); // [P(u=0), P(u=1)]
let info = item.information(&theta).unwrap(); // matriks 2x2
```

Menjalankan siklus adaptif penuh lewat `McatSession` (Fase 8 — orkestrator
end-to-end, cara yang disarankan untuk dipakai dari web API):

```rust
use mirtcat_engine::{McatSession, SessionConfig};

let mut session = McatSession::new(SessionConfig::default(), &bank, &group, 0)?;
let mut rng = rand::rngs::StdRng::from_entropy();

loop {
    let item = session.next_item(&bank, &group, &mut rng)?;
    // ..sajikan `item` ke peserta, terima respons (kategori 0-based)..
    let response: usize = /* dari peserta */ 1;
    let decision = session.submit_response(item, response, &bank, &group, None)?;
    if decision.stop_now {
        break;
    }
}

let result = session.finalize(); // McatResult (Fase 7)
```

Lihat [`docs/panduan/00-instalasi-dan-quickstart.md`](docs/panduan/00-instalasi-dan-quickstart.md)
untuk quickstart lengkap fondasi IRT, dan
[`docs/panduan/08-integrasi-axum.md`](docs/panduan/08-integrasi-axum.md)
untuk quickstart `McatSession` penuh (termasuk contoh Axum siap jalan di
`examples/08_axum_api`). Contoh lain per fase ada di
[`examples/`](examples/) (`cargo run --example <nama>`).

## Dokumentasi

- [`docs/teori/`](docs/teori/) — landasan teori & rujukan akademik tiap
  algoritma (mulai dari
  [`00-fondasi-irt-multidimensional.md`](docs/teori/00-fondasi-irt-multidimensional.md)).
- [`docs/panduan/`](docs/panduan/) — panduan penggunaan API per fase
  (mulai dari
  [`00-instalasi-dan-quickstart.md`](docs/panduan/00-instalasi-dan-quickstart.md)).
- `cargo doc --no-deps --open` — dokumentasi API level crate (`rustdoc`).

## Scope & Non-Scope

**Tujuan:**
- Port perilaku & hasil numerik MCAT dari `mirt`/`mirtCAT` ke Rust, seakurat
  mungkin (divalidasi silang dengan fixture yang dihasilkan langsung dari R
  — lih. `tests/fixtures/`).
- API murni Rust (trait + struct), framework-agnostic — tidak ada
  dependency web di `[dependencies]`; contoh Axum hanya dev-dependency.
- Tiap algoritma diuji & didemonstrasikan independen (unit test, integration
  test, example).
- Landasan teori didokumentasikan dengan rujukan akademik, bukan cuma
  rujukan ke source R.

**Di luar scope:**
- **Kalibrasi/estimasi parameter item** (EM, MH-RM, marginal maximum
  likelihood) — domain `mirt`, bukan `mirtCAT`/engine ini.
- **Shadow testing** / optimal test design berbasis `lpSolve` (fitur
  tambahan `mirtCAT` untuk constraint kompleks).
- Antarmuka HTML/Shiny (`createShinyGUI`, dst.) — posisi library ini setara
  `mirtCAT::Design`/`TestClass`/`PersonClass`, bukan lapisan GUI.
- Model IRT unidimensional non-MCAT bukan target utama, meski fondasi IRT
  cukup umum sehingga `nfact == 1` otomatis valid sebagai kasus khusus.

## Atribusi

Algoritma & perilaku di-port dari **mirt** dan **mirtCAT** (R), berlisensi
GPL (>= 3):

- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29.
- Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
  Interfaces for Multidimensional Item Response Theory Applications.
  *Journal of Statistical Software*, 71(5), 1–39.

`mirtcat-engine` adalah **clean-room reimplementation** dari rumus/algoritma
yang dipublikasikan di paper akademik (lih. daftar pustaka lengkap di
`PLAN.md`), bukan terjemahan baris demi baris source `.R`/`.cpp` milik
`mirt`/`mirtCAT` — source R dipakai semata sebagai rujukan perilaku yang
harus disamai (nama kriteria, urutan pipeline, semantik parameter), bukan
sebagai kode yang disalin. Ini memungkinkan `mirtcat-engine` berlisensi
dual MIT/Apache-2.0 (lih. bawah) walau merujuk pada library GPL.

## Lisensi

Dual-lisensi di bawah [MIT](LICENSE-MIT) atau [Apache License, Version 2.0](LICENSE-APACHE),
sesuai pilihan Anda — standar ekosistem Rust.
