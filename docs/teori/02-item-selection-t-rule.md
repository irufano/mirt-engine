# 02 — `Trule`/`TPrule`: Trace Berbobot

> Landasan teori untuk `src/item_selection/criteria/t_rule.rs` (Fase 2).

## Definisi

`Trule` memaksimalkan **trace berbobot** (jumlah diagonal, dikali bobot
per-dimensi) matriks informasi Fisher kumulatif:

```
skor(kandidat) = sum_i weights[i] * total_info[i,i]
```

Hanya elemen **diagonal** yang dipakai — `total_info[i,i]` adalah informasi
Fisher "murni" untuk dimensi `i` (mengabaikan kovariansi/korelasi info
antar dimensi, yang ada di off-diagonal). `weights` default `[1.0; nfact]`
(trace polos, semua dimensi setara); `weights` custom memungkinkan
memprioritaskan dimensi tertentu (mis. sub-skala yang secara substantif
lebih penting bagi keputusan yang diambil dari hasil tes).

`TPrule` menambahkan `inverse(gcov)` ke `total_info` sebelum menghitung
trace — lih. `docs/teori/02-item-selection-overview.md` §4.

## `Trule` vs `Wrule`: kenapa dua kriteria berbobot?

Keduanya menerima `weights`, tapi beda makna: `Trule` menjumlahkan
kontribusi **independen** tiap dimensi (diagonal saja) — cocok kalau
"pentingnya dimensi" itu memang independen satu sama lain. `Wrule` memakai
bentuk kuadratik penuh `w' * I * w` (termasuk off-diagonal) — nilainya
adalah informasi Fisher tentang **skor komposit** `w' * theta` (kombinasi
linear dimensi-dimensi, mis. skor total tertimbang). Kalau kandidat
memiliki info yang **tidak selaras** dengan arah `weights` (off-diagonal
besar relatif terhadap diagonal), `Trule` dan `Wrule` bisa memilih item
berbeda — lih. `docs/teori/02-item-selection-w-rule.md`.

## Referensi

- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection.
  *Psychometrika*, 74(2), 273–296.
