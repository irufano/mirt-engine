# 04 — `delta_thetas`: Stopping Rule Berbasis Stabilitas `theta`

## Quickstart

```rust
use mirt_engine::stopping_rule::{evaluate, StoppingConfig};

let config = StoppingConfig {
    min_items: 5,
    max_items: bank.len(),
    delta_thetas: vec![0.02, 0.02], // stop bila |Δtheta| < 0.02 di SEMUA dimensi
    ..Default::default()
};
let (decision, debug) = evaluate(&config, &person, &bank, true, None)?;
```

**Penting**: `delta_thetas` default `vec![0.0]` (nonaktif) — harus diset
eksplisit ke ambang positif untuk mengaktifkan kriteria ini.

## Kapan pakai

- Sebagai kriteria **tambahan** di samping `min_SEM`/`Ci`/`Sprt` (bukan
  pengganti) — `delta_thetas` selalu dievaluasi independen dari
  `config.classify` (lih. `docs/teori/04-stopping-rule-delta-thetas.md`
  §2), jadi bisa dikombinasikan bebas dengan mode klasifikasi apa pun.
- Berguna sebagai deteksi "theta sudah konvergen secara praktis" walau SE
  formal belum di bawah ambang `min_SEM` — kadang sesi bisa berhenti lebih
  cepat lewat jalur ini.

## Memilih ambang

Ambang terlalu kecil (mis. `0.001`) nyaris tidak pernah terpenuhi (theta
selalu bergerak sedikit tiap item baru); terlalu besar (mis. `0.5`)
berhenti prematur walau estimasi masih jauh dari stabil. Titik awal wajar:
`0.01`-`0.05` untuk skala theta standar (`N(0,1)`).

## Membaca `debug.delta_theta_values`

```rust
let config = StoppingConfig { debug: true, ..config };
let (decision, debug) = evaluate(&config, &person, &bank, false, None)?;
let debug = debug.unwrap();
if debug.delta_theta_values.is_empty() {
    println!("belum ada previous_theta -- ini estimasi ulang pertama");
} else {
    println!("|Δtheta| mentah: {:?}", debug.delta_theta_values);
}
```

`decision.met_delta_thetas` seluruhnya `false` sampai ADA `previous_theta`
tercatat (`PersonState::previous_theta()`, diisi otomatis oleh
`set_theta`/`set_estimate`) — pada langkah pertama re-estimasi, kriteria
ini belum bisa dievaluasi sama sekali (bukan "kebetulan langsung stabil").

Contoh lengkap (dim2 berosilasi beberapa langkah sebelum menetap --
perhatikan `met_delta_thetas` per-dimensi selama proses itu): `cargo run
--example 04_stopping_rule_delta_thetas`.
