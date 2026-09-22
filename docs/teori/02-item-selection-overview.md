# 02 — Pemilihan Item (Item Selection): Overview

> Landasan teori untuk `src/item_selection/` (Fase 2). Lih. dokumen per
> algoritma untuk rumus & referensi rinci tiap kriteria.

## 1. Kenapa `nfact > 1` membatasi pilihan kriteria

Sebagian besar kriteria seleksi item unidimensional klasik (`MI`, `MEPV`,
`MLWI`, `MPWI`, `MEI`, keluarga `IKL*`) bergantung pada **integrasi
kuadratur** atas grid `theta` (mis. menghitung expected posterior variance
lewat integral numerik atas distribusi posterior). Di ruang laten
multidimensional, grid kuadratur beranak-pinak `quadpts^nfact` titik —
untuk `nfact = 5` dengan `quadpts = 21` per dimensi, itu ≈ 4 miliar titik.
Ini yang disebut *curse of dimensionality*: kriteria berbasis kuadratur
langsung tidak praktis begitu `nfact` lebih dari 2–3 (lih. juga
`docs/teori/03-ability-estimation.md` untuk masalah serupa di estimasi
kemampuan).

`mirtCAT` menghindarinya untuk `nfact > 1` dengan memakai kriteria yang
**tidak** butuh integrasi kuadratur sama sekali:

- **D/T/A/E/W-rule** — bersandar pada **matriks informasi Fisher**
  (turunan analitik tertutup, `O(nfact^2)` sampai `O(nfact^3)` per
  evaluasi, bukan integral), dievaluasi *pada satu titik* `theta` (estimasi
  terbaik saat ini), bukan atas seluruh distribusi posterior.
- **KL/KLn** — pointwise Kullback-Leibler di dua titik (`theta ± delta`),
  bukan integral atas grid.
- **seq/random** — tidak butuh info sama sekali.

`mirtcat-engine` mengikuti pembatasan ini secara **struktural**: enum
`item_selection::Criterion` cuma berisi 9 varian di atas — kriteria yang
tidak valid untuk `nfact > 1` (`MI`, `MEPV`, `MLWI`, `MPWI`, `MEI`,
`IKL*`) sengaja tidak dimodelkan sama sekali, sehingga mustahil
dikonstruksi. Ini realisasi Prinsip Desain #1 `PLAN.md` ("type-safe, enum
bukan string bebas") yang lebih kuat dari validasi runtime: kesalahan
terdeteksi saat *compile*, bukan saat tes sedang berjalan.

## 2. Matriks informasi Fisher kumulatif (`info_thetas`)

D/T/A/E/W-rule semuanya mengevaluasi fungsi (determinan, trace, invers,
eigenvalue, bentuk kuadratik) dari **matriks informasi Fisher kumulatif**
`nfact × nfact` — jumlah informasi dari seluruh item yang sudah dijawab,
ditambah kandidat yang sedang dievaluasi, semua pada titik `theta` estimasi
terkini.

**Detail penting**: `PersonState::info_thetas(bank)` **menghitung ulang**
dari daftar item terjawab setiap dipanggil — bukan mengakumulasi
inkremental. Ini meniru persis `Person$Update.info_mats` di `mirtCAT`
(diverifikasi lewat inspeksi source R terpasang): info tiap item terjawab
selalu dievaluasi di estimasi `theta` **terbaru**, bukan "dibekukan" di
`theta` saat item itu dijawab. Kalau diakumulasi inkremental, info dari
item-item awal akan jadi basi begitu `theta` diestimasi ulang (Fase 3)
setelah item-item berikutnya dijawab — determinan/eigenvalue/trace yang
dihitung dari campuran info "segar" dan "basi" tidak lagi mencerminkan
presisi asimtotik yang sesungguhnya di estimasi terkini.

## 3. Ringkasan tiap kriteria

| Kriteria | Nilai yang dimaksimalkan | Bersandar pada | `weights`? |
|---|---|---|---|
| `seq` | — (index terkecil) | — | tidak |
| `random` | — (uniform random) | — | tidak |
| `KL`/`KLn` | KL divergence pointwise item kandidat | probabilitas kandidat sendiri | tidak |
| `Drule`/`DPrule` | `det(total_info)` | info kumulatif | tidak |
| `Trule`/`TPrule` | `sum_i w_i * total_info[i,i]` | info kumulatif | **ya** |
| `Arule`/`APrule` | `-sum_i w_i * pseudo_inv(total_info)[i,i]` | info kumulatif | **ya** |
| `Erule`/`EPrule` | `min_eigenvalue(total_info)` | info kumulatif | tidak |
| `Wrule`/`WPrule` | `w' * total_info * w` | info kumulatif | **ya** |

`total_info` untuk D/T/A/E/W-rule = `info_thetas(bank)` (item terjawab) +
info Fisher kandidat pada `theta` saat ini, ditambah `inverse(gcov)` bila
varian `*Prule`.

## 4. Varian `*Prule`: informasi prior

`DPrule`/`TPrule`/`APrule`/`EPrule`/`WPrule` menambahkan `inverse(gcov)`
(presisi prior populasi, `gcov` dari `GroupParams`) ke `total_info` sebelum
dievaluasi — pendekatan Bayesian: makin sedikit item terjawab, makin besar
pengaruh relatif prior terhadap keputusan seleksi (mendekati prior belief
tentang populasi); makin banyak item terjawab, pengaruh prior makin kecil
relatif terhadap info yang sudah terkumpul dari data respons.

Konsekuensi praktis penting: **tanpa** prior, matriks informasi dari satu
item dikotomus tunggal (di `nfact > 1`) selalu **rank 1** (hasil kali luar
gradien terhadap satu skalar) — sebelum item pertama dijawab, `total_info`
untuk tiap kandidat individual juga rank 1, membuat `Drule` (determinan)
dan `Erule` (eigenvalue minimum) **persis nol** untuk semua kandidat
(tidak ada yang "menang" secara berarti — argmax di antara nilai identik).
Varian `*Prule` tidak kena masalah ini: `inverse(gcov)` selalu full rank
(matriks kovarians populasi valid), jadi `total_info` full rank sejak
langkah pertama.

## 5. Estimator kemampuan otomatis (`criteria_estimator`)

`mirtCAT` memilih metode estimasi ulang `theta` (Fase 3) secara otomatis
berdasar kriteria seleksi yang dipakai: `ML` untuk `{Drule, Trule, Erule,
Wrule, Arule}` (non-prior), `MAP` untuk seluruh varian `*Prule` — alasannya
konsisten secara Bayesian: kriteria yang sudah menggabungkan prior
(`*Prule`) idealnya dipasangkan dengan estimator yang juga memakai prior
(`MAP`), sementara kriteria murni-data (non-prior) dipasangkan dengan
estimator murni-likelihood (`ML`). Ini keputusan Fase 3 (belum
diimplementasikan) — dicatat di sini karena berkaitan langsung dengan
pilihan `Criterion`.

## Referensi

- Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
  61(2), 331–354.
- van der Linden, W. J. (1999). Multidimensional adaptive testing with a
  minimum error-variance criterion. *Journal of Educational and Behavioral
  Statistics*, 24(4), 398–412.
- Mulder, J., & van der Linden, W. J. (2009). Multidimensional adaptive
  testing with optimal design criteria for item selection.
  *Psychometrika*, 74(2), 273–296.
- Chang, H.-H., & Ying, Z. (1996). A global information approach to
  computerized adaptive testing. *Applied Psychological Measurement*,
  20(3), 213–229.
- Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
  Interfaces for Multidimensional Item Response Theory Applications.
  *Journal of Statistical Software*, 71(5), 1–39.
  doi:10.18637/jss.v071.i05
