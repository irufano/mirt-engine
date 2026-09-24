# 07 — Hasil Bersih Akhir untuk Theta

## Kapan pakai

Di akhir sesi tes (`StopDecision::stop_now == true`, lih.
`docs/panduan/04-stopping-rule-overview.md`), rakit `McatResult` dari
`PersonState` final + `StopDecision` terakhir -- struktur bersih & stabil
untuk dikirim ke klien (API JSON, laporan, dsb.), tanpa mengekspos detail
internal state sesi (`PersonState` mentah, matriks informasi kumulatif).

## Merakit `McatResult`

```rust
use mirt_engine::McatResult;

// `stop_decision` dari `stopping_rule::evaluate()` (Fase 4) di langkah
// yang memutuskan sesi berhenti.
let result = McatResult::from_person(&person, &stop_decision);

println!("theta akhir: {:?}", result.thetas.as_slice());
println!("SE akhir:    {:?}", result.se_thetas.as_slice());
println!("kolom:       {:?}", result.column_names); // ["Theta_1", "Theta_2", ...]
```

`person` diteruskan sebagai referensi (`&PersonState`) -- tetap bisa
dipakai setelahnya (mis. memanggil `person.info_thetas(&bank)`, lih. di
bawah).

## Field yang tersedia

| Field | Tipe | Keterangan |
|---|---|---|
| `thetas` | `ThetaVector` | Estimasi akhir, panjang `nfact`. |
| `se_thetas` | `SEVector` | SE akhir, panjang `nfact`. |
| `thetas_history` | `Vec<ThetaVector>` | Riwayat estimasi, 1 baris per item terjawab. |
| `thetas_se_history` | `Vec<SEVector>` | Riwayat SE, sejalan dengan `thetas_history`. |
| `classification` | `Vec<ClassifyDecision>` | Keputusan per-dimensi, SELALU panjang `nfact` (lih. §"Klasifikasi" di bawah). |
| `column_names` | `Vec<String>` | `["Theta_1", ..., "Theta_nfact"]`. |
| `items_answered` | `Vec<usize>` | Index item (urutan `ItemBank`) yang terjawab. |
| `responses` | `Vec<Option<i32>>` | Kategori respons, sejalan `items_answered`. |
| `terminated_early` | `bool` | `true` bila berhenti karena presisi (bukan `max_items`/`max_time`). |

**Catatan panjang `thetas_history`/`thetas_se_history`**: `N_item` baris
(BUKAN `N_item + 1` seperti R -- R menyisipkan baris theta pra-tes sebagai
baris pertama; di sini sengaja tidak, supaya sejalan 1:1 dengan
`items_answered`/`responses`. Lih. `docs/teori/07-hasil-akhir-theta.md`
§3 untuk detail & alasan.

## Klasifikasi (`classification`)

Selalu panjang `nfact`, default `NoDecision` per-dimensi bila
`classify_type == None`:

```rust
use mirt_engine::ClassifyDecision;

for (name, decision) in result.column_names.iter().zip(&result.classification) {
    println!("{name}: {decision:?}"); // AboveCutoff / BelowCutoff / NoDecision
}
```

Untuk mode `Sprt` (satu keputusan gabungan di `StopDecision`, lih.
`docs/panduan/04-stopping-rule-sprt.md`), `McatResult::classification`
mem-*broadcast* keputusan itu ke seluruh dimensi -- SPRT inheren satu uji
hipotesis gabungan, bukan `nfact` uji independen.

## Mengakses `info_thetas` (matriks informasi kumulatif)

`McatResult` **sengaja tidak menyimpan** matriks informasi kumulatif
(`info_thetas`, `nfact × nfact`) -- ukurannya tumbuh kuadratik dengan
`nfact` dan jarang dibutuhkan konsumen biasa. Bila Anda perlu kovarians
estimasi secara manual (mis. untuk laporan psikometrik lanjutan), panggil
`PersonState::info_thetas()` (Fase 2) LANGSUNG, SEBELUM/SESUDAH
`McatResult::from_person` (keduanya aman -- `from_person` hanya meminjam
`person`):

```rust
let info = person.info_thetas(&bank)?; // InfoMatrix nfact x nfact
let cov = info.inverse()?;             // kovarians asimtotik (padanan solve(info))
```

## Serialisasi JSON

Aktifkan feature `serde` di `Cargo.toml`:

```toml
[dependencies]
mirt-engine = { version = "...", features = ["serde"] }
```

```rust
let json = serde_json::to_string_pretty(&result)?;
```

`ThetaVector`/`SEVector` diserialisasi sebagai array JSON polos (`[f64,
...]`, bukan objek nalgebra), dan `ClassifyDecision` sebagai string varian
(`"AboveCutoff"`/`"BelowCutoff"`/`"NoDecision"`) -- aman disimpan ke
DB/cookie/dikirim lewat HTTP dan di-deserialisasi kembali persis
(`serde_json::from_str::<McatResult>(...)`).

## Contoh lengkap

Sesi CAT sintetis 3 dimensi end-to-end (Fase 1–4: initial theta, item
selection, ability estimation, stopping rule) sampai berhenti, cetak
`McatResult` lengkap, lalu serialisasi ke JSON:

```sh
cargo run --example 07_final_result --features serde
```
