//! Optimizer Newton-Raphson generik dengan gradien & Hessian
//! finite-difference — dipakai `Ml`, `Map`, `Wle` (Fase 3).
//!
//! Padanan level-algoritma dari `nlm()` R yang dipakai
//! `mirt::MAP.mirt`/`WLE.mirt` (`mirt/R/fscores.internal.R:684-738`,
//! `1047-1119`): `nlm` sendiri **tidak** diberi gradien analitik di sana
//! (fungsi objektif cuma mengembalikan skalar, tanpa atribut `gradient`),
//! artinya `nlm` memakai turunan numerik internalnya sendiri. Jadi
//! pendekatan finite-difference di sini setia di level "menemukan optimum
//! fungsi objektif yang identik", bukan replikasi byte-exact algoritma
//! internal `nlm` (yang memang tidak dijamin unik/deterministik lintas
//! implementasi berbeda untuk fungsi objektif yang sama).

use nalgebra::{DMatrix, DVector};

const DEFAULT_MAX_ITER: usize = 100;
const FD_STEP: f64 = 1e-5;
const GRAD_TOL: f64 = 1e-8;
const STEP_TOL: f64 = 1e-10;

/// Batas mutlak pencarian `theta` per-dimensi, persis `max_theta=20` default
/// `mirtCAT` (`TestClass.R:73`) — barrier numerik yang mencegah pola respons
/// ekstrem (mis. `Ml` pada semua-benar/semua-salah) melesat ke infinity
/// literal selama iterasi; klasifikasi akhir "divergen" (`theta -> +-inf`)
/// diputuskan oleh caller (`ml.rs`) berdasarkan SE, bukan di sini.
pub(crate) const MAX_THETA: f64 = 20.0;

/// Hasil optimisasi: titik optimum, Hessian numerik di titik itu (dipakai
/// caller untuk SE via `sqrt(diag(inverse(-hessian)))`), dan jejak untuk
/// `AbilityEstimationDebug`.
pub(crate) struct NewtonResult {
    pub theta: DVector<f64>,
    pub hessian_at_optimum: DMatrix<f64>,
    pub iterations: usize,
    pub objective_trace: Vec<f64>,
    pub converged: bool,
}

/// Maksimalkan `objective` (skalar, diasumsikan cukup mulus di sekitar
/// optimum) lewat Newton-Raphson dengan gradien & Hessian
/// finite-difference (central difference), backtracking line search bila
/// langkah Newton tidak menaikkan nilai objektif (atau Hessian tidak negatif
/// definit lokal), dan clamp tiap komponen `theta` ke
/// `[-MAX_THETA, MAX_THETA]` tiap iterasi.
pub(crate) fn maximize(objective: impl Fn(&DVector<f64>) -> f64, initial: DVector<f64>) -> NewtonResult {
    let n = initial.len();
    let mut theta = clamp(initial);
    let mut trace = Vec::new();
    let mut converged = false;
    let mut iterations = 0usize;
    let mut hessian = DMatrix::<f64>::zeros(n, n);
    let mut f_val = objective(&theta);
    trace.push(f_val);

    for iter in 0..DEFAULT_MAX_ITER {
        iterations = iter + 1;
        let grad = gradient(&objective, &theta);
        if grad.norm() < GRAD_TOL {
            converged = true;
            hessian = hessian_fd(&objective, &theta);
            break;
        }
        hessian = hessian_fd(&objective, &theta);

        let newton_step = hessian.clone().try_inverse().map(|inv| -(&inv * &grad));
        // Arah Newton hanya valid (menaikkan objective secara lokal) bila
        // dot(step, grad) > 0; kalau tidak (Hessian tidak negatif definit,
        // atau singular), mundur ke steepest ascent kecil yang selalu naik.
        let step = match newton_step {
            Some(s) if s.dot(&grad) > 0.0 => s,
            _ => grad.clone() * 0.1,
        };

        let mut alpha = 1.0;
        let mut next_theta = clamp(&theta + &step * alpha);
        let mut next_f = objective(&next_theta);
        let mut backtracks = 0;
        while next_f < f_val && backtracks < 30 {
            alpha *= 0.5;
            next_theta = clamp(&theta + &step * alpha);
            next_f = objective(&next_theta);
            backtracks += 1;
        }

        let step_size = (&next_theta - &theta).norm();
        theta = next_theta;
        f_val = next_f;
        trace.push(f_val);

        if step_size < STEP_TOL {
            converged = true;
            hessian = hessian_fd(&objective, &theta);
            break;
        }
    }

    NewtonResult {
        theta,
        hessian_at_optimum: hessian,
        iterations,
        objective_trace: trace,
        converged,
    }
}

fn clamp(theta: DVector<f64>) -> DVector<f64> {
    DVector::from_iterator(
        theta.len(),
        theta.iter().map(|&v| v.clamp(-MAX_THETA, MAX_THETA)),
    )
}

fn gradient(f: &impl Fn(&DVector<f64>) -> f64, theta: &DVector<f64>) -> DVector<f64> {
    let n = theta.len();
    let mut grad = DVector::zeros(n);
    for i in 0..n {
        let mut plus = theta.clone();
        let mut minus = theta.clone();
        plus[i] += FD_STEP;
        minus[i] -= FD_STEP;
        grad[i] = (f(&plus) - f(&minus)) / (2.0 * FD_STEP);
    }
    grad
}

fn hessian_fd(f: &impl Fn(&DVector<f64>) -> f64, theta: &DVector<f64>) -> DMatrix<f64> {
    let n = theta.len();
    let mut hess = DMatrix::zeros(n, n);
    let f0 = f(theta);
    for i in 0..n {
        for j in i..n {
            let h = if i == j {
                let mut plus = theta.clone();
                let mut minus = theta.clone();
                plus[i] += FD_STEP;
                minus[i] -= FD_STEP;
                (f(&plus) - 2.0 * f0 + f(&minus)) / (FD_STEP * FD_STEP)
            } else {
                let mut pp = theta.clone();
                let mut pm = theta.clone();
                let mut mp = theta.clone();
                let mut mm = theta.clone();
                pp[i] += FD_STEP;
                pp[j] += FD_STEP;
                pm[i] += FD_STEP;
                pm[j] -= FD_STEP;
                mp[i] -= FD_STEP;
                mp[j] += FD_STEP;
                mm[i] -= FD_STEP;
                mm[j] -= FD_STEP;
                (f(&pp) - f(&pm) - f(&mp) + f(&mm)) / (4.0 * FD_STEP * FD_STEP)
            };
            hess[(i, j)] = h;
            hess[(j, i)] = h;
        }
    }
    hess
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn maximizes_simple_quadratic_bowl() {
        // f(x,y) = -(x-2)^2 - (y+1)^2, maksimum di (2, -1), f=0.
        let objective = |t: &DVector<f64>| -(t[0] - 2.0).powi(2) - (t[1] + 1.0).powi(2);
        let result = maximize(objective, DVector::from_vec(vec![0.0, 0.0]));
        assert!(result.converged);
        assert_relative_eq!(result.theta[0], 2.0, epsilon = 1e-4);
        assert_relative_eq!(result.theta[1], -1.0, epsilon = 1e-4);
    }

    #[test]
    fn clamps_monotonically_increasing_objective_to_max_theta() {
        // f(x) = x -- tidak ada maksimum berhingga, optimizer harus berhenti
        // di batas MAX_THETA (bukan meledak ke infinity/NaN).
        let objective = |t: &DVector<f64>| t[0];
        let result = maximize(objective, DVector::from_vec(vec![0.0]));
        assert_relative_eq!(result.theta[0], MAX_THETA, epsilon = 1e-6);
        assert!(result.theta[0].is_finite());
    }

    #[test]
    fn hessian_at_optimum_is_negative_for_concave_bowl() {
        let objective = |t: &DVector<f64>| -(t[0]).powi(2);
        let result = maximize(objective, DVector::from_vec(vec![3.0]));
        assert!(result.hessian_at_optimum[(0, 0)] < 0.0);
    }

    #[test]
    fn objective_trace_is_nondecreasing_at_each_accepted_step() {
        let objective = |t: &DVector<f64>| -(t[0] - 1.5).powi(2);
        let result = maximize(objective, DVector::from_vec(vec![-5.0]));
        for w in result.objective_trace.windows(2) {
            assert!(w[1] >= w[0] - 1e-9);
        }
    }
}
