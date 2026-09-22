# 02 — `Drule`/`DPrule`: D-Optimality

## Quickstart

```rust
use mirtcat_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::DRule { prior: false }, // atau `prior: true` untuk DPrule
    ..Default::default()
};
```

`Drule` tidak memakai `weights` — field itu diabaikan kalau diisi.

## Kapan dipakai

- Ingin presisi **gabungan** seluruh dimensi laten (volume elipsoid
  kepercayaan minimum), bukan salah satu dimensi diprioritaskan atas
  lainnya (kalau itu yang diinginkan, pakai `Trule`/`Wrule` dengan
  `weights`).
- Kriteria paling umum dipakai untuk MCAT "generik" tanpa preferensi
  khusus antar dimensi — sering jadi pilihan default di literatur (Segall,
  1996).

## Penting: pakai `prior: true` di awal tes

```rust
let config = SelectionConfig {
    criterion: Criterion::DRule { prior: true }, // DPrule
    ..Default::default()
};
```

Tanpa prior, `Drule` **tidak bisa membedakan kandidat sama sekali** sampai
cukup item terjawab (lih. dokumen teori — rank-deficiency). Kalau tes
dimulai dari nol item terjawab dan `nfact >= 2`, `DPrule` adalah pilihan
yang jauh lebih aman untuk langkah-langkah awal. Setelah beberapa item
terjawab (info kumulatif sudah full rank), perbedaan `Drule` vs `DPrule`
mengecil.

Contoh lengkap (membandingkan `Drule` vs `DPrule` langkah demi langkah):
`cargo run --example 02_item_selection_d_rule`.
