# 02 — `Trule`/`TPrule`: Trace Berbobot

## Quickstart

```rust
use mirtcat_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::TRule { prior: false }, // atau `prior: true` untuk TPrule
    weights: Some(vec![2.0, 1.0, 0.5]), // dimensi 1 paling diutamakan
    ..Default::default()
};
```

`weights: None` → semua dimensi setara (`vec![1.0; nfact]`).

## Kapan dipakai

- Ada preferensi eksplisit antar dimensi laten yang **independen** (mis.
  keputusan lolos/tidak sebuah tes lebih bergantung pada satu sub-skala
  dibanding lainnya).
- Lebih murah komputasi dibanding `Arule` (tidak perlu pseudo-invers) dan
  lebih stabil di awal tes dibanding `Drule`/`Erule` murni (trace tetap
  nonzero walau `total_info` rank-defisien — beda dari determinan/
  eigenvalue minimum).

## Catatan praktis

Karena hanya diagonal yang dipakai, `Trule` **tidak** menghukum kandidat
yang informasinya "tumpang tindih arah" dengan info yang sudah terkumpul
(off-diagonal besar) — kalau itu yang ingin dihindari (favorkan item yang
menambah arah info *baru*, bukan menguatkan arah yang sudah kuat), pakai
`Drule`/`Erule`/`Wrule` alih-alih `Trule`.

Contoh lengkap (bobot default vs custom, plus `TPrule`):
`cargo run --example 02_item_selection_t_rule`.
