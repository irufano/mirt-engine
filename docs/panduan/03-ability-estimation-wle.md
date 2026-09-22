# 03 — `Wle`: Weighted Likelihood Estimation

## Quickstart

```rust
use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::{EstimationConfig, Method};

let config = EstimationConfig { method: Method::Wle, ..Default::default() };
let ((theta, se), _) = estimate(&config, &person, &bank, &group)?;
```

## Kapan pakai

- Ingin estimator dengan bias lebih kecil dibanding `Ml` (koreksi Warm,
  1989), tanpa perlu menentukan prior eksplisit seperti `Map`.
- Awal sesi CAT (item terjawab masih sedikit) — `Wle` tetap memberi
  estimasi **finite** untuk pola respons ekstrem (semua benar/semua salah),
  beda dari `Ml` yang divergen ke `±infinity` di kasus yang sama (lih.
  `docs/teori/03-ability-estimation-wle.md` §3).

`Wle` **tidak** butuh `GroupParams` yang bermakna (parameter `group` di
`estimate()` diabaikan untuk method ini — cuma dipertahankan di signature
untuk keseragaman API lintas method) — koreksi bias Warm murni berdasar
struktur informasi Fisher item terjawab, bukan prior populasi.

## Perbandingan langsung dengan `Ml`

```rust
use mirtcat_engine::{EstimationConfig, Method};

let ml = estimate(&EstimationConfig { method: Method::Ml, ..Default::default() }, &person, &bank, &group)?;
let wle = estimate(&EstimationConfig { method: Method::Wle, ..Default::default() }, &person, &bank, &group)?;
// pola respons moderat -> ml.0.0 ≈ wle.0.0 (bias kecil, keduanya dekat)
// pola respons ekstrem  -> ml.0.0 = ±infinity, wle.0.0 tetap finite
```

Untuk pola respons yang JAUH dari ekstrem (mis. setengah benar setengah
salah), `Ml` dan `Wle` biasanya sangat dekat — koreksi bias Warm makin
signifikan justru saat mendekati pola ekstrem, tempat `Ml` mulai tidak
stabil.

Contoh lengkap (perbandingan langsung dengan `Ml` pada pola ekstrem yang
sama): `cargo run --example 03_ability_estimation_wle`.
