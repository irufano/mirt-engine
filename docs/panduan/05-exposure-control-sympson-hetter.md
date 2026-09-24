# 05 — Sympson–Hetter (`SH`): Loop Terima/Tolak Per-Item

## Quickstart

```rust
use mirt_engine::exposure_control::{select_with_exposure, ExposureControlConfig, ExposureMode};
use mirt_engine::rank_candidates;

let mode = ExposureMode::SympsonHetter(vec![0.3; n_items]); // 30% peluang lolos tiap item
let exp_config = ExposureControlConfig::new(mode, n_items, false)?;

let mut valid_items = vec![true; n_items]; // per PESERTA, bukan global bank
let mut person = PersonState::new(nfact);

for _ in 0..max_items {
    let candidates: Vec<usize> = (0..n_items)
        .filter(|idx| !person.answered_items().contains(idx) && valid_items[*idx])
        .collect();
    let scores = rank_candidates(&sel_config, &candidates, &person, &bank, &group)?;
    let (chosen, _) = select_with_exposure(
        &exp_config, &scores, person.n_answered(), &mut valid_items, &mut rng,
    )?;
    person.record_answer(chosen, get_response(chosen));
}
```

## Kapan pakai

- Anda punya target **peluang lolos per-item** yang jelas (mis. hasil studi
  keamanan bank soal: "item X maksimal boleh dipilih pada 25% kesempatan").
- Butuh kontrol per-ID-item yang presisi, bukan sekadar variasi posisi
  penyajian (bandingkan dengan `05-exposure-control-randomesque.md`).

## Mengatur nilai `exposure`

`exposure[item] ∈ [0, 1]`, diindeks per **ID item** (posisi ke-`i` di
vektor = item ke-`i` di bank, BUKAN posisi penyajian — lih.
`05-exposure-control-overview.md` §"pengindeksan berbeda"). Nilai lebih
kecil = lebih jarang lolos (bukan kuota mutlak, melainkan peluang tiap kali
item itu jadi kandidat terbaik — lih. teori §1). Nilai `1.0` = tanpa
pembatasan efektif untuk item itu (selalu lolos begitu jadi kandidat
terbaik, sama seperti tanpa exposure control).

`ExposureMode::from_values()` mendeteksi mode `SH` otomatis bila SELURUH
nilai vektor `∈ [0, 1]` — tidak perlu konstruksi eksplisit
`ExposureMode::SympsonHetter(...)` kecuali ingin tipe yang lebih eksplisit
di kode Anda.

## `valid_items`: WAJIB per-peserta, WAJIB persisten sepanjang sesi

Kesalahan paling umum: membuat `valid_items` baru di setiap langkah, atau
berbagi satu `valid_items` lintas peserta berbeda. Keduanya SALAH:

- **Baru tiap langkah** → item yang ditolak di langkah 1 bisa dicoba lagi
  di langkah 2, mengingkari mekanisme SH ("item ditolak = invalid untuk
  SISA sesi ini", lih. teori §3).
- **Dibagi lintas peserta** → penolakan untuk peserta A ikut memengaruhi
  kandidat peserta B, padahal exposure control seharusnya beroperasi
  per-sesi independen.

Pola benar: `let mut valid_items = vec![true; n_items];` SATU KALI per
sesi peserta baru, diteruskan (bukan diklon ulang) ke setiap pemanggilan
`select_with_exposure()` dalam sesi itu.

## Membaca jejak debug

```rust
let exp_config = ExposureControlConfig::new(mode, n_items, true)?; // debug=true
let (chosen, debug) = select_with_exposure(&exp_config, &scores, pos, &mut valid_items, &mut rng)?;
let debug = debug.unwrap();
for (item, accepted) in &debug.attempts {
    println!("item {item}: {}", if *accepted { "diterima" } else { "ditolak" });
}
if debug.forced_accept {
    println!("diterima lewat fallback 'kandidat tersisa satu', bukan draw normal");
}
```

`debug.attempts` mencatat SETIAP kandidat yang dicoba dalam satu
pemanggilan (bisa lebih dari satu bila ada penolakan), urut percobaan —
entri terakhir adalah `chosen`.

Contoh lengkap (simulasi 1000 peserta, distribusi frekuensi vs baseline):
`cargo run --example 05_exposure_control_sympson_hetter`.
