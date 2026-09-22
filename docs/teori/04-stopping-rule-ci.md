# 04 — Klasifikasi `CI` (Confidence Interval)

> Landasan teori untuk `ClassifyType::Ci` (`src/stopping_rule/classify_ci.rs`).

## 1. Rumus

Persis `DesignClass.R:332-343`:

```r
z <- -abs(person$thetas - .Object@classify) / diff   # diff = SE terkini, NA -> Inf
.Object@met_classify <- as.vector(z < qnorm(.Object@classify_alpha))
if(stage > 1L && all(.Object@met_classify)){
    sig <- z < qnorm(.Object@classify_alpha)
    direction <- ifelse((person$thetas - .Object@classify) > 0, 'above cutoff', 'below cutoff')
    direction[!sig] <- 'no decision'
    stop_now <- TRUE
    classify_decision <- as.character(direction)
}
```

Interpretasi: `z_i` adalah jarak (senilai negatif) `theta_i` dari
`cutoff_i` dalam satuan `SE_i`. Secara setara, ini adalah uji apakah
interval kepercayaan `theta_i ± z_alpha * SE_i` seluruhnya berada di satu
sisi `cutoff_i` (tidak "menyentuh"/melewati cutoff) — nama `classify_CI`
merujuk pada interval ini. `classify_alpha = (1 - classify_CI) / 2`
(`DesignClass.R:212`), default `0.05` (setara `classify_CI = 0.90`).

## 2. Keputusan **per-dimensi**, independen dari gerbang `all()`

Beda penting dari cara R menyimpan hasilnya: variabel `sig` di R secara
matematis **identik** dengan `met_classify` (`z < qnorm(alpha)`, dihitung
ulang persis sama) — R hanya mengekspos label arah (`direction`) SAAT
`all(met_classify)` sudah `TRUE` (di dalam blok `if`), sehingga per-dimensi
"belum yakin" tidak pernah terlihat eksplisit sebagai `'no decision'` di R
sebelum keputusan akhir.

`classify_ci::evaluate()` sengaja menghitung **kedua** nilai (status `met`
DAN label `ClassifyDecision`) untuk **setiap** dimensi secara independen
dari gerbang `all()` — mengekspos status "satu dimensi sudah yakin, satu
belum" secara real-time (`NoDecision` untuk dimensi yang belum `met`, tanpa
menunggu seluruh dimensi selesai). Ini adalah nilai TAMBAH dibanding
perilaku R (bukan penyimpangan hasil akhir — begitu semua dimensi `met`,
label akhirnya identik dengan yang dihasilkan R), berguna untuk observability
per-langkah (lih. `examples/04_stopping_rule_ci.rs`).

Stop hanya terjadi bila **SEMUA** dimensi `met` (`all()`) — persis semantik
`docs/teori/04-stopping-rule-overview.md` §2.

## 3. `NaN` SE diperlakukan sebagai `Infinity`

Sama seperti `min_SEM` (`docs/teori/04-stopping-rule-min-sem.md` §1):
`SE_i` bernilai `NaN` (Hessian estimasi tidak invertible) membuat `z_i`
menjadi `-Infinity/Infinity`-semu; `classify_ci::evaluate()` menangani ini
eksplisit lewat substitusi `SE_i -> Infinity` sebelum menghitung `z`,
menghasilkan `z_i = 0` (netral, selalu `!met`) — bukan `NaN` yang akan
gagal senyap di perbandingan `<`.

## Referensi

- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*, bab stopping rule berbasis klasifikasi. Springer.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 7.
  Springer.
