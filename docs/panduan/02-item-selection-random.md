# 02 — `random`: Baseline Exposure-Neutral

## Quickstart

```rust
use mirt_engine::{select_next_item, Criterion, SelectionConfig};
use rand::rngs::StdRng;
use rand::SeedableRng;

let config = SelectionConfig {
    criterion: Criterion::Random,
    ..Default::default()
};
let mut rng = StdRng::seed_from_u64(42); // seed tetap -> hasil reproducible
```

## Kenapa `select_next_item()` butuh parameter `rng`

Beda dari sketsa awal `PLAN.md`, `select_next_item()` menerima
`rng: &mut impl rand::Rng` secara eksplisit (bukan memakai
`rand::thread_rng()` internal) — supaya `Criterion::Random` bisa diuji
deterministik dan dipakai dengan seed tetap di simulasi (mis. Monte Carlo
yang harus reproducible). Kriteria lain mengabaikan parameter ini
sepenuhnya (tidak ada biaya untuk menyediakannya).

Untuk penggunaan produksi non-deterministik, pakai `rand::thread_rng()`:

```rust
use rand::thread_rng;
let mut rng = thread_rng();
```

## Kapan dipakai

- Baseline exposure-neutral (lih. dokumen teori).
- Kontrol eksperimen: membandingkan hasil belajar/kelelahan peserta
  terhadap urutan item acak vs adaptif.
- Sebagai fallback ketika `nfact`/parameter item belum lengkap dan kriteria
  informasi belum bisa dievaluasi.

## Reproducibility

Seed yang sama pada `StdRng` menghasilkan urutan pilihan yang **sama
persis** untuk sesi/simulasi yang identik (bank & kandidat sama) —
diverifikasi di `examples/02_item_selection_random.rs` dan unit test
`item_selection::criteria::random::tests::same_seed_is_reproducible`.

Contoh lengkap: `cargo run --example 02_item_selection_random`.
