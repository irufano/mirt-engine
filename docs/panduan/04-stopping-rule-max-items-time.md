# 04 — `max_items`/`max_time`: Batas Keras Independen

## Quickstart

```rust
use std::time::Duration;
use mirt_engine::stopping_rule::{evaluate, StoppingConfig};

let config = StoppingConfig {
    max_items: 30,
    max_time: Some(Duration::from_secs(60 * 20)), // 20 menit
    ..Default::default()
};

// Setelah tiap item, catat waktu yang dihabiskan (opsional -- tanpa ini,
// max_time tidak akan pernah tercapai):
person.record_item_time(Duration::from_secs(15));

let (decision, _) = evaluate(&config, &person, &bank, false, None)?;
```

## Kapan pakai

- **Selalu** — pasangkan `max_items` (dan `max_time` bila relevan) dengan
  kriteria presisi apa pun (`min_SEM`/`Ci`/`Sprt`) sebagai jaring pengaman,
  bahkan bila diharapkan jarang terpicu. Tanpa ini, pola respons yang tidak
  terduga (mis. SPRT ambigu, lih. `docs/panduan/04-stopping-rule-sprt.md`)
  bisa membuat sesi berjalan tanpa batas.
- **Wajib direncanakan sebagai kondisi berhenti UTAMA** (bukan cadangan)
  untuk model `nfact` tinggi (>4-5) — lih.
  `docs/panduan/04-stopping-rule-overview.md` §"nfact tinggi".
- `max_time` untuk sesi dengan batas waktu operasional (ujian terjadwal).

## Menentukan `max_items` yang realistis

Untuk `nfact` rendah (1-3) dengan item bank yang cukup besar & informatif,
`max_items` bisa diset longgar (presisi biasanya tercapai lebih dulu).
Untuk `nfact` tinggi, JALANKAN simulasi dulu (mis. adaptasi
`examples/04_stopping_rule_max_items_time.rs`) untuk melihat SE yang
REALISTIS dicapai dalam jumlah item yang tersedia — jangan menebak
`min_SEM` sembarangan lalu berharap `max_items` besar akan "menyelesaikan"
masalah presisi yang secara statistik memang sulit dipenuhi (lih.
`docs/teori/04-stopping-rule-overview.md` §3).

## `max_time`: mengaktifkan pelacakan waktu

`PersonState::record_item_time(Duration)` bersifat OPSIONAL — bila tidak
pernah dipanggil, `total_time()` selalu nol dan `max_time` efektif
tidak aktif (aman, tidak mengubah perilaku peserta lain yang tidak
memerlukannya):

```rust
let start = std::time::Instant::now();
// ... peserta menjawab item ...
person.record_answer(item_idx, response);
person.record_item_time(start.elapsed());
```

## Membedakan alasan berhenti

```rust
if decision.stop_now && !decision.terminated_early {
    // berhenti karena max_items ATAU max_time ATAU CustomStop --
    // bandingkan person.n_answered() vs config.max_items,
    // dan person.total_time() vs config.max_time, untuk tahu mana persisnya
    println!("n_answered={} / max_items={}", person.n_answered(), config.max_items);
    println!("total_time={:?} / max_time={:?}", person.total_time(), config.max_time);
}
```

Contoh lengkap (dua skenario: `max_items` sebagai batas praktis `nfact=6`,
dan `max_time` menghentikan sesi walau presisi nyaris tercapai): `cargo run
--example 04_stopping_rule_max_items_time`.
