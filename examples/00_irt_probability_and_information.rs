//! Contoh Fase 0: mendefinisikan beberapa item 2D/3D (dikotomus & graded
//! response), lalu mencetak probabilitas kategori (peluang benar/salah peserta dengan kemampuan theta tertentu) dan
//! matriks informasi Fisher (seberapa presisi/informatif item itu dalam menentukan kemampuan peserta di titik theta)
//! pada beberapa titik `theta`.
//!
//! Jalankan dengan:
//! ```sh
//! cargo run --example 00_irt_probability_and_information
//! ```

use mirt_engine::irt::{Dichotomous, GradedResponse, ItemResponseModel};
use mirt_engine::theta::ThetaVector;

fn print_item_at(name: &str, item: &dyn ItemResponseModel, theta: &ThetaVector) {
    let probs = item
        .probabilities(theta)
        .expect("theta valid untuk item ini");
    let info = item.information(theta).expect("theta valid untuk item ini");
    println!("  theta = {:?}", theta.as_slice());
    print!("    P(kategori) = [");
    for (i, p) in probs.iter().enumerate() {
        if i > 0 {
            print!(", ");
        }
        print!("{:.4}", p);
    }
    println!("]");
    println!("    info matrix ({0}x{0}):", item.nfact());
    for i in 0..item.nfact() {
        print!("      [");
        for j in 0..item.nfact() {
            if j > 0 {
                print!(", ");
            }
            print!("{:8.4}", info.get(i, j));
        }
        println!("]");
    }
    let _ = name;
}

fn main() {
    println!("== Item dikotomus 2D (M2PL) ==");
    let m2pl = Dichotomous::m2pl(vec![1.2, 0.8], 0.3).expect("parameter valid");
    for t in [
        ThetaVector::from_vec(vec![0.0, 0.0]),
        ThetaVector::from_vec(vec![1.0, -1.0]),
        ThetaVector::from_vec(vec![2.0, 2.0]),
    ] {
        print_item_at("m2pl_2d", &m2pl, &t);
    }

    println!("\n== Item dikotomus 2D (M3PL, guessing=0.2) ==");
    let m3pl = Dichotomous::m3pl(vec![1.0, 1.0], -0.5, 0.2).expect("parameter valid");
    print_item_at("m3pl_2d", &m3pl, &ThetaVector::from_vec(vec![0.0, 0.0]));

    println!("\n== Item graded response 2D (4 kategori) ==");
    let graded =
        GradedResponse::new(vec![1.1, 0.6], vec![1.5, 0.2, -1.2]).expect("boundary terurut");
    for t in [
        ThetaVector::from_vec(vec![-2.0, -2.0]),
        ThetaVector::from_vec(vec![0.0, 0.0]),
        ThetaVector::from_vec(vec![2.0, 2.0]),
    ] {
        print_item_at("graded_2d", &graded, &t);
    }

    println!("\n== Item dikotomus 3D (M2PL) ==");
    let m2pl_3d = Dichotomous::m2pl(vec![0.9, 0.7, 1.1], 0.0).expect("parameter valid");
    print_item_at(
        "m2pl_3d",
        &m2pl_3d,
        &ThetaVector::from_vec(vec![0.5, -0.5, 1.0]),
    );
}
