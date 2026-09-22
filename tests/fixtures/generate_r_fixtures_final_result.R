# Skrip R — sumber ground truth untuk parity testing Fase 7 (Hasil Bersih
# Akhir untuk Theta).
#
# Desain sesi SENGAJA disamakan persis dengan skenario `ci_session` Fase 4
# (`generate_r_fixtures_stopping_rule.R`: seed 2024, item params & gcov
# identik) -- sesi itu sudah dikonfirmasi berhenti lebih awal (item ke-4 dari
# 30) dengan `classification` 3-dimensi CAMPURAN (bukan seluruhnya searah),
# kasus paling ketat untuk memverifikasi `McatResult` per-dimensi. Beda dari
# fixture Fase 4 (yang cuma menyimpan `n_answered`/`classification` akhir),
# di sini kita simpan SELURUH field `mirtCAT_post_internal()` yang relevan
# untuk `McatResult`: `thetas`, `SE_thetas`, `thetas_history`,
# `thetas_SE_history`, `classification`, `items_answered`,
# `scored_responses`, `terminated_early`.
#
# **Catatan penting `thetas_history`/`thetas_SE_history`**: R menyisipkan
# baris theta PRA-TES (sebelum item manapun dijawab) sebagai baris pertama
# (`PersonClass.R$initialize`), jadi panjangnya `N_item + 1`, BUKAN `N_item`
# seperti `McatResult` Rust (deviasi sengaja, lih.
# `docs/teori/07-hasil-akhir-theta.md`). Test integrasi Rust men-strip baris
# pertama sebelum membandingkan.
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures_final_result.R
# Output: tests/fixtures/final_result.json

library(mirt)
library(mirtCAT)
library(jsonlite)

set.seed(2024)
n <- 30
a1 <- rep(c(1.0, 1.2, 0.9, 1.1, 1.3, 0.8), 5)
a2 <- rep(c(0.8, 0.6, 1.2, 0.7, 0.5, 1.1), 5)
a3 <- rep(c(0.9, 1.1, 0.7, 1.3, 0.6, 1.0), 5)
d  <- rep(c(0.3, -0.5, 0.0, 0.4, -0.2, 0.1), 5)
pars <- data.frame(a1 = a1, a2 = a2, a3 = a3, d = d)
gcov <- matrix(c(
  1.00,  0.20, -0.10,
  0.20,  1.50,  0.15,
  -0.10, 0.15,  0.80
), 3, 3, byrow = TRUE)
obj <- generate.mirt_object(pars, "2PL", latent_covariance = gcov)
pattern <- sample(0:1, n, replace = TRUE)

classify_cutoffs <- c(0, 0, 0)
classify_ci_value <- 0.05
min_items_ci <- 3L

res <- mirtCAT(
  mo = obj, criteria = "Drule", start_item = 1,
  local_pattern = matrix(pattern, nrow = 1),
  design = list(
    classify = classify_cutoffs, classify_CI = classify_ci_value,
    min_items = min_items_ci, max_items = n
  )
)
stopifnot(length(res$items_answered) < n)
stopifnot(length(unique(as.character(res$classification))) > 1L) # kasus campuran

out <- list(
  source = paste(
    "mirtCAT::mirtCAT() end-to-end (identik skenario ci_session Fase 4) --",
    "R 4.6.1 / mirt 1.47 / mirtCAT 1.14"
  ),
  tolerance = 1e-6,
  nfact = 3,
  n_items = n,
  gmeans = c(0, 0, 0),
  gcov = lapply(seq_len(3), function(i) as.numeric(gcov[i, ])),
  item_params = list(a1 = pars$a1, a2 = pars$a2, a3 = pars$a3, d = pars$d),
  local_pattern = as.integer(pattern),
  classify_cutoffs = classify_cutoffs,
  classify_ci = classify_ci_value,
  min_items = min_items_ci,
  max_items = n,
  result = list(
    thetas = as.numeric(res$thetas[1, ]),
    se_thetas = as.numeric(res$SE_thetas[1, ]),
    thetas_history = lapply(seq_len(nrow(res$thetas_history)), function(i) {
      as.numeric(res$thetas_history[i, ])
    }),
    thetas_se_history = lapply(seq_len(nrow(res$thetas_SE_history)), function(i) {
      as.numeric(res$thetas_SE_history[i, ])
    }),
    classification = as.character(res$classification),
    column_names = colnames(res$thetas),
    items_answered = as.integer(res$items_answered),
    scored_responses = as.integer(res$scored_responses),
    terminated_early = as.logical(res$terminated_early)
  )
)

write(
  toJSON(out, auto_unbox = TRUE, digits = 15, pretty = TRUE, null = "null"),
  file = "tests/fixtures/final_result.json"
)

cat("Fixture ditulis ke tests/fixtures/final_result.json\n")
cat("n_answered:", length(res$items_answered),
    "classification:", as.character(res$classification), "\n")
cat("thetas_history nrow:", nrow(res$thetas_history),
    "(N_item + 1, termasuk baris pra-tes):", length(res$items_answered) + 1, "\n")
