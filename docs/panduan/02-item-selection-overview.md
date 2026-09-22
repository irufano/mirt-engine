# 02 — Pemilihan Item (Item Selection): Overview

## Konsep dasar

`item_selection::select_next_item()` memilih item selanjutnya dari daftar
kandidat (index `ItemBank` yang belum dijawab) sesuai `SelectionConfig`.
Lih. `docs/teori/02-item-selection-overview.md` untuk landasan & kenapa
`nfact > 1` membatasi kriteria yang tersedia.

```rust
use mirtcat_engine::{
    select_next_item, Criterion, GroupParams, PersonState, SelectionConfig,
};
use rand::rngs::StdRng;
use rand::SeedableRng;

// `bank: ItemBank` sudah ada (lih. docs/panduan/00-instalasi-dan-quickstart.md)
let group = GroupParams::standard_normal(bank.nfact().unwrap());
let person = PersonState::new(bank.nfact().unwrap());
let config = SelectionConfig::default(); // Criterion::Seq
let candidates: Vec<usize> = (0..bank.len()).collect();
let mut rng = StdRng::seed_from_u64(0); // dipakai hanya oleh Criterion::Random

let (chosen, _debug) =
    select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
```

## Memilih `Criterion`

| Situasi | Kriteria yang masuk akal |
|---|---|
| Kuesioner non-adaptif, urutan tetap | `Criterion::Seq` |
| Baseline exposure-neutral / kontrol eksperimen | `Criterion::Random` |
| Tidak butuh matriks informasi, cukup cepat | `Criterion::Kl`/`Criterion::KlN` |
| Presisi keseluruhan (volume elipsoid kepercayaan) | `Criterion::DRule` |
| Presisi per-dimensi, bisa diberi bobot | `Criterion::TRule` |
| Minimalkan varians estimasi (mendekati minimum error-variance) | `Criterion::ARule` |
| Cegah satu dimensi terus terabaikan | `Criterion::ERule` |
| Peduli skor komposit tertentu (`w' * theta`) | `Criterion::WRule` |

Semua varian di atas punya field `prior: bool` — `true` memakai varian
`*Prule` (lih. §4 dokumen teori). **Disarankan `prior: true`** di awal tes
(saat item terjawab masih sedikit) untuk D/T/A/E/W-rule — tanpa prior,
`Drule`/`Erule` khususnya degenerate (nilai kriteria persis nol untuk semua
kandidat) sampai `>= nfact` item terjawab dengan arah info yang saling
bebas.

## `weights`: kapan dan bagaimana

`SelectionConfig.weights` (`Option<Vec<f64>>`, panjang harus = `nfact`)
dipakai `TRule`/`ARule`/`WRule` (+ varian prior), **diabaikan** kriteria
lain. `None` → default `vec![1.0; nfact]` (semua dimensi setara).

```rust
use mirtcat_engine::{Criterion, SelectionConfig};

let config = SelectionConfig {
    criterion: Criterion::TRule { prior: false },
    weights: Some(vec![2.0, 0.5, 1.0]), // dimensi 1 diutamakan
    debug: false,
};
```

Gunakan `SelectionConfig::new(criterion, weights, nfact, debug)` bila ingin
validasi eksplisit (`Err(MirtCatError::DimensionMismatch)` kalau panjang
`weights` tidak cocok `nfact`) alih-alih membangun struct literal langsung.

**Trule vs Wrule**, keduanya menerima `weights` tapi beda formula: `Trule`
menjumlahkan diagonal berbobot saja (`sum_i w_i * I[i,i]`); `Wrule` memakai
bentuk kuadratik penuh termasuk off-diagonal (`w' * I * w`) — informasi
tentang skor komposit `w' * theta`, bukan sekadar "dimensi mana yang
diprioritaskan". Pakai `Wrule` kalau yang dipedulikan benar-benar skor
gabungan (mis. total tertimbang beberapa sub-skala), `Trule` kalau cuma
mau memberi bobot relatif antar dimensi independen.

## `PersonState`: theta & item terjawab

```rust
use mirtcat_engine::PersonState;

let mut person = PersonState::new(3); // theta nol, belum ada item terjawab
// atau: PersonState::from_theta(theta0) hasil Fase 1 (initial_theta::initialize)

person.record_answer(item_index); // catat item terjawab (index ItemBank)
person.set_theta(new_theta);      // update theta (hasil estimasi ulang Fase 3)
```

`PersonState::info_thetas(&bank)` **menghitung ulang** matriks informasi
kumulatif dari daftar item terjawab tiap dipanggil (bukan field yang
di-cache) — otomatis konsisten begitu `set_theta()` dipanggil (lih. dokumen
teori §2 untuk kenapa ini penting, bukan sekadar detail implementasi).

## Kandidat & error

`candidates: &[usize]` adalah index `ItemBank` yang **belum dijawab** —
`select_next_item()` sendiri tidak melacak "item mana yang sudah dijawab",
itu tanggung jawab caller (biasanya: `(0..bank.len()).filter(|i|
!person.answered_items().contains(i))`, seperti contoh-contoh di
`examples/02_item_selection_*.rs`).

Error (`Err(MirtCatError)`) terjadi bila: `candidates` kosong, ada index di
luar `bank`, atau panjang `weights` (default maupun manual) tidak cocok
`nfact` — tidak pernah panic.

## Debug trace

```rust
let config = SelectionConfig { debug: true, ..config };
let (chosen, debug) = select_next_item(&config, &candidates, &person, &bank, &group, &mut rng)?;
let debug = debug.unwrap();
println!("nilai kriteria tiap kandidat: {:?}", debug.candidate_scores);
println!("info sebelum/sesudah: {:?} -> {:?}", debug.info_before, debug.info_after);
```

`candidate_scores` kosong untuk `Seq`/`Random` (tidak ada nilai kriteria
untuk keduanya); `info_before`/`info_after` cuma `Some` untuk D/T/A/E/W-rule.
Mengaktifkan debug **tidak** mengubah item yang dipilih (prinsip desain #7,
`PLAN.md`).

## Contoh lengkap per kriteria

```sh
cargo run --example 02_item_selection_seq
cargo run --example 02_item_selection_random
cargo run --example 02_item_selection_kl
cargo run --example 02_item_selection_d_rule
cargo run --example 02_item_selection_t_rule
cargo run --example 02_item_selection_a_rule
cargo run --example 02_item_selection_e_rule
cargo run --example 02_item_selection_w_rule
```

Panduan rinci per algoritma: `docs/panduan/02-item-selection-{seq,random,
kl,d-rule,t-rule,a-rule,e-rule,w-rule}.md`.
