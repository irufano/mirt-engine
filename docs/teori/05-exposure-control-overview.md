# 05 — Exposure Control: Overview

> Landasan teori untuk `src/exposure_control/` (Fase 5). Lih. dokumen per
> mode untuk rumus & referensi rinci tiap mekanisme.

## 1. Ground truth: `findNextCATItem()` & `Design$initialize()`

Source `mirtCAT::findNextCATItem()` (cabang `design@exposure_type`,
`mirtCAT/R/findNextCATItem.R`) dan `mirtCAT::Design$initialize()` (deteksi
mode dari nilai vektor `exposure`, `mirtCAT/R/DesignClass.R`) — keduanya
diperiksa langsung dari source terpasang mirtCAT 1.14
(`getFromNamespace("findNextCATItem", "mirtCAT")`,
`selectMethod("initialize", "Design")`), bukan hanya dibaca dari
dokumentasi.

Dua mode didukung, `exposure_type`:

- **`"none"`** (default) — tanpa pembatasan, selalu `index[which.max(crit)]`.
  Di Rust, ini direpresentasikan dengan **tidak memanggil**
  `exposure_control::select_with_exposure()` sama sekali (bukan varian enum
  `ExposureMode::None`) — pola yang sama seperti `Criterion` Fase 2
  (ketiadaan pemanggilan, bukan varian khusus, untuk "fitur nonaktif").
- **`"SH"`** (Sympson–Hetter) — lih. `05-exposure-control-sympson-hetter.md`.
- **`"sample"`** (randomesque) — lih. `05-exposure-control-randomesque.md`.

## 2. Deteksi mode otomatis (`ExposureMode::from_values`)

`Design$initialize()` R (diperiksa lewat `selectMethod`, bukan dugaan):

```r
exposure_type <- ifelse(all(design$exposure <= 1 & design$exposure >= 0), "SH", "sample")
exposure <- if (exposure_type == "SH") design$exposure else as.integer(design$exposure)
if (exposure_type == "sample")
  if (!all(exposure == design$exposure & exposure >= 1))
    stop("sampling exposure method does not contain integer values >= 1")
```

Poin non-obvious: begitu **satu saja** elemen vektor `> 1`, SELURUH vektor
beralih ke mode `"sample"` — termasuk elemen yang aslinya `<= 1`. Elemen itu
lalu wajib bilangan bulat `>= 1` juga (praktis berarti harus persis `1`,
karena integer `>=1` dan `<=1` sekaligus hanya `1`). `as.integer()` R
**TRUNCATE** ke nol (bukan pembulatan) — `2.9` jadi `2`, lalu gagal validasi
`2 == 2.9`. `ExposureMode::from_values()` (`src/exposure_control/mod.rs`)
mereplikasi persis lewat `f64::trunc()`.

Fixture R (`tests/fixtures/exposure_control.json`, bagian
`mode_detection`) merekam `res$design@exposure_type`/`@exposure` SUNGGUHAN
dari beberapa vektor contoh (bukan dihitung ulang manual) —
`tests/integration_exposure_control.rs::mode_detection_matches_r_design_initialize`
membandingkannya EKSAK (deterministik, tanpa RNG sama sekali, beda dari dua
fixture lain di bawah).

## 3. Pengindeksan berbeda antar mode — titik paling mudah salah

- **`SympsonHetter`**: `design@exposure[item]` — diindeks per **ID item**.
  "Berapa peluang item spesifik ini boleh lolos, di posisi manapun dalam
  tes?"
- **`Randomesque`**: `design@exposure[lastitem + 1L]` — diindeks per
  **posisi penyajian** (`lastitem` = jumlah item terjawab, jadi
  `lastitem+1` = giliran ke berapa). "Untuk giliran ke-n, ambil acak dari
  berapa kandidat teratas?"

Keduanya sama-sama vektor sepanjang `n_items`, tapi maknanya sama sekali
berbeda — menukar keduanya adalah bug diam-diam (tidak ada error tipe yang
akan menangkapnya, karena keduanya `Vec<f64>`/`Vec<u32>` panjang sama).

## 4. `seq`/`random` tidak pernah masuk exposure control

Temuan yang **tidak** eksplisit tersirat di sketsa awal: di
`findNextCATItem()`, kriteria `"seq"` dan `"random"` masing-masing
`return()` LEBIH AWAL, sebelum `crit` (skor kandidat) dihitung sama sekali.
Blok exposure control (`design@exposure_type != "none"`) ada JAUH di bawah,
setelah `crit` dihitung dari kriteria berbasis skor
(`KL`/`KLn`/`Drule`/`Trule`/`Arule`/`Erule`/`Wrule` + varian `*Prule`) —
`seq`/`random` tidak pernah sampai ke sana.

Direplikasi lewat [`crate::item_selection::rank_candidates`] (fungsi baru
Fase 5, diekstrak dari `select_next_item` Fase 2 — lih. §5): mengembalikan
`Err` untuk `Criterion::Seq`/`Criterion::Random`, bukan `Vec` kosong. Ini
memaksa pemanggil menyadari exposure control tidak berlaku untuk kedua
kriteria itu, alih-alih diam-diam menerima input yang tidak bermakna.

## 5. Reuse: skor kriteria dari Fase 2, bukan duplikasi

`item_selection::rank_candidates()` (diekstrak dari `select_next_item`,
perilaku & nilai numerik identik, sudah dibuktikan lewat suite test Fase 2
yang tetap hijau setelah refactor) adalah SATU-SATUNYA sumber skor kandidat
yang dikonsumsi `select_with_exposure()`. Prinsip Desain #1 `PLAN.md`
(jangan duplikasi) — exposure control murni mekanisme *di atas* ranking
yang sudah ada, tidak mengimplementasikan ulang rumus kriteria apa pun.

## 6. Status "item tidak valid untuk sesi ini" — keputusan desain

R menyimpan `person$valid_item` (vektor boolean sepanjang `n_items`) di
objek `Person`, dimutasi in-place oleh mode `SH` untuk item yang ditolak —
**persisten** sepanjang sisa sesi peserta itu (bukan cuma satu langkah).

`PersonState` (Fase 2, sudah selesai & teruji) **sengaja tidak** ditambahi
field ini. Mengikuti preseden Fase 4 (`allow_constrain_breaks` jadi
parameter `bool` yang diteruskan pemanggil, bukan field `PersonState`,
karena tanggung jawab lintas-fase yang belum ada orchestrator-nya —
Fase 8): `select_with_exposure()` menerima `valid_items: &mut [bool]`
sebagai parameter eksplisit, dimiliki & disiapkan pemanggil. Ini juga
menyiapkan Fase 6 (constraint `independent` di `mirtCAT` JUGA memutasi
`valid_item`) untuk berbagi struktur yang sama tanpa desain ulang
`PersonState` nanti.

## Peta dokumen per-mode

| Mode | Teori | Panduan |
|---|---|---|
| Sympson–Hetter (`SH`) | `05-exposure-control-sympson-hetter.md` | `05-exposure-control-sympson-hetter.md` |
| Randomesque (`sample`) | `05-exposure-control-randomesque.md` | `05-exposure-control-randomesque.md` |

## Referensi

- Sympson, J. B., & Hetter, R. D. (1985). Controlling item-exposure rates
  in computerized adaptive testing. *Proceedings of the 27th Annual
  Meeting of the Military Testing Association*, 973–977.
- McBride, J. R., & Martin, J. T. (1983). Reliability and validity of
  adaptive ability tests in a military setting. In *New Horizons in
  Testing* (pp. 223–236). Academic Press.
- Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
  item exposure control strategies for computerized adaptive testing
  developed on the item response theory framework. *International
  Journal on E-Learning*, 6(1).
- Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
  Interfaces for Multidimensional Item Response Theory Applications.
  *Journal of Statistical Software*, 71(5), 1–39.
