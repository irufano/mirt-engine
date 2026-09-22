# 03 — `Fixed`: Identity (Tanpa Estimasi Ulang)

> Landasan teori untuk `Method::Fixed` (`src/ability_estimation/fixed.rs`).

## 1. Semantik

`Fixed` **tidak** mengestimasi apa pun — mengembalikan `(person.theta(),
person.se())` persis apa adanya, tanpa memanggil optimisasi atau kuadratur
sama sekali. Padanan langsung cabang `method != 'fixed'` yang dilewati di
`mirtCAT::Design$Update.thetas` (`mirtCAT/R/DesignClass.R:87-89`):

```r
} else {
    person$Update_thetas(person$thetas,
                         person$thetas_SE_history[nrow(person$thetas_SE_history),])
}
```

Ketika `method == 'fixed'`, `mirtCAT` memanggil `Update_thetas` dengan
`theta`/SE **riwayat terakhir** apa adanya — tidak pernah memanggil
`mirt::fscores()`. Ini beda mendasar dari method lain (§Fase 3), yang semua
memanggil `fscores()` dan mengubah `theta`/SE berdasar respons baru.

## 2. Kapan dipakai

`Fixed` dipakai untuk **kuesioner non-adaptif**: seluruh item disajikan
dalam urutan tetap (biasanya dipasangkan dengan `Criterion::Seq`, Fase 2),
dan `theta` peserta hanya dihitung **sekali** di akhir (lewat method lain,
mis. `EAP`/`MAP`, di luar siklus item-per-item) — bukan diestimasi ulang
setiap item dijawab. Menjaga skor akhir tidak "terpengaruh urutan
penyajian" secara adaptif, sesuai untuk skenario di mana adaptivitas item
selection sendiri tidak dipakai (`nfact > 1` dengan kriteria `seq`, lih.
`docs/teori/02-item-selection-overview.md`).

## 3. Kenapa bukan sekadar "tidak memanggil `estimate()`"

Alternatif naif: caller cukup TIDAK memanggil `ability_estimation::estimate()`
sama sekali untuk kuesioner non-adaptif. Tapi `Fixed` sebagai varian
`Method` eksplisit tetap berguna: memungkinkan `EstimationConfig` yang
SAMA (satu titik konfigurasi) dipakai konsisten di seluruh siklus CAT
(termasuk sesi campuran adaptif/non-adaptif, atau logging/debug yang
seragam lewat `AbilityEstimationDebug` — walau untuk `Fixed` seluruh
fieldnya `None`, lih. `docs/panduan/03-ability-estimation-fixed.md`), tanpa
percabangan `if method == fixed { skip } else { estimate(...) }` tersebar
di kode caller.

## Referensi

- Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
  Interfaces for Multidimensional Item Response Theory Applications.
  *Journal of Statistical Software*, 71(5), 1–39. doi:10.18637/jss.v071.i05
