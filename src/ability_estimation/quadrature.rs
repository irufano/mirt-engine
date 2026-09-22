//! Kuadratur untuk EAP grid-based & titik Quasi-Monte Carlo (QMC) untuk EAP
//! dimensi tinggi.
//!
//! Ground truth algoritma diverifikasi dari source `mirt`/`mirtCAT`
//! terpasang (R 4.6.1 / mirt 1.47 / mirtCAT 1.14, diunduh & dibaca langsung
//! dari CRAN untuk memastikan port ini setia):
//! - `mirtCAT/R/TestClass.R:58-72` — tabel `quadpts` default per `nfact`
//!   (`switch(nfact, '1'=61,'2'=31,'3'=15,'4'=9,'5'=7,3)`) & `theta_range`
//!   default `c(-6, 6)`.
//! - `mirt/R/utils.R::QMC_quad`/`QUnif` — leaped-Halton sequence (basis =
//!   bilangan prima pertama per dimensi, `leap=409`) ditransformasi lewat
//!   `qnorm()` jadi awan kuasi-`N(0,I)`.

use crate::error::MirtCatError;
use crate::theta::ThetaVector;

/// Rentang integrasi grid EAP default, persis `mirtCAT` (`theta_range`
/// default `c(-6, 6)` di `TestClass.R:59`) — konstanta internal, sengaja
/// tidak dibuat configurable (mengikuti `mirtCAT` yang hardcode nilai ini
/// untuk seluruh sesi CAT).
pub(crate) const THETA_LIM: (f64, f64) = (-6.0, 6.0);

/// Leap (skip) Halton sequence, sama seperti default `leap=409` di
/// `mirt::QMC_quad`/`QUnif` — mengurangi korelasi antar-dimensi untuk
/// sequence berdimensi banyak.
const QMC_LEAP: u64 = 409;

/// Tabel `quadpts` default per `nfact`, persis `mirtCAT/R/TestClass.R:58-60`
/// (`switch(nfact, '1'=61, '2'=31, '3'=15, '4'=9, '5'=7, 3)`) — mengecil
/// seiring `nfact` supaya ukuran grid `quadpts^nfact` tidak meledak
/// kombinatorial (curse of dimensionality).
pub fn default_quadpts(nfact: usize) -> usize {
    match nfact {
        1 => 61,
        2 => 31,
        3 => 15,
        4 => 9,
        5 => 7,
        _ => 3,
    }
}

/// Grid kuadratur reguler: cartesian product `quadpts` titik per dimensi
/// pada `[-6, 6]`, total `quadpts^nfact` titik — persis `thetaComb(seq(
/// theta_lim[1], theta_lim[2], length.out=quadpts), nfact)` di `mirt`.
///
/// Error bila `nfact == 0` atau `quadpts < 2` (linspace butuh minimal 2
/// titik untuk merentang interval `[-6, 6]`).
pub fn build_grid(nfact: usize, quadpts: usize) -> Result<Vec<ThetaVector>, MirtCatError> {
    if nfact == 0 {
        return Err(MirtCatError::InvalidParameter(
            "quadrature::build_grid: nfact harus >= 1".into(),
        ));
    }
    if quadpts < 2 {
        return Err(MirtCatError::InvalidParameter(format!(
            "quadrature::build_grid: quadpts harus >= 2, didapat {quadpts}"
        )));
    }
    let axis = linspace(THETA_LIM.0, THETA_LIM.1, quadpts);
    let total = quadpts.pow(nfact as u32);
    let mut grid = Vec::with_capacity(total);
    for p in 0..total {
        let mut coords = vec![0.0; nfact];
        let mut rem = p;
        for coord in coords.iter_mut() {
            *coord = axis[rem % quadpts];
            rem /= quadpts;
        }
        grid.push(ThetaVector::from_vec(coords));
    }
    Ok(grid)
}

fn linspace(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    if n == 1 {
        return vec![lo];
    }
    let step = (hi - lo) / (n - 1) as f64;
    (0..n).map(|i| lo + step * i as f64).collect()
}

/// Titik Quasi-Monte Carlo: leaped-Halton sequence (basis = bilangan prima
/// pertama per dimensi) di `[0,1]^nfact`, ditransformasi lewat invers CDF
/// normal standar — awan kuasi-`N(0, I)`. Persis `mirt::QMC_quad(npts,
/// nfact, lim, leap=409)`.
///
/// **Catatan penting** (quirk asli `mirt`, di-port apa adanya): titik ini
/// **tidak** diskalakan ke `gmeans`/`gcov` grup — selalu awan kuasi-standar
/// `N(0, I)` terlepas dari prior grup sesungguhnya. Bobot EAP untuk titik
/// QMC karenanya seragam (`1/npts`), bukan `dmvnorm` seperti grid biasa —
/// lih. `eap.rs`.
pub fn build_qmc_points(nfact: usize, npts: usize) -> Result<Vec<ThetaVector>, MirtCatError> {
    if nfact == 0 {
        return Err(MirtCatError::InvalidParameter(
            "quadrature::build_qmc_points: nfact harus >= 1".into(),
        ));
    }
    if npts == 0 {
        return Err(MirtCatError::InvalidParameter(
            "quadrature::build_qmc_points: npts harus >= 1".into(),
        ));
    }
    let bases = halton_bases(nfact);
    let mut points = Vec::with_capacity(npts);
    for i in 0..npts {
        let index = 1 + (i as u64) * QMC_LEAP;
        let coords: Vec<f64> = bases
            .iter()
            .map(|&base| inverse_normal_cdf(radical_inverse(index, base)))
            .collect();
        points.push(ThetaVector::from_vec(coords));
    }
    Ok(points)
}

/// Basis Halton per dimensi: `nfact` bilangan prima pertama, dengan
/// penggantian bila `leap` (409) sendiri termasuk di antaranya — persis
/// `mirt::QUnif`: `if(leap > 1 && any(leap == pr)) pr <- c(pr[leap!=pr],
/// pr.[p+1])`. Praktis tidak pernah terpicu untuk `nfact` realistis di
/// domain ini (409 adalah bilangan prima ke-80), tapi tetap direplikasi
/// untuk kesetiaan port.
fn halton_bases(nfact: usize) -> Vec<u64> {
    let mut primes = first_n_primes(nfact + 1);
    if let Some(pos) = primes[..nfact].iter().position(|&p| p == QMC_LEAP) {
        primes[pos] = primes[nfact];
    }
    primes.truncate(nfact);
    primes
}

fn first_n_primes(n: usize) -> Vec<u64> {
    let mut primes: Vec<u64> = Vec::with_capacity(n);
    let mut candidate = 2u64;
    while primes.len() < n {
        if primes.iter().all(|&p| !candidate.is_multiple_of(p)) {
            primes.push(candidate);
        }
        candidate += 1;
    }
    primes
}

/// Radical inverse (van der Corput) basis `base` untuk `index` — setara
/// `mirt::QUnif`'s `sHalton`/`digitsBase` (digit basis-`base` dibalik
/// urutannya jadi pecahan `[0,1)`; padding digit tambahan di R tidak
/// mengubah nilai karena berkontribusi nol pada posisi signifikan tinggi).
fn radical_inverse(mut index: u64, base: u64) -> f64 {
    let mut result = 0.0;
    let mut f = 1.0 / base as f64;
    while index > 0 {
        result += f * (index % base) as f64;
        index /= base;
        f /= base as f64;
    }
    result
}

/// Invers CDF normal standar (aproksimasi rasional Acklam, presisi relatif
/// ~1.15e-9) — padanan `qnorm()` R, dipakai `build_qmc_points`. Hand-roll
/// murni supaya tidak menambah dependency statistik baru (mis. `statrs`),
/// mengikuti gaya `irt::probability` yang sudah hand-roll `logistic()`.
/// `p` harus di `(0, 1)` — dijamin oleh `radical_inverse` (selalu `(0,1)`
/// strict untuk `index >= 1`), jadi tidak perlu validasi tambahan di sini.
///
/// Konstanta di bawah dikutip persis presisi publikasi asli Acklam
/// (17 digit signifikan) -- `#[allow(excessive_precision)]` karena
/// memotongnya untuk memuaskan lint akan mengurangi kesetiaan terhadap
/// referensi, bukan cuma gaya kode.
#[allow(clippy::excessive_precision)]
pub(crate) fn inverse_normal_cdf(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383577518672690e+02,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    const P_LOW: f64 = 0.02425;
    const P_HIGH: f64 = 1.0 - P_LOW;

    if p < P_LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= P_HIGH {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn default_quadpts_matches_mirtcat_table() {
        assert_eq!(default_quadpts(1), 61);
        assert_eq!(default_quadpts(2), 31);
        assert_eq!(default_quadpts(3), 15);
        assert_eq!(default_quadpts(4), 9);
        assert_eq!(default_quadpts(5), 7);
        assert_eq!(default_quadpts(6), 3);
        assert_eq!(default_quadpts(20), 3);
    }

    #[test]
    fn build_grid_size_matches_quadpts_pow_nfact() {
        let grid = build_grid(2, 5).unwrap();
        assert_eq!(grid.len(), 25);
        let grid3 = build_grid(3, 4).unwrap();
        assert_eq!(grid3.len(), 64);
    }

    #[test]
    fn build_grid_1d_spans_theta_lim_endpoints() {
        let grid = build_grid(1, 5).unwrap();
        let values: Vec<f64> = grid.iter().map(|t| t[0]).collect();
        assert_relative_eq!(values.iter().cloned().fold(f64::INFINITY, f64::min), -6.0);
        assert_relative_eq!(
            values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
            6.0
        );
    }

    #[test]
    fn build_grid_rejects_invalid_input() {
        assert!(build_grid(0, 5).is_err());
        assert!(build_grid(2, 1).is_err());
    }

    #[test]
    fn build_qmc_points_returns_requested_count_within_reasonable_range() {
        let pts = build_qmc_points(3, 50).unwrap();
        assert_eq!(pts.len(), 50);
        // Kuasi-normal -- semua titik harus masuk akal (bukan NaN/tak
        // terhingga, dan mayoritas dalam rentang +/-5 SD).
        for p in &pts {
            for j in 0..3 {
                assert!(p[j].is_finite());
                assert!(p[j].abs() < 6.0);
            }
        }
    }

    #[test]
    fn build_qmc_points_rejects_invalid_input() {
        assert!(build_qmc_points(0, 10).is_err());
        assert!(build_qmc_points(2, 0).is_err());
    }

    #[test]
    fn radical_inverse_matches_hand_computed_base2() {
        // index=1 -> "1" basis 2 -> 0.1_2 = 0.5
        assert_relative_eq!(radical_inverse(1, 2), 0.5);
        // index=2 -> "10" basis 2 -> radical inverse membalik urutan -> 0.01_2 = 0.25
        assert_relative_eq!(radical_inverse(2, 2), 0.25);
        // index=3 -> "11" basis 2 -> 0.11_2 = 0.75
        assert_relative_eq!(radical_inverse(3, 2), 0.75);
    }

    #[test]
    fn inverse_normal_cdf_matches_known_quantiles() {
        assert_relative_eq!(inverse_normal_cdf(0.5), 0.0, epsilon = 1e-9);
        assert_relative_eq!(inverse_normal_cdf(0.975), 1.959963984540054, epsilon = 1e-8);
        assert_relative_eq!(
            inverse_normal_cdf(0.025),
            -1.959963984540054,
            epsilon = 1e-8
        );
        assert_relative_eq!(inverse_normal_cdf(0.999), 3.090232306167813, epsilon = 1e-7);
    }

    #[test]
    fn first_n_primes_matches_known_sequence() {
        assert_eq!(first_n_primes(6), vec![2, 3, 5, 7, 11, 13]);
    }
}
