# 04 — Stopping Rule: Overview

## Konsep dasar

```rust
use mirt_engine::stopping_rule::{evaluate, StoppingConfig};

// person: PersonState sudah diupdate (record_answer + set_estimate) untuk langkah ini
let config = StoppingConfig {
    min_items: 5,
    max_items: bank.len(),
    min_sem: vec![0.3], // broadcast ke seluruh nfact
    ..Default::default()
};
let (decision, _debug) = evaluate(&config, &person, &bank, false, None)?;
if decision.stop_now {
    // sesi selesai -- lih. decision.terminated_early untuk membedakan
    // "berhenti karena presisi" vs "berhenti karena max_items/max_time"
}
```

Panggil `evaluate()` setelah SETIAP item dijawab & `theta`/SE diestimasi
ulang (Fase 3) — persis siklus `mirtCAT`: administer item -> re-estimasi
theta -> cek stopping rule -> ulangi atau berhenti.

## Memilih kombinasi kriteria

| Kebutuhan | Konfigurasi |
|---|---|
| Presisi seragam per-peserta, `nfact` rendah (1-3) | `min_sem: vec![0.3]` (default), biarkan `classify: ClassifyType::None` |
| Keputusan lulus/tidak-lulus relatif satu cutoff tetap | `classify: ClassifyType::Ci { cutoffs, alpha }` |
| Keputusan klasifikasi 2 kelompok dengan target tingkat kesalahan eksplisit | `classify: ClassifyType::Sprt { lower, upper, alpha, beta }` |
| `nfact` tinggi (>4-5) | **Jangan andalkan presisi murni** — set `max_items` realistis; lih. §"nfact tinggi" di bawah & `docs/teori/04-stopping-rule-overview.md` §3 |
| Butuh batas waktu keras (mis. sesi ujian terjadwal) | `max_time: Some(Duration::from_secs(...))` |
| Ada logika berhenti kustom di luar kriteria bawaan | `custom_stop: Some(&closure)` |

`min_items`/`max_items` **selalu** aktif bersamaan dengan mode klasifikasi
apa pun (`None`/`Ci`/`Sprt`) — keduanya bukan alternatif, melainkan lapisan
independen (lih. `docs/teori/04-stopping-rule-overview.md` §1).

## `nfact` tinggi: `max_items` sebagai kondisi berhenti praktis

Pada model dimensi tinggi, semantik "all dimensi" (lih.
`docs/teori/04-stopping-rule-overview.md` §2-3) membuat `min_SEM`/
`delta_thetas`/`Ci` sulit — kadang mustahil secara praktis — terpenuhi
sebelum item bank habis. **Jangan** menganggap ini kegagalan konfigurasi:
untuk `nfact > 4-5`, rencanakan `max_items` sebagai kondisi berhenti UTAMA
yang diharapkan, bukan cadangan. Cek `decision.terminated_early` untuk tahu
mana yang sungguh terjadi:

```rust
if decision.stop_now {
    if decision.terminated_early {
        println!("berhenti karena presisi tercapai");
    } else {
        println!("berhenti karena max_items/max_time (atau customStop)");
    }
}
```

Lih. `examples/04_stopping_rule_max_items_time.rs` (skenario `nfact=6`)
untuk demonstrasi langsung.

## Hook `CustomStop`

```rust
use mirt_engine::stopping_rule::CustomStop;

let jangan_pernah_berhenti_sebelum_10_item = |person: &mirt_engine::PersonState| {
    person.n_answered() >= 10 && /* kondisi kustom lain */ true
};
let (decision, _) = evaluate(&config, &person, &bank, false, Some(&jangan_pernah_berhenti_sebelum_10_item))?;
```

**Penting**: hasil `CustomStop` MENIMPA TOTAL `decision.stop_now`, bukan
di-OR-kan dengan hasil kriteria lain — closure di atas bisa membatalkan
keputusan berhenti dari `max_items` sekalipun. Lih.
`docs/teori/04-stopping-rule-overview.md` §6 untuk detail & rasional.

## Guard `allow_constrain_breaks`

`false` (default) mencegah berhenti di tengah blok item constraint yang
belum tuntas (Fase 6, belum diimplementasikan). Sebelum Fase 6 ada,
parameter `in_unfinished_constraint_block` di `evaluate()` selalu diisi
`false` oleh pemanggil (tidak ada constraint untuk dicek) — begitu Fase 6
ada, isi dengan status constraint sungguhan dari modul tersebut.

## Debug trace

```rust
let config = StoppingConfig { debug: true, ..config };
let (decision, debug) = evaluate(&config, &person, &bank, false, None)?;
let debug = debug.unwrap();
println!("SE saat ini: {:?}", debug.se_values.as_slice());
println!("|Δtheta|: {:?}", debug.delta_theta_values); // kosong bila belum ada previous_theta
println!("SPRT LL ratio & bounds: {:?} {:?}", debug.sprt_log_likelihood_ratio, debug.sprt_bounds);
```

Field SPRT `None` untuk mode `None`/`Ci`. Mengaktifkan debug **tidak**
mengubah `StopDecision` yang dikembalikan (prinsip desain #7, `PLAN.md`).

## Error

`Err(MirtCatError)` terjadi bila: panjang `min_sem`/`delta_thetas` bukan
`1` (broadcast) atau `nfact`, panjang `Ci::cutoffs`/`Sprt::lower`/
`Sprt::upper` bukan persis `nfact`, atau index item terjawab di luar
jangkauan `bank` (mode `Sprt`, yang butuh menghitung likelihood atas item
terjawab) — tidak pernah panic.

## Contoh lengkap per mekanisme

```sh
cargo run --example 04_stopping_rule_min_sem
cargo run --example 04_stopping_rule_delta_thetas
cargo run --example 04_stopping_rule_ci
cargo run --example 04_stopping_rule_sprt
cargo run --example 04_stopping_rule_max_items_time
```

Panduan rinci per mekanisme: `docs/panduan/04-stopping-rule-{min-sem,
delta-thetas,ci,sprt,max-items-time}.md`.
