# 05 — Sympson–Hetter (`SH`): Loop Terima/Tolak Per-Item

> Landasan teori untuk `src/exposure_control/sympson_hetter.rs`.

## 1. Rumus

Persis `findNextCATItem.R` (cabang `design@exposure_type == "SH"`,
diperiksa langsung dari source terpasang mirtCAT 1.14):

```r
while (TRUE) {
    item <- index[which.max(crit)][1L]
    comp <- runif(1, 0, 1)
    if (design@exposure[item] >= comp && person$valid_item[item])
        break
    if (length(crit) == 1L)
        break
    person$valid_item[item] <- FALSE
    pick <- index != item
    index <- index[pick]
    crit <- crit[pick]
}
```

Tiap iterasi: ambil kandidat *terbaik* (`which.max(crit)`, tie pertama
menang), gambar `comp ~ Uniform(0,1)`, terima bila
`exposure[item] >= comp`. `exposure[item]` diindeks per **ID item** (bukan
posisi — lih. `05-exposure-control-overview.md` §3): secara probabilistik,
`P(terima | item dicoba) = exposure[item]`, karena `P(comp <= x) = x` untuk
`comp` kontinu Uniform(0,1). Inilah mekanisme intinya — `exposure[item]`
BUKAN batas kuota keras, melainkan **peluang lolos setiap kali item itu
menjadi kandidat terbaik**.

## 2. Dua kasus tepi (edge case) yang eksplisit di rumus

1. **Ditolak & kandidat tersisa `> 1`**: item ditandai
   `person$valid_item[item] <- FALSE` (invalid untuk SISA sesi peserta ini,
   lih. §3), dibuang dari pool lokal, loop ulang dengan kandidat terbaik
   BERIKUTNYA dari pool yang menyusut.
2. **Kandidat tersisa TEPAT SATU** (`length(crit) == 1L`): **dipaksa
   diterima TANPA dites** — baris `break` ini di R ada SEBELUM baris
   `person$valid_item[item] <- FALSE`, jadi item yang diterima lewat
   fallback ini TIDAK ditandai invalid, meski gagal draw `comp` sebelumnya.
   Fallback ini mencegah macet (tanpanya, loop bisa berjalan tanpa akhir
   bila item terakhir kebetulan punya `exposure` sangat rendah).

`sympson_hetter::select()` (Rust) mereplikasi urutan pengecekan ini persis
— lih. `Outcome::forced_accept` untuk menandai kasus (2) secara eksplisit
di jejak debug.

## 3. Persistensi `valid_item` — bukan sekadar satu langkah

Detail non-obvious: `person` di R adalah objek *reference class* (R5),
dimutasi IN-PLACE. `person$valid_item[item] <- FALSE` di dalam satu
pemanggilan `findNextCATItem()` **tetap berlaku** pada pemanggilan
berikutnya (langkah/item selanjutnya) untuk peserta yang sama — item yang
ditolak sekali TIDAK PERNAH dicoba lagi sepanjang SISA sesi peserta itu,
bukan cuma "dilewati untuk giliran ini".

`select_with_exposure()` mereplikasi ini lewat parameter eksplisit
`valid_items: &mut [bool]` yang WAJIB diteruskan pemanggil sebagai slice
yang SAMA di setiap langkah sesi peserta yang sama (lih.
`05-exposure-control-overview.md` §6 untuk alasan ini bukan field
`PersonState`).

## 4. Properti matematis: `exposure=0.0` tidak pernah lolos

Konsekuensi langsung §1: bila `exposure[item] = 0.0`, syarat
`exposure[item] >= comp` hanya benar bila `comp = 0.0` TEPAT — peluang nol
untuk distribusi kontinu. Ini properti **EXACT**, bukan "sangat jarang" —
dibuktikan lewat simulasi 100+ sesi di
`tests/integration_exposure_control.rs::sh_simulation_zero_exposure_item_never_selected_and_start_item_always_selected`
(item exposure=0.0 di fixture R: 0 dari 1800 percobaan selama 300 peserta
sungguhan; Rust: 0 dari simulasi independennya sendiri).

Satu pengecualian: fallback §2 kasus (2) — bila item exposure=0.0 itu
kebetulan jadi SATU-SATUNYA kandidat tersisa di suatu langkah, ia tetap
dipaksa diterima. Ini bukan kontradiksi: forced-accept adalah keputusan
"tidak ada pilihan lain", bukan celah pada uji `exposure[item] >= comp`
itu sendiri.

## 5. `start_item` sepenuhnya bypass exposure control

Item pertama tiap sesi (`design@start_item`) dikembalikan lewat cabang
`findNextCATItem()` yang SAMA SEKALI TERPISAH (`if (all(is.na(person$responses))
&& start) return(design@start_item)`) — jauh sebelum blok
`exposure_type` dievaluasi. Praktis: `start_item` akan selalu terpilih di
langkah pertama SETIAP sesi, terlepas dari nilai `exposure[start_item]`.

## Referensi

- Sympson, J. B., & Hetter, R. D. (1985). Controlling item-exposure rates
  in computerized adaptive testing. *Proceedings of the 27th Annual
  Meeting of the Military Testing Association*, 973–977.
- Georgiadou, E., Triantafillou, E., & Economides, A. (2007). A review of
  item exposure control strategies for computerized adaptive testing
  developed on the item response theory framework. *International
  Journal on E-Learning*, 6(1).
