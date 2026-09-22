# 02 — `KL`/`KLn`: Kullback-Leibler Pointwise

> Landasan teori untuk `src/item_selection/criteria/kl.rs` (Fase 2).

## Definisi

Untuk setiap item kandidat, `KL` menghitung divergensi Kullback-Leibler
antara distribusi probabilitas kategori respons item itu pada dua titik
`theta` yang berjarak `2*delta`:

```
p0 = probabilitas(item, theta - delta)
p1 = probabilitas(item, theta + delta)
KL = sum_k p1_k * (ln(p1_k) - ln(p0_k))
```

`delta` adalah skalar (bukan vektor per-dimensi) — di-broadcast rata ke
semua dimensi `theta`. Item dipilih dengan memaksimalkan `KL` — intuisinya:
item yang probabilitas responsnya berubah paling drastis di sekitar
estimasi `theta` saat ini adalah item yang paling **informatif secara
lokal** tentang `theta` sebenarnya (paling mampu membedakan peserta yang
sedikit di atas vs sedikit di bawah estimasi saat ini).

## `KLn`: interval yang membesar

`KLn` memakai formula identik, tapi `delta` diskalakan:

```
delta_efektif = delta * sqrt(n_answered)
```

`n_answered` = jumlah item yang sudah dijawab **sebelum** langkah seleksi
ini. Rasionalnya (Chang & Ying, 1996): di awal tes, estimasi `theta` masih
sangat tidak pasti — interval sempit (`delta` kecil) cukup untuk menangkap
ketidakpastian lokal. Makin banyak item dijawab, makin presisi estimasi
`theta`, tapi juga makin perlu membedakan item pada rentang yang lebih
luas untuk terus informatif (interval terlalu sempit relatif terhadap SE
yang sudah mengecil jadi kurang diskriminatif). Skala `sqrt(n)` mendekati
pendekatan "informasi global" (rata-rata informasi Fisher atas interval
kepercayaan yang menyempit seiring `n`) tanpa perlu integrasi kuadratur
eksplisit — inilah yang membuat `KL`/`KLn` valid untuk `nfact > 1` (beda
dari `IKL`/`IKLn`, varian berbasis integrasi kuadratur yang **tidak**
valid multidimensional, lih. `docs/teori/02-item-selection-overview.md`).

## Kenapa tidak butuh matriks informasi kumulatif

Beda dari D/T/A/E/W-rule, `KL`/`KLn` **tidak** memakai `info_thetas`
(matriks kumulatif dari item terjawab) sama sekali — nilainya murni fungsi
dari probabilitas **item kandidat itu sendiri** pada `theta ± delta`. Ini
konsekuensi definisinya (divergensi antar dua titik, bukan presisi
estimasi kumulatif) — bukan keterbatasan implementasi.

## Referensi

- Chang, H.-H., & Ying, Z. (1996). A global information approach to
  computerized adaptive testing. *Applied Psychological Measurement*,
  20(3), 213–229.
