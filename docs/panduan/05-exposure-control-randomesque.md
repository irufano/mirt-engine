# 05 — Randomesque (`'sample'`): Top-*n* Random

## Quickstart

```rust
use mirt_engine::exposure_control::{select_with_exposure, ExposureControlConfig, ExposureMode};
use mirt_engine::rank_candidates;

let mode = ExposureMode::Randomesque(vec![3; n_items]); // top-3 di semua posisi
let exp_config = ExposureControlConfig::new(mode, n_items, false)?;

let mut person = PersonState::new(nfact);
for _ in 0..max_items {
    let candidates: Vec<usize> = (0..n_items)
        .filter(|idx| !person.answered_items().contains(idx))
        .collect();
    let scores = rank_candidates(&sel_config, &candidates, &person, &bank, &group)?;
    // valid_items diabaikan mode ini, tapi tetap wajib sebagai parameter --
    // boleh dibuat baru tiap langkah (tidak ada state yang perlu bertahan).
    let mut valid_items = vec![true; n_items];
    let (chosen, _) = select_with_exposure(
        &exp_config, &scores, person.n_answered(), &mut valid_items, &mut rng,
    )?;
    person.record_answer(chosen, get_response(chosen));
}
```

## Kapan pakai

- Butuh variasi urutan penyajian antar peserta TANPA perlu menghitung
  target peluang per-item (lebih sederhana dikonfigurasi dibanding
  `05-exposure-control-sympson-hetter.md`).
- Toleran terhadap kontrol yang lebih kasar: randomesque membatasi
  KANDIDAT (top-*n* per posisi), bukan peluang lolos tiap item secara
  presisi.

## Mengatur nilai `exposure`

`exposure[posisi] ∈ {1, 2, 3, ...}`, diindeks per **posisi penyajian**
(elemen ke-`k` dipakai saat `k` item sudah terjawab, BUKAN per ID item —
lih. `05-exposure-control-overview.md` §"pengindeksan berbeda"). Nilai
umum:

- `1` di semua posisi = tanpa pembatasan efektif (identik dengan tanpa
  exposure control sama sekali — selalu argmax deterministik).
  Pertimbangkan: apakah Anda benar-benar perlu `Randomesque` di kasus ini,
  atau cukup `mode = None` (tidak memanggil `select_with_exposure` sama
  sekali)?
- `3`-`5` di posisi-posisi AWAL (di mana peserta paling banyak berbagi
  urutan item yang sama, karena `theta` awal seragam) — variasi terbesar
  ada di sini. Posisi lanjut biasanya sudah cukup bervariasi secara alami
  (theta peserta sudah menyebar), jadi `exposure=1` (deterministik) di
  posisi lanjut cukup umum.

`ExposureMode::from_values()` mendeteksi mode `Randomesque` otomatis bila
ADA nilai `> 1` di vektor — TAPI begitu terdeteksi, SELURUH elemen vektor
(termasuk yang `<= 1`) wajib bilangan bulat `>= 1`, kalau tidak
`from_values()` mengembalikan `Err` (lih. teori §"Deteksi mode otomatis").

## `valid_items`: diabaikan, tapi tetap parameter wajib

Beda dari `SympsonHetter`, mode ini tidak pernah memutasi `valid_items` —
tidak ada konsep "item dilarang" (lih. teori §4, tabel perbandingan). Boleh
membuat `valid_items` baru (semua `true`) di setiap langkah tanpa risiko —
parameter ini tetap ada di `select_with_exposure()` supaya satu signature
dispatcher melayani kedua mode.

## Membaca jejak debug

```rust
let exp_config = ExposureControlConfig::new(mode, n_items, true)?; // debug=true
let (chosen, debug) = select_with_exposure(&exp_config, &scores, pos, &mut valid_items, &mut rng)?;
let debug = debug.unwrap();
println!("dipertimbangkan (top-n): {:?}", debug.attempts); // semua (item, true)
assert!(!debug.forced_accept); // selalu false untuk mode ini
```

`debug.attempts` di mode ini berisi seluruh himpunan top-*n* yang
dipertimbangkan (semua ditandai `true` — bukan riwayat terima/tolak
seperti `SympsonHetter`), urutan di `Vec` TIDAK bermakna peringkat, hanya
keanggotaan.

Contoh lengkap (baseline vs `exposure=1` vs `exposure=3`, membuktikan
`exposure=1` identik baseline dan `exposure=3` menyebar ke top-3):
`cargo run --example 05_exposure_control_randomesque`.
