# 03 — Estimasi Kemampuan (Ability Estimation): Overview

## Konsep dasar

`ability_estimation::estimate()` mengestimasi ulang `(theta, SE)` peserta
**seluruh vektor sekaligus** (bukan dimensi-per-dimensi) berdasar
`EstimationConfig`. Lih. `docs/teori/03-ability-estimation-overview.md`
untuk landasan & tabel `quadpts` default.

```rust
use mirt_engine::ability_estimation::estimate;
use mirt_engine::{EstimationConfig, GroupParams, PersonState};

// `bank: ItemBank` sudah ada (lih. docs/panduan/00-instalasi-dan-quickstart.md)
let group = GroupParams::standard_normal(bank.nfact().unwrap());
let mut person = PersonState::new(bank.nfact().unwrap());
person.record_answer(0, 1); // item index 0, kategori respons 1 (mis. "benar")

let config = EstimationConfig::default(); // Method::Map
let ((theta, se), _debug) = estimate(&config, &person, &bank, &group)?;
```

## Memilih `Method`

| Situasi | Method yang masuk akal |
|---|---|
| Default umum, robust terhadap respons ekstrem | `Method::Map` (default) |
| Butuh estimasi tanpa asumsi prior (mis. riset/kalibrasi) | `Method::Ml` — **hati-hati**, bisa divergen (lih. panduan ML) |
| Awal tes (item sedikit), ingin lebih robust dari ML tapi tanpa prior eksplisit | `Method::Wle` |
| Butuh interval kepercayaan Bayesian penuh (bukan cuma titik optimum) | `Method::Eap { qmc: false }` (atau `qmc: true` untuk `nfact > 4`) |
| Pelaporan berbasis skor mentah (tabel skor->kemampuan) | `Method::EapSum` |
| Kuesioner non-adaptif (urutan tetap, tidak re-estimasi per-item) | `Method::Fixed` |

## `PersonState`: theta, SE & respons terjawab

Fase 3 memperluas `PersonState` (Fase 2) dengan SE & kategori respons —
`record_answer` sekarang butuh **kategori respons**, bukan cuma index item
(diperlukan untuk menghitung likelihood):

```rust
use mirt_engine::PersonState;

let mut person = PersonState::new(3); // theta & SE nol (placeholder mentah)
// atau, hasil Fase 1 (initial_theta::initialize):
// let (theta0, se0, _, _) = initial_theta::initialize(&config, &group, 0)?;
// let mut person = PersonState::from_theta_and_se(theta0, se0);

person.record_answer(item_index, response_category); // catat item + kategori respons
let ((theta, se), _) = estimate(&config, &person, &bank, &group)?;
person.set_estimate(theta, se); // simpan hasil untuk langkah berikutnya
```

`response_category` adalah index kategori 0-based (mis. `0`/`1` untuk item
dikotomus — `0` = kategori pertama/"salah", `1` = kategori kedua/"benar",
mengikuti urutan `ItemResponseModel::probabilities()`).

## `quadpts`: kapan perlu diatur manual

`EstimationConfig::quadpts` (dipakai `Eap`/`EapSum` saja) default ke tabel
`nfact` (`quadrature::default_quadpts`) — cukup untuk kebanyakan kasus.
Naikkan manual saat:

- Butuh presisi lebih di `nfact` rendah (mis. laporan akhir, bukan langkah
  adaptif per-item — biaya komputasi lebih longgar).
- Memakai `Eap { qmc: true }` — QMC **tidak** otomatis dinaikkan jumlah
  titiknya dari default (sama seperti `mirtCAT`), jadi manfaat penuhnya
  baru terasa setelah `quadpts` dinaikkan manual (lih.
  `docs/panduan/03-ability-estimation-eap.md`).

## Debug trace

```rust
let config = EstimationConfig { debug: true, ..config };
let ((theta, se), debug) = estimate(&config, &person, &bank, &group)?;
let debug = debug.unwrap();
println!("method: {:?}", debug.method);
println!("n_quadrature_points: {:?}", debug.n_quadrature_points); // Some untuk Eap/EapSum
println!("iterations: {:?}", debug.iterations);                   // Some untuk Map/Ml/Wle
println!("converged: {:?}", debug.converged);
```

Field bernilai `None` untuk method yang tidak relevan (mis.
`n_quadrature_points` untuk `Map`/`Ml`/`Wle`/`Fixed`, `iterations` untuk
`Eap`/`EapSum`/`Fixed`). Mengaktifkan debug **tidak** mengubah `(theta,
SE)` yang dikembalikan (prinsip desain #7, `PLAN.md`).

## Error

`Err(MirtCatError)` terjadi bila: ada index item terjawab di luar `bank`,
kategori respons di luar jangkauan kategori item terkait, atau `quadpts`
tidak valid (`Eap`/`EapSum`, mis. `< 2` untuk grid) — tidak pernah panic.

## Contoh lengkap per method

```sh
cargo run --example 03_ability_estimation_eap
cargo run --example 03_ability_estimation_map
cargo run --example 03_ability_estimation_ml
cargo run --example 03_ability_estimation_wle
cargo run --example 03_ability_estimation_eap_sum
cargo run --example 03_ability_estimation_fixed
```

Panduan rinci per method: `docs/panduan/03-ability-estimation-{eap,map,ml,
wle,eap-sum,fixed}.md`.
