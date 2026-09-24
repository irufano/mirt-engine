# 06 — Content Balancing

## Kapan pakai

Content balancing relevan bila item bank punya beberapa kategori konten
(mis. mata pelajaran/subtopik) dan tes perlu menjaga proporsi kategori yang
disajikan mendekati target tertentu — mis. "50% item Aljabar, 50% item
Geometri" — alih-alih murni mengikuti kriteria seleksi item (`Drule`, dst.)
yang bisa saja condong ke satu kategori saja bila kategori itu kebetulan
lebih informatif untuk kandidat tertentu.

## Mendefinisikan kategori & target proporsi

```rust
use mirt_engine::content_balancing::{ContentBalancingConfig, ContentCategory};
use std::collections::HashMap;

// Satu kategori per item di bank, indeks == ItemIndex.
let categories: Vec<ContentCategory> = vec![
    "Aljabar".into(), "Aljabar".into(), "Geometri".into(), "Geometri".into(),
];

let mut target_prop = HashMap::new();
target_prop.insert(ContentCategory::from("Aljabar"), 0.5);
target_prop.insert(ContentCategory::from("Geometri"), 0.5);

let config = ContentBalancingConfig::new(categories, target_prop, false)?;
```

Tiga aturan validasi (dicek di `ContentBalancingConfig::new`, error saat
konstruksi — bukan runtime):

1. `target_prop` harus menjumlah **persis `1.0`** (toleransi `1e-6`) —
   padanan persis validasi `content_prop does not sum to 1` mirtCAT (lih.
   `docs/teori/06-content-balancing.md` §2).
2. `target_prop` tidak boleh berisi nilai negatif.
3. Setiap kategori yang muncul di `category_per_item` WAJIB punya entri di
   `target_prop` — TIDAK ada default diam-diam ke `0.0`; typo nama kategori
   akan langsung ketahuan sebagai error, bukan bocor jadi target 0 yang
   tidak disengaja.

`category_per_item` biasanya diisi dari `ItemMeta.content` tiap item di
bank (`bank.meta(i).content`, lih. `src/bank.rs`) — modul ini sendiri
sengaja tidak bergantung ke `ItemBank` langsung (lih. teori §6), jadi
pemanggil yang bertanggung jawab membangun vektornya.

## Alur pemakaian per-langkah

```rust
use mirt_engine::content_balancing::narrow_candidates;
use mirt_engine::{rank_candidates, select_next_item, SelectionConfig};

let candidates: Vec<usize> = (0..n_items)
    .filter(|idx| !person.answered_items().contains(idx))
    .collect();

// Content balancing (Fase 6) beroperasi PALING AWAL, sebelum ranking &
// exposure control.
let (narrowed, _debug) =
    narrow_candidates(&cb_config, person.answered_items(), &candidates)?;

// Lanjut ke item selection (Fase 2) seperti biasa, hanya kandidatnya
// sudah disempitkan.
let (chosen, _) = select_next_item(&sel_config, &narrowed, &person, &bank, &group, &mut rng)?;
person.record_answer(chosen, response);
```

Atau, bila sesi juga memakai exposure control (Fase 5), sisipkan
`narrow_candidates()` sebelum `rank_candidates`:

```rust
let (narrowed, _) = narrow_candidates(&cb_config, person.answered_items(), &candidates)?;
let scores = rank_candidates(&sel_config, &narrowed, &person, &bank, &group)?;
let (chosen, _) = select_with_exposure(&exp_config, &scores, person.n_answered(), &mut valid_items, &mut rng)?;
```

## Larangan kombinasi dengan `Criterion::Seq`

```rust
cb_config.validate_with_criterion(sel_config.criterion)?;
```

Panggil ini saat merangkai `ContentBalancingConfig` dengan `SelectionConfig`
sesi Anda — error `MirtCatError::ContentBalancingIncompatibleWithSeq` bila
`criterion == Criterion::Seq` (padanan persis larangan mirtCAT: "content
designs are not supported for seq criteria"). `Criterion::Random` dan
seluruh kriteria berskor (`Kl`/`KlN`/`DRule`/`TRule`/`ARule`/`ERule`/
`WRule`) **didukung** — HANYA `Seq` yang dilarang.

## Fallback saat kategori habis

Bila kategori prioritas kehabisan kandidat di `candidate_pool` (semua item
kategori itu sudah terjawab), `narrow_candidates()` **otomatis kembali ke
pool penuh** — bukan error, bukan pool kosong. Periksa lewat
`ContentBalancingDebug::fallback_used` (`config.debug = true`) bila perlu
memantau seberapa sering ini terjadi; frekuensi tinggi biasanya menandakan
target proporsi tidak realistis relatif ukuran bank per kategori.

## Contoh lengkap

Simulasi sesi 10 item, target 50/50 dua kategori, cetak proporsi realisasi
tiap langkah untuk menunjukkan konvergensi:

```sh
cargo run --example 06_content_balancing
```
