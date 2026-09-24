# 02 — `KL`/`KLn`: Kullback-Leibler Pointwise

## Quickstart

```rust
use mirt_engine::{select_next_item, Criterion, SelectionConfig};

let config_kl = SelectionConfig {
    criterion: Criterion::Kl { delta: 0.1 },
    ..Default::default()
};

let config_kln = SelectionConfig {
    criterion: Criterion::KlN { delta: 0.1 },
    ..Default::default()
};
```

`delta` sama untuk kedua varian — `select_next_item()` yang menskalakannya
dengan `sqrt(person.n_answered())` untuk `KlN` secara internal (lih.
dokumen teori). Nilai umum untuk `delta`: `0.1`–`0.3` pada skala `theta`
standar (`N(0,1)`).

## Kapan dipakai

- `KL`: interval lokal tetap masuk akal sepanjang tes; lebih murah
  komputasi dibanding D/T/A/E/W-rule (tidak perlu determinan/invers/
  eigenvalue matriks `nfact × nfact`).
- `KLn`: tes lebih panjang, ingin interval evaluasi ikut melebar mengikuti
  presisi yang membaik — mendekati perilaku "informasi global" tanpa biaya
  integrasi kuadratur.
- Cocok jadi kriteria default saat `nfact` besar (>4) dan D/T/A/E/W-rule
  jadi lebih mahal komputasi (matriks makin besar) — `KL`/`KLn` biayanya
  tidak bergantung `nfact` selain lewat panjang vektor probabilitas.

## Catatan praktis

- `delta` terlalu kecil → semua kandidat punya `KL` mirip (kurang
  diskriminatif). `delta` terlalu besar → evaluasi jadi terlalu global,
  mengabaikan struktur lokal di sekitar estimasi `theta`.
- `KLn` di `n_answered = 0` (`delta_efektif = 0`) menghasilkan `KL = 0`
  untuk **semua** kandidat (kedua titik evaluasi berimpit) — argmax jadi
  arbitrer di langkah pertama. Ini konsisten dengan definisi, bukan bug;
  kombinasikan dengan `start_item` eksplisit (di luar Fase 2, kebijakan
  level orkestrasi sesi) untuk item pertama.

Contoh lengkap (perbandingan `KL` vs `KLn` seiring `n_answered` bertambah):
`cargo run --example 02_item_selection_kl`.
