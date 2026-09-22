# 02 — `Erule`/`EPrule`: Eigenvalue Minimum

> Landasan teori untuk `src/item_selection/criteria/e_rule.rs` (Fase 2).

## Definisi

`Erule` memaksimalkan **eigenvalue minimum** matriks informasi Fisher
kumulatif:

```
skor(kandidat) = min_eigenvalue(total_info)
```

Eigenvalue minimum sebuah matriks informasi mengukur presisi di **arah
yang paling lemah terinformasi** — kalau satu dimensi laten (atau kombinasi
dimensi) punya informasi jauh lebih sedikit dari yang lain, eigenvalue
minimum akan kecil, tidak peduli seberapa besar informasi di arah lain.
Memaksimalkan eigenvalue minimum secara eksplisit **mencegah satu dimensi
terus-menerus diabaikan** oleh proses seleksi item — beda dari `Drule`
(memaksimalkan volume/determinan **gabungan**, yang bisa "puas" dengan info
sangat besar di satu dimensi dan kecil di dimensi lain asal produknya
besar) dan `Trule` (jumlah, punya masalah yang sama).

`Erule` tidak memakai `weights` (lih. `PLAN.md`: `weights` hanya dipakai
`Trule`/`Arule`/`Wrule`) — konsisten dengan sifatnya sebagai jaminan
"presisi minimum di semua arah", bukan preferensi relatif antar dimensi.

`EPrule` menambahkan `inverse(gcov)` ke `total_info` sebelum menghitung
eigenvalue minimum — lih. `docs/teori/02-item-selection-overview.md` §4.

## Rank-deficiency di awal tes

Sama seperti `Drule` (lih. `docs/teori/02-item-selection-d-rule.md`):
info Fisher satu item dikotomus selalu rank 1, jadi sebelum item pertama
dijawab, eigenvalue minimum `total_info` (tanpa prior) **persis nol**
untuk semua kandidat individual — `Erule` tidak bisa membedakan kandidat
sampai info kumulatif full rank. `EPrule` tidak kena masalah ini.

## Referensi

- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection.
  *Psychometrika*, 74(2), 273–296.
