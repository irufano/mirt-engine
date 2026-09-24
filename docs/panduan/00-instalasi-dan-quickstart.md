# 00 — Instalasi & Quickstart

## Instalasi

`mirt-engine` belum dipublikasikan ke crates.io (masih Fase 0 — lih.
`PLAN.md`). Untuk memakainya dari project Rust lain di mesin yang sama,
tambahkan sebagai path/git dependency di `Cargo.toml`:

```toml
[dependencies]
mirt-engine = { path = "../mirt-engine" }
# atau, setelah dipublikasikan:
# mirt-engine = "0.1"
```

Dependency inti: [`nalgebra`](https://crates.io/crates/nalgebra) (aljabar
linear), [`rand`](https://crates.io/crates/rand), dan
[`thiserror`](https://crates.io/crates/thiserror). Tidak ada dependency ke
framework web apa pun — `mirt-engine` murni logic, aman dipakai di Axum,
Actix, atau context non-web (simulasi batch, CLI).

## Konsep dasar

Tiga tipe inti (lih. `docs/teori/00-fondasi-irt-multidimensional.md` untuk
landasan teorinya):

- **`ThetaVector`** — vektor kemampuan multidimensional peserta.
- **`InfoMatrix`** — matriks informasi Fisher (atau kovarians) `nfact × nfact`.
- **`ItemResponseModel`** (trait) — dihitung untuk setiap item: probabilitas
  kategori & informasi Fisher pada satu titik `theta`. Ada dua implementasi
  siap pakai:
  - **`Dichotomous`** — item 2 kategori (M2PL/M3PL/M4PL).
  - **`GradedResponse`** — item politomus terurut (graded response).

## Quickstart: mendefinisikan item & menghitung probabilitas/informasi

```rust
use mirt_engine::irt::{Dichotomous, GradedResponse, ItemResponseModel};
use mirt_engine::theta::ThetaVector;

fn main() {
    // Item M2PL 2 dimensi: slope a=(1.2, 0.8), intercept d=0.3.
    let item = Dichotomous::m2pl(vec![1.2, 0.8], 0.3)
        .expect("parameter valid"); // error bila `a` kosong / asymptote tak valid

    let theta = ThetaVector::from_vec(vec![0.5, -0.2]);

    let p = item.probabilities(&theta).unwrap(); // [P(u=0), P(u=1)]
    let info = item.information(&theta).unwrap(); // matriks 2x2

    println!("P(u=1|theta) = {:.4}", p[1]);
    println!("info[0][0]   = {:.4}", info.get(0, 0));

    // Item graded response 2 dimensi, 4 kategori (3 boundary, harus menurun).
    let graded = GradedResponse::new(vec![1.1, 0.6], vec![1.5, 0.2, -1.2])
        .expect("boundary harus terurut menurun");
    let p_graded = graded.probabilities(&theta).unwrap();
    println!("P(kategori) graded = {:?}", p_graded);
}
```

Jalankan contoh lengkap (item 2D/3D, beberapa titik `theta`):

```sh
cargo run --example 00_irt_probability_and_information
```

## Item bank & parameter populasi

Untuk mengelola banyak item sekaligus, pakai `ItemBank` (memvalidasi
seluruh item punya `nfact` yang sama) dan `GroupParams` (rata-rata & 
kovarians populasi, sumber theta awal di Fase 1):

```rust
use mirt_engine::{GroupParams, ItemBank, ItemMeta};
use mirt_engine::irt::Dichotomous;
use mirt_engine::theta::{InfoMatrix, ThetaVector};

let mut bank = ItemBank::new();
bank.push(
    Box::new(Dichotomous::m2pl(vec![1.2, 0.8], 0.3).unwrap()),
    ItemMeta::with_content(0, "aljabar"),
).unwrap();

let group = GroupParams::new(
    ThetaVector::zeros(2),
    InfoMatrix::identity(2),
).unwrap(); // atau GroupParams::standard_normal(2)
```

## Error handling

Semua batasan yang di `mirtCAT`/`mirt` berupa `stop(...)` diterjemahkan
jadi `Result<_, MirtCatError>` di `mirt-engine` — dicek saat konstruksi
(mis. `u <= g`, boundary graded tidak terurut, dimensi `theta` tidak cocok
`nfact` item), bukan `panic!`. Selalu tangani `Result` yang dikembalikan
konstruktor/`probabilities()`/`information()`.

## Langkah selanjutnya

Fase 0 hanya menyediakan fondasi model IRT (probabilitas & informasi per
item). Siklus adaptif penuh (theta awal, seleksi item, estimasi kemampuan,
stopping rule, exposure control, content balancing, hasil akhir) dibangun
di fase-fase berikutnya — lih. `PLAN.md` untuk peta lengkap dan status
progres tiap fase.
