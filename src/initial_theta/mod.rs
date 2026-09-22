//! Fase 1 — Theta Awal (Initial Theta): titik awal vektor kemampuan
//! (`theta₀`) dan SE awal sebelum item pertama disajikan, mengikuti
//! perilaku `mirtCAT` untuk `nfact > 1` (lih. `PLAN.md` §Fase 1).
//!
//! Perilaku diverifikasi langsung terhadap source `mirtCAT` terpasang
//! (R 4.6.1, mirtCAT 1.14 — `getRefClass("Person")$methods("initialize")`
//! dan `getFromNamespace("run_local", "mirtCAT")`):
//! - `Person$initialize`: `thetas <<- matrix(numeric(nfact), nrow = 1L)` —
//!   theta awal literal **nol**, ditimpa `thetas.start_in` bila diberikan
//!   (`if (!is.null(thetas.start_in) && !is.matrix(thetas.start_in)) thetas
//!   <<- matrix(thetas.start_in, nrow = 1L)`).
//! - `run_local()`: `Person$new(..., theta_SEs = sqrt(diag(test@gp$gcov)),
//!   thetas.start_in = if (is.matrix(thetas.start_in)) thetas.start_in[n, ]
//!   else thetas.start_in, ...)` — SE awal **selalu** dari `sqrt(diag(gcov))`
//!   (hanya diagonal), terlepas dari override theta; matriks `N × nfact`
//!   diindeks per baris `n` (peserta ke-`n`) untuk simulasi offline banyak
//!   peserta — di Rust ini jadi `ThetaStart::PerPersonMatrix` + parameter
//!   `person_index` pada [`initialize`].

use crate::bank::GroupParams;
use crate::error::MirtCatError;
use crate::theta::{SEVector, ThetaVector};

/// Sumber titik awal `theta`. Default [`ThetaStart::Zero`] — persis
/// `Person$initialize()` (`thetas <<- matrix(numeric(nfact), nrow = 1L)`).
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ThetaStart {
    /// Vektor nol sepanjang `nfact` (default).
    #[default]
    Zero,
    /// Override manual untuk satu peserta (padanan `thetas.start` berupa
    /// vektor panjang `nfact`).
    Fixed(ThetaVector),
    /// Override per-peserta untuk simulasi offline banyak peserta (padanan
    /// `thetas.start` berupa matriks `N × nfact`, diindeks per baris di
    /// `run_local()`).
    PerPersonMatrix(Vec<ThetaVector>),
}

/// Konfigurasi inisialisasi theta awal.
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InitialThetaConfig {
    /// Sumber theta awal (langsung/dari populasi).
    pub start: ThetaStart,
    /// Bobot per-dimensi, dipakai belakangan oleh `Trule`/`TPrule`,
    /// `Arule`/`APrule`, `Wrule`/`WPrule` (Fase 2). `None` → default
    /// `vec![1.0; nfact]`.
    pub weights: Option<Vec<f64>>,
    /// Aktifkan jejak debug (prinsip desain #7 `PLAN.md`). Default `false`
    /// — tidak ada alokasi/komputasi tambahan di jalur ini saat nonaktif.
    pub debug: bool,
}

/// Jejak debug opsional dari [`initialize`] — hanya `Some` bila
/// `InitialThetaConfig::debug == true`; tidak memengaruhi nilai utama yang
/// dikembalikan (prinsip desain #7).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InitialThetaDebug {
    /// Jalur `ThetaStart` yang dipakai.
    pub source: ThetaStart,
    /// `gmeans` sumber (dari `GroupParams`), untuk observability — bukan
    /// dipakai menghitung `theta₀` (yang selalu nol kecuali dioverride).
    pub gmeans_used: ThetaVector,
    /// Diagonal `gcov` sumber, sebelum diakar-kuadratkan menjadi SE₀.
    pub gcov_diag_used: Vec<f64>,
}

/// Inisialisasi `(theta₀, SE₀, weights)` untuk satu peserta, mengikuti
/// `Person$initialize()` + `run_local()` `mirtCAT`.
///
/// `person_index` hanya dipakai untuk `ThetaStart::PerPersonMatrix`
/// (padanan indeks baris `n` di `thetas.start_in[n, ]`); diabaikan untuk
/// `Zero`/`Fixed`.
///
/// Error bila dimensi `ThetaStart::Fixed`/entri `PerPersonMatrix` atau
/// `weights` manual tidak cocok dengan `nfact` model (`group.nfact()`), atau
/// `person_index` di luar jangkauan `PerPersonMatrix`.
pub fn initialize(
    config: &InitialThetaConfig,
    group: &GroupParams,
    person_index: usize,
) -> Result<(ThetaVector, SEVector, Vec<f64>, Option<InitialThetaDebug>), MirtCatError> {
    let nfact = group.nfact();

    let theta0 = match &config.start {
        ThetaStart::Zero => ThetaVector::zeros(nfact),
        ThetaStart::Fixed(theta) => {
            theta.validate_nfact(nfact, "initial_theta::initialize: ThetaStart::Fixed")?;
            theta.clone()
        }
        ThetaStart::PerPersonMatrix(rows) => {
            let theta = rows.get(person_index).ok_or_else(|| {
                MirtCatError::InvalidParameter(format!(
                    "initial_theta::initialize: person_index {person_index} di luar \
                     jangkauan ThetaStart::PerPersonMatrix (panjang {})",
                    rows.len()
                ))
            })?;
            theta.validate_nfact(
                nfact,
                "initial_theta::initialize: ThetaStart::PerPersonMatrix",
            )?;
            theta.clone()
        }
    };

    // SE awal selalu dari sqrt(diag(gcov)) -- terlepas dari override theta
    // (lih. run_local(): `theta_SEs = sqrt(diag(test@gp$gcov))` dihitung
    // independen dari `thetas.start_in`).
    let se0 = SEVector::from_covariance_diagonal(group.gcov());

    let weights = match &config.weights {
        None => vec![1.0; nfact],
        Some(w) => {
            if w.len() != nfact {
                return Err(MirtCatError::DimensionMismatch {
                    expected: nfact,
                    actual: w.len(),
                    context: "initial_theta::initialize: weights",
                });
            }
            w.clone()
        }
    };

    let debug = config.debug.then(|| InitialThetaDebug {
        source: config.start.clone(),
        gmeans_used: group.gmeans().clone(),
        gcov_diag_used: (0..nfact).map(|i| group.gcov().get(i, i)).collect(),
    });

    Ok((theta0, se0, weights, debug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theta::InfoMatrix;
    use approx::assert_relative_eq;
    use nalgebra::DMatrix;

    fn group_2d_non_diagonal() -> GroupParams {
        let gcov = InfoMatrix::from_matrix(DMatrix::from_row_slice(2, 2, &[1.44, 0.3, 0.3, 2.25]))
            .unwrap();
        GroupParams::new(ThetaVector::zeros(2), gcov).unwrap()
    }

    #[test]
    fn default_start_is_zero_theta_along_nfact() {
        let group = GroupParams::standard_normal(3);
        let config = InitialThetaConfig::default();
        let (theta0, _, _, _) = initialize(&config, &group, 0).unwrap();
        assert_eq!(theta0.as_slice(), &[0.0, 0.0, 0.0]);
    }

    #[test]
    fn fixed_override_is_used_as_is() {
        let group = GroupParams::standard_normal(2);
        let config = InitialThetaConfig {
            start: ThetaStart::Fixed(ThetaVector::from_vec(vec![0.5, -0.3])),
            ..Default::default()
        };
        let (theta0, _, _, _) = initialize(&config, &group, 0).unwrap();
        assert_eq!(theta0.as_slice(), &[0.5, -0.3]);
    }

    #[test]
    fn per_person_matrix_indexes_by_person_index() {
        let group = GroupParams::standard_normal(2);
        let config = InitialThetaConfig {
            start: ThetaStart::PerPersonMatrix(vec![
                ThetaVector::from_vec(vec![0.1, 0.2]),
                ThetaVector::from_vec(vec![-0.4, 0.9]),
            ]),
            ..Default::default()
        };
        let (theta0, _, _, _) = initialize(&config, &group, 1).unwrap();
        assert_eq!(theta0.as_slice(), &[-0.4, 0.9]);

        let err = initialize(&config, &group, 2).unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn se0_is_sqrt_of_gcov_diagonal_ignoring_off_diagonal() {
        let group = group_2d_non_diagonal();
        let config = InitialThetaConfig::default();
        let (_, se0, _, _) = initialize(&config, &group, 0).unwrap();
        assert_relative_eq!(se0[0], 1.2);
        assert_relative_eq!(se0[1], 1.5);
    }

    #[test]
    fn se0_unaffected_by_theta_override() {
        let group = group_2d_non_diagonal();
        let config = InitialThetaConfig {
            start: ThetaStart::Fixed(ThetaVector::from_vec(vec![0.5, -0.3])),
            ..Default::default()
        };
        let (_, se0, _, _) = initialize(&config, &group, 0).unwrap();
        assert_relative_eq!(se0[0], 1.2);
        assert_relative_eq!(se0[1], 1.5);
    }

    #[test]
    fn default_weights_are_ones() {
        let group = GroupParams::standard_normal(3);
        let config = InitialThetaConfig::default();
        let (_, _, weights, _) = initialize(&config, &group, 0).unwrap();
        assert_eq!(weights, vec![1.0, 1.0, 1.0]);
    }

    #[test]
    fn custom_weights_are_used_when_length_matches() {
        let group = GroupParams::standard_normal(2);
        let config = InitialThetaConfig {
            weights: Some(vec![0.5, 2.0]),
            ..Default::default()
        };
        let (_, _, weights, _) = initialize(&config, &group, 0).unwrap();
        assert_eq!(weights, vec![0.5, 2.0]);
    }

    #[test]
    fn fixed_theta_dimension_mismatch_errors() {
        let group = GroupParams::standard_normal(3);
        let config = InitialThetaConfig {
            start: ThetaStart::Fixed(ThetaVector::from_vec(vec![0.1, 0.2])),
            ..Default::default()
        };
        let err = initialize(&config, &group, 0).unwrap_err();
        assert_eq!(
            err,
            MirtCatError::DimensionMismatch {
                expected: 3,
                actual: 2,
                context: "initial_theta::initialize: ThetaStart::Fixed",
            }
        );
    }

    #[test]
    fn weights_dimension_mismatch_errors() {
        let group = GroupParams::standard_normal(3);
        let config = InitialThetaConfig {
            weights: Some(vec![1.0, 1.0]),
            ..Default::default()
        };
        let err = initialize(&config, &group, 0).unwrap_err();
        assert_eq!(
            err,
            MirtCatError::DimensionMismatch {
                expected: 3,
                actual: 2,
                context: "initial_theta::initialize: weights",
            }
        );
    }

    #[test]
    fn debug_false_yields_none_and_debug_true_yields_filled_debug_without_changing_main_values() {
        let group = group_2d_non_diagonal();
        let config_no_debug = InitialThetaConfig::default();
        let (theta_a, se_a, w_a, debug_a) = initialize(&config_no_debug, &group, 0).unwrap();
        assert!(debug_a.is_none());

        let config_debug = InitialThetaConfig {
            debug: true,
            ..Default::default()
        };
        let (theta_b, se_b, w_b, debug_b) = initialize(&config_debug, &group, 0).unwrap();

        assert_eq!(theta_a, theta_b);
        assert_eq!(se_a, se_b);
        assert_eq!(w_a, w_b);

        let debug = debug_b.expect("debug=true harus menghasilkan Some");
        assert_eq!(debug.source, ThetaStart::Zero);
        assert_eq!(debug.gmeans_used.as_slice(), &[0.0, 0.0]);
        assert_relative_eq!(debug.gcov_diag_used[0], 1.44);
        assert_relative_eq!(debug.gcov_diag_used[1], 2.25);
    }
}
