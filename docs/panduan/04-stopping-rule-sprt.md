# 04 — Klasifikasi `SPRT` (Sequential Probability Ratio Test)

## Quickstart

```rust
use mirtcat_engine::stopping_rule::{evaluate, ClassifyDecision, ClassifyType, StoppingConfig};
use mirtcat_engine::ThetaVector;

let config = StoppingConfig {
    min_items: 1,
    max_items: bank.len(),
    classify: ClassifyType::Sprt {
        lower: ThetaVector::from_vec(vec![-1.0, -1.0]), // hipotesis "kelompok bawah"
        upper: ThetaVector::from_vec(vec![1.0, 1.0]),   // hipotesis "kelompok atas"
        alpha: 0.05, // target tingkat kesalahan Tipe I
        beta: 0.05,  // target tingkat kesalahan Tipe II
    },
    debug: true,
    ..Default::default()
};
let (decision, debug) = evaluate(&config, &person, &bank, false, None)?;
let debug = debug.unwrap();
println!("LL ratio={:?} bounds={:?}", debug.sprt_log_likelihood_ratio, debug.sprt_bounds);
println!("keputusan: {:?}", decision.classification[0]); // panjang SELALU 1 -- lih. teori §2
```

## Kapan pakai

- Keputusan klasifikasi 2 KELOMPOK (bukan cutoff tunggal seperti `CI`) —
  "peserta ini lebih cocok hipotesis `lower` atau `upper`?" — dengan target
  tingkat kesalahan (`alpha`/`beta`) yang eksplisit dan dapat
  dipertanggungjawabkan secara statistik (Wald, 1945).
- Cocok untuk sertifikasi/lisensi dua-kelas (lulus/tidak) di mana kedua
  hipotesis punya titik referensi berbeda (bukan satu cutoff simetris).

## Memilih `lower`/`upper`/`alpha`/`beta`

`lower`/`upper` biasanya ditempatkan simetris di sekitar titik cutoff
konseptual (mis. `cutoff ± 1.0` pada skala `N(0,1)`) — jarak yang lebih
lebar antara keduanya membuat keputusan lebih cepat tercapai (perbedaan
likelihood lebih jelas), tapi kurang sensitif untuk peserta yang benar-benar
dekat cutoff. `alpha`/`beta` lebih kecil menuntut lebih banyak bukti
(item) sebelum keputusan diambil — trade-off yang sama seperti `Ci`.

**Peringatan penting**: SPRT bisa TIDAK PERNAH mencapai keputusan untuk pola
respons ambigu (`LL_diff` tetap di sekitar 0) — SELALU pasangkan dengan
`max_items` yang wajar sebagai jaring pengaman (lih.
`examples/04_stopping_rule_sprt.rs` skenario "ambigu").

## Membaca hasil

```rust
if decision.stop_now {
    if decision.terminated_early {
        println!("keputusan SPRT tercapai: {:?}", decision.classification[0]);
    } else {
        println!("berhenti tanpa keputusan SPRT (max_items/max_time tercapai)");
    }
}
```

`decision.classification` untuk mode `Sprt` SELALU berpanjang **1**
(keputusan gabungan untuk seluruh vektor `theta`, bukan per-dimensi) —
beda dari mode `Ci` yang berpanjang `nfact`. Lih.
`docs/teori/04-stopping-rule-sprt.md` §2.

Contoh lengkap (3 skenario: konsisten `upper`, konsisten `lower`, ambigu):
`cargo run --example 04_stopping_rule_sprt`.
