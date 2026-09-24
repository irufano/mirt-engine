# 02 — `Wrule`/`WPrule`: Kriteria Informasi Berbobot Kustom

## Quickstart

```rust
use mirt_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::WRule { prior: false }, // atau `prior: true` untuk WPrule
    weights: Some(vec![1.0, 1.0, 0.5]), // skor komposit: dim1+dim2 penuh, dim3 setengah
    ..Default::default()
};
```

`weights: None` → default `vec![1.0; nfact]` (skor komposit = jumlah polos
semua dimensi).

## Kapan dipakai (vs `Trule`)

Pakai `Wrule` kalau yang benar-benar dipedulikan adalah presisi sebuah
**skor gabungan tertimbang** `weights' * theta` — mis. skor akhir ujian
yang secara eksplisit menjumlahkan beberapa sub-skala dengan bobot
tertentu, atau indeks komposit yang jadi dasar keputusan (lolos/tidak,
peringkat). Pakai `Trule` kalau yang dipedulikan adalah presisi **tiap
dimensi secara independen** (mis. laporan per-subskala terpisah, tanpa
penjumlahan).

Bedanya baru terlihat signifikan kalau matriks informasi item-item di bank
punya off-diagonal besar (item yang membebankan pada kombinasi dimensi,
bukan satu dimensi murni) — untuk item bank yang didominasi *simple
structure* (tiap item hanya membebankan kuat pada satu dimensi), `Trule`
dan `Wrule` sering memilih item yang sama.

Contoh lengkap (bobot setara vs timpang, plus `WPrule`):
`cargo run --example 02_item_selection_w_rule`.
