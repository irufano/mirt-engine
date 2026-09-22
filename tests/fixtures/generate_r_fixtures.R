# Skrip R — sumber ground truth untuk parity testing Fase 0.
#
# Membangun beberapa item (2D M2PL, 2D M3PL, 2D graded response 4-kategori,
# 3D M2PL) dari parameter tetap (BUKAN hasil kalibrasi/EM) memakai
# `mirtCAT::generate.mirt_object()`, lalu mengekstrak probabilitas kategori
# (`mirt::probtrace()`) dan matriks informasi Fisher penuh
# (`mirt::iteminfo(..., multidim_matrix = TRUE)`) pada beberapa titik theta
# yang sama seperti `examples/00_irt_probability_and_information.rs`.
#
# Catatan implementasi: `generate.mirt_object()` membangun objek `mirt`
# lewat `mirt(..., pars = sv)` dengan data dummy 2-baris (pola respons
# all-0/all-1). Ini memicu pengecekan derajat-kebebasan internal `mirt`
# (ruang pola respons teoretis `prod(K_i)` harus lebih besar dari total
# parameter bebas) -- untuk objek 1-item itu gagal ("too few degrees of
# freedom"). Solusinya BUKAN mengubah data respons (data itu tidak pernah
# dipakai untuk estimasi -- semua parameter di-pasang lewat `pars = sv`),
# melainkan menambah beberapa item "pengisi" (filler) di objek yang sama
# supaya ruang pola respons teoretis cukup besar; hanya item pertama
# (`extract.item(obj, 1)`) yang datanya dipakai sebagai fixture.
#
# Cara pakai (butuh R + paket `mirt`, `mirtCAT`, `jsonlite` terpasang):
#   Rscript tests/fixtures/generate_r_fixtures.R
# Output: tests/fixtures/irt_probability_information.json

library(mirt)
library(mirtCAT)
library(jsonlite)

theta_grid <- function(mat) {
  lapply(seq_len(nrow(mat)), function(i) as.numeric(mat[i, ]))
}

eval_item <- function(obj, item_index, Theta) {
  it <- extract.item(obj, item_index)
  P <- probtrace(it, Theta)
  info_list <- iteminfo(it, Theta, multidim_matrix = TRUE)
  if (nrow(Theta) == 1) info_list <- list(info_list)
  list(
    probabilities = lapply(seq_len(nrow(P)), function(i) as.numeric(P[i, ])),
    information = lapply(info_list, function(m) {
      lapply(seq_len(nrow(m)), function(r) as.numeric(m[r, ]))
    })
  )
}

cases <- list()

## 1) M2PL 2D: a = (1.2, 0.8), d = 0.3 -- item pertama, + 5 filler 2PL 2D
pars_m2pl_2d <- data.frame(
  a1 = c(1.2, 1.0, 0.9, 1.1, 0.8, 1.3),
  a2 = c(0.8, 0.7, 1.1, 0.6, 1.2, 0.5),
  d  = c(0.3, -0.5, 0.0, 0.4, -0.2, 0.1)
)
obj_m2pl_2d <- generate.mirt_object(pars_m2pl_2d, "2PL")
theta_m2pl_2d <- rbind(c(0, 0), c(1, -1), c(2, 2))
cases[["m2pl_2d"]] <- c(
  list(nfact = 2, theta = theta_grid(theta_m2pl_2d)),
  eval_item(obj_m2pl_2d, 1, theta_m2pl_2d)
)

## 2) M3PL 2D: a = (1.0, 1.0), d = -0.5, g = 0.2 -- item pertama, + 5 filler 3PL 2D
pars_m3pl_2d <- data.frame(
  a1 = c(1.0, 1.1, 0.9, 1.2, 0.8, 1.0),
  a2 = c(1.0, 0.7, 1.1, 0.6, 1.2, 0.9),
  d  = c(-0.5, 0.2, -0.1, 0.4, -0.3, 0.0),
  g  = c(0.2, 0.15, 0.25, 0.1, 0.2, 0.18)
)
obj_m3pl_2d <- generate.mirt_object(pars_m3pl_2d, "3PL")
theta_m3pl_2d <- rbind(c(0, 0))
cases[["m3pl_2d"]] <- c(
  list(nfact = 2, theta = theta_grid(theta_m3pl_2d)),
  eval_item(obj_m3pl_2d, 1, theta_m3pl_2d)
)

## 3) Graded response 2D, 4 kategori: a = (1.1, 0.6), d = (1.5, 0.2, -1.2)
##    -- item pertama, + 5 filler graded 2D 4-kategori
pars_graded_2d <- data.frame(
  a1 = c(1.1, 1.0, 0.9, 1.2, 0.8, 1.0),
  a2 = c(0.6, 0.7, 1.0, 0.5, 0.9, 0.6),
  d1 = c(1.5, 1.2, 1.8, 1.0, 1.4, 1.6),
  d2 = c(0.2, 0.1, 0.4, -0.1, 0.3, 0.0),
  d3 = c(-1.2, -1.0, -0.8, -1.5, -1.1, -1.3)
)
obj_graded_2d <- generate.mirt_object(pars_graded_2d, "graded")
theta_graded_2d <- rbind(c(-2, -2), c(0, 0), c(2, 2))
cases[["graded_2d"]] <- c(
  list(nfact = 2, theta = theta_grid(theta_graded_2d)),
  eval_item(obj_graded_2d, 1, theta_graded_2d)
)

## 4) M2PL 3D: a = (0.9, 0.7, 1.1), d = 0.0 -- item pertama, + 5 filler 2PL 3D
pars_m2pl_3d <- data.frame(
  a1 = c(0.9, 1.0, 0.8, 1.1, 0.7, 1.2),
  a2 = c(0.7, 0.6, 0.9, 0.5, 1.0, 0.8),
  a3 = c(1.1, 0.9, 1.0, 0.7, 1.2, 0.6),
  d  = c(0.0, 0.3, -0.2, 0.1, -0.4, 0.2)
)
obj_m2pl_3d <- generate.mirt_object(pars_m2pl_3d, "2PL")
theta_m2pl_3d <- rbind(c(0.5, -0.5, 1.0))
cases[["m2pl_3d"]] <- c(
  list(nfact = 3, theta = theta_grid(theta_m2pl_3d)),
  eval_item(obj_m2pl_3d, 1, theta_m2pl_3d)
)

out <- list(
  source = "mirt::probtrace() + mirt::iteminfo(multidim_matrix=TRUE), via mirtCAT::generate.mirt_object() (R 4.6.1 / mirt 1.47 / mirtCAT 1.14)",
  tolerance = 1e-6,
  cases = cases
)

write(toJSON(out, auto_unbox = TRUE, digits = 15, pretty = TRUE),
      file = "tests/fixtures/irt_probability_information.json")

cat("Fixture ditulis ke tests/fixtures/irt_probability_information.json\n")
