# 03 — `Eap`: Expected A Posteriori

## Quickstart

```rust
use mirt_engine::ability_estimation::estimate;
use mirt_engine::{EstimationConfig, Method};

// Grid-based (default quadpts dari tabel nfact)
let config_grid = EstimationConfig {
    method: Method::Eap { qmc: false },
    ..Default::default()
};

// QMC-based, quadpts dinaikkan manual untuk manfaat penuh (lih. §di bawah)
let config_qmc = EstimationConfig {
    method: Method::Eap { qmc: true },
    quadpts: Some(2000),
    ..Default::default()
};

let ((theta, se), _) = estimate(&config_grid, &person, &bank, &group)?;
```

## Kapan pakai grid vs QMC

- **Grid (`qmc: false`)**: default yang wajar untuk `nfact <= 4` — murah,
  akurat, tidak butuh tuning tambahan.
- **QMC (`qmc: true`)**: disarankan untuk `nfact > 4` — grid di dimensi
  ini dipaksa memakai `quadpts` sangat kecil (`<=7`, lih. tabel overview),
  QMC memungkinkan jumlah titik total lebih besar tanpa `quadpts^nfact`.
  **Naikkan `quadpts` manual** (mis. beberapa ribu) — default tetap
  mengikuti tabel `nfact` yang sama seperti grid (`mirtCAT` tidak
  membesarkan otomatis untuk QMC, di-port apa adanya).

Peringatan otomatis (`log::warn!`) muncul saat `Eap { qmc: false }` dipakai
dengan `person.nfact() > 4` — pasang logger (`env_logger`, `tracing-log`,
dst.) untuk melihatnya; tidak menghentikan eksekusi (bukan error).

## Catatan penting: QMC tidak diskalakan ke `gmeans`/`gcov`

Titik QMC selalu berbentuk awan kuasi-`N(0,I)` standar, **terlepas** dari
`gmeans`/`gcov` grup sesungguhnya (quirk asli `mirt`, lih.
`docs/teori/03-ability-estimation-eap.md` §2). Untuk grup dengan prior jauh
dari standar (`gmeans` besar atau `gcov` sangat berbeda dari identitas),
pertimbangkan `Method::Map` sebagai alternatif yang tidak punya
keterbatasan ini.

## Debug trace: jumlah titik kuadratur

```rust
let config = EstimationConfig { method: Method::Eap { qmc: false }, debug: true, ..Default::default() };
let (_, debug) = estimate(&config, &person, &bank, &group)?;
println!("n_quadrature_points = {:?}", debug.unwrap().n_quadrature_points);
```

`n_quadrature_points` = `quadpts^nfact` untuk grid, `quadpts` (jumlah total
titik, sesuai parameter) untuk QMC.

Contoh lengkap (grid vs QMC pada pola respons yang sama, perbandingan
waktu komputasi): `cargo run --example 03_ability_estimation_eap`.
