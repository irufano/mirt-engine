# 03 — `Map`: Maximum A Posteriori (Default)

## Quickstart

```rust
use mirt_engine::ability_estimation::estimate;
use mirt_engine::EstimationConfig;

let config = EstimationConfig::default(); // Method::Map -- ini default-nya
let ((theta, se), _) = estimate(&config, &person, &bank, &group)?;
```

`Map` adalah default `EstimationConfig::method` — tidak perlu diset
eksplisit kecuali beralih dari method lain kembali ke `Map`.

## Kapan pakai

- **Default umum** — robust untuk sebagian besar skenario CAT, terutama di
  awal tes (item sedikit, risiko pola respons ekstrem tinggi) karena selalu
  memberi estimasi finite (lih. `docs/teori/03-ability-estimation-map.md`
  §4).
- Prior (`GroupParams::gmeans`/`gcov`) tersedia dari kalibrasi populasi
  sesungguhnya (bukan cuma `standard_normal` placeholder) — makin akurat
  prior, makin bermanfaat `Map` dibanding `Ml`/`Wle`.

## Prior: `GroupParams`

```rust
use mirt_engine::theta::InfoMatrix;
use mirt_engine::{GroupParams, ThetaVector};

// Prior standar N(0, I) -- dipakai kalau tidak ada info populasi lain
let group = GroupParams::standard_normal(nfact);

// Prior kustom (mis. hasil kalibrasi populasi sesungguhnya)
let gmeans = ThetaVector::from_vec(vec![0.2, -0.1]);
let gcov = InfoMatrix::identity(2); // atau matriks kovarians terkalibrasi
let group = GroupParams::new(gmeans, gcov)?;
```

`gcov` yang lebih SEMPIT (varians kecil) membuat `Map` lebih "percaya diri"
pada `gmeans` sebagai titik awal, menarik `theta_hat` lebih dekat ke sana;
`gcov` LEBAR membuat `Map` mendekati `Ml` murni (data mendominasi). Lih.
`docs/teori/03-ability-estimation-map.md` §3.

## Debug trace: konvergensi optimizer

```rust
let config = EstimationConfig { debug: true, ..Default::default() };
let (_, debug) = estimate(&config, &person, &bank, &group)?;
let debug = debug.unwrap();
println!("iterasi: {:?}, konvergen: {:?}", debug.iterations, debug.converged);
println!("jejak log-posterior tiap iterasi: {:?}", debug.log_likelihood_trace);
```

`converged == Some(false)` (jarang terjadi untuk objektif MAP yang cekung)
mengindikasikan optimizer mencapai batas iterasi maksimum tanpa gradien
mendekati nol — pertimbangkan memeriksa `theta`/SE hasil dengan lebih hati-hati
bila ini terjadi.

Contoh lengkap (efek prior sempit vs lebar): `cargo run --example
03_ability_estimation_map`.
