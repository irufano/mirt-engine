# 06 — Content Balancing

> Landasan teori untuk `src/content_balancing/` (Fase 6).

## 1. Ground truth: `Design$initialize()` & `findNextCATItem()`

Source `mirtCAT::Design$initialize()` (setup `content`/`content_prop`,
`mirtCAT/R/DesignClass.R`) dan `mirtCAT::findNextCATItem()` (cabang
`design@use_content`, `mirtCAT/R/findNextCATItem.R`) — keduanya diperiksa
langsung dari source terpasang mirtCAT 1.14 (`getMethod("initialize",
"Design")`, `getFromNamespace("findNextCATItem", "mirtCAT")`), bukan hanya
dibaca dari dokumentasi.

Content balancing menjaga proporsi kategori konten item yang disajikan
(mis. "Aljabar" vs "Geometri") mendekati target yang ditentukan pengguna,
beroperasi **sebelum** exposure control (Fase 5) dalam pipeline seleksi
item.

## 2. `content_prop` WAJIB menjumlah persis 1

```r
if (!closeEnough(sum(design$content_prop) - 1, -1e-6, 1e-6))
  stop("content_prop does not sum to 1")
```

Ini **koreksi** atas sketsa awal desain modul ini, yang semula menduga
target proporsi fleksibel (tidak wajib persis `1.0`). Source mirtCAT
terpasang membuktikan sebaliknya: validasi terjadi SEBELUM target
direindeks ke kategori yang benar-benar ada di bank (lih. §3), jadi target
proporsi untuk kategori yang TIDAK muncul di bank pun ikut dihitung ke
jumlah itu. `ContentBalancingConfig::new` (`src/content_balancing/mod.rs`)
mereplikasi validasi ini persis, toleransi `1e-6`.

## 3. Kategori direindeks ke urutan `factor()` (alfabetis)

```r
.Object@content <- factor(design$content)
tmp <- design$content_prop
tmp <- tmp[match(names(table(.Object@content)), names(tmp))]
.Object@content_prop <- tmp
tmp[1L:length(tmp)] <- 0
.Object@content_prop_empirical <- tmp
```

`content_prop_empirical` diinisialisasi vektor **nol**, bukan dihitung
`0/0` — kategori yang belum pernah terjawab tetap terhitung `0.0` sepanjang
sesi belum ada item terjawab sama sekali di kategori itu, bukan hilang dari
perhitungan (padanan `factor()` + `table()` R yang selalu mengembalikan
SEMUA level, termasuk level berfrekuensi nol). `narrow_candidates()`
(`src/content_balancing/mod.rs`) mereplikasi ini: `category_order`
(himpunan kategori terurut alfabetis, deduplikasi dari `category_per_item`)
selalu dipakai sebagai basis perhitungan proporsi empiris, bukan kategori
yang benar-benar muncul di `answered_items` saja.

## 4. Pemilihan kategori prioritas

```r
dif <- design@content_prop - design@content_prop_empirical
tmp <- names(dif)[which.max(dif)]
cpick <- design@content[which_not_answered]
pick <- cpick == tmp
if (sum(pick) > 0L) {
  index <- index[pick]
  crit <- crit[pick]
}
```

Kategori dengan `target - empirical` **terbesar** (paling kurang jatah)
jadi prioritas. `which.max` R mengembalikan index maksimum **pertama** pada
urutan `names(dif)` (alfabetis, lih. §3) — tie-break yang sama persis
dengan pola `argmax` yang sudah ada di `item_selection::argmax`
(`src/item_selection/mod.rs`, `>=` menjaga entri pertama pada seri).
`narrow_candidates()` menerapkan tie-break yang sama pada `category_order`.

**Tidak ada `else`** pada blok `if (sum(pick) > 0L)` — bila kategori
prioritas TIDAK punya kandidat tersisa (`pick` semuanya `FALSE`),
`index`/`crit` tetap UTUH, bukan kosong atau error. Ini fallback ke pool
penuh, direplikasi persis oleh `narrow_candidates()` (`fallback_used` di
`ContentBalancingDebug`).

## 5. Hanya `Criterion::Seq` yang dilarang

```r
if (.Object@use_content && criteria == "seq")
  stop("content designs are not supported for seq criteria", call. = FALSE)
```

`criteria == "random"` justru **didukung** — `findNextCATItem()` punya
cabang content-balancing terpisah di dalam blok `criteria == "random"`
(duplikat kode R, bukan bug), fungsional setara "sempitkan ke kategori
prioritas, fallback ke pool penuh bila kategori itu habis" seperti kriteria
berskor. `ContentBalancingConfig::validate_with_criterion()`
(`src/content_balancing/mod.rs`) mereplikasi larangan yang tepat: hanya
`Criterion::Seq` yang ditolak (`MirtCatError::ContentBalancingIncompatibleWithSeq`).

## 6. Urutan kerja & keputusan desain `narrow_candidates`

Urutan nyata `findNextCATItem()` untuk kriteria berskor: `crit`/`index`
(skor SELURUH kandidat belum dijawab) dihitung dulu lewat fungsi kriteria
(Fase 2), BARU disempitkan berdasar kategori, BARU exposure control
(Fase 5) jalan di atas hasil sempitan.

Karena skor tiap kandidat (`Drule`/`Trule`/dst.) dihitung independen
terhadap kandidat lain, menyempitkan pool `ItemIndex` **sebelum** memanggil
`item_selection::rank_candidates` menghasilkan nilai numerik identik dengan
menyempitkan **sesudah** ranking (urutan R) — dibuktikan lewat replay
fixture R di `tests/integration_content_balancing.rs` (kecocokan EKSAK item
demi item, bukan cuma keanggotaan set: skenario fixture deterministik
penuh, kriteria `Drule` tanpa exposure control, tie-break "pertama"
konsisten di kedua sisi). `content_balancing::narrow_candidates()` karena
itu menerima `candidate_pool: &[ItemIndex]` MENTAH (belum diranking),
bukan `candidate_scores` hasil `rank_candidates`.

`ContentBalancingConfig` sengaja tetap **decoupled dari `ItemBank`** (pola
sama seperti `exposure_control::ExposureControlConfig`, yang menyimpan
`Vec<f64>` sendiri, bukan bergantung ke `ItemBank`) — `category_per_item`
dikonstruksi eksplisit oleh pemanggil. `ItemMeta.content: Option<String>`
(`src/bank.rs`) sudah disiapkan sebelumnya untuk fase ini (komentar
eksplisit "dipakai content balancing, Fase 6"); `McatSession` (Fase 8)
adalah titik yang tepat untuk mengisi `category_per_item` dari
`bank.meta(i).content`, menjaga `content_balancing` tetap bisa diuji tanpa
`ItemBank` sungguhan.

## Peringatan interaksi dengan exposure control (Fase 5)

Bila kombinasi filter content + exposure control sangat ketat, kandidat
tersisa bisa menyusut drastis di beberapa langkah — `select_with_exposure`
(Fase 5) mengembalikan `Err` eksplisit bila `candidate_scores` kosong,
bukan panic. `narrow_candidates()` sendiri tidak pernah menghasilkan pool
kosong (fallback ke pool penuh, §4) — risiko kehabisan kandidat murni
tanggung jawab exposure control di langkah setelahnya.

## Referensi

- Kingsbury, G. G., & Zara, A. R. (1989). Procedures for selecting items
  for computerized adaptive tests. *Applied Measurement in Education*,
  2(4), 359–375.
- Veldkamp, B. P., & van der Linden, W. J. (2002). Multidimensional
  adaptive testing with constraints on test content. *Psychometrika*,
  67(4), 575–588.
- Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
  Interfaces for Multidimensional Item Response Theory Applications.
  *Journal of Statistical Software*, 71(5), 1–39.
