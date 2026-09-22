# Skrip R — sumber ground truth untuk parity testing Fase 6 (Content
# Balancing). Item bank (8 item, 2D) mengikuti pola item bank Fase 5
# (`generate_r_fixtures_exposure_control.R`) tapi disederhanakan jadi 2D
# supaya kategori konten bisa dipetakan langsung ke dimensi ("Aljabar" ->
# item kuat di dim1, "Geometri" -> item kuat di dim2) -- pola yang sama
# dipakai contoh unit test `narrowed_pool_composes_directly_with_rank_candidates`
# di `src/content_balancing/mod.rs`.
#
# Kriteria `Drule`, TANPA exposure control (`exposure_type` default "none")
# -- membuat seluruh sesi DETERMINISTIK PENUH (tie-break kategori & tie-break
# kriteria sama-sama "pertama pada urutan tertentu", tidak ada RNG sama
# sekali begitu `local_pattern` tetap). Ini lebih kuat dari fixture Fase 5
# (yang perlu verifikasi statistik/keanggotaan karena RNG): `items_answered`
# R vs Rust dicocokkan EKSAK, item demi item.
#
# Dua skenario:
#
# 1) "main": content_prop 50/50 dua kategori, item cukup di kedua kategori
#    sepanjang sesi -- membuktikan alternasi kategori mengikuti defisit
#    terbesar dengan benar sepanjang sesi penuh.
#
# 2) "fallback": satu kategori ("Geometri") sengaja HANYA berisi 1 item,
#    target proporsi tinggi (0.5) -- begitu 1 item itu terjawab, kategori
#    itu HABIS tapi target belum terpenuhi (proporsi realisasi cuma
#    1/n_answered, terus < 0.5.  Karena satu-satunya item kategori itu
#    sudah dijawab, `cpick == tmp` tidak pernah match lagi) -- sesi harus
#    tetap lanjut (fallback ke pool penuh, TIDAK error) sampai max_items.
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures_content_balancing.R
# Output: tests/fixtures/content_balancing.json

library(mirt)
library(mirtCAT)
library(jsonlite)

## --- Skenario 1: "main" (50/50, kedua kategori cukup item) ---------------

pars_main <- data.frame(
  a1 = c(2.0, 1.8, 1.9, 2.1, 0.1, 0.1, 0.1, 0.1),
  a2 = c(0.1, 0.1, 0.1, 0.1, 2.0, 1.8, 1.9, 2.1),
  d  = c(0.3, -0.5, 0.0, 0.2, -0.3, 0.4, -0.1, 0.1)
)
n_items_main <- nrow(pars_main)
obj_main <- generate.mirt_object(pars_main, "2PL")
content_main <- c("Aljabar", "Aljabar", "Aljabar", "Aljabar",
                   "Geometri", "Geometri", "Geometri", "Geometri")
content_prop_main <- c(Aljabar = 0.5, Geometri = 0.5)

set.seed(42)
pattern_main <- sample(0:1, n_items_main, replace = TRUE)
res_main <- mirtCAT(
  mo = obj_main, criteria = "Drule", start_item = 1,
  local_pattern = matrix(pattern_main, nrow = 1),
  design = list(
    content = content_main,
    content_prop = content_prop_main,
    max_items = n_items_main
  )
)
main_case <- list(
  n_items = n_items_main,
  item_params = list(a1 = pars_main$a1, a2 = pars_main$a2, d = pars_main$d),
  content = content_main,
  content_prop = as.list(content_prop_main),
  start_item = 1L,
  max_items = n_items_main,
  items_answered = as.integer(res_main$items_answered),
  thetas_history = lapply(seq_len(nrow(res_main$thetas_history)), function(i) {
    as.numeric(res_main$thetas_history[i, ])
  })
)

## --- Skenario 2: "fallback" (kategori "Geometri" cuma 1 item) ------------

pars_fb <- data.frame(
  a1 = c(2.0, 1.8, 1.9, 2.1, 1.7, 0.1),
  a2 = c(0.1, 0.1, 0.1, 0.1, 0.1, 2.0),
  d  = c(0.3, -0.5, 0.0, 0.2, -0.2, -0.1)
)
n_items_fb <- nrow(pars_fb)
obj_fb <- generate.mirt_object(pars_fb, "2PL")
content_fb <- c("Aljabar", "Aljabar", "Aljabar", "Aljabar", "Aljabar", "Geometri")
content_prop_fb <- c(Aljabar = 0.5, Geometri = 0.5)

set.seed(7)
pattern_fb <- sample(0:1, n_items_fb, replace = TRUE)
res_fb <- mirtCAT(
  mo = obj_fb, criteria = "Drule", start_item = 1,
  local_pattern = matrix(pattern_fb, nrow = 1),
  design = list(
    content = content_fb,
    content_prop = content_prop_fb,
    max_items = n_items_fb
  )
)
fallback_case <- list(
  n_items = n_items_fb,
  item_params = list(a1 = pars_fb$a1, a2 = pars_fb$a2, d = pars_fb$d),
  content = content_fb,
  content_prop = as.list(content_prop_fb),
  start_item = 1L,
  max_items = n_items_fb,
  items_answered = as.integer(res_fb$items_answered),
  thetas_history = lapply(seq_len(nrow(res_fb$thetas_history)), function(i) {
    as.numeric(res_fb$thetas_history[i, ])
  })
)

out <- list(
  source = paste(
    "mirtCAT::mirtCAT() end-to-end, criteria='Drule', exposure_type='none'",
    "(deterministik penuh, tanpa RNG kecuali local_pattern) --",
    "R 4.6.1 / mirt 1.47 / mirtCAT 1.14"
  ),
  nfact = 2,
  gmeans = c(0, 0),
  gcov = list(c(1, 0), c(0, 1)),
  main = main_case,
  fallback = fallback_case
)

write(
  toJSON(out, auto_unbox = TRUE, digits = 15, pretty = TRUE, null = "null"),
  file = "tests/fixtures/content_balancing.json"
)

cat("Fixture ditulis ke tests/fixtures/content_balancing.json\n")
