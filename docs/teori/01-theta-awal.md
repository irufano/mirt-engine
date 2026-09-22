# 01 — Theta Awal (Initial Theta)

> Landasan teori untuk `src/initial_theta/mod.rs` (Fase 1).

## 1. Kenapa dibutuhkan titik awal

Sebelum item pertama dijawab, siklus adaptif MCAT (item selection, ability
estimation, stopping rule) butuh sebuah titik awal `theta_0` dan
ketidakpastian awal `SE_0` untuk mulai beroperasi — mis. kriteria seleksi
item D/T/A/E/W-rule (Fase 2) mengevaluasi matriks informasi Fisher relatif
terhadap `theta` saat ini, dan estimator MAP (Fase 3) butuh titik awal untuk
optimisasi numerik.

## 2. Prior populasi multivariat sebagai dasar

Pada IRT multidimensional, populasi peserta diasumsikan mengikuti distribusi
normal multivariat `theta ~ N(gmeans, gcov)`, hasil ekstraksi model
terkalibrasi (Segall, 1996; Reckase, 2009, bab 3). Model yang diidentifikasi
dengan skala standar (yang paling umum) menetapkan `gmeans = 0`, sehingga
titik tengah distribusi — pilihan titik awal yang wajar sebelum ada
informasi respons — bertepatan dengan vektor nol.

`mirtcat-engine` mengikuti perilaku `mirtCAT` (`Person$initialize()`) apa
adanya: **`theta_0` literal vektor nol** sepanjang `nfact`, bukan hasil
membaca `gmeans` secara eksplisit setiap inisialisasi (meski secara konsep
keduanya berhimpit untuk model yang lazim). Ini penting: `GroupParams`
menyediakan `gmeans` untuk transparansi/debug (lih. `InitialThetaDebug`) dan
untuk fase-fase lain (mis. prior di MAP, Fase 3) — bukan untuk menghitung
`theta_0`.

`SE_0` diambil dari **akar diagonal** `gcov` populasi:

```
SE_0[i] = sqrt(gcov[i][i])
```

Hanya diagonal yang diambil — off-diagonal `gcov` menyatakan kovariansi
antar-dimensi, bukan presisi satu dimensi (lih. juga
`docs/teori/00-fondasi-irt-multidimensional.md` §1). `SE_0` dihitung dari
`gcov` **selalu**, terlepas dari apakah `theta_0` dioverride manual — dua
hal ini independen di `mirtCAT` (lih. §4).

## 3. Override manual

Peneliti/operator tes kadang punya informasi eksternal tentang peserta
(mis. skor tes sebelumnya) yang lebih baik dari titik nol sebagai tebakan
awal. `mirtCAT` mendukung ini lewat `thetas.start`:

- **Vektor** panjang `nfact` — dipakai untuk satu peserta (sesi CAT
  interaktif).
- **Matriks `N × nfact`** — satu baris per peserta, dipakai untuk simulasi
  offline banyak peserta sekaligus (`local_pattern` dengan `N` baris
  respons), diindeks per baris.

`mirtcat-engine` merepresentasikan ini sebagai `ThetaStart::Fixed` (vektor)
dan `ThetaStart::PerPersonMatrix` (kumpulan vektor, diindeks lewat parameter
`person_index` pada `initialize()`).

## 4. Verifikasi terhadap source `mirtCAT`

Perilaku di atas diverifikasi langsung terhadap source `mirtCAT` 1.14
terpasang (bukan ditebak dari dokumentasi), lewat introspeksi R:

```r
getRefClass("Person")$methods("initialize")
getFromNamespace("run_local", "mirtCAT")
```

Potongan relevan (`Person$initialize`, `PersonClass.R`):

```r
thetas <<- matrix(numeric(nfact), nrow = 1L)               # theta awal = nol
thetas_SE_history <<- matrix(theta_SEs, 1L)                # SE awal = argumen theta_SEs
if (!is.null(thetas.start_in) && !is.matrix(thetas.start_in))
    thetas <<- matrix(thetas.start_in, nrow = 1L)           # override (vektor)
```

Potongan relevan (`run_local()`, pemanggil `Person$new()`):

```r
person <- Person$new(nfact = nfact, nitems = nitems,
    theta_SEs = sqrt(diag(test@gp$gcov)),                   # SE0 selalu dari gcov
    thetas.start_in = if (is.matrix(thetas.start_in))
        thetas.start_in[n, ]                                # override per-baris (matriks)
    else thetas.start_in,
    ...)
```

Ini mengonfirmasi: `theta_SEs` dihitung **sebelum** dan **independen** dari
`thetas.start_in` — override theta tidak pernah mengubah `SE_0`. Fixture
integration test (`tests/fixtures/generate_r_fixtures_initial_theta.R`)
menjalankan pipeline `mirtCAT()` off-line sungguhan dan membaca baris
pertama `thetas_history`/`thetas_SE_history` dari objek `Person` yang
sebenarnya dipakai `run_local()`, untuk kasus default maupun override, pada
model 2D dan 3D dengan `gcov` non-diagonal — memverifikasi baik nilai `0`
default maupun independensi `SE_0` dari override secara numerik langsung
terhadap R (bukan hanya pembacaan source).

## 5. Bobot per-dimensi (`weights`)

`weights` (default `[1.0; nfact]`) diinisialisasi bersamaan `theta_0`/`SE_0`
walau baru dipakai di Fase 2, oleh kriteria `Trule`/`TPrule`, `Arule`/
`APrule`, `Wrule`/`WPrule` — kriteria-kriteria itu mengevaluasi trace
matriks informasi **berbobot** per dimensi (mis. dimensi yang secara
substantif lebih penting diberi bobot lebih besar).

## Referensi

- Segall, D. O. (1996). Multidimensional adaptive testing.
  *Psychometrika*, 61(2), 331–354.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 3.
  Springer.
