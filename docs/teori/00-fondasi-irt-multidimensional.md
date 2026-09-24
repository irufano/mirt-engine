# 00 — Fondasi Model IRT Multidimensional

> Landasan teori untuk `src/theta.rs`, `src/irt/`, dan `src/bank.rs` (Fase 0).

## 1. Vektor kemampuan `theta` dan struktur data

Pada IRT multidimensional (MIRT), kemampuan peserta bukan skalar tunggal
melainkan vektor `theta = (theta_1, ..., theta_m)` di ruang laten `m` dimensi
(`nfact = m`). Ketidakpastian estimasi direpresentasikan sebagai matriks
kovarians `nfact × nfact`; standard error (SE) per-dimensi yang biasa
dilaporkan adalah akar diagonal matriks tersebut — **bukan** seluruh
matriks, karena off-diagonal menyatakan kovariansi antar-dimensi, bukan
presisi satu dimensi (Reckase, 2009, bab 3).

`mirt-engine` merepresentasikan ini sebagai tiga newtype tipis di atas
`nalgebra`:

- `ThetaVector` — vektor `theta`, panjang `nfact`.
- `SEVector` — vektor SE per-dimensi, panjang `nfact`.
- `InfoMatrix` — matriks persegi `nfact × nfact`, dipakai baik untuk
  matriks informasi Fisher maupun matriks kovarians populasi (`gcov`).

## 2. Model item dikotomus multidimensional (M2PL/M3PL/M4PL)

Untuk item dengan dua kategori respons (`0`/`1`), probabilitas menjawab
benar pada `theta` adalah:

```
z(theta)     = d + a_1*theta_1 + ... + a_m*theta_m
P*(theta)    = 1 / (1 + exp(-z(theta)))                  (kurva logistik "murni")
P(u=1|theta) = g + (u - g) * P*(theta)
P(u=0|theta) = 1 - P(u=1|theta)
```

di mana `a = (a_1, ..., a_m)` adalah vektor **slope/discrimination**
per-dimensi, `d` adalah **intercept** (terkait kesulitan), `g` adalah
**lower asymptote/guessing**, dan `u` adalah **upper asymptote**. Model ini
mencakup tiga kasus khusus lewat parameterisasi yang sama:

| Model | `g` | `u` |
|---|---|---|
| M2PL | `0` | `1` |
| M3PL | bebas `[0,1)` | `1` |
| M4PL | bebas `[0,1)` | bebas `(g,1]` |

Bentuk ini mengikuti Reckase (2009, bab 4) dan setara secara fungsional
dengan `P_dich()` di `mirt/src/traceLinePts.cpp` (dibaca sebagai rujukan
*perilaku* yang harus disamai — bukan disalin, lih. `PLAN.md` §1.3).

`mirt-engine` juga mendefinisikan **arah kesulitan multidimensional**
secara implisit lewat kombinasi `a`/`d` (setara konsep `MDIFF`/`MDISC` di
`mirt`): `MDISC = ||a||` (norma Euclidean vektor slope) menyatakan daya beda
gabungan lintas dimensi, dan `MDIFF = -d / MDISC` menyatakan kesulitan pada
arah slope tersebut (Reckase, 1985; 2009).

## 3. Graded response model (GRM) multidimensional

Untuk item politomus terurut (`K` kategori, `K > 2`), GRM mendefinisikan
`K - 1` **kurva batas kumulatif** (boundary curves):

```
Pstar_0(theta)     = 1
Pstar_k(theta)     = 1 / (1 + exp(-(d_k + a·theta)))   untuk k = 1..(K-1)
Pstar_K(theta)     = 0
P_k(theta)         = Pstar_{k-1}(theta) - Pstar_k(theta)   untuk k = 1..K
```

`Pstar_k` diinterpretasikan sebagai `P(respons >= kategori k+1 | theta)`.
Supaya `P_k >= 0` untuk semua `k`, boundary `d_1 > d_2 > ... > d_{K-1}`
**harus** terurut menurun ketat — `mirt-engine` memvalidasi ini saat
konstruksi (`GradedResponse::new`, error `MirtCatError::InvalidParameter`
bila tidak terurut), berbeda dari `mirt` yang secara diam-diam menghasilkan
probabilitas nol untuk parameter tak-valid (`P_graded()`,
`mirt/src/traceLinePts.cpp`) — konsisten dengan prinsip desain #5
("validasi ketat & error eksplisit").

Probabilitas kategori dipotong (clip) ke `[1e-20, 1 - 1e-20]` — menyamai
`mirt` — untuk mencegah nilai persis `0`/`1` yang membuat informasi Fisher
(pembagian oleh `P_k`) meledak secara numerik.

Rujukan: Muraki & Carlson (1995) untuk perluasan multidimensional GRM;
Samejima untuk formulasi boundary-curve aslinya (unidimensional).

## 4. Informasi Fisher multidimensional

Untuk item dengan kategori respons `1..K`, matriks informasi Fisher
`nfact × nfact` pada `theta` didefinisikan sebagai jumlah "outer product of
gradients" per kategori, dibagi probabilitas kategori tersebut:

```
I(theta) = sum_{k=1}^{K} [ grad(P_k)(theta) · grad(P_k)(theta)^T ] / P_k(theta)
```

Ini adalah bentuk umum informasi Fisher untuk keluarga eksponensial
(dipakai `mirt::iteminfo(..., multidim_matrix = TRUE)` via
`ItemInfo2(..., MD = TRUE)`, `mirt/R/utils.R`), dan untuk M2PL/M3PL/M4PL
mereduksi ke bentuk tertutup yang dikenal (Reckase, 2009, persamaan 4.x):

```
I_2PL(theta) = P(theta) * (1 - P(theta)) * a * a^T
```

dengan `P(theta) = P(u=1|theta)`. Untuk M3PL/M4PL, faktor skalarnya
berubah menjadi `((u-g) * P*(theta) * (1-P*(theta)))^2 / (P(theta)(1-P(theta)))`,
tapi struktur `a * a^T`-nya sama. `mirt-engine` menguji kedua bentuk
tertutup ini sebagai *sanity check* unit test (kasus 1 dimensi) sebelum
diverifikasi lintas-dimensi lewat fixture (lih. §5).

Matriks informasi inilah yang menjadi dasar seluruh kriteria seleksi item
D/T/A/E/W-rule di Fase 2 (`Drule` = determinan, `Trule` = trace, `Arule` =
negatif trace invers, `Erule` = eigenvalue minimum, `Wrule` = trace
berbobot) — lih. `InfoMatrix::{determinant, trace, trace_weighted, inverse,
min_eigenvalue}` di `src/theta.rs`.

## 5. Validasi silang dengan `mirt`/`mirtCAT`

`mirt-engine` adalah *clean-room reimplementation* dari formula di atas
(bukan port baris-demi-baris dari `mirt`/`mirtCAT`, lih. `PLAN.md` §1.3).
Untuk memverifikasi kesesuaian numerik, `tests/fixtures/generate_r_fixtures.R`
membangun item dari parameter tetap lewat `mirtCAT::generate.mirt_object()`
lalu mengekstrak `mirt::probtrace()`/`mirt::iteminfo(multidim_matrix=TRUE)`
sebagai *ground truth*, dibandingkan dengan `tests/integration_irt.rs`
(toleransi `1e-6`).

> **Status**: fixture saat ini (`tests/fixtures/irt_probability_information.json`)
> dihasilkan oleh eksekusi R sungguhan (`Rscript
> tests/fixtures/generate_r_fixtures.R`, R 4.6.1, `mirt` 1.47, `mirtCAT`
> 1.14) — hasil `ItemResponseModel` Rust cocok dengan `mirt::probtrace()`/
> `mirt::iteminfo(multidim_matrix=TRUE)` sampai belasan digit signifikan,
> jauh di bawah toleransi `1e-6`. Kalkulator cross-check Python independen
> (`tests/fixtures/compute_expected_fixture.py`, dipakai sementara sebelum
> R terpasang) tetap disimpan sebagai verifikasi kedua yang tidak butuh
> toolchain R — hasilnya identik dengan fixture R sampai >=10 digit.
> Catatan teknis: `generate.mirt_object()` butuh beberapa item "pengisi"
> (filler) per objek supaya ruang pola respons teoretis (`prod(K_i)`)
> cukup besar relatif jumlah parameter bebas — objek 1-item gagal dengan
> error derajat-kebebasan `mirt` (lih. komentar di
> `generate_r_fixtures.R`).

## Referensi

- Reckase, M. D. (2009). *Multidimensional Item Response Theory*. Springer.
- Reckase, M. D. (1985). The difficulty of test items that measure more
  than one ability. *Applied Psychological Measurement*, 9(4), 401–412.
- Muraki, E., & Carlson, J. E. (1995). Full-information factor analysis for
  polytomous item responses. *Applied Psychological Measurement*, 19(1),
  73–90.
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29. doi:10.18637/jss.v048.i06
