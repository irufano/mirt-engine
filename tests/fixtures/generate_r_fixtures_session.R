# Skrip R — sumber ground truth untuk parity testing Fase 8 (McatSession,
# integrasi end-to-end). Beda dari fixture Fase 7 (yang mereplay theta
# LANGSUNG dari fixture, hanya menguji `stopping_rule::evaluate` +
# `McatResult::from_person`), fixture ini dipakai `McatSession` SUNGGUHAN
# (`next_item()`/`submit_response()`) -- pipa item selection + content
# balancing + exposure control + ability estimation + stopping rule
# digabung dalam SATU sesi, persis skenario yang disyaratkan checklist
# integrasi Fase 8 `PLAN.md`.
#
# **Skenario sengaja dibuat DETERMINISTIK PENUH** (pola sama seperti fixture
# Fase 6 `generate_r_fixtures_content_balancing.R`, diperluas): kriteria
# `DPrule` (argmax, bukan RNG), exposure control mode SH dengan SELURUH
# nilai exposure = 1.0 (selalu diterima pada percobaan PERTAMA -- lih.
# `findNextCATItem.R`: `if (design@exposure[item] >= comp && ...) break`,
# `comp <- runif(1,0,1)` selalu <= 1.0, jadi TIDAK PERNAH ada percobaan
# tolak/ulang), content balancing 2 kategori 50/50. Karena tidak ada titik
# manapun yang hasilnya bergantung pada nilai RNG konkret (RNG tetap
# dikonsumsi R/Rust di titik exposure control, tapi hasilnya SELALU sama
# terlepas dari nilainya), `items_answered` bisa dibandingkan EKSAK item
# demi item terhadap `McatSession` Rust, TANPA perlu menyamakan RNG stream
# 1:1 (lih. catatan `PLAN.md` §Fase 5 soal kenapa itu biasanya mustahil).
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures_session.R
# Output: tests/fixtures/session.json

library(mirt)
library(mirtCAT)
library(jsonlite)

pars <- data.frame(
  a1 = c(1.5, 0.3, 1.4, 0.3, 1.6, 0.3, 1.3, 0.3, 1.5, 0.3, 1.4, 0.3),
  a2 = c(0.3, 1.5, 0.3, 1.4, 0.3, 1.6, 0.3, 1.3, 0.3, 1.5, 0.3, 1.4),
  d  = c(0.2, -0.1, 0.3, -0.2, 0.0, 0.1, -0.3, 0.2, -0.1, 0.3, 0.0, -0.2)
)
n_items <- nrow(pars)
gcov <- matrix(c(
  1.0, 0.2,
  0.2, 1.0
), 2, 2, byrow = TRUE)
obj <- generate.mirt_object(pars, "2PL", latent_covariance = gcov)

# item genap (1-based ganjil: 1,3,5,...) kuat di dim1 -> "Aljabar";
# item genap 0-based (2,4,6,...) kuat di dim2 -> "Geometri".
content <- rep(c("Aljabar", "Geometri"), n_items / 2)
content_prop <- c(Aljabar = 0.5, Geometri = 0.5)
exposure_sh <- rep(1.0, n_items)

set.seed(4242)
pattern <- sample(0:1, n_items, replace = TRUE)

res <- mirtCAT(
  mo = obj, criteria = "DPrule", start_item = 1,
  local_pattern = matrix(pattern, nrow = 1),
  design = list(
    content = content,
    content_prop = content_prop,
    exposure = exposure_sh,
    min_items = 4L,
    max_items = n_items,
    min_SEM = 0.55
  )
)

stopifnot(length(res$items_answered) <= n_items)
cat(sprintf(
  "items_answered = %d dari %d (terminated_early diharapkan %s)\n",
  length(res$items_answered), n_items,
  ifelse(length(res$items_answered) < n_items, "TRUE", "FALSE (habis max_items)")
))

out <- list(
  n_items = n_items,
  nfact = 2L,
  # 1e-4 (bukan 1e-6 seperti fixture Fase 7) -- fixture INI menjalankan
  # optimizer MAP Rust SUNGGUHAN tiap langkah (bukan replay nilai theta R
  # langsung), jadi ada beda numerik kecil vs `nlm()` R (beda implementasi
  # optimizer, sama-sama konvergen ke optimum yang sama tapi bukan bit-
  # identik) -- pola toleransi sama seperti `ability_estimation.json`
  # (`eap_map_ml: 1e-4`, Fase 3).
  tolerance = 1e-4,
  item_params = list(a1 = pars$a1, a2 = pars$a2, d = pars$d),
  gcov = list(c(gcov[1, 1], gcov[1, 2]), c(gcov[2, 1], gcov[2, 2])),
  content = content,
  content_prop = as.list(content_prop),
  exposure = exposure_sh,
  start_item = 1L,
  min_items = 4L,
  max_items = n_items,
  min_sem = 0.55,
  local_pattern = pattern,
  result = list(
    items_answered = as.integer(res$items_answered),
    scored_responses = as.integer(res$scored_responses),
    thetas = as.numeric(res$thetas),
    se_thetas = as.numeric(res$SE_thetas),
    thetas_history = lapply(seq_len(nrow(res$thetas_history)), function(i) {
      as.numeric(res$thetas_history[i, ])
    }),
    thetas_se_history = lapply(seq_len(nrow(res$thetas_SE_history)), function(i) {
      as.numeric(res$thetas_SE_history[i, ])
    })
  )
)

write_json(out, "tests/fixtures/session.json", auto_unbox = TRUE, digits = 15)
cat("Fixture ditulis ke tests/fixtures/session.json\n")
