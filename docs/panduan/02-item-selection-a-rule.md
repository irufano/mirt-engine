# 02 — `Arule`/`APrule`: Minimum (Weighted) Error-Variance

## Quickstart

```rust
use mirt_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::ARule { prior: false }, // atau `prior: true` untuk APrule
    weights: Some(vec![1.0, 1.0, 2.0]), // dimensi 3 diprioritaskan
    ..Default::default()
};
```

## Kapan dipakai

- Tujuan tes adalah **meminimalkan varians/SE** estimasi `theta` secara
  langsung (bukan proxy seperti determinan/trace informasi) — `Arule`
  paling dekat secara konseptual dengan "minimalkan SE akhir".
  `weights` memungkinkan memprioritaskan dimensi mana yang SE-nya paling
  penting diminimalkan.
- Butuh estimasi presisi yang valid **sejak awal tes**, termasuk saat
  matriks informasi masih rank-defisien — beda dari `Drule`/`Erule` murni
  yang macet total (nilai identik nol) di kondisi itu, `Arule` tetap
  memberi nilai yang bermakna lewat pseudo-invers (lih. dokumen teori).

## Biaya komputasi

`Arule` adalah kriteria D/T/A/E/W-rule yang paling mahal secara komputasi
per kandidat (perlu dekomposisi SVD untuk pseudo-invers, bukan sekadar
determinan/trace/eigenvalue) — untuk bank item besar & `nfact` besar,
pertimbangkan `Trule`/`Wrule` (lebih murah) kalau perbedaan presisi
praktisnya kecil untuk kasus penggunaan tertentu.

Contoh lengkap (termasuk kasus rank-defisien di langkah pertama):
`cargo run --example 02_item_selection_a_rule`.
