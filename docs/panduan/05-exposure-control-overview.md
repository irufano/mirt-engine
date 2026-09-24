# 05 — Exposure Control: Overview

## Kapan pakai

Exposure control MURNI relevan bila item bank punya item yang jauh lebih
"disukai" kriteria seleksi (mis. `Drule`) dibanding item lain — tanpa
pembatasan, item favorit itu akan diberikan hampir ke SEMUA peserta,
sementara item lain nyaris tidak pernah terpakai. Dua situasi umum yang
membutuhkannya:

- **Keamanan bank soal**: item populer yang sering muncul lebih berisiko
  "bocor" (peserta saling bertukar info soal).
- **Pemanfaatan bank soal merata**: investasi pengembangan item mahal;
  item yang tidak pernah dipakai adalah pemborosan.

Bila item bank sudah cukup homogen (kriteria seleksi tidak condong tajam
ke item tertentu), exposure control kurang berdampak — pertimbangkan
`ExposureMode` sebagai OPSIONAL, bukan wajib.

## Memilih mode

```rust
use mirt_engine::exposure_control::{ExposureControlConfig, ExposureMode};

// Otomatis, gaya mirtCAT: deteksi dari rentang nilai.
let mode = ExposureMode::from_values(&exposure_values)?;
let config = ExposureControlConfig::new(mode, n_items, false)?;
```

- Seluruh nilai `∈ [0, 1]` → `SympsonHetter` — lih.
  `05-exposure-control-sympson-hetter.md`. Cocok bila tujuannya **peluang
  lolos per-item** yang terukur (mis. "item ini maksimal boleh muncul ke
  30% peserta").
- Ada nilai `> 1` → `Randomesque` — lih.
  `05-exposure-control-randomesque.md`. Cocok bila tujuannya **variasi
  urutan penyajian** tanpa perlu menghitung target peluang per item
  (lebih sederhana dikonfigurasi, tapi kontrol lebih kasar).

Atau konstruksi eksplisit tanpa deteksi otomatis:

```rust
let mode = ExposureMode::SympsonHetter(vec![0.3; n_items]);
// atau
let mode = ExposureMode::Randomesque(vec![3; n_items]);
```

## Kriteria yang didukung

Exposure control HANYA berlaku untuk kriteria berbasis skor —
`Kl`/`KlN`/`DRule`/`TRule`/`ARule`/`ERule`/`WRule` (+ varian `*Prule`).
`Criterion::Seq`/`Random` **tidak berlaku** (lih.
`05-exposure-control-overview.md` teori §4) — `rank_candidates()` akan
mengembalikan `Err` bila dipanggil dengan kriteria itu; jangan panggil
`select_with_exposure()` sama sekali bila `criterion` sesi Anda `Seq`
atau `Random`.

## Alur pemakaian per-langkah

```rust
use mirt_engine::{rank_candidates, SelectionConfig};
use mirt_engine::exposure_control::select_with_exposure;

let candidates: Vec<usize> = (0..n_items)
    .filter(|idx| !person.answered_items().contains(idx) && valid_items[*idx])
    .collect();
let scores = rank_candidates(&sel_config, &candidates, &person, &bank, &group)?;
let (chosen, _debug) = select_with_exposure(
    &exp_config,
    &scores,
    person.n_answered(), // posisi penyajian, 0-based
    &mut valid_items,    // WAJIB slice yang SAMA tiap langkah sesi ini
    &mut rng,
)?;
person.record_answer(chosen, response);
```

Tiga hal yang WAJIB diperhatikan pemanggil (lih. dokumentasi teori tiap
mode untuk alasan lengkap):

1. `candidates` sudah difilter agar TIDAK memuat item `valid_items[..] ==
   false` — sama seperti R memfilter `not_answered` sebelum `crit`
   dihitung.
2. `valid_items` (panjang `n_items`, diindeks per ID item) harus slice
   yang **sama** diteruskan di setiap langkah sesi peserta yang sama —
   mode `SympsonHetter` memutasinya untuk item yang ditolak, dan itu WAJIB
   persisten sepanjang sesi.
3. `position_in_test` = `person.n_answered()` (jumlah item TERJAWAB
   sejauh ini, sebelum item ini dipilih) — dipakai mode `Randomesque`
   untuk mengindeks posisi.

## Digabung dengan content balancing (Fase 6)

`content_balancing::narrow_candidates()` (Fase 6, lih.
`docs/panduan/06-content-balancing.md`) beroperasi SEBELUM exposure
control dalam pipeline — sisipkan sebelum `rank_candidates`:

```rust
let (narrowed, _) = narrow_candidates(&cb_config, person.answered_items(), &candidates)?;
let scores = rank_candidates(&sel_config, &narrowed, &person, &bank, &group)?;
let (chosen, _) = select_with_exposure(&exp_config, &scores, person.n_answered(), &mut valid_items, &mut rng)?;
```

Bila kombinasi filter content + exposure sangat ketat, kandidat tersisa
bisa menyusut drastis di beberapa langkah — waspadai risiko "kehabisan
item" (lih. `select_with_exposure()` mengembalikan `Err` eksplisit bila
`candidate_scores` kosong, bukan panic; `narrow_candidates()` sendiri
tidak pernah menghasilkan pool kosong, lih. fallback di
`docs/panduan/06-content-balancing.md`).

## Peta dokumen per-mode

| Mode | Panduan |
|---|---|
| Sympson–Hetter (`SH`) | `05-exposure-control-sympson-hetter.md` |
| Randomesque (`sample`) | `05-exposure-control-randomesque.md` |

Contoh lengkap (simulasi 1000 peserta, distribusi frekuensi item vs
baseline): `cargo run --example 05_exposure_control_sympson_hetter`,
`cargo run --example 05_exposure_control_randomesque`.
