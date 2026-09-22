# 02 — `Drule`/`DPrule`: D-Optimality

> Landasan teori untuk `src/item_selection/criteria/d_rule.rs` (Fase 2).

## Definisi

`Drule` memaksimalkan **determinan** matriks informasi Fisher kumulatif:

```
skor(kandidat) = det(total_info)
total_info = info_thetas(item terjawab) + info(kandidat, theta)
```

Determinan matriks informasi berbanding terbalik dengan **volume** elipsoid
kepercayaan asimtotik `theta` (D-optimality dalam teori desain eksperimen
klasik) — memaksimalkan determinan meminimalkan volume elipsoid itu, yakni
memaksimalkan presisi **gabungan** di seluruh dimensi laten sekaligus
(bukan per-dimensi independen seperti `Trule`).

`DPrule` menambahkan `inverse(gcov)` (presisi prior populasi) ke
`total_info` sebelum menghitung determinan — lih.
`docs/teori/02-item-selection-overview.md` §4.

## Rank-deficiency di awal tes

Info Fisher dari **satu** item dikotomus di `nfact > 1` selalu berbentuk
hasil kali luar (*outer product*) satu vektor gradien terhadap dirinya
sendiri — matriks ini selalu **rank 1**. Konsekuensinya: sebelum item
pertama dijawab (`info_thetas = 0`), `total_info` untuk tiap kandidat
individual juga rank 1 (`< nfact` untuk `nfact >= 2`), sehingga
`det(total_info) = 0` **persis** untuk semua kandidat — `Drule` (tanpa
prior) tidak bisa membedakan kandidat sampai `>= nfact` item terjawab
dengan arah info yang saling bebas linear. `DPrule` tidak kena masalah ini
(`inverse(gcov)` selalu full rank).

## Referensi

- Segall, D. O. (1996). Multidimensional adaptive testing.
  *Psychometrika*, 61(2), 331–354.
- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection.
  *Psychometrika*, 74(2), 273–296.
