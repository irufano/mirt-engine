# Skrip R — sumber ground truth untuk parity testing Fase 2 (Item Selection).
#
# Dua jenis fixture dihasilkan, keduanya dari pipeline `mirtCAT()` sungguhan
# (bukan simulasi/pembacaan source saja), pada item bank 3D tetap (10 item,
# gcov non-diagonal supaya varian `*Prule` teruji nyata):
#
# 1) "step0": nilai kriteria MENTAH tiap kandidat (seluruh 10 item, belum
#    ada yang dijawab) untuk setiap kriteria, diambil lewat
#    `computeCriteria()` pada objek `design_elements=TRUE` (state sebelum
#    item pertama dijawab -- diverifikasi lewat inspeksi langsung
#    (`res$person$items_answered` semua NA, `info_thetas` nol) bahwa objek
#    ini memang state pra-simulasi, BUKAN state di tengah simulasi).
#    Mencakup kasus rank-defisien untuk Arule/APrule (info dari 1 kandidat
#    saja, sebelum ada info kumulatif) -- kasus numerik paling ketat.
#
# 2) "sequence": urutan lengkap item yang diadministrasikan
#    (`res$items_answered`) + `res$thetas_history` (theta SEBELUM tiap item
#    dipilih, baris 1 = theta0) dari simulasi `mirtCAT()` end-to-end dengan
#    `local_pattern` tetap, untuk tiap kriteria, dibatasi 6 item pertama
#    (`design$max_items`). Dipakai integration test Rust untuk "replay":
#    inject `thetas_history[k]` sebagai `PersonState::theta` sebelum tiap
#    langkah (menggantikan estimasi ulang Fase 3 yang belum ada), lalu
#    verifikasi `select_next_item()` Rust memilih item yang SAMA persis
#    dengan `items_answered[k]` R di tiap langkah -- termasuk efek
#    `info_thetas` yang dihitung ULANG di theta terbaru tiap langkah
#    (`Person$Update.info_mats`, lih. `PLAN.md` §Fase 2).
#
# `random` sengaja tidak diikutkan (RNG R vs `rand` Rust tidak sama bit demi
# bit, tidak ada perilaku numerik yang bisa dibandingkan tepat).
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures_item_selection.R
# Output: tests/fixtures/item_selection.json

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
kl_delta <- 0.2

set.seed(2024)
pattern <- sample(0:1, n_items, replace = TRUE)

criteria_list <- c(
  "Drule", "DPrule", "Trule", "TPrule", "Arule", "APrule",
  "Erule", "EPrule", "Wrule", "WPrule", "KL", "KLn"
)

## 1) step0: nilai kriteria mentah, 0 item dijawab.
step0 <- list()
for (crit in criteria_list) {
  des <- list(weights = weights, KL_delta = kl_delta)
  res <- mirtCAT(
    mo = obj, criteria = crit, start_item = 1,
    local_pattern = matrix(rep(NA_integer_, n_items), nrow = 1),
    design = des, design_elements = TRUE
  )
  vals <- computeCriteria(res, criteria = crit)
  step0[[crit]] <- list(
    candidate_values = as.numeric(vals),
    candidate_items = as.integer(names(vals))
  )
}

## 2) sequence: simulasi penuh dibatasi 6 item.
sequence <- list()
for (crit in criteria_list) {
  des <- list(weights = weights, KL_delta = kl_delta, max_items = 6)
  res <- mirtCAT(
    mo = obj, criteria = crit, start_item = 1,
    local_pattern = matrix(pattern, nrow = 1),
    design = des
  )
  sequence[[crit]] <- list(
    items_answered = as.integer(res$items_answered),
    thetas_history = lapply(seq_len(nrow(res$thetas_history)), function(i) {
      as.numeric(res$thetas_history[i, ])
    })
  )
}

## 3) seq baseline (bukan kriteria informasi, tapi sanity check urutan).
res_seq <- mirtCAT(
  mo = obj, criteria = "seq", start_item = 1,
  local_pattern = matrix(pattern, nrow = 1),
  design = list(max_items = 6)
)
sequence[["seq"]] <- list(
  items_answered = as.integer(res_seq$items_answered),
  thetas_history = lapply(seq_len(nrow(res_seq$thetas_history)), function(i) {
    as.numeric(res_seq$thetas_history[i, ])
  })
)

out <- list(
  source = paste(
    "mirtCAT::computeCriteria() (step0, state pra-simulasi via design_elements=TRUE)",
    "dan mirtCAT::mirtCAT() end-to-end (sequence, local mode, max_items=6) --",
    "R 4.6.1 / mirt 1.47 / mirtCAT 1.14"
  ),
  tolerance = 1e-6,
  nfact = 3,
  n_items = n_items,
  weights = weights,
  kl_delta = kl_delta,
  gmeans = c(0, 0, 0),
  gcov = lapply(seq_len(3), function(i) as.numeric(gcov[i, ])),
  item_params = list(
    a1 = pars$a1, a2 = pars$a2, a3 = pars$a3, d = pars$d
  ),
  local_pattern = as.integer(pattern),
  step0 = step0,
  sequence = sequence
)

write(
  toJSON(out, auto_unbox = TRUE, digits = 15, pretty = TRUE, null = "null"),
  file = "tests/fixtures/item_selection.json"
)

cat("Fixture ditulis ke tests/fixtures/item_selection.json\n")
