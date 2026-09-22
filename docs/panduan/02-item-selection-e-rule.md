# 02 — `Erule`/`EPrule`: Eigenvalue Minimum

## Quickstart

```rust
use mirtcat_engine::{select_next_item, Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::ERule { prior: false }, // atau `prior: true` untuk EPrule
    ..Default::default()
};
```

`Erule` tidak memakai `weights` — field itu diabaikan kalau diisi.

## Kapan dipakai

- Prioritas utama adalah **tidak ada dimensi laten yang terabaikan** —
  cocok untuk profil kemampuan multidimensional di mana keputusan akhir
  bergantung pada semua dimensi secara setara pentingnya (mis. laporan
  hasil per-subskala, bukan skor total).
- Sebagai pelengkap `Drule`: kalau simulasi menunjukkan `Drule` cenderung
  "mengabaikan" salah satu dimensi meski volume elipsoid kecil, `Erule`
  memaksa keseimbangan yang tidak dijamin `Drule`.

## Penting: pakai `prior: true` di awal tes

Sama seperti `Drule`, `Erule` **tidak bisa membedakan kandidat** sampai
cukup item terjawab (lih. dokumen teori). Pakai `EPrule` untuk
langkah-langkah awal.

Contoh lengkap: `cargo run --example 02_item_selection_e_rule`.
