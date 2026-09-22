# 03 — EAP (Expected A Posteriori)

> Landasan teori untuk `Method::Eap { qmc }` (`src/ability_estimation/eap.rs`).
> Lih. `docs/teori/03-ability-estimation-overview.md` untuk konteks
> integrasi dimensi tinggi & tabel `quadpts` default.

## 1. Rumus dasar (Bock & Aitkin, 1981)

EAP adalah rata-rata posterior `theta` — mean distribusi posterior
`p(theta | respons)`, didekati lewat kuadratur numerik atas titik `theta_g`
(`g = 1..N`):

```
L(theta_g)  = prod_i P_{k_i}(theta_g)          -- likelihood pola respons di titik g
              (i = item terjawab, k_i = kategori respons yang diobservasi)

W(theta_g)  = prior(theta_g)                    -- bobot titik g (lih. §2)

theta_hat   = sum_g theta_g * L(theta_g) * W(theta_g)
              -----------------------------------------
                   sum_g L(theta_g) * W(theta_g)

SE_j        = sqrt( sum_g (theta_g[j] - theta_hat[j])^2 * L(theta_g) * W(theta_g)
                     ----------------------------------------------------
                          sum_g L(theta_g) * W(theta_g) )
```

`SE` di sini hanya diagonal matriks kovarians posterior (varians per
dimensi) — persis `SE <- sqrt(diag(vcov))` di `mirt::EAP()`
(`mirt/R/fscores.internal.R:1121-1165`), off-diagonal (kovarians
antar-dimensi) dihitung tapi tidak diekspos sebagai `SE` (sama seperti
`SEVector::from_covariance_diagonal` di Fase 0/1).

Item yang **belum dijawab** tidak berkontribusi ke `L(theta_g)` — hanya
item di `person.answered_items()`/`person.responses()` yang ikut dihitung.

## 2. Dua mode integrasi: grid vs QMC

### Grid-based (`qmc: false`)

`theta_g` = cartesian product `quadpts` titik per dimensi pada `[-6, 6]`
(`quadrature::build_grid`). Bobot:

```
W(theta_g) = dmvnorm(theta_g; gmeans, gcov)
```

(densitas normal multivariat prior populasi, dinormalisasi implisit lewat
pembagian oleh `sum_g L*W` di rumus §1 — tidak perlu dinormalisasi eksplisit
ke jumlah 1 lebih dulu, konstanta skala apa pun pada `W` hilang di rasio).

### QMC-based (`qmc: true`)

`theta_g` = leaped-Halton sequence ditransformasi invers CDF normal
(`quadrature::build_qmc_points`, lih.
`docs/teori/03-ability-estimation-overview.md` §3 untuk detail algoritma).

**Bobot seragam**: `W(theta_g) = 1` untuk semua `g` — dikonfirmasi dari
source `mirt` (`fscores.internal.R:838-847`):

```r
ThetaShort <- Theta <- if(QMC){
    tmp <- QMC_quad(npts=quadpts, nfact=nfact, lim=theta_lim)  # TIDAK diskalakan gmeans/gcov
} else {
    theta <- seq(theta_lim[1L], theta_lim[2L], length.out=quadpts)
    thetaComb(theta, nfact)
}
prior <- if(QMC) rep(1, nrow(Theta)) else den_fun(Theta, mean=gp$gmeans, sigma=gp$gcov, ...)
```

**Implikasi praktis**: karena titik QMC sendiri sudah "berbentuk" awan
kuasi-`N(0,I)` (bukan `N(gmeans, gcov)` sesungguhnya), akurasi EAP QMC-based
untuk grup dengan `gmeans != 0` atau `gcov != I` bergantung pada seberapa
representatif awan standar itu relatif terhadap prior sesungguhnya — untuk
grup non-standar yang jauh dari `N(0,I)`, EAP QMC-based bisa kurang akurat
dibanding grid-based pada `nfact` rendah (di mana grid tetap murah). QMC
baru betul-betul unggul di `nfact` tinggi, di mana grid dipaksa memakai
`quadpts` sangat kecil (tabel overview §2) sementara QMC bisa memakai
jumlah titik total berapa pun (tidak terikat `quadpts^nfact`).

## 3. Kenapa dua algoritma satu modul

Sesuai keputusan desain PLAN.md (paralel dengan `KL`/`KLn` di Fase 2): grid
& QMC adalah **satu algoritma** (EAP) dengan **mode integrasi** berbeda,
bukan dua algoritma estimasi yang beda secara konsep — keduanya
mengestimasi rumus posterior mean yang identik (§1), hanya beda cara
mendekati integralnya secara numerik.

## Referensi

- Bock, R. D., & Aitkin, M. (1981). Marginal maximum likelihood estimation
  of item parameters: Application of an EM algorithm. *Psychometrika*,
  46(4), 443–459.
- Cai, L. (2010). High-dimensional exploratory item factor analysis by a
  Metropolis–Hastings Robbins–Monro algorithm. *Psychometrika*, 75(1),
  33–57.
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29.
