# 04 — `min_SEM`: Stopping Rule Berbasis Standard Error

> Landasan teori untuk mode `ClassifyType::None` (`src/stopping_rule/sem.rs`)
> — mode DEFAULT `StoppingConfig::classify`.

## 1. Rumus

Persis `DesignClass.R:346-347`:

```r
diff <- person$thetas_SE_history[nrow(...), ]   # SE terkini per-dimensi
diff[is.na(diff)] <- Inf                         # NA (SE tak terhitung) -> Inf
.Object@met_SEM <- diff < .Object@min_SEM
if(stage > 1L && !any(is.nan(diff)) && all(.Object@met_SEM))
    stop_now <- TRUE
```

Per-dimensi: `met_i = SE_i < min_SEM_i`. Berhenti hanya bila **SEMUA**
dimensi memenuhi (`all()`) — lih.
`docs/teori/04-stopping-rule-overview.md` §2 untuk motivasi semantik ini.

`sem::evaluate()` mereplikasi `NA -> Inf` lewat pengecekan `is_nan()`
eksplisit: `met_i = !se_i.is_nan() && se_i < min_sem_i`. `SE` bisa `NaN`
saat Hessian estimasi (Fase 3, method `Map`/`Ml`/`Wle`) tidak invertible —
kasus ini secara aman diperlakukan sebagai "belum presisi", bukan panic
atau `false` yang salah arah.

## 2. `min_SEM` sebagai skalar vs vektor

`mirtCAT` mengizinkan `min_SEM` berupa satu angka (di-broadcast ke seluruh
`nfact` dimensi) ATAU vektor persis panjang `nfact` (ambang berbeda per
dimensi) — divalidasi saat konstruksi `Design`
(`DesignClass.R:275-276`: `length(min_SEM) != 1L && length(min_SEM) !=
nfact` -> error). `StoppingConfig::min_sem: Vec<f64>` mereplikasi aturan
broadcast ini lewat fungsi privat `stopping_rule::broadcast()`, dipakai
juga oleh `delta_thetas` (lih.
`docs/teori/04-stopping-rule-delta-thetas.md`).

Default `min_SEM = 0.3` (`DesignClass.R:118`) — ambang yang cukup ketat
untuk model unidimensional/dimensi rendah, tapi lih.
`docs/teori/04-stopping-rule-overview.md` §3 untuk kenapa ambang ini
sering perlu dilonggarkan (atau digantikan `max_items` sebagai batas
praktis) pada `nfact` tinggi.

## Referensi

- Weiss, D. J., & Kingsbury, G. G. (1984). Application of computerized
  adaptive testing to educational problems. *Journal of Educational
  Measurement*, 21(4), 361–375.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 7.
  Springer.
