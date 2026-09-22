# 03 — `Fixed`: Identity (Kuesioner Non-Adaptif)

## Quickstart

```rust
use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::{EstimationConfig, Method};

let config = EstimationConfig { method: Method::Fixed, ..Default::default() };
let ((theta, se), _) = estimate(&config, &person, &bank, &group)?;
// theta == person.theta(), se == person.se() -- tidak berubah
```

## Kapan pakai

- Kuesioner non-adaptif: seluruh item disajikan urutan tetap, `theta`
  tidak diestimasi ulang setiap item dijawab — biasanya dipasangkan dengan
  `Criterion::Seq` (Fase 2). Hitung `theta` akhir **sekali** di akhir sesi
  lewat method lain (mis. `Eap`/`Map`), bukan `Fixed`.
- Menahan `theta` tetap konstan sementara (mis. fase kalibrasi/latihan
  sebelum sesi adaptif sesungguhnya dimulai).

## Perilaku

`Fixed` mengembalikan `person.theta()`/`person.se()` **persis** apa adanya
— tidak memanggil optimisasi atau kuadratur sama sekali, terlepas dari
berapa banyak item sudah dijawab (`person.answered_items()` sepenuhnya
diabaikan). Seluruh field `AbilityEstimationDebug` bernilai `None` (tidak
ada iterasi optimizer, tidak ada titik kuadratur — tidak ada yang
dihitung).

```rust
let config = EstimationConfig { method: Method::Fixed, debug: true, ..Default::default() };
let (_, debug) = estimate(&config, &person, &bank, &group)?;
let debug = debug.unwrap();
assert!(debug.n_quadrature_points.is_none());
assert!(debug.iterations.is_none());
```

Contoh lengkap (theta/SE tetap konstan sepanjang beberapa item dijawab):
`cargo run --example 03_ability_estimation_fixed`.
