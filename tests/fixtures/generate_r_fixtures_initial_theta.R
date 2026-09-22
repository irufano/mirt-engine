# Skrip R — sumber ground truth untuk parity testing Fase 1 (Theta Awal).
#
# Menjalankan pipeline `mirtCAT()` off-line yang sesungguhnya (mode
# `local_pattern`, method MAP) di atas objek `mirt` terkalibrasi (parameter
# tetap + kovarians populasi non-identitas via
# `generate.mirt_object(..., latent_covariance = ...)`, BUKAN hasil
# kalibrasi/EM), lalu membaca baris pertama `thetas_history`/
# `thetas_SE_history` dari objek `Person` yang benar-benar dipakai
# `run_local()` — inilah "theta0"/"SE0" sebelum item pertama dijawab.
#
# Diverifikasi lewat pembacaan langsung source `mirtCAT` terpasang
# (`getRefClass("Person")$methods("initialize")` dan
# `getFromNamespace("run_local", "mirtCAT")`, R 4.6.1/mirtCAT 1.14):
# - `Person$initialize`: `thetas <<- matrix(numeric(nfact), nrow = 1L)`
#   (nol), ditimpa `thetas.start_in` bila diberikan.
# - `run_local()`: `Person$new(..., theta_SEs = sqrt(diag(test@gp$gcov)),
#   thetas.start_in = ...)` — SE awal SELALU dari `sqrt(diag(gcov))`,
#   terlepas dari override theta; hanya diagonal yang diambil.
#
# Tiap kasus diuji dua kali: tanpa override (`thetas.start = NULL`, harus
# theta0 = 0) dan dengan override manual (`thetas.start = c(...)`, harus
# theta0 = nilai override, SE0 tidak berubah).
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures_initial_theta.R
# Output: tests/fixtures/initial_theta.json

library(mirt)
library(mirtCAT)
library(jsonlite)

run_case <- function(obj, nfact, thetas_start = NULL) {
  set.seed(1)
  pattern <- matrix(sample(0:1, length(obj@ParObjects$pars) - 1L, replace = TRUE), nrow = 1)
  design <- if (is.null(thetas_start)) list() else list(thetas.start = thetas_start)
  res <- mirtCAT(mo = obj, local_pattern = pattern, start_item = 1,
                  method = "MAP", criteria = "seq", design = design)
  list(
    theta0 = as.numeric(res$thetas_history[1, ]),
    se0 = as.numeric(res$thetas_SE_history[1, ])
  )
}

cases <- list()

## 1) 2D, gcov non-diagonal: sqrt(diag) = (1.2, 1.5), off-diagonal 0.3
##    (harus diabaikan) -- item pertama + 5 filler 2PL 2D
pars_2d <- data.frame(
  a1 = c(1.2, 1.0, 0.9, 1.1, 0.8, 1.3),
  a2 = c(0.8, 0.7, 1.1, 0.6, 1.2, 0.5),
  d  = c(0.3, -0.5, 0.0, 0.4, -0.2, 0.1)
)
gcov_2d <- matrix(c(1.44, 0.3, 0.3, 2.25), 2, 2)
obj_2d <- generate.mirt_object(pars_2d, "2PL", latent_covariance = gcov_2d)
cases[["dim2_default"]] <- c(
  list(nfact = 2, gmeans = c(0, 0),
       gcov = lapply(seq_len(2), function(i) as.numeric(gcov_2d[i, ])),
       thetas_start = NULL),
  run_case(obj_2d, 2)
)
cases[["dim2_override"]] <- c(
  list(nfact = 2, gmeans = c(0, 0),
       gcov = lapply(seq_len(2), function(i) as.numeric(gcov_2d[i, ])),
       thetas_start = c(0.5, -0.3)),
  run_case(obj_2d, 2, thetas_start = c(0.5, -0.3))
)

## 2) 3D, gcov non-diagonal -- item pertama + 5 filler 2PL 3D
pars_3d <- data.frame(
  a1 = c(0.9, 1.0, 0.8, 1.1, 0.7, 1.2),
  a2 = c(0.7, 0.6, 0.9, 0.5, 1.0, 0.8),
  a3 = c(1.1, 0.9, 1.0, 0.7, 1.2, 0.6),
  d  = c(0.0, 0.3, -0.2, 0.1, -0.4, 0.2)
)
gcov_3d <- matrix(c(
  1.00, 0.10, -0.05,
  0.10, 1.96,  0.20,
  -0.05, 0.20, 0.81
), 3, 3, byrow = TRUE)
obj_3d <- generate.mirt_object(pars_3d, "2PL", latent_covariance = gcov_3d)
cases[["dim3_default"]] <- c(
  list(nfact = 3, gmeans = c(0, 0, 0),
       gcov = lapply(seq_len(3), function(i) as.numeric(gcov_3d[i, ])),
       thetas_start = NULL),
  run_case(obj_3d, 3)
)
cases[["dim3_override"]] <- c(
  list(nfact = 3, gmeans = c(0, 0, 0),
       gcov = lapply(seq_len(3), function(i) as.numeric(gcov_3d[i, ])),
       thetas_start = c(-0.4, 0.8, 0.1)),
  run_case(obj_3d, 3, thetas_start = c(-0.4, 0.8, 0.1))
)

out <- list(
  source = "mirtCAT::mirtCAT() (local mode, method='MAP') -- thetas_history[1,]/thetas_SE_history[1,] dari objek Person sungguhan, via mirtCAT::generate.mirt_object() (R 4.6.1 / mirt 1.47 / mirtCAT 1.14)",
  tolerance = 1e-6,
  cases = cases
)

write(toJSON(out, auto_unbox = TRUE, digits = 15, pretty = TRUE, null = "null"),
      file = "tests/fixtures/initial_theta.json")

cat("Fixture ditulis ke tests/fixtures/initial_theta.json\n")
