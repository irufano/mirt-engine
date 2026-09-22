# 03 — WLE (Weighted Likelihood Estimation)

> Landasan teori untuk `Method::Wle` (`src/ability_estimation/wle.rs`).

## 1. Motivasi: bias ML pada sampel item terbatas

Estimator ML (`docs/teori/03-ability-estimation-ml.md`) punya bias sistematik
untuk jumlah item terbatas (bias `O(1/n_items)`) — Warm (1989) menunjukkan
koreksi bias orde-pertama bisa ditambahkan langsung ke *persamaan skor*
(bukan mengoreksi `theta_hat` setelah dihitung), menghasilkan estimator baru
("weighted likelihood") yang bias-nya lebih kecil, sekaligus **tetap
berhingga** untuk pola respons yang membuat ML divergen (semua benar/semua
salah) — properti praktis yang sangat berguna di awal CAT (item terjawab
masih sedikit, risiko pola ekstrem tinggi).

## 2. Rumus 1D klasik (Warm, 1989) vs generalisasi multidimensional `mirt`

Formula asli Warm (univariat) menambahkan suku koreksi ke persamaan skor:

```
S(theta) + J(theta) / (2 * I(theta)) = 0
```

di mana `S` skor (turunan log-likelihood), `I` informasi Fisher, `J` terkait
turunan ketiga log-likelihood (skewness kurva informasi).

`mirt::WLE.mirt()` (`mirt/R/fscores.internal.R:705-738`) menggeneralisasi
ke multidimensional lewat **objektif langsung** (bukan solusi tertutup
persamaan skor):

```
theta_hat = argmax_theta [ log L(theta) + 0.5 * log(det(I(theta))) ]
```

di mana `I(theta)` matriks informasi Fisher **kumulatif** (`nfact × nfact`)
dari seluruh item terjawab, dievaluasi di `theta`. Suku `0.5*log(det(I))`
inilah generalisasi koreksi bias Warm ke multidimensional — determinan info
Fisher menggantikan skalar informasi 1D. `ability_estimation::wle::estimate()`
memakai `crate::irt::cumulative_information` (Fase 0/2, sudah teruji) untuk
`I(theta)`, `InfoMatrix::determinant()` untuk suku determinan.

Untuk `nfact = 1`, `det(I)` = skalar info itu sendiri — konsisten dengan
formula 1D klasik (walau bukan solusi tertutup Warm yang identik persis,
karena `mirt` sendiri memaksimalkan objektif ini secara numerik, bukan
menyelesaikan persamaan skor Warm secara analitik).

## 3. Kenapa suku `log(det(I))` menahan divergensi

`log(det(I(theta)))` **menurun** seiring `|theta|` menjauh dari pusat
distribusi respons (info Fisher item mengecil saat probabilitas jenuh
mendekati 0/1) — efeknya berlawanan arah dengan `log L(theta)` yang terus
naik pada pola ekstrem (lih.
`docs/teori/03-ability-estimation-ml.md` §2). Kombinasi keduanya membuat
objektif WLE punya maksimum berhingga bahkan saat `log L(theta)` sendiri
tidak terbatas — inilah properti "tetap berhingga pada respons ekstrem"
yang jadi motivasi utama WLE.

## 4. Optimisasi

Sama seperti `Map`/`Ml`: Newton-Raphson finite-difference
(`ability_estimation::optim::maximize`). `mirt::WLE.mirt()` sendiri juga
dioptimasi lewat `nlm()` tanpa gradien analitik — determinan matriks
informasi yang berubah terhadap `theta` (turunan `d(det I)/d(theta)`) tidak
punya bentuk tertutup sederhana di `mirt` sendiri (dites numerik), jadi
pendekatan finite-difference di sini justru **setia**, bukan penyederhanaan.

## Referensi

- Warm, T. A. (1989). Weighted likelihood estimation of ability in item
  response theory. *Psychometrika*, 54(3), 427–450.
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29.
