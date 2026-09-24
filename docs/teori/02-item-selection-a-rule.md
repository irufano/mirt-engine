# 02 — `Arule`/`APrule`: Minimum (Weighted) Error-Variance

> Landasan teori untuk `src/item_selection/criteria/a_rule.rs` (Fase 2).

## Definisi

`Arule` meminimalkan **trace berbobot dari invers** matriks informasi
Fisher kumulatif — `inverse(total_info)` adalah (perkiraan) matriks
kovarians asimtotik estimator `theta_hat`, jadi diagonalnya adalah varians
asimtotik per-dimensi. Nilai kriteria disajikan **sudah dinegasikan** agar
konsisten dengan konvensi argmax dipakai seluruh kriteria lain:

```
skor(kandidat) = -sum_i weights[i] * pseudo_inverse(total_info)[i,i]
```

(van der Linden, 1999) — memaksimalkan skor ini ≡ meminimalkan varians
asimtotik berbobot, mendekati minimum error-variance untuk estimasi
`theta`.

## Pseudo-invers, bukan invers biasa

Info Fisher dari **satu** item dikotomus di `nfact > 1` selalu **rank 1**
(lih. juga `docs/teori/02-item-selection-d-rule.md`) — sebelum cukup item
terjawab, `total_info` bisa **singular**, dan invers biasa tidak
terdefinisi. `mirt-engine` memakai **pseudo-invers Moore-Penrose** (via
SVD, `InfoMatrix::pseudo_inverse`) yang tetap memberi nilai valid & finite
untuk matriks rank-defisien: arah yang belum terinformasi (`singular value
~ 0`) diperlakukan sebagai kontribusi nol ke pseudo-invers, bukan varians
tak-hingga.

**Ini detail perilaku yang tidak disebut eksplisit di rencana awal Fase 2**
— ditemukan lewat verifikasi numerik langsung terhadap
`mirtCAT::computeCriteria(criteria = "Arule")` (nilai kriteria pada
kandidat dengan info rank-defisien cocok presisi tinggi dengan hitungan
pseudo-invers, TIDAK cocok dengan invers biasa yang akan gagal/`Inf` untuk
kasus itu). `mirtCAT` sendiri memakai pendekatan serupa secara internal
(C++, lewat rutin aljabar linear yang robust terhadap singularitas) —
detail ini divalidasi lewat perilaku numerik yang harus disamai (lih.
`PLAN.md` §1.3 soal clean-room reimplementation), bukan disalin dari
sumbernya.

`APrule` menambahkan `inverse(gcov)` ke `total_info` sebelum inversi —
selalu full rank, jadi tidak butuh jalur pseudo-invers untuk kasus
degenerate (meski `pseudo_inverse()` tetap dipakai seragam, karena untuk
matriks full-rank hasilnya identik dengan invers biasa).

## Referensi

- van der Linden, W. J. (1999). Multidimensional adaptive testing with a
  minimum error-variance criterion. *Journal of Educational and
  Behavioral Statistics*, 24(4), 398–412.
