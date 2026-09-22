# 03 — Estimasi Kemampuan (Ability Estimation): Overview

> Landasan teori untuk `src/ability_estimation/` (Fase 3). Lih. dokumen per
> method untuk rumus & referensi rinci tiap `Method`.

## 1. `mirtCAT` mendelegasikan penuh ke `mirt::fscores()`

Source `mirtCAT::Design$Update.thetas` (`mirtCAT/R/DesignClass.R:57-94`,
diverifikasi lewat inspeksi langsung source R terpasang) ternyata **hanya**
memanggil `mirt::fscores(test@mo, method=method, response.pattern=...,
quadpts=test@quadpts, ...)` — tidak ada logika estimasi tersendiri di
`mirtCAT`. Ground truth algoritma tiap method (`EAP`/`MAP`/`ML`/`WLE`/
`EAPsum`) karenanya ada di `mirt/R/fscores.internal.R`, bukan di `mirtCAT`.
`ability_estimation::estimate()` adalah padanan langsung `mirt::fscores()`
dipanggil dengan satu pola respons (satu peserta) — **bukan** padanan
`Design$Update.thetas` (yang menambahkan logika sesi-level seperti fallback
otomatis `ML -> MAP` pada pola respons ekstrem, lih. §5).

## 2. Curse of dimensionality: grid kuadratur `quadpts^nfact`

`EAP` (dan `EAPsum`) grid-based mengintegrasikan posterior lewat kuadratur
numerik: jumlah titik grid = `quadpts^nfact` (cartesian product `quadpts`
titik per dimensi). Ini meledak kombinatorial seiring `nfact` naik — problem
yang sama seperti dibahas di `docs/teori/02-item-selection-overview.md` §1
untuk kriteria seleksi item berbasis kuadratur.

`mirtCAT` menahan ledakan ini dengan **mengecilkan `quadpts` seiring
`nfact` naik** (`mirtCAT/R/TestClass.R:58-60`, dikonfirmasi dari source R
terpasang):

| `nfact` | `quadpts` default | Total titik grid |
|---|---|---|
| 1 | 61 | 61 |
| 2 | 31 | 961 |
| 3 | 15 | 3.375 |
| 4 | 9 | 6.561 |
| 5 | 7 | 16.807 |
| `>5` | 3 | `3^nfact` |

`ability_estimation::quadrature::default_quadpts(nfact)` mereplikasi tabel
ini persis. `quadpts` bisa dioverride manual lewat
`EstimationConfig::quadpts`. Rentang integrasi (`theta_lim`) tetap `[-6,6]`
untuk seluruh `nfact` — konstanta internal, tidak configurable (mengikuti
`mirtCAT` yang hardcode nilai ini).

Trade-off tabel di atas jelas: `quadpts` mengecil supaya total titik tetap
terkendali, tapi **resolusi per-dimensi** ikut menurun (`quadpts=3` untuk
`nfact>5` berarti cuma 3 titik `{-6, 0, 6}` per dimensi — sangat kasar).
Inilah motivasi Quasi-Monte Carlo di §3.

## 3. Motivasi Quasi-Monte Carlo (Cai, 2010)

Alternatif untuk integrasi dimensi tinggi: alih-alih grid REGULER (yang
butuh `quadpts^nfact` titik untuk resolusi seragam di semua dimensi), pakai
**titik quasi-random** yang tersebar merata (low-discrepancy) di ruang
`nfact`-dimensi dengan jumlah titik **total** yang ditentukan bebas (tidak
harus `quadpts^nfact`) — memungkinkan resolusi jauh lebih baik per titik
untuk `nfact` besar, dengan total titik yang sama.

`mirt`/`mirtCAT` memakai **leaped-Halton sequence** (`mirt::QMC_quad`/
`QUnif`, `mirt/R/utils.R:2824-2944`): basis bilangan prima pertama per
dimensi, dengan "leap" (skip) `409` mengurangi korelasi antar-dimensi untuk
sequence berdimensi banyak — teknik umum di literatur QMC (dibahas Cai,
2010, dalam konteks estimasi dimensi tinggi untuk IRT). Titik `[0,1]^nfact`
hasil Halton ditransformasi lewat invers CDF normal standar (`qnorm()`)
jadi awan kuasi-`N(0,I)`.

**Catatan penting (quirk asli `mirt`, di-port apa adanya)**: titik QMC
**tidak** diskalakan ke `gmeans`/`gcov` grup sesungguhnya — selalu
mengasumsikan awan kuasi-standar `N(0,I)`. Bobot EAP untuk titik QMC
karenanya **seragam** (`1/npts`), bukan `dmvnorm(titik; gmeans, gcov)`
seperti grid biasa. Detail lengkap & implikasi praktis di
`docs/teori/03-ability-estimation-eap.md` §2.

## 4. Peringatan `EAP` + `nfact > 4` tanpa QMC

Dikonfirmasi persis dari source `mirtCAT/R/mirtCAT_preamble.R:184-191`:

```r
if(extract.mirt(mo, 'nfact') > 4L){
    if('EAP' %in% c(preCAT_method, method) && !test_object@fscores_args$QMC){
        warning('EAP methods, particularly without quasi-Monte Carlo
                 integration, are not recommended for higher-dimensional
                 models. Consider using MAP instead', call.=FALSE)
    }
}
```

Alasan: `EAP` grid-based di `nfact > 4` memakai `quadpts <= 7` (tabel §2) —
resolusi terlalu kasar untuk integrasi yang stabil, sementara metode
optimisasi langsung (`MAP`/`ML`/`WLE`) tidak punya masalah ini sama sekali
(tidak butuh grid). `ability_estimation::estimate()` mereplikasi peringatan
ini lewat `log::warn!` (bukan error keras — persis perilaku R yang cuma
`warning()`) saat `Method::Eap { qmc: false }` dipakai dengan `nfact > 4`.

## 5. Yang BUKAN cakupan Fase 3

- **Fallback otomatis `ML -> MAP`** untuk pola respons ekstrem (semua item
  terjawab bernilai identik) — ini logika `Design$Update.thetas` level
  SESI CAT (Fase 8), bukan `mirt::fscores()`/`ability_estimation::estimate()`
  itu sendiri. `Method::Ml` di Fase 3 sengaja dibiarkan divergen apa adanya
  (lih. `docs/teori/03-ability-estimation-ml.md`) — keputusan tentang KAPAN
  beralih metode adalah kebijakan orkestrasi sesi, bukan properti estimator
  itu sendiri.
- **`criteria_estimator` otomatis** (pemilihan `ML`/`MAP` berdasar kriteria
  seleksi item yang dipakai) — dibahas di
  `docs/teori/02-item-selection-overview.md` §5, juga Fase 8.

## Peta dokumen per-method

| Method | Teori | Panduan |
|---|---|---|
| `Eap { qmc }` | `03-ability-estimation-eap.md` | `03-ability-estimation-eap.md` |
| `Map` | `03-ability-estimation-map.md` | `03-ability-estimation-map.md` |
| `Ml` | `03-ability-estimation-ml.md` | `03-ability-estimation-ml.md` |
| `Wle` | `03-ability-estimation-wle.md` | `03-ability-estimation-wle.md` |
| `EapSum` | `03-ability-estimation-eap-sum.md` | `03-ability-estimation-eap-sum.md` |
| `Fixed` | `03-ability-estimation-fixed.md` | `03-ability-estimation-fixed.md` |

## Referensi

- Bock, R. D., & Aitkin, M. (1981). Marginal maximum likelihood estimation
  of item parameters: Application of an EM algorithm. *Psychometrika*,
  46(4), 443–459.
- Cai, L. (2010). High-dimensional exploratory item factor analysis by a
  Metropolis–Hastings Robbins–Monro algorithm. *Psychometrika*, 75(1),
  33–57.
- Warm, T. A. (1989). Weighted likelihood estimation of ability in item
  response theory. *Psychometrika*, 54(3), 427–450.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 5.
  Springer.
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29.
