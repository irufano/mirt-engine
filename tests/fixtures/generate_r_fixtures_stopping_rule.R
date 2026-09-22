# Skrip R — sumber ground truth untuk parity testing Fase 4 (Stopping Rule).
#
# Metodologi **replay**, persis Fase 2 (item_selection): `mirtCAT()` tidak
# mengekspos state internal `design@met_SEM`/`design@met_classify` dkk.
# pasca-simulasi (dikonfirmasi lewat inspeksi `names(res)` -- field yang
# tersedia hanya `items_answered`, `thetas_history`, `thetas_SE_history`,
# `item_time`, dan `classification`/`classify_values` BILA `classify`/
# `sprt_lower` dikonfigurasi). Strategi:
#
# 1) Satu run BASELINE (`max_items = n`, tanpa kriteria presisi apa pun)
#    untuk mendapatkan trayektori PENUH `thetas_history`/`thetas_SE_history`
#    (30 baris, seluruh item dijawab) -- dipakai Rust untuk "replay":
#    `PersonState::set_estimate` + `record_answer` step demi step, lalu
#    `stopping_rule::evaluate()` dipanggil tiap step.
# 2) Dua run TERPISAH dengan kriteria presisi aktif (`min_SEM` untuk skenario
#    "min_sem_session", `classify`+`classify_CI` untuk skenario
#    "ci_session") yang SUNGGUH berhenti lebih awal dari `n` (dibuktikan
#    `length(items_answered) < n`) -- karena mirtCAT() menjalankan formula
#    stopping rule internal yang SAMA (`Update.stop_now`,
#    `DesignClass.R:312-381`) untuk memutuskan kapan berhenti, titik berhenti
#    sungguhan ini adalah ground truth yang valid: test Rust hanya perlu
#    replay trayektori baseline lewat `stopping_rule::evaluate()` dengan
#    config yang sama, dan verifikasi `stop_now` PERTAMA kali `true` persis
#    di step yang sama.
#
# **Catatan penting soal SPRT** -- lih. `src/stopping_rule/sprt.rs` untuk
# detail lengkap: `personLogLik()` (`mirtCAT` util.R:380) memanggil
# `Theta <- as.matrix(Theta)`, yang untuk `Theta` berupa plain numeric
# vector panjang `nfact > 1` menghasilkan matriks KOLOM (`nfact` observasi
# 1-dimensi terpisah, bukan 1 observasi `nfact`-dimensi) -- bug/limitasi
# upstream, dikonfirmasi: `mirtCAT()` end-to-end dengan
# `design=list(sprt_lower=c(-1,-1,-1), sprt_upper=c(1,1,1))` pada `nfact=3`
# error keras (`'length = 3' in coercion to logical(1)'`). Mitigasi: fixture
# SPRT di sini memanggil `personLogLik()` LANGSUNG dengan `Theta` di-shape
# eksplisit sebagai matriks `1 x nfact` (`matrix(c(...), nrow=1)`), pada
# beberapa prefix trayektori (jumlah item terjawab `k`) dari `local_pattern`
# yang sama -- bukan dari simulasi `mirtCAT()` mode SPRT end-to-end.
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures_stopping_rule.R
# Output: tests/fixtures/stopping_rule.json

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

## 1) Baseline: trayektori penuh (tanpa kriteria presisi).
res_baseline <- mirtCAT(
  mo = obj, criteria = "Drule", start_item = 1,
  local_pattern = matrix(pattern, nrow = 1),
  design = list(max_items = n, min_items = 1)
)
stopifnot(length(res_baseline$items_answered) == n)

## 2) min_sem_session: min_SEM=0.71 -- dikonfirmasi berhenti di item 20 dari 30
##    (SE per-dimensi turun di bawah 0.71 pertama kali persis di baris ke-20
##    `thetas_SE_history` baseline).
min_sem_value <- 0.71
min_items_sem <- 3L
res_sem <- mirtCAT(
  mo = obj, criteria = "Drule", start_item = 1,
  local_pattern = matrix(pattern, nrow = 1),
  design = list(min_SEM = min_sem_value, min_items = min_items_sem, max_items = n)
)
stopifnot(length(res_sem$items_answered) < n)

## 3) ci_session: classify=(0,0,0), classify_CI=0.05 -- dikonfirmasi berhenti
##    di item 4 dengan klasifikasi 3-dimensi CAMPURAN (bukan seluruhnya searah),
##    kasus paling ketat untuk memverifikasi keputusan per-dimensi.
classify_cutoffs <- c(0, 0, 0)
classify_ci_value <- 0.05
min_items_ci <- 3L
res_ci <- mirtCAT(
  mo = obj, criteria = "Drule", start_item = 1,
  local_pattern = matrix(pattern, nrow = 1),
  design = list(
    classify = classify_cutoffs, classify_CI = classify_ci_value,
    min_items = min_items_ci, max_items = n
  )
)
stopifnot(length(res_ci$items_answered) < n)

## 4) sprt_loglik: personLogLik() LANGSUNG (lih. catatan bug di atas) pada
##    beberapa prefix trayektori baseline.
de <- mirtCAT(
  mo = obj, criteria = "Drule", start_item = 1,
  local_pattern = matrix(pattern, nrow = 1),
  design = list(max_items = n, min_items = 1),
  design_elements = TRUE
)
person <- de$person
test <- de$test
personLogLik <- getFromNamespace("personLogLik", "mirtCAT")

sprt_lower <- c(-1, -1, -1)
sprt_upper <- c(1, 1, 1)
sprt_prefixes <- c(1L, 5L, 10L, 15L)
sprt_cases <- lapply(sprt_prefixes, function(k) {
  p2 <- person$copy()
  p2$items_answered[seq_len(k)] <- as.integer(seq_len(k))
  p2$responses[seq_len(k)] <- as.integer(pattern[seq_len(k)])
  ll0 <- personLogLik(person = p2, test = test, Theta = matrix(sprt_lower, nrow = 1))
  ll1 <- personLogLik(person = p2, test = test, Theta = matrix(sprt_upper, nrow = 1))
  list(
    n_answered = k,
    answered_items = as.integer(seq_len(k)),
    responses = as.integer(pattern[seq_len(k)]),
    ll0 = as.numeric(ll0),
    ll1 = as.numeric(ll1)
  )
})

out <- list(
  source = paste(
    "mirtCAT::mirtCAT() end-to-end (baseline trayektori penuh, min_sem_session,",
    "ci_session) dan mirtCAT:::personLogLik() panggilan LANGSUNG (sprt_loglik,",
    "lih. catatan bug as.matrix() upstream di komentar skrip ini) --",
    "R 4.6.1 / mirt 1.47 / mirtCAT 1.14"
  ),
  tolerance = 1e-6,
  nfact = 3,
  n_items = n,
  gmeans = c(0, 0, 0),
  gcov = lapply(seq_len(3), function(i) as.numeric(gcov[i, ])),
  item_params = list(a1 = pars$a1, a2 = pars$a2, a3 = pars$a3, d = pars$d),
  local_pattern = as.integer(pattern),
  baseline = list(
    items_answered = as.integer(res_baseline$items_answered),
    thetas_history = lapply(seq_len(nrow(res_baseline$thetas_history)), function(i) {
      as.numeric(res_baseline$thetas_history[i, ])
    }),
    thetas_SE_history = lapply(seq_len(nrow(res_baseline$thetas_SE_history)), function(i) {
      as.numeric(res_baseline$thetas_SE_history[i, ])
    })
  ),
  min_sem_session = list(
    min_sem = min_sem_value,
    min_items = min_items_sem,
    n_answered = length(res_sem$items_answered)
  ),
  ci_session = list(
    cutoffs = classify_cutoffs,
    classify_ci = classify_ci_value,
    min_items = min_items_ci,
    n_answered = length(res_ci$items_answered),
    classification = as.character(res_ci$classification)
  ),
  sprt_loglik = list(
    lower = sprt_lower,
    upper = sprt_upper,
    cases = sprt_cases
  )
)

write(
  toJSON(out, auto_unbox = TRUE, digits = 15, pretty = TRUE, null = "null"),
  file = "tests/fixtures/stopping_rule.json"
)

cat("Fixture ditulis ke tests/fixtures/stopping_rule.json\n")
cat("min_sem_session n_answered:", length(res_sem$items_answered), "\n")
cat("ci_session n_answered:", length(res_ci$items_answered),
    "classification:", as.character(res_ci$classification), "\n")
