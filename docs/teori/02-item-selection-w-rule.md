# 02 — `Wrule`/`WPrule`: Kriteria Informasi Berbobot Kustom

> Landasan teori untuk `src/item_selection/criteria/w_rule.rs` (Fase 2).

## Definisi

`Wrule` memaksimalkan **bentuk kuadratik penuh** matriks informasi Fisher
kumulatif terhadap vektor bobot:

```
skor(kandidat) = weights' * total_info * weights
                = sum_i sum_j weights[i] * weights[j] * total_info[i,j]
```

Beda dari `Trule` (yang hanya menjumlahkan **diagonal** berbobot,
mengabaikan off-diagonal), `Wrule` memasukkan **seluruh** elemen matriks
informasi, termasuk kovariansi info antar-dimensi. Secara matematis, ini
setara dengan **informasi Fisher tentang skor komposit** `c = weights' *
theta` (kombinasi linear dimensi-dimensi laten) — kalau yang jadi perhatian
sesungguhnya bukan tiap dimensi secara terpisah, melainkan sebuah skor
gabungan tertimbang (mis. skor total ujian yang menjumlahkan beberapa
sub-skala dengan bobot berbeda), `Wrule` adalah kriteria yang secara
langsung mengoptimalkan presisi skor *itu*, bukan proxy tidak langsung
lewat presisi per-dimensi.

`WPrule` menambahkan `inverse(gcov)` ke `total_info` sebelum menghitung
bentuk kuadratik — lih. `docs/teori/02-item-selection-overview.md` §4.

## Kontras numerik dengan `Trule`

Untuk matriks informasi **diagonal** (off-diagonal nol), `Wrule` dan
`Trule` berbobot kuadrat vs linear terhadap `weights` — urutannya bisa
tetap sama untuk banyak kasus, tapi **tidak dijamin identik** begitu ada
off-diagonal signifikan (info dari item yang membebankan pada kombinasi
dimensi, bukan satu dimensi murni). Contoh konkret (diverifikasi numerik
terhadap `mirtCAT::computeCriteria`): matriks info `[[10, 1], [1, 0.1]]`
dengan `weights = [0.5, 2.0]` menghasilkan `Trule = 0.5*10 + 2*0.1 = 5.2`
tapi `Wrule = 0.5^2*10 + 2*0.5*2*1 + 2^2*0.1 = 2.5 + 2.0 + 0.4 = 4.9` — beda
karena kontribusi off-diagonal (`2 * w1 * w2 * I12 = 2.0` masuk ke `Wrule`,
tidak ke `Trule`).

## Referensi

- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection.
  *Psychometrika*, 74(2), 273–296.
