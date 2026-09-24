# 03 — `Ml`: Maximum Likelihood

## Quickstart

```rust
use mirtcat_engine::ability_estimation::estimate;
use mirtcat_engine::{EstimationConfig, Method};

let config = EstimationConfig { method: Method::Ml, ..Default::default() };
let ((theta, se), _) = estimate(&config, &person, &bank, &group)?;
```

## ⚠️ Bisa mengembalikan `±infinity`

`Ml` **tidak** punya prior yang menahan estimasi — pola respons di mana
**seluruh** item terjawab punya kategori sama (semua benar, atau semua
salah) membuat `theta` divergen:

```rust
let (theta, se) = /* semua item terjawab benar */;
assert_eq!(theta[0], f64::INFINITY);
assert!(se.as_slice()[0].is_nan());
```

**Selalu cek `theta[i].is_finite()`** sebelum memakai hasil `Ml` di kode
yang butuh nilai numerik terbatas (mis. sebelum dipakai sebagai input
kriteria seleksi item Fase 2, atau ditampilkan ke pengguna). Untuk
menghindari kasus ini sama sekali, pertimbangkan `Method::Wle` (tetap
terbatas pada pola ekstrem, lih. `docs/panduan/03-ability-estimation-wle.md`)
atau `Method::Map` (default, selalu finite).

## Kapan pakai

- Riset/analisis yang secara eksplisit butuh estimasi TANPA pengaruh prior
  (mis. membandingkan sensitivitas hasil terhadap pilihan prior).
- Item terjawab sudah cukup banyak & beragam kategorinya (risiko divergensi
  rendah) — data cukup informatif untuk mendominasi tanpa perlu prior.

**Tidak disarankan** sebagai method utama di awal sesi CAT (item sedikit) —
pakai `Map` (default) atau `Wle` untuk keandalan lebih baik.

## Menangani hasil divergen di kode caller

```rust
let ((theta, se), _) = estimate(&config, &person, &bank, &group)?;
if theta.as_slice().iter().any(|v| !v.is_finite()) {
    // fallback: pakai Map, atau tahan theta lama, sesuai kebijakan sesi
}
```

`mirtCAT` sendiri menangani ini secara otomatis di level sesi (fallback
`ML -> MAP` untuk pola respons identik seluruhnya) — bukan tanggung jawab
`ability_estimation::estimate()` (lih.
`docs/teori/03-ability-estimation-ml.md` §3), jadi caller perlu
mengimplementasikan kebijakan serupa sendiri bila diperlukan.

Contoh lengkap (pola moderat vs ekstrem): `cargo run --example
03_ability_estimation_ml`.
