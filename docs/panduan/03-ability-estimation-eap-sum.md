# 03 — `EapSum`: EAP Berbasis Summed Score

## Quickstart

```rust
use mirtcat_engine::ability_estimation::{estimate, eap_sum_score_table};
use mirtcat_engine::{EstimationConfig, Method};

let config = EstimationConfig { method: Method::EapSum, ..Default::default() };

// Estimasi untuk skor peserta SEKARANG saja (konsisten dengan method lain):
let ((theta, se), _) = estimate(&config, &person, &bank, &group)?;

// Tabel LENGKAP: seluruh kemungkinan skor total -> (theta, SE):
let table = eap_sum_score_table(&config, &person, &bank, &group)?;
for row in &table {
    println!("skor={} -> theta={:?}, se={:?}", row.sum_score, row.theta.as_slice(), row.se.as_slice());
}
```

`eap_sum_score_table` **hanya** valid dipanggil dengan
`config.method == Method::EapSum` — error (bukan panic) bila method lain.

## Kapan pakai

- Pelaporan skor mentah -> kemampuan (mis. dokumen skor peserta yang
  menampilkan "total benar: X dari N", butuh `theta` yang konsisten dengan
  angka itu, bukan pola respons spesifik).
- Identitas/urutan item spesifik dianggap tidak relevan untuk estimasi
  akhir — dua peserta dengan skor total sama mendapat `theta` yang
  **identik**, walau pola respons item-per-itemnya berbeda (lih.
  `docs/teori/03-ability-estimation-eap-sum.md` §1).

**Tidak cocok** untuk estimasi adaptif per-item di tengah CAT (di sana,
pola respons item-per-item — bukan cuma jumlah — punya informasi yang
relevan; pakai `Eap`/`Map`/`Ml`/`Wle` untuk itu).

## `quadpts`

Sama seperti `Eap`: default dari tabel `nfact`
(`quadrature::default_quadpts`), bisa dioverride lewat
`EstimationConfig::quadpts`. `EapSum` selalu grid-based — tidak ada varian
QMC (`mirt`/`mirtCAT` juga tidak menyediakannya untuk method ini).

Contoh lengkap (cetak tabel skor->theta/SE penuh):
`cargo run --example 03_ability_estimation_eap_sum`.
