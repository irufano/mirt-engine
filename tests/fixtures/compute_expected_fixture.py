#!/usr/bin/env python3
"""
Kalkulator cross-check INDEPENDEN (bukan R, bukan port dari kode Rust) untuk
fixture parity testing Fase 0 mirtcat-engine.

Sumber formula (dipakai langsung, tidak menyalin kode `mirt`/`mirtCAT`):
  - Dikotomus (M2PL/M3PL/M4PL): Reckase (2009), Multidimensional Item
    Response Theory, bab 4; juga cocok dgn struktur P_dich() di
    mirt/src/traceLinePts.cpp (dibaca sebagai rujukan perilaku, bukan
    disalin -- lih. PLAN.md 1.3).
  - Graded response: Muraki & Carlson (1995); Samejima boundary-curve
    formulation; juga cocok dgn struktur P_graded() di
    mirt/src/traceLinePts.cpp.
  - Informasi Fisher multidimensional (MD): I(theta) = sum_k outer(grad
    P_k, grad P_k) / P_k -- formula umum "outer-product-of-gradients"
    utk exponential family, dipakai jg oleh mirt::iteminfo(multidim_matrix
    = TRUE) (lih. mirt/R/utils.R::ItemInfo2, MD=TRUE cabang).

STATUS: skrip ini sempat dipakai sebagai fixture SEMENTARA saat R belum
terpasang di environment pengembangan. Fixture resmi sekarang dihasilkan
oleh tests/fixtures/generate_r_fixtures.R (R 4.6.1 / mirt 1.47 / mirtCAT
1.14 -- lih. field "source" di tests/fixtures/irt_probability_information.json)
dan hasilnya sudah dicocokkan persis dengan output skrip Python ini (>= 10
digit signifikan), jadi keduanya saling menguatkan. Skrip ini tetap
disimpan sebagai cross-check independen kedua yang tidak butuh R -- berguna
utk verifikasi cepat/CI tanpa toolchain R, atau bila fixture R perlu
diperluas dan ingin dicek dulu secara manual sebelum reinstall R.
"""
import json
import math


def logistic(z: float) -> float:
    z = max(-35.0, min(35.0, z))
    return 1.0 / (1.0 + math.exp(-z))


def dich_probabilities(a, d, g, u, theta):
    z = d + sum(aj * tj for aj, tj in zip(a, theta))
    pstar = logistic(z)
    p1 = g + (u - g) * pstar
    return [1.0 - p1, p1]


def dich_information(a, d, g, u, theta):
    z = d + sum(aj * tj for aj, tj in zip(a, theta))
    pstar = logistic(z)
    p0, p1 = dich_probabilities(a, d, g, u, theta)
    factor = (u - g) * pstar * (1.0 - pstar)
    grad1 = [factor * aj for aj in a]
    scale = 1.0 / p1 + 1.0 / p0
    n = len(a)
    return [[grad1[i] * grad1[j] * scale for j in range(n)] for i in range(n)]


PROB_CLIP = 1e-20


def graded_boundary_curves(a, d, theta):
    pstar = [1.0]
    for dk in d:
        z = dk + sum(aj * tj for aj, tj in zip(a, theta))
        pstar.append(logistic(z))
    pstar.append(0.0)
    return pstar


def graded_probabilities(a, d, theta):
    pstar = graded_boundary_curves(a, d, theta)
    probs = []
    for k in range(len(pstar) - 1):
        p = pstar[k] - pstar[k + 1]
        p = min(max(p, PROB_CLIP), 1.0 - PROB_CLIP)
        probs.append(p)
    return probs


def graded_information(a, d, theta):
    pstar = graded_boundary_curves(a, d, theta)
    probs = graded_probabilities(a, d, theta)
    n = len(a)
    n_boundaries = len(pstar)
    grad_pstar = [[0.0] * n for _ in range(n_boundaries)]
    for k in range(1, n_boundaries - 1):
        deriv = pstar[k] * (1.0 - pstar[k])
        for j in range(n):
            grad_pstar[k][j] = a[j] * deriv
    info = [[0.0] * n for _ in range(n)]
    for k in range(len(probs)):
        grad_p = [grad_pstar[k][j] - grad_pstar[k + 1][j] for j in range(n)]
        p_k = probs[k]
        for i in range(n):
            for j in range(n):
                info[i][j] += grad_p[i] * grad_p[j] / p_k
    return info


def eval_case(nfact, theta_points, prob_fn, info_fn):
    return {
        "nfact": nfact,
        "theta": [list(t) for t in theta_points],
        "probabilities": [prob_fn(t) for t in theta_points],
        "information": [info_fn(t) for t in theta_points],
    }


cases = {}

# 1) M2PL 2D
a, d = [1.2, 0.8], 0.3
theta_pts = [(0.0, 0.0), (1.0, -1.0), (2.0, 2.0)]
cases["m2pl_2d"] = eval_case(
    2, theta_pts,
    lambda t: dich_probabilities(a, d, 0.0, 1.0, t),
    lambda t: dich_information(a, d, 0.0, 1.0, t),
)

# 2) M3PL 2D
a, d, g = [1.0, 1.0], -0.5, 0.2
theta_pts = [(0.0, 0.0)]
cases["m3pl_2d"] = eval_case(
    2, theta_pts,
    lambda t: dich_probabilities(a, d, g, 1.0, t),
    lambda t: dich_information(a, d, g, 1.0, t),
)

# 3) Graded response 2D, 4 kategori
a, d = [1.1, 0.6], [1.5, 0.2, -1.2]
theta_pts = [(-2.0, -2.0), (0.0, 0.0), (2.0, 2.0)]
cases["graded_2d"] = eval_case(
    2, theta_pts,
    lambda t: graded_probabilities(a, d, t),
    lambda t: graded_information(a, d, t),
)

# 4) M2PL 3D
a, d = [0.9, 0.7, 1.1], 0.0
theta_pts = [(0.5, -0.5, 1.0)]
cases["m2pl_3d"] = eval_case(
    3, theta_pts,
    lambda t: dich_probabilities(a, d, 0.0, 1.0, t),
    lambda t: dich_information(a, d, 0.0, 1.0, t),
)

out = {
    "source": "PYTHON_CROSSCHECK (bukan R) -- formula tertutup Reckase (2009)/Muraki & Carlson (1995), "
              "dipakai sementara karena Rscript tidak tersedia di sandbox. Ganti dengan output "
              "tests/fixtures/generate_r_fixtures.R begitu R tersedia.",
    "tolerance": 1e-6,
    "cases": cases,
}

print(json.dumps(out, indent=2))
