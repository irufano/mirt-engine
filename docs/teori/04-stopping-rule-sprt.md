# 04 — Klasifikasi `SPRT` (Sequential Probability Ratio Test)

> Landasan teori untuk `ClassifyType::Sprt` (`src/stopping_rule/sprt.rs`).
> **Beda semantik fundamental** dari `Ci`
> (`docs/teori/04-stopping-rule-ci.md`): BUKAN "all dimensi", melainkan
> satu uji rasio log-likelihood sekuensial **gabungan** untuk seluruh
> vektor `theta`.

## 1. Rumus (Wald, 1945)

Persis `DesignClass.R:320-331` (bounds di `DesignClass.R:271-272`):

```r
LL0 <- personLogLik(person=person, test=test, Theta=.Object@sprt_lower)
LL1 <- personLogLik(person=person, test=test, Theta=.Object@sprt_upper)
LL_diff <- LL1 - LL0
a <- log(sprt_beta / (1 - sprt_alpha))
b <- log((1 - sprt_beta) / sprt_alpha)
if(LL_diff < a) decision <- 'below cutoff'   # bukti kuat mendukung theta_lower
if(LL_diff > b) decision <- 'above cutoff'   # bukti kuat mendukung theta_upper
# LL_diff dalam [a, b] -> belum ada keputusan, tes lanjut
```

`LL0`/`LL1` adalah log-likelihood **seluruh item terjawab** dievaluasi di
titik hipotesis `theta_lower`/`theta_upper` (bukan estimasi `theta`
berjalan) — `LL_diff` adalah log rasio likelihood `L(upper)/L(lower)`.
Batas `[a, b]` diturunkan dari target tingkat kesalahan Tipe I (`alpha`) &
Tipe II (`beta`) yang diinginkan (default keduanya `0.05`) — semakin ketat
`alpha`/`beta`, semakin lebar `[a, b]`, semakin banyak bukti (item) yang
dibutuhkan sebelum keputusan diambil.

`sprt::evaluate()` menghitung `LL0`/`LL1` lewat
[`crate::ability_estimation::log_likelihood_at`] (Fase 3, sudah teruji) —
**tidak** menulis ulang loop probabilitas item (Prinsip Desain #1
`PLAN.md`).

## 2. Keputusan gabungan, bukan per-dimensi

Beda krusial dari `Ci`: `theta_lower`/`theta_upper` adalah dua **titik**
`nfact`-dimensi (dua hipotesis vektor lengkap), dan `LL_diff` adalah SATU
angka skalar (log rasio likelihood ANTARA kedua hipotesis titik itu) — bukan
`nfact` angka terpisah. Karenanya `StopDecision::classification` untuk mode
`Sprt` selalu berpanjang **1** (keputusan tunggal untuk seluruh vektor),
BUKAN `nfact` seperti mode `Ci`. Ini bukan penyederhanaan Rust — persis
`person$classify_decision` R yang skalar (bukan vektor) untuk mode SPRT.

## 3. Bug/limitasi upstream: SPRT multidimensional via `mirtCAT()`

**Temuan riset penting** (memengaruhi strategi fixture parity, lih.
`tests/fixtures/generate_r_fixtures_stopping_rule.R`): `personLogLik()`
(`mirtCAT::util.R:380`) memanggil `Theta <- as.matrix(Theta)` di baris
pertama. Untuk `Theta` berupa **plain numeric vector** panjang `nfact > 1`
(seperti `sprt_lower`/`sprt_upper` yang dikonfigurasi lewat
`design=list(...)` pada pipeline `mirtCAT()` end-to-end), `as.matrix()`
menghasilkan matriks **kolom** `nfact x 1` — ditafsirkan sebagai `nfact`
observasi 1-dimensi TERPISAH, bukan 1 observasi `nfact`-dimensi.

Dikonfirmasi lewat eksperimen langsung (R 4.6.1/mirtCAT 1.14 terpasang
DAN checkout source 1.14.3, keduanya punya kode identik): memanggil
`mirtCAT()` end-to-end dengan `design=list(sprt_lower=c(-1,-1),
sprt_upper=c(1,1))` pada model `nfact=2` menghasilkan error keras
(`'length = 2' in coercion to logical(1)'`) di dalam `Update.stop_now` —
SPRT multidimensional via pipeline `mirtCAT()` penuh **tidak berfungsi**
untuk `nfact > 1`.

**Mitigasi**: `personLogLik()` bekerja BENAR bila `Theta` sudah di-shape
eksplisit sebagai matriks `1 x nfact` sebelum dipanggil (`as.matrix()` pada
matriks yang sudah berbentuk benar adalah identity) — dikonfirmasi numerik
(nilai `LL` berbeda & masuk akal untuk hipotesis `lower` vs `upper`, bukan
lagi identik-kebetulan seperti pemanggilan yang salah bentuk). Fixture
parity crate ini (`tests/fixtures/stopping_rule.json`, bagian
`sprt_loglik`) dibangun dari pemanggilan `personLogLik()` LANGSUNG dengan
`Theta = matrix(c(...), nrow=1)`, bukan dari simulasi `mirtCAT()` mode
SPRT end-to-end. Port Rust di `sprt.rs` **tidak** mewarisi bug ini —
`log_likelihood_at` sudah menerima `ThetaVector` multidimensional dengan
orientasi yang benar sejak awal (tidak ada langkah `as.matrix()`-ambigu
yang setara di Rust).

## Referensi

- Wald, A. (1945). Sequential tests of statistical hypotheses. *Annals of
  Mathematical Statistics*, 16(2), 117–186.
- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*, bab stopping rule sekuensial. Springer.
