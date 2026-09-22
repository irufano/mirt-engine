# 05 — Randomesque (`'sample'`): Top-*n* Random

> Landasan teori untuk `src/exposure_control/randomesque.rs`.

## 1. Rumus

Persis `findNextCATItem.R` (cabang `design@exposure_type == "sample"`,
diperiksa langsung dari source terpasang mirtCAT 1.14):

```r
exposure <- design@exposure[lastitem + 1L]
if (exposure == 1L) {
    item <- index[which.max(crit)][1L]
} else {
    rnk <- length(crit) - rank(crit, ties.method = "random") + 1L
    pick <- which(rnk %in% 1L:exposure)
    item <- index[sample(pick, 1L)]
}
```

`exposure` di sini adalah `design@exposure[lastitem + 1L]` — diindeks per
**posisi penyajian** (`lastitem` = jumlah item terjawab, jadi `lastitem+1`
= giliran ke berapa saat ini; lih. `05-exposure-control-overview.md` §3),
BUKAN per ID item seperti `SympsonHetter`.

- **`exposure == 1`**: deterministik, identik dengan tanpa exposure
  control sama sekali (`which.max(crit)`, tie pertama menang).
- **`exposure > 1`**: (a) beri tiap kandidat **peringkat descending**
  berdasarkan `crit`, dengan tie-break **random**
  (`rank(..., ties.method="random")` — tie mendapat urutan seragam-acak,
  bukan urutan tetap berdasar index); (b) ambil `exposure` kandidat
  berperingkat teratas; (c) pilih SATU secara **uniform random** di antara
  himpunan itu (bukan berbobot peringkat — kandidat rank-1 dan rank-`n`
  dalam himpunan punya peluang terpilih yang SAMA).

## 2. Replikasi tie-break random di Rust

`rank(crit, ties.method="random")` R memberi tiap elemen tied sebuah
peringkat unik yang diacak secara seragam. `randomesque::select()`
mereplikasi efek statistik ini TANPA menerjemahkan `rank()` R
literal — caranya: **shuffle acak** seluruh kandidat lebih dulu
(`Vec::shuffle`), BARU *stable sort* descending berdasarkan skor. Karena
`sort_by` Rust stabil, kandidat yang tied mempertahankan urutan relatif
dari hasil shuffle — yang sudah seragam-acak. Hasilnya: efek yang sama
persis (peringkat acak di antara yang tied), tanpa perlu implementasi
`rank()` R generik.

## 3. Bila `exposure > jumlah kandidat tersisa`

Tidak dijaga eksplisit di rumus R (`pick <- which(rnk %in% 1L:exposure)`
otomatis membatasi diri ke seluruh kandidat yang ADA bila `exposure`
melebihi jumlahnya — `rnk` maksimum toh cuma `length(crit)`).
`randomesque::select()` mereplikasi lewat
`n = exposure_for_position.min(candidates.len())` — pemilihan otomatis
jadi uniform random atas SELURUH kandidat tersisa, bukan error atau panic.

## 4. Beda fundamental dari Sympson–Hetter

| | Sympson–Hetter | Randomesque |
|---|---|---|
| Indeks vektor `exposure` | per ID item | per posisi penyajian |
| Mekanisme | terima/tolak probabilistik (loop) | pilih uniform dari top-*n* |
| Efek "tidak pernah lolos" | `exposure[item]=0.0` (lih. `05-exposure-control-sympson-hetter.md` §4) | tidak ada — semua kandidat di luar top-*n* MUNGKIN saja masuk top-*n* di posisi lain/sesi lain (peringkat kriteria berubah seiring info kumulatif) |
| State lintas-langkah | ya (`valid_items` persisten) | tidak — tiap langkah independen, tidak ada state yang dibawa |

Randomesque TIDAK punya konsep "item dilarang" — ia murni membatasi
KANDIDAT MANA yang boleh bersaing di satu posisi tertentu, sehingga item
yang "kalah" di suatu posisi tetap bisa jadi top-*n* di posisi lain begitu
kandidat lain sudah terjawab & peringkat kriteria berubah.

## Referensi

- McBride, J. R., & Martin, J. T. (1983). Reliability and validity of
  adaptive ability tests in a military setting. In *New Horizons in
  Testing* (pp. 223–236). Academic Press.
- Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
  item exposure control strategies for computerized adaptive testing
  developed on the item response theory framework. *International
  Journal on E-Learning*, 6(1).
