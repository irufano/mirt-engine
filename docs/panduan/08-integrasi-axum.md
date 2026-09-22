# 08 — Integrasi End-to-End (`McatSession`) & Contoh Axum

## Kapan pakai

`McatSession` (Fase 8) menyatukan seluruh siklus adaptif (Fase 1–7) jadi
satu orkestrator yang bisa dipanggil per-langkah dari luar: mulai sesi →
(pilih item → sajikan → terima respons → estimasi ulang → cek stop) →
hasil akhir. Pakai ini alih-alih memanggil `initial_theta`/
`item_selection`/`ability_estimation`/`stopping_rule`/`exposure_control`/
`content_balancing` satu-satu secara manual (seperti contoh
`examples/07_final_result.rs`) begitu Anda butuh menjalankan sesi
sungguhan dari luar crate -- terutama di web API, di mana state sesi harus
bertahan ANTAR-REQUEST HTTP tanpa server perlu menyimpannya di memori.

## Siklus dasar

```rust
use mirtcat_engine::{McatSession, SessionConfig};

let config = SessionConfig {
    // ..isi selection/estimation/stopping/exposure/content_balancing/
    // start_item/debug sesuai kebutuhan Anda (lih. tabel di bawah)..
    ..Default::default()
};
let mut session = McatSession::new(config, &bank, &group, 0)?;

let mut rng = rand::rngs::StdRng::from_entropy();
loop {
    let item = session.next_item(&bank, &group, &mut rng)?;
    // ..sajikan `item` ke peserta, terima respons (kategori 0-based)..
    let response: usize = /* dari peserta */ 1;
    let decision = session.submit_response(item, response, &bank, &group, None)?;
    if decision.stop_now {
        break;
    }
}

let result = session.finalize(); // McatResult (Fase 7)
```

`bank`/`group` diteruskan sebagai **parameter** ke `next_item`/
`submit_response`, BUKAN disimpan di dalam `McatSession` -- lih.
§"Kenapa `McatSession` tidak menyimpan `ItemBank`" di bawah.

## `SessionConfig`

| Field | Tipe | Keterangan |
|---|---|---|
| `initial_theta` | `InitialThetaConfig` | Fase 1 -- titik awal `theta`/SE. |
| `selection` | `SelectionConfig` | Fase 2 -- kriteria pemilihan item. |
| `estimation` | `EstimationConfig` | Fase 3 -- metode estimasi ulang `theta`. |
| `stopping` | `StoppingConfig` | Fase 4 -- kapan sesi berhenti. |
| `exposure` | `Option<ExposureControlConfig>` | Fase 5. `None` = nonaktif. |
| `content_balancing` | `Option<ContentBalancingConfig>` | Fase 6. `None` = nonaktif. |
| `start_item` | `usize` | Item pertama sesi -- disajikan TANPA lewat kriteria/content balancing/exposure control (padanan `design@start_item` `mirtCAT`). |
| `debug` | `bool` | Saklar tunggal -- diteruskan otomatis ke SELURUH sub-config di atas saat `McatSession::new()` (lih. §"Debug log"). |

`McatSession::new()` juga memvalidasi kombinasi tidak sah (mis. content
balancing + `Criterion::Seq`, lih. `ContentBalancingConfig::validate_with_criterion`)
dan `start_item` di luar jangkauan `bank` -- error `MirtCatError` sebelum
sesi sempat dibuat.

## Urutan pipeline `next_item()`

1. Item **pertama** sesi selalu `config.start_item`, tanpa terkecuali.
2. Selain itu: kandidat (belum dijawab & masih valid untuk exposure
   control Sympson-Hetter) → content balancing (bila aktif, dilewati untuk
   `Criterion::Seq`) → kriteria seleksi item (+ exposure control bila
   aktif, hanya untuk kriteria berskor -- `Seq`/`Random` tidak pernah masuk
   exposure control).

Urutan ini persis `mirtCAT::findNextCATItem()` R (dibaca langsung dari
source, bukan ditebak) -- lih. komentar modul `src/session.rs` untuk
rincian & rujukan baris kode R.

## Mencegah state korup

`submit_response(item, ...)` menolak (`Err`) bila `item` bukan hasil
`next_item()` terakhir yang belum dijawab. Alur yang benar SELALU
bergantian `next_item()` → `submit_response()` → `next_item()` → ...;
`McatSession` melacak ini lewat `pending_item()`.

## Kenapa `McatSession` tidak menyimpan `ItemBank`/`GroupParams`

`ItemBank` menyimpan `Box<dyn ItemResponseModel>` (trait object) -- tidak
bisa diserialisasi secara generik, dan field bertipe referensi
(`&ItemBank`) akan mengikat `McatSession` ke satu lifetime tertentu.
Keduanya bertentangan dengan tujuan utama fase ini: **sesi harus bisa
di-serialize/deserialize utuh** supaya server TIDAK WAJIB menyimpan state
di memori antar-request HTTP. `McatSession` karena itu hanya menyimpan
`PersonState` + `SessionConfig` + state internal kecil (`valid_items`,
`pending_item`, dst.) -- semuanya `Serialize`/`Deserialize` di balik
feature `serde`, sementara `bank`/`group` dibangun sekali oleh pemanggil
(biasanya sekali per proses server, dibagi lewat `Arc`) dan diteruskan
ulang tiap panggilan.

## Pola state antar-request HTTP

Aktifkan feature `serde`:

```toml
[dependencies]
mirtcat-engine = { version = "...", features = ["serde"] }
```

```rust
let json = serde_json::to_string(&session)?;   // simpan (cookie/Redis/DB)
// .. request HTTP berikutnya ..
let mut session: McatSession = serde_json::from_str(&json)?; // pulihkan
```

Contoh `examples/08_axum_api/app.rs` menunjukkan pola ini secara konkret:
`AppState.sessions` menyimpan `McatSession` sebagai **string JSON hasil
serialisasi** (bukan struct langsung) di `HashMap` in-memory -- tiap
request deserialize dari string, mutasi (`next_item`/`submit_response`),
serialize balik, simpan. Ini sengaja mensimulasikan apa yang terjadi
dengan Redis/DB sungguhan. **Untuk produksi**, ganti `HashMap` in-memory
itu dengan store eksternal (Redis/PostgreSQL/dsb.) -- `HashMap` in-memory
hilang begitu proses server berhenti dan tidak dibagi antar-instance saat
di-scale horizontal ke banyak server.

## Debug log (`StepDebug`)

`config.debug = true` mengaktifkan jejak debug SELURUH sub-fase (Fase
1-6) untuk tiap langkah, diakses lewat `session.debug_log() ->
&[StepDebug]` (append-only) dan `session.initial_theta_debug()`. Tidak
pernah memengaruhi item yang dipilih atau `StopDecision`/`McatResult` yang
dikembalikan -- murni observability, cocok diekspos lewat endpoint
tersendiri (lih. `GET /session/:id/debug` di bawah) untuk audit/visualisasi
tanpa membebani jalur utama saat `debug = false` (default).

## Contoh Axum

```sh
cargo run --example 08_axum_api --features serde
```

Server berjalan di `http://127.0.0.1:3000` dengan 4 endpoint:

| Endpoint | Body request | Balasan |
|---|---|---|
| `POST /session` | - | `{"session_id": "1", "next_item": 0}` |
| `POST /session/:id/answer` | `{"item": 0, "response": 1}` | `{"stop_now": false, "next_item": 7}` |
| `GET /session/:id/result` | - | `McatResult` JSON (`409` bila sesi belum `stop_now`) |
| `GET /session/:id/debug` | - | `[StepDebug, ...]` JSON (`404` bila `SessionConfig::debug == false`) |

Uji manual:

```sh
curl -s -X POST localhost:3000/session
# {"session_id":"1","next_item":0}

curl -s -X POST localhost:3000/session/1/answer \
  -H 'content-type: application/json' -d '{"item":0,"response":1}'
# ulangi dengan "item" = "next_item" balasan sebelumnya sampai stop_now == true

curl -s localhost:3000/session/1/result
```

Test HTTP end-to-end otomatis (tanpa perlu server sungguhan berjalan --
lewat `tower::ServiceExt::oneshot`, in-process): lih.
`tests/integration_axum.rs`.
