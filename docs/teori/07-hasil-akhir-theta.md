# 07 — Hasil Bersih Akhir untuk Theta

> Landasan teori untuk `src/result.rs` (Fase 7).

## 1. Ground truth: `mirtCAT_post_internal()` -- DAN jebakan versi terpasang

Source checkout `mirtCAT/R/mirtCAT_preamble.R:245-285` (`mirtCAT_post_internal`)
merangkum `Person` (state internal sesi, `mirtCAT/R/PersonClass.R`) jadi
`list` kelas `'mirtCAT'` yang dikembalikan `mirtCAT()` ke pemanggil di akhir
sesi. Tapi **source checkout ini TERNYATA BERBEDA dari fungsi yang
sungguh ter-compile di package 1.14 terpasang** -- dikonfirmasi lewat
`getFromNamespace("mirtCAT_post_internal", "mirtCAT")` langsung, pola
temuan yang sama seperti Fase 2/3/6 (checkout != installed).

Dua perbedaan penting hasil pembandingan langsung:

1. **Field `terminated_early` TIDAK PERNAH ada** di objek hasil terpasang.
   Source checkout `mirtCAT_post_internal` menulis `ret$terminated_early <-
   person[[i]]$terminated_early`, tapi versi TERPASANG menulis
   `ret$terminated_sucessfully <- person[[i]]$terminated_sucessfully`
   (field GUI, selalu `FALSE` untuk sesi `local_pattern` offline) --
   BUKAN `terminated_early` sama sekali. Dikonfirmasi lebih dalam:
   `getMethod("Update.stop_now", "Design")` versi terpasang TIDAK PERNAH
   menulis `person$terminated_early <- TRUE` di manapun (checkout
   `DesignClass.R` punya 2 baris itu; versi ter-compile tidak). `logic`
   `stop_now`/`classify_decision` sendiri **identik** antara checkout &
   terpasang -- hanya penulisan field `terminated_early` yang hilang.
2. Struktur field lain (`thetas`, `SE_thetas`, `thetas_history`,
   `thetas_SE_history`, `classification`, `items_answered`,
   `scored_responses`) **konsisten** antara checkout & terpasang,
   dikonfirmasi lewat `str()` pada hasil `mirtCAT()` sungguhan
   (`tests/fixtures/generate_r_fixtures_final_result.R`).

**Implikasi desain**: `McatResult::terminated_early` (`src/result.rs`) tetap
diteruskan dari `StopDecision::terminated_early` Rust (Fase 4) -- field itu
adalah SINYAL INTERNAL yang berguna (stop karena presisi vs `max_items`),
dan logika `Update.stop_now` yang menghasilkannya sendiri sudah
diverifikasi benar terhadap R (lih. `docs/teori/04-stopping-rule-*.md`).
Tapi TIDAK ADA field R literal untuk memparitas-ujikan nilainya secara
langsung -- `tests/integration_final_result.rs` karena itu hanya
memverifikasi bahwa untuk skenario yang SUNGGUH berhenti lebih awal
(dibuktikan `n_answered < max_items` R), `StopDecision::terminated_early`
Rust juga `true`, bukan mencocokkan field R secara eksak (field itu tidak
ada untuk dicocokkan).

## 2. Reuse eksplisit: `ClassifyDecision` & `info_thetas` BUKAN dibangun ulang

Sketsa desain awal `PLAN.md` §Fase 7 mengusulkan `result::ClassifyDecision`
(enum baru) dan accessor `info_thetas` terpisah -- keduanya **duplikat**
yang sudah ada:

- `stopping_rule::ClassifyDecision` (Fase 4, `src/stopping_rule/mod.rs`)
  sudah persis enum yang dibutuhkan (`AboveCutoff`/`BelowCutoff`/
  `NoDecision`) -- dipakai ulang langsung sebagai
  `crate::stopping_rule::ClassifyDecision`.
- `PersonState::info_thetas(&self, bank: &ItemBank)` (Fase 2,
  `src/item_selection/mod.rs`) sudah persis accessor "matriks informasi
  kumulatif, dihitung sesuai permintaan, bukan bagian hasil baku" yang
  dimaksud sketsa awal ("padanan `design_elements=TRUE` +
  `extract.mirtCAT()`"). `McatResult` sengaja TIDAK menyimpan field
  `info_thetas` -- pengguna yang butuh kovarians estimasi manual memanggil
  `person.info_thetas(&bank)` selama `PersonState`/`ItemBank` masih hidup,
  SEBELUM `McatResult::from_person` dipanggil (yang mengonsumsi `&PersonState`
  secara referensi, bukan memindahkan kepemilikannya -- `person` tetap bisa
  dipakai setelahnya).

Ini pola sama seperti Prinsip Desain #1 `PLAN.md` yang sudah diterapkan
berulang di Fase 2 (`argmax` dipakai ulang exposure control), Fase 4
(`log_likelihood_at` dipakai ulang SPRT), dan Fase 6 (`ItemMeta.content`
disiapkan sebelumnya).

## 3. Deviasi `thetas_history`/`thetas_se_history`: `N_item` vs `N_item + 1`

`PersonClass.R$initialize` menyisipkan baris `theta`/SE PRA-TES (sebelum
item manapun dijawab) sebagai baris PERTAMA `thetas_history`/
`thetas_SE_history`:

```r
thetas_history <<- matrix(thetas, 1L, nfact)       # baris ke-1: theta AWAL
thetas_SE_history <<- matrix(theta_SEs, 1L)        # baris ke-1: SE AWAL
```

lalu `Update_thetas()` (dipanggil sekali per item terjawab) me-`rbind`
satu baris baru tiap panggilan -- hasil akhir R punya `nrow = N_item + 1`
(dikonfirmasi lewat fixture `tests/fixtures/final_result.json`: sesi 4
item terjawab, `thetas_history` R 5 baris).

`PersonState::thetas_history()`/`thetas_se_history()`
(`src/item_selection/mod.rs`, Fase 7) SENGAJA tidak menyertakan baris
pra-tes itu -- kosong sampai re-estimasi PERTAMA terjadi (dipanggil dari
`set_theta`/`set_estimate`), sehingga panjangnya persis `N_item`, sejalan
1:1 dengan `answered_items()`/`responses()`. Alasan:

1. **Konsisten dengan `items_answered`/`responses`** -- ketiganya jadi
   sama panjang, indeks `k` di ketiga array itu semua merujuk item
   terjawab ke-`k` yang SAMA. R tidak punya sifat ini (`thetas_history[k+1,
   ]` yang berkorespondensi dengan `items_answered[k]`, off-by-one yang
   gampang keliru dipakai konsumen API).
2. **Judul fase ini secara eksplisit "Hasil BERSIH Akhir"** -- baris
   pra-tes (theta nol/SE prior, sebelum ada informasi apa pun) bukan
   bagian dari "riwayat estimasi", melainkan titik AWAL sebelum estimasi
   ada. Konsumen yang butuh titik awal itu tetap bisa membangunnya sendiri
   dari `initial_theta::initialize()` (Fase 1) -- informasi itu tidak
   hilang, cuma tidak digandakan ke dalam array riwayat.

`tests/integration_final_result.rs` men-strip baris pertama fixture R
sebelum membandingkan (`thetas_history_r[1..]`), didokumentasikan eksplisit
di komentar test.

## 4. Deviasi `classification` untuk mode `Sprt`: selalu panjang `nfact`

`StopDecision::classification` (Fase 4) panjangnya BERVARIASI sesuai
`ClassifyType`: kosong (`None`), `nfact` (`Ci`, per-dimensi), atau `1`
(`Sprt` -- satu keputusan gabungan; `mirtCAT` sendiri menimpa
`person$classify_decision` jadi SKALAR untuk mode ini, dikonfirmasi
`Update.stop_now` terpasang: `person$classify_decision <- decision` tanpa
indeks, menimpa `rep('no decision', nfact)` awal sepenuhnya).

`McatResult::classification` (spesifikasi desain fase ini, `PLAN.md`
§Fase 7 "classification: vektor karakter panjang nfact ... bukan satu
keputusan global") SELALU panjang `nfact`:

- `None` (kosong) -> `vec![NoDecision; nfact]`.
- `Ci` (sudah `nfact`) -> diteruskan apa adanya.
- `Sprt` (panjang 1) -> *broadcast* ke seluruh dimensi. SPRT inheren SATU
  uji hipotesis gabungan (Wald, 1945) atas seluruh vektor `theta`
  sekaligus, bukan `nfact` uji independen -- keputusan yang sama secara
  substantif berlaku untuk tiap dimensi (tidak ada informasi per-dimensi
  yang hilang dengan broadcasting, karena SPRT R sendiri tidak pernah
  menghitung keputusan per-dimensi terpisah untuk mode ini).

`normalize_classification()` (`src/result.rs`) mengimplementasikan
normalisasi ini, diuji lewat ketiga skenario di
`result::tests::classification_*`.

## Referensi

- Segall, D. O. (1996). Multidimensional adaptive testing. *Psychometrika*,
  61(2), 331–354.
- Chalmers, R. P. (2016). Generating Adaptive and Non-Adaptive Test
  Interfaces for Multidimensional Item Response Theory Applications.
  *Journal of Statistical Software*, 71(5), 1–39.
- Wald, A. (1945). Sequential tests of statistical hypotheses. *Annals of
  Mathematical Statistics*, 16(2), 117–186. (dasar SPRT, relevan untuk
  keputusan `classification` gabungan §4)
