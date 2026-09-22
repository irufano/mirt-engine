# Skrip R — sumber ground truth untuk parity testing Fase 5 (Exposure
# Control). Item bank (10 item, 3D, gcov non-diagonal) & `weights` SAMA
# persis dengan `generate_r_fixtures_item_selection.R` (Fase 2) supaya nilai
# kriteria yang dipakai exposure control sudah tervalidasi lewat fixture
# itu -- fixture ini fokus MURNI pada mekanika exposure control di atasnya.
#
# Tiga jenis fixture, ketiganya diverifikasi langsung terhadap perilaku
# `mirtCAT` 1.14 terpasang (bukan disimulasikan/ditebak):
#
# 1) "mode_detection": beberapa vektor `exposure` contoh + `exposure_type`/
#    `exposure` yang SUNGGUHAN terdeteksi `mirtCAT::Design$initialize()`
#    (diambil dari `res$design@exposure_type`/`res$design@exposure` objek
#    `design_elements=TRUE`, BUKAN dihitung ulang manual) -- perbandingan
#    EKSAK, deterministik, tidak melibatkan RNG sama sekali.
#
# 2) "sample_replay": SATU simulasi end-to-end (`local_pattern` tetap,
#    exposure_type="sample", exposure=3 di semua posisi) -- `items_answered`
#    + `thetas_history` (theta SEBELUM tiap item dipilih, sama seperti
#    fixture Fase 2). Dipakai integration test Rust untuk verifikasi
#    KEANGGOTAAN: `items_answered[k]` (R) harus selalu ada di himpunan top-3
#    kandidat by kriteria Drule pada theta `thetas_history[k]` (dihitung
#    ULANG oleh Rust sendiri lewat `rank_candidates`, BUKAN dibandingkan RNG
#    stream 1:1 -- keduanya random, hanya keanggotaan yang dites, persis
#    `PLAN.md` §Fase 5 checklist integrasi).
#
# 3) "sh_simulation": simulasi BANYAK peserta (exposure_type="SH", vektor
#    exposure per-ID-item dgn variasi 0.0/0.3/0.6/0.8/1.0) -- direkam
#    `item_counts` (frekuensi administrasi tiap item, diagregasi lintas
#    seluruh peserta & langkah). Dipakai integration test Rust untuk
#    verifikasi STATISTIK (bukan match RNG): item exposure=0.0 harus TIDAK
#    PERNAH muncul (properti matematis SH yang exact, bukan cuma "kecil
#    kemungkinan" -- lih. `PLAN.md` §Fase 5 "Perilaku yang Harus Disamai"),
#    item start_item selalu muncul tepat `n_sim` kali (dipaksa sebagai item
#    pertama, bypass exposure control -- lih. `findNextCATItem()`:
#    `if (all(is.na(person$responses)) && start) return(design@start_item)`).
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures_exposure_control.R
# Output: tests/fixtures/exposure_control.json

library(mirt)
library(mirtCAT)
library(jsonlite)

pars <- data.frame(
  a1 = c(1.0, 0.9, 1.1, 0.8, 1.2, 0.7, 1.3, 0.6, 1.0, 0.9),
  a2 = c(0.7, 1.1, 0.6, 1.2, 0.5, 1.3, 0.4, 1.4, 0.8, 1.0),
  a3 = c(0.9, 0.6, 1.2, 0.5, 1.3, 0.4, 1.4, 0.3, 1.1, 0.7),
  d  = c(0.3, -0.5, 0.0, 0.4, -0.2, 0.1, -0.3, 0.2, -0.1, 0.5)
)
gcov <- matrix(c(
  1.00, 0.20, -0.10,
  0.20, 1.50,  0.15,
  -0.10, 0.15,  0.80
), 3, 3, byrow = TRUE)
obj <- generate.mirt_object(pars, "2PL", latent_covariance = gcov)
n_items <- nrow(pars)
weights <- c(1, 0.5, 2)

blank_pattern <- matrix(rep(NA_integer_, n_items), nrow = 1)

## 1) mode_detection: deteksi otomatis SUNGGUHAN dari Design$initialize().
detection_cases <- list(
  list(name = "all_zero_one", values = c(0.0, 0.5, 1.0, 0.2, 0.8, 1.0, 0.0, 0.3, 0.6, 1.0)),
  list(name = "one_above_one", values = c(1, 3, 1, 1, 1, 1, 1, 1, 1, 1)),
  list(name = "all_ones_ambiguous_defaults_to_SH", values = rep(1.0, n_items))
)
mode_detection <- lapply(detection_cases, function(case) {
  res <- mirtCAT(
    mo = obj, criteria = "Drule", start_item = 1,
    local_pattern = blank_pattern,
    design = list(exposure = case$values), design_elements = TRUE
  )
  list(
    name = case$name,
    input = case$values,
    exposure_type = res$design@exposure_type,
    exposure = as.numeric(res$design@exposure)
  )
})

## 2) sample_replay: satu simulasi randomesque (exposure=3 semua posisi).
set.seed(2024)
pattern <- sample(0:1, n_items, replace = TRUE)
exposure_sample <- rep(3L, n_items)
res_sample <- mirtCAT(
  mo = obj, criteria = "Drule", start_item = 5,
  local_pattern = matrix(pattern, nrow = 1),
  design = list(weights = weights, exposure = exposure_sample, max_items = 6)
)
sample_replay <- list(
  start_item = 5L,
  exposure = exposure_sample,
  items_answered = as.integer(res_sample$items_answered),
  thetas_history = lapply(seq_len(nrow(res_sample$thetas_history)), function(i) {
    as.numeric(res_sample$thetas_history[i, ])
  })
)

## 3) sh_simulation: banyak peserta, exposure SH campuran.
set.seed(99)
n_sim <- 300
max_items <- 6
patterns <- matrix(sample(0:1, n_sim * n_items, replace = TRUE), nrow = n_sim)
exposure_sh <- c(0.0, 1.0, 0.3, 1.0, 1.0, 0.6, 1.0, 0.8, 1.0, 1.0)
res_sh <- mirtCAT(
  mo = obj, criteria = "Drule", start_item = 5,
  local_pattern = patterns,
  design = list(weights = weights, exposure = exposure_sh, max_items = max_items)
)
all_items <- unlist(lapply(res_sh, function(x) x$items_answered))
item_counts <- as.integer(sapply(seq_len(n_items), function(i) sum(all_items == i)))

out <- list(
  source = paste(
    "mirtCAT::Design$initialize() (mode_detection, via design_elements=TRUE)",
    "dan mirtCAT::mirtCAT() end-to-end (sample_replay: 1 peserta;",
    "sh_simulation: 300 peserta x 6 item) -- R 4.6.1 / mirt 1.47 / mirtCAT 1.14"
  ),
  nfact = 3,
  n_items = n_items,
  weights = weights,
  gmeans = c(0, 0, 0),
  gcov = lapply(seq_len(3), function(i) as.numeric(gcov[i, ])),
  item_params = list(
    a1 = pars$a1, a2 = pars$a2, a3 = pars$a3, d = pars$d
  ),
  mode_detection = mode_detection,
  sample_replay = sample_replay,
  sh_simulation = list(
    start_item = 5L,
    exposure = exposure_sh,
    n_sim = n_sim,
    max_items = max_items,
    item_counts = item_counts
  )
)

write(
  toJSON(out, auto_unbox = TRUE, digits = 15, pretty = TRUE, null = "null"),
  file = "tests/fixtures/exposure_control.json"
)

cat("Fixture ditulis ke tests/fixtures/exposure_control.json\n")
