# 04 — `min_SEM`: Stopping Rule Berbasis Standard Error

## Quickstart

```rust
use mirt_engine::stopping_rule::{evaluate, StoppingConfig};

let config = StoppingConfig {
    min_items: 5,
    max_items: bank.len(),
    min_sem: vec![0.3, 0.3], // ambang per-dimensi, atau vec![0.3] untuk broadcast
    ..Default::default()
};
let (decision, _) = evaluate(&config, &person, &bank, false, None)?;
println!("met_SEM per-dimensi: {:?}", decision.met_sem);
```

`ClassifyType::None` (default `StoppingConfig::classify`) — tidak perlu
diset eksplisit kecuali beralih dari mode `Ci`/`Sprt` kembali ke `min_SEM`.

## Kapan pakai

- **Default umum** — tidak butuh cutoff/hipotesis klasifikasi eksplisit,
  hanya ingin presisi estimasi seragam per-peserta.
- `nfact` rendah-sedang (1-3) — pada `nfact` tinggi, lih. peringatan di
  `docs/panduan/04-stopping-rule-overview.md` §"nfact tinggi".

## Memilih ambang `min_sem`

Ambang lebih kecil = presisi lebih tinggi tapi tes lebih panjang (dan
risiko `max_items` tercapai lebih dulu, lih.
`docs/teori/04-stopping-rule-overview.md` §3). Default `0.3` (setara SE
mirtCAT) adalah titik awal yang wajar untuk model dikotomus standar; naikkan
(misal `0.4`-`0.5`) untuk tes yang perlu lebih pendek, turunkan (`0.15`-`0.2`)
untuk keputusan berisiko tinggi yang butuh presisi ekstra.

`min_sem: vec![0.3]` (panjang 1) di-broadcast ke seluruh `nfact` dimensi
otomatis. Berikan `vec![0.2, 0.4, 0.3]` (persis `nfact`) untuk ambang
BERBEDA per dimensi (mis. satu subskala lebih penting daripada yang lain).

## Membaca `decision.met_sem`

```rust
for (i, &met) in decision.met_sem.iter().enumerate() {
    println!("dimensi {i}: {}", if met { "presisi tercapai" } else { "belum" });
}
```

Ingat: `decision.stop_now` hanya `true` bila **SEMUA** elemen `met_sem`
`true` (lih. `docs/teori/04-stopping-rule-min-sem.md` §1) — satu `false`
di antara sekian banyak `true` tetap membuat sesi lanjut. `met_sem` hanya
diisi berarti saat `config.classify == ClassifyType::None`; untuk mode
`Ci`/`Sprt`, nilainya tetap `vec![false; nfact]` (tidak dipakai).

Contoh lengkap (dim1 konvergen cepat, dim2 lambat -- perhatikan tes tidak
berhenti sampai keduanya siap): `cargo run --example
04_stopping_rule_min_sem`.
