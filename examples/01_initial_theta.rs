//! Contoh Fase 1: inisialisasi theta awal (`theta₀`), SE awal (`SE₀`), dan
//! bobot per-dimensi (`weights`) untuk satu peserta — default (theta nol,
//! SE dari `sqrt(diag(gcov))`) vs override manual.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 01_initial_theta
//! ```

use mirt_engine::initial_theta::{initialize, InitialThetaConfig, ThetaStart};
use mirt_engine::theta::InfoMatrix;
use mirt_engine::{GroupParams, ThetaVector};
use nalgebra::DMatrix;

fn print_result(
    label: &str,
    theta0: &ThetaVector,
    se0: &mirt_engine::SEVector,
    weights: &[f64],
) {
    println!("  {label}");
    println!("    theta0  = {:?}", theta0.as_slice());
    println!("    SE0     = {:?}", se0.as_slice());
    println!("    weights = {:?}", weights);
}

fn main() {
    // Model 3D terkalibrasi dengan gcov populasi non-identitas (mis. hasil
    // kalibrasi mirt sungguhan), gmeans 0 seperti biasa.
    let gcov = InfoMatrix::from_matrix(DMatrix::from_row_slice(
        3,
        3,
        &[
            1.00, 0.10, -0.05, //
            0.10, 1.96, 0.20, //
            -0.05, 0.20, 0.81, //
        ],
    ))
    .expect("gcov persegi");
    let group = GroupParams::new(ThetaVector::zeros(3), gcov).expect("dimensi gmeans/gcov cocok");

    println!("== Default (tanpa override) ==");
    let config_default = InitialThetaConfig::default();
    let (theta0, se0, weights, _) =
        initialize(&config_default, &group, 0).expect("konfigurasi default selalu valid");
    print_result(
        "theta0 = nol, SE0 = sqrt(diag(gcov))",
        &theta0,
        &se0,
        &weights,
    );

    println!("\n== Override manual (Fixed) ==");
    let config_fixed = InitialThetaConfig {
        start: ThetaStart::Fixed(ThetaVector::from_vec(vec![-0.4, 0.8, 0.1])),
        weights: Some(vec![1.0, 0.5, 2.0]),
        debug: false,
    };
    let (theta0, se0, weights, _) =
        initialize(&config_fixed, &group, 0).expect("dimensi override cocok dengan nfact");
    print_result(
        "theta0 = override, SE0 tetap dari gcov",
        &theta0,
        &se0,
        &weights,
    );

    println!("\n== Override per-peserta (simulasi offline 2 peserta) ==");
    let config_multi = InitialThetaConfig {
        start: ThetaStart::PerPersonMatrix(vec![
            ThetaVector::from_vec(vec![0.2, -0.1, 0.0]),
            ThetaVector::from_vec(vec![-0.6, 0.3, 0.4]),
        ]),
        ..Default::default()
    };
    for person_index in 0..2 {
        let (theta0, se0, weights, _) = initialize(&config_multi, &group, person_index)
            .expect("person_index dalam jangkauan PerPersonMatrix");
        print_result(&format!("peserta #{person_index}"), &theta0, &se0, &weights);
    }

    println!("\n== Dengan debug trace ==");
    let config_debug = InitialThetaConfig {
        debug: true,
        ..Default::default()
    };
    let (theta0, se0, weights, debug) = initialize(&config_debug, &group, 0).unwrap();
    let debug = debug.expect("debug=true menghasilkan Some");
    print_result("theta0 = nol (Zero)", &theta0, &se0, &weights);
    println!("  source          = {:?}", debug.source);
    println!("  gmeans_used     = {:?}", debug.gmeans_used.as_slice());
    println!("  gcov_diag_used  = {:?}", debug.gcov_diag_used);

    println!("\n== Dengan debug trace + theta tertentu (Fixed) ==");
    let config_debug_fixed = InitialThetaConfig {
        start: ThetaStart::Fixed(ThetaVector::from_vec(vec![1.0, -0.5, 0.25])),
        debug: true,
        ..Default::default()
    };
    let (theta0, se0, weights, debug) =
        initialize(&config_debug_fixed, &group, 0).expect("dimensi override cocok dengan nfact");
    let debug = debug.expect("debug=true menghasilkan Some");
    print_result("theta0 = override (Fixed)", &theta0, &se0, &weights);
    println!("  source          = {:?}", debug.source);
    println!("  gmeans_used     = {:?}", debug.gmeans_used.as_slice());
    println!("  gcov_diag_used  = {:?}", debug.gcov_diag_used);
}
