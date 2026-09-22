# Skrip R — sumber ground truth untuk parity testing Fase 3 (Ability
# Estimation).
#
# Beda dari Fase 2 (yang memakai `mirtCAT()` penuh), fixture ini memanggil
# `mirt::fscores()` LANGSUNG -- karena source `mirtCAT::Design$Update.thetas`
# (`DesignClass.R:57-94`) ternyata cuma mendelegasikan penuh ke
# `mirt::fscores()` (dikonfirmasi lewat inspeksi source `mirtCAT` terpasang),
# jadi ground truth algoritma sesungguhnya ada di `mirt`, bukan `mirtCAT`.
#
# Tiap `case` di bawah:
# - Item bank tetap (`generate.mirt_object`, parameter fixed, bukan hasil
#   kalibrasi/EM), `nfact` 2/3/5.
# - `response_pattern` panjang `n_items`, `NA` untuk item yang BELUM
#   dijawab -- padanan `PersonState::answered_items()` Rust yang cuma
#   melacak item terjawab (bukan seluruh bank + filler NA). Diverifikasi
#   empiris (lih. `src/ability_estimation/eap_sum.rs` untuk detail) bahwa
#   `mirt::fscores(response.pattern=...)` dengan sebagian `NA` menghasilkan
#   angka IDENTIK dengan model yang berisi HANYA item terjawab -- jadi
#   fixture ini representatif untuk perilaku Rust apa adanya.
# - `quadpts` eksplisit disamakan dengan tabel default `mirtCAT`
#   (`TestClass.R:58-60`: nfact 1->61,2->31,3->15,4->9,5->7,>5->3) --
#   `mirt::fscores()` sendiri punya default BERBEDA (`121/61/31/19/11/7/5`),
#   jadi WAJIB dioverride manual di sini supaya representatif terhadap
#   `mirtCAT`.
# - Method dievaluasi: EAP, MAP, ML, WLE, EAPsum (skor peserta SEKARANG +
#   tabel skor penuh untuk 1 kasus).
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures_ability_estimation.R
# Output: tests/fixtures/ability_estimation.json

library(mirt)
library(mirtCAT)
library(jsonlite)

mirtcat_quadpts <- function(nfact) {
  switch(as.character(nfact), "1" = 61, "2" = 31, "3" = 15, "4" = 9, "5" = 7, 3)
}

extract_theta_se <- function(fs, nfact) {
  fs <- as.matrix(fs)
  list(
    theta = as.numeric(fs[1, 1:nfact]),
    se = as.numeric(fs[1, (nfact + 1):(2 * nfact)])
  )
}

run_case <- function(name, obj, nfact, item_params, response_pattern, mean_vec, cov_mat, methods) {
  quadpts <- mirtcat_quadpts(nfact)
  rp <- matrix(response_pattern, nrow = 1)

  results <- list()
  for (m in methods) {
    # `response.pattern` + `full.scores` default (TRUE saat response.pattern
    # diberikan) -> output SATU baris `F1..SE_F1..` tanpa kolom Sum.Scores,
    # sama untuk seluruh method termasuk EAPsum (dikonfirmasi empiris).
    fs <- suppressWarnings(fscores(
      obj, method = m, response.pattern = rp,
      quadpts = quadpts, theta_lim = c(-6, 6),
      mean = mean_vec, cov = cov_mat
    ))
    results[[m]] <- extract_theta_se(fs, nfact)
  }

  list(
    name = name,
    nfact = nfact,
    quadpts = quadpts,
    item_params = item_params,
    response_pattern = as.list(ifelse(is.na(response_pattern), NA, response_pattern)),
    gmeans = as.numeric(mean_vec),
    gcov = lapply(seq_len(nrow(cov_mat)), function(i) as.numeric(cov_mat[i, ])),
    results = results
  )
}

## `item_params` sebagai list kolom (a1, a2, [a3, a4, a5,] d) supaya mudah
## direkonstruksi jadi `ItemBank` Rust tanpa parsing tambahan.
as_item_params <- function(pars_df) {
  as.list(pars_df)
}

## Tabel skor penuh EAPsum untuk 1 kasus kecil (dipakai verifikasi
## `eap_sum_score_table`/`by_score` Rust). PENTING: `which.items`/
## `response.pattern` TIDAK membatasi item yang dipakai tabel Sum.Scores --
## `mirt::EAPsum()` selalu memakai SELURUH item di objek model. Jadi supaya
## representatif terhadap `PersonState` Rust (yang cuma menghitung item
## terjawab), objek model di sini harus HANYA berisi item yang benar-benar
## "dijawab" pada skenario -- tidak ada filler yang perlu disingkirkan.
run_eapsum_table <- function(name, obj, nfact, mean_vec, cov_mat) {
  quadpts <- mirtcat_quadpts(nfact)
  fs <- suppressWarnings(fscores(
    obj, method = "EAPsum", quadpts = quadpts, theta_lim = c(-6, 6),
    full.scores = FALSE, mean = mean_vec, cov = cov_mat, verbose = FALSE
  ))
  list(
    name = name,
    nfact = nfact,
    quadpts = quadpts,
    gmeans = as.numeric(mean_vec),
    gcov = lapply(seq_len(nrow(cov_mat)), function(i) as.numeric(cov_mat[i, ])),
    sum_scores = as.integer(fs$Sum.Scores),
    theta = lapply(seq_len(nrow(fs)), function(i) as.numeric(fs[i, 2:(1 + nfact)])),
    se = lapply(seq_len(nrow(fs)), function(i) as.numeric(fs[i, (2 + nfact):(1 + 2 * nfact)]))
  )
}

cases <- list()

## === Bank 2D: 6 item M2PL ===
pars_2d <- data.frame(
  a1 = c(1.2, 0.3, 0.9, 1.0, 0.7, 1.1),
  a2 = c(0.3, 1.2, 0.9, 0.4, 1.0, 0.6),
  d  = c(0.2, -0.1, 0.0, 0.4, -0.3, 0.1)
)
obj_2d <- generate.mirt_object(pars_2d, "2PL")
methods_all <- c("EAP", "MAP", "ML", "WLE", "EAPsum")

## Catatan: pola respons untuk kasus "mixed" dipilih supaya ML memberi
## estimasi FINITE (bukan quasi-separated) -- responden 1,0,1,1,0,1 dites
## awal ternyata membuat pola "semua item loading-dim1 benar, semua
## loading-dim2 salah", cukup separable untuk membuat ML divergen ke
## batas max_theta (fenomena nyata di multidimensional ML, analog "perfect
## separation" regresi logistik) -- kasus itu representatif tapi TIDAK
## representatif untuk perbandingan numerik ketat EAP/MAP/ML, jadi dipisah
## sendiri sebagai kasus "2d_all_correct" (memang didesain ekstrem).
cases[["2d_full_mixed"]] <- run_case(
  "2d_full_mixed", obj_2d, 2, as_item_params(pars_2d),
  c(1, 0, 1, 0, 1, 0),
  c(0, 0), diag(2), methods_all
)

cases[["2d_partial_3of6"]] <- run_case(
  "2d_partial_3of6", obj_2d, 2, as_item_params(pars_2d),
  c(1, NA, 0, NA, 1, NA),
  c(0, 0), diag(2), methods_all
)

cases[["2d_all_correct"]] <- run_case(
  "2d_all_correct", obj_2d, 2, as_item_params(pars_2d),
  c(1, 1, 1, 1, 1, 1),
  c(0, 0), diag(2), c("EAP", "MAP", "ML", "WLE")
)

cases[["2d_narrow_prior_map"]] <- run_case(
  "2d_narrow_prior_map", obj_2d, 2, as_item_params(pars_2d),
  c(1, 0, 1, 1, 0, 1),
  c(0.5, -0.5), diag(c(0.25, 0.25)), c("MAP")
)

cases[["2d_wide_prior_map"]] <- run_case(
  "2d_wide_prior_map", obj_2d, 2, as_item_params(pars_2d),
  c(1, 0, 1, 1, 0, 1),
  c(0.5, -0.5), diag(c(25, 25)), c("MAP")
)

## === Bank 3D: 6 item M2PL ===
pars_3d <- data.frame(
  a1 = c(1.1, 0.3, 0.4, 0.9, 1.0, 0.5),
  a2 = c(0.3, 1.1, 0.4, 0.6, 0.5, 1.0),
  a3 = c(0.4, 0.4, 1.1, 0.5, 0.6, 0.9),
  d  = c(0.1, -0.2, 0.0, 0.3, -0.1, 0.2)
)
obj_3d <- generate.mirt_object(pars_3d, "2PL")

cases[["3d_full_mixed"]] <- run_case(
  "3d_full_mixed", obj_3d, 3, as_item_params(pars_3d),
  c(1, 0, 1, 0, 1, 1),
  c(0, 0, 0), diag(3), methods_all
)

## === Bank 5D: 6 item M2PL, verifikasi eksplisit ukuran grid default
## nfact=5 (quadpts=7, total 7^5=16807 titik) vs Rust ===
pars_5d <- data.frame(
  a1 = c(1.0, 0.2, 0.2, 0.2, 0.2, 0.8),
  a2 = c(0.2, 1.0, 0.2, 0.2, 0.2, 0.7),
  a3 = c(0.2, 0.2, 1.0, 0.2, 0.2, 0.6),
  a4 = c(0.2, 0.2, 0.2, 1.0, 0.2, 0.5),
  a5 = c(0.2, 0.2, 0.2, 0.2, 1.0, 0.4),
  d  = c(0.0, 0.1, -0.1, 0.2, -0.2, 0.0)
)
obj_5d <- generate.mirt_object(pars_5d, "2PL")

cases[["5d_eap_grid_size"]] <- run_case(
  "5d_eap_grid_size", obj_5d, 5, as_item_params(pars_5d),
  c(1, 0, 1, 0, 1, 1),
  c(0, 0, 0, 0, 0), diag(5), c("EAP")
)

## Tabel skor penuh EAPsum: 4 item 2D (cukup degrees-of-freedom tanpa
## filler -- 2^4=16 pola respons > 12 parameter bebas), semuanya "dijawab"
## (tidak ada NA) supaya representasi tabel Rust (item = seluruh
## `person.answered_items()`) match 1:1 dengan model R (seluruh item objek).
pars_eapsum <- data.frame(
  a1 = c(1.2, 0.3, 0.7, 0.5),
  a2 = c(0.3, 0.9, 0.4, 0.8),
  d  = c(0.0, 0.1, -0.1, 0.2)
)
obj_eapsum <- generate.mirt_object(pars_eapsum, "2PL")
eapsum_table <- run_eapsum_table("eapsum_4item_table", obj_eapsum, 2, c(0, 0), diag(2))
eapsum_table$item_params <- as_item_params(pars_eapsum)
eapsum_table$response_pattern <- as.list(c(1, 0, 1, 1))

out <- list(
  source = paste(
    "mirt::fscores() langsung (BUKAN mirtCAT()), karena mirtCAT::Design$Update.thetas",
    "cuma mendelegasikan ke fscores() -- R 4.6.1 / mirt 1.47 / mirtCAT 1.14",
    "(mirtCAT dipakai hanya untuk generate.mirt_object())"
  ),
  tolerance = list(
    eap_map_ml = 1e-4,
    wle_eapsum = 1e-3,
    se = 1e-3
  ),
  cases = cases,
  eapsum_table = eapsum_table
)

write(toJSON(out, auto_unbox = TRUE, digits = 15, pretty = TRUE, na = "null"),
      file = "tests/fixtures/ability_estimation.json")

cat("Fixture ditulis ke tests/fixtures/ability_estimation.json\n")
