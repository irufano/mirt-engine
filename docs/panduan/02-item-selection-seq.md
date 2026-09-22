# 02 — `seq`: Baseline Non-Adaptif

## Quickstart

```rust
use mirtcat_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::Seq,
    ..Default::default()
};
```

`Criterion::Seq` adalah `Criterion::default()` (mengikuti default `criteria
= "seq"` di `mirtCAT`), jadi `SelectionConfig::default()` sudah cukup kalau
`seq` yang diinginkan.

## Kapan dipakai

- Kuesioner/survei non-adaptif, urutan item harus tetap.
- Baseline pembanding presisi terhadap kriteria adaptif lain (D/T/A/E/W-rule,
  KL) — jalankan simulasi yang sama dengan `seq` vs kriteria adaptif,
  bandingkan SE akhir pada jumlah item yang sama.
- `random`/`seq` sama-sama tidak butuh `GroupParams`/`weights` yang berarti
  — parameter itu tetap wajib di signature `select_next_item()` (konsisten
  dengan kriteria lain) tapi diabaikan sepenuhnya.

## Catatan

`candidates` yang diberikan ke `select_next_item()` menentukan urutan:
kalau kandidat sudah difilter (mis. lewat content balancing, Fase 6), `seq`
tetap memilih index **terkecil di antara kandidat yang tersisa**, bukan
"item ke-n secara absolut" — untuk kuesioner utuh tanpa filter, keduanya
sama.

Contoh lengkap: `cargo run --example 02_item_selection_seq`.
