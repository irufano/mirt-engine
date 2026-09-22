# 02 — `seq`: Baseline Non-Adaptif

> Landasan teori untuk `src/item_selection/criteria/seq.rs` (Fase 2).

## Definisi

`seq` bukan kriteria informasi — item bank disajikan **berurutan** sesuai
index: item ke-`n` yang disajikan adalah item bank ke-`n`. Formalnya,
dari himpunan kandidat (item belum dijawab) `C`, `seq` memilih
`min(C)`.

Ini padanan langsung kuesioner kertas-pensil tradisional: semua peserta
menjawab item yang sama, dalam urutan yang sama, tanpa adaptasi terhadap
respons. Dipakai sebagai:

- **Baseline perbandingan** — mengukur seberapa besar keuntungan presisi
  yang didapat dari kriteria adaptif (D/T/A/E/W-rule, KL) dibanding
  administrasi non-adaptif dengan jumlah item yang sama.
- **Kuesioner/survei** yang memang tidak dimaksudkan adaptif (mis. skala
  kepribadian standar yang harus diadministrasikan utuh dan berurutan
  untuk alasan psikometrik/legal).

## Kenapa valid untuk `nfact > 1`

`seq` tidak melibatkan model IRT atau matriks informasi sama sekali —
validitasnya untuk multidimensional trivial: index item tidak bergantung
pada `nfact`.

## Referensi

- Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
  Interfaces for Multidimensional Item Response Theory Applications.
  *Journal of Statistical Software*, 71(5), 1–39. (perbedaan mode adaptif
  vs non-adaptif di `mirtCAT`)
