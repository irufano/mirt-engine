# 03 — ML (Maximum Likelihood)

> Landasan teori untuk `Method::Ml` (`src/ability_estimation/ml.rs`).

## 1. Rumus

ML memaksimalkan log-likelihood murni, tanpa suku prior:

```
theta_hat = argmax_theta [ log L(theta) ]
          = argmax_theta [ sum_i log P_{k_i}(theta) ]
```

atas item terjawab `i` dengan kategori respons teramati `k_i`. Sama seperti
`Map` (`docs/teori/03-ability-estimation-map.md`), dioptimasi lewat
Newton-Raphson finite-difference (`ability_estimation::optim::maximize`) —
padanan `mirt::MAP.mirt(..., ML=TRUE)` (`mirt/R/fscores.internal.R:684-703`,
cabang `ML=TRUE` yang melewati suku `log(prior)`).

## 2. Potensi divergensi pada respons ekstrem

Tanpa prior, `log L(theta)` untuk 2PL/3PL/4PL monoton (mendekati batas
asimtotik) seiring `theta -> +infinity` (probabilitas benar mendekati batas
atas) atau `theta -> -infinity` (probabilitas benar mendekati batas bawah).
Kalau **seluruh** item terjawab punya kategori respons yang sama (semua
benar, atau semua salah), `log L(theta)` terus naik tanpa pernah mencapai
maksimum berhingga — estimasi ML divergen.

**Diverifikasi empiris** (R 4.6.1 / mirt 1.47, dijalankan langsung,
bukan cuma dibaca dari dokumentasi): pola respons semua-benar pada item
bank riil menghasilkan `mirt::fscores(method='ML')` -> `theta = Inf, SE =
NA`. `ability_estimation::ml::estimate()` mereplikasi ini persis: saat SE
hasil optimisasi `> 30` (ambang literal dari `mirt::ML()`,
`mirt/R/fscores.internal.R:1094-1097`) atau Hessian tidak invertible,
`theta` dilaporkan sebagai `±infinity` (tanda dari nilai mentah hasil
optimisasi) dan `SE` sebagai `NaN` — bukan angka besar yang menyesatkan
seolah presisi (SE literal 30+ berarti interval kepercayaan praktis tak
berhingga).

```r
# mirt/R/fscores.internal.R:1094-1097 (fungsi ML())
} else if(any(SEest > 30)){
    est[SEest > 30] <- Inf * sign(est[SEest > 30])
    SEest[SEest > 30] <- NA
}
```

## 3. Yang BUKAN cakupan `Method::Ml`

`mirtCAT::Design$Update.thetas` (`DesignClass.R:63-65`) punya fallback
otomatis: kalau `method == 'ML'` DAN seluruh respons terjawab identik
(`length(unique(na.omit(responses2))) < 2L`), method diganti diam-diam jadi
`'MAP'` sebelum memanggil `fscores()` — mencegah pengguna CAT sesungguhnya
pernah melihat `theta = Inf` di tengah sesi.

**Ini logika sesi CAT (Fase 8), bukan tanggung jawab
`ability_estimation::estimate()`.** `Method::Ml` di Fase 3 sengaja mentah
(divergen apa adanya) — keputusan KAPAN beralih ke `MAP` sebagai
pengamanan adalah kebijakan orkestrasi level lebih tinggi, terpisah dari
definisi matematis estimator ML itu sendiri. Lih.
`docs/teori/03-ability-estimation-overview.md` §5.

## Referensi

- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 5.
  Springer.
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29.
