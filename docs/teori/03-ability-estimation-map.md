# 03 — MAP (Maximum A Posteriori)

> Landasan teori untuk `Method::Map` (`src/ability_estimation/map.rs`),
> **default** `EstimationConfig::method` — sama seperti default `mirtCAT`
> (`criteria_estimator <- 'MAP'`, `mirtCAT/R/DesignClass.R:104`).

## 1. Rumus

MAP memaksimalkan log-posterior langsung (bukan integrasi kuadratur seperti
EAP):

```
theta_hat = argmax_theta [ log L(theta) + log dmvnorm(theta; gmeans, gcov) ]
```

di mana `log L(theta) = sum_i log P_{k_i}(theta)` atas item terjawab
(`i`), `dmvnorm(theta; gmeans, gcov)` densitas normal multivariat prior
populasi. Persis `mirt::MAP.mirt()`
(`mirt/R/fscores.internal.R:684-703`):

```r
L <- sum(log(itemtrace)[as.logical(patdata)])   # log-likelihood
prior <- den_fun(ThetaShort, mean=mu, sigma=gp$gcov, ...)
L <- (-1) * (L + log(prior))                     # nlm MEMINIMALKAN -> negasi
```

`SE` dari akar diagonal invers Hessian log-posterior negatif di titik
optimum (observed information):

```
SE_j = sqrt( [ (-Hessian)^-1 ]_jj )
```

## 2. Optimisasi: Newton-Raphson finite-difference

`mirt::MAP()` memanggil `nlm()` (R base) untuk mencari optimum — **tanpa**
gradien analitik disuplai (`MAP.mirt()` cuma mengembalikan skalar, tidak
ada atribut `gradient`), artinya `nlm` memakai turunan numerik internalnya
sendiri secara default.

`ability_estimation::optim::maximize()` melakukan hal setara: Newton-Raphson
dengan gradien & Hessian **finite-difference** (central difference),
backtracking line search, dan clamp `theta` ke `[-20, 20]`
(`optim::MAX_THETA`, persis `max_theta=20` default `mirtCAT`,
`TestClass.R:73`). Ini port yang setia di level **algoritma** (sama-sama
konvergen ke optimum fungsi objektif yang identik) — bukan replikasi
byte-exact internal `nlm`, yang toh tidak dijamin unik untuk fungsi objektif
cembung yang sama.

## 3. Efek prior: sempit vs lebar

`gcov` (kovarians prior) mengontrol seberapa kuat `theta_hat` "ditarik" ke
arah `gmeans`:

- **Prior sempit** (`gcov` kecil, informasi prior kuat) — `theta_hat`
  mendekati `gmeans`, hampir tidak peduli data respons kecuali sangat
  banyak/ekstrem.
- **Prior lebar** (`gcov` besar, informasi prior lemah) — `theta_hat`
  mendekati estimasi ML murni (§ `docs/teori/03-ability-estimation-ml.md`),
  data respons mendominasi.

Formal: kontribusi prior ke skor (gradien log-posterior) adalah
`-inverse(gcov) * (theta - gmeans)` — makin kecil `gcov`, makin besar
`inverse(gcov)`, makin kuat "gaya tarik" ke `gmeans` relatif terhadap skor
likelihood.

## 4. MAP vs ML pada respons ekstrem

Beda penting dari `Ml` (`docs/teori/03-ability-estimation-ml.md`): karena
`log dmvnorm(theta; gmeans, gcov)` adalah fungsi cekung (concave) yang
menurun tanpa batas seiring `|theta|` menjauh dari `gmeans`, `MAP` **selalu**
punya maksimum berhingga — bahkan untuk pola respons yang membuat `ML`
divergen ke `±infinity` (semua item terjawab benar/salah), `MAP` tetap
memberi estimasi finite (ditarik prior). Ini alasan `mirtCAT` memakai `MAP`
sebagai default, bukan `ML`.

## Referensi

- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 5.
  Springer.
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29.
