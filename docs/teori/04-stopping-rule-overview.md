# 04 — Stopping Rule: Overview

> Landasan teori untuk `src/stopping_rule/` (Fase 4). Lih. dokumen per
> mekanisme untuk rumus & referensi rinci tiap kriteria berhenti.

## 1. Ground truth: `Update.stop_now`

Source `mirtCAT::Design$Update.stop_now` (`mirtCAT/R/DesignClass.R:312-381`,
diverifikasi identik pada dua versi: R 4.6.1/mirtCAT 1.14 terpasang dan
checkout source 1.14.3) mengevaluasi **beberapa kondisi berhenti** dalam
urutan tetap tiap kali satu item selesai dijawab & `theta` diestimasi ulang:

1. Bila `n_answered >= min_items`: evaluasi kriteria presisi
   (`min_SEM`/`Ci`/`Sprt` sesuai mode klasifikasi aktif), LALU (independen)
   `delta_thetas`.
2. `max_items` tercapai.
3. `max_time` tercapai.
4. Guard `allow_constrain_breaks` (§5) — bisa MEMBATALKAN keputusan (1)-(3).
5. Hook `customStop` pengguna — bisa MENIMPA TOTAL seluruh keputusan
   sebelumnya (§6).

`stopping_rule::evaluate()` mereplikasi urutan ini persis. Catatan: konsep
`.Object@stage` R (transisi preCAT -> CAT) diabaikan di sini karena preCAT
di luar scope crate ini (belum diimplementasikan) — tanpa preCAT, `stage`
R selalu `2L` (`> 1L` selalu benar), jadi gerbangnya efektif tidak pernah
menghalangi apa pun.

## 2. Semantik "ALL dimensi" — konsekuensi `theta` sebagai vektor

Beda mendasar MCAT vs CAT unidimensional: `theta` adalah **vektor**
`nfact`-dimensi, bukan skalar. `mirtCAT` (`DesignClass.R:347,355`) memakai
`all()` R untuk menggabungkan status per-dimensi jadi satu keputusan
berhenti:

- `met_SEM = all(SE_j < min_SEM_j for j in 1..nfact)`
- `met_delta_thetas = all(|Δtheta_j| < delta_thetas_j for j in 1..nfact)`
- Mode `CI`: `met_classify = all(z_j < qnorm(alpha) for j in 1..nfact)`

Konsekuensinya: tes **tidak** berhenti hanya karena SATU dimensi sudah
presisi — dimensi yang paling *lambat* konvergen (biasanya dimensi dengan
item pool paling sedikit/lemah relatif terhadapnya) menentukan kapan
seluruh tes berhenti. Lih. §3 untuk implikasi statistik ini pada `nfact`
besar, dan `docs/teori/04-stopping-rule-min-sem.md`/`04-stopping-rule-delta-thetas.md`/
`04-stopping-rule-ci.md` untuk rumus rinci tiap kriteria.

## 3. Implikasi `nfact` besar: `max_items` sebagai batas praktis

Makin banyak `nfact`, makin banyak syarat individual yang harus SEMUA
terpenuhi bersamaan (`all()` atas `nfact` kondisi) — secara statistik ini
makin sulit dipenuhi murni karena ada lebih banyak "kesempatan gagal"
(dimensi mana pun yang masih di atas ambang menahan seluruh tes). Ditambah
lagi, informasi Fisher total per item dibagi ke `nfact` dimensi sekaligus
(satu item dikotomus tunggal berkontribusi ke SEMUA dimensi yang
dimuatinya, bukan hanya satu) — makin banyak dimensi yang harus "dilayani"
oleh item pool yang sama, makin encer informasi per-dimensi per-item.

Akibat praktis (dibuktikan empiris di
`examples/04_stopping_rule_max_items_time.rs`, skenario `nfact=6`):
`min_SEM`/`delta_thetas`/`CI` sering **tidak pernah** terpenuhi sebelum
item bank habis pada model dimensi tinggi — `max_items` (§4) berubah dari
sekadar jaring pengaman teoretis menjadi kondisi berhenti yang PALING SERING
memicu penghentian sungguhan. Ini bukan bug, melainkan konsekuensi
matematis langsung dari semantik "all dimensi" di §2 — didokumentasikan
eksplisit di sini sesuai instruksi `PLAN.md` §Fase 4.

## 4. `max_items`/`max_time`: batas keras independen

Beda dari kriteria presisi (§2), `max_items`/`max_time` dicek TANPA syarat
`n_answered >= min_items` — berlaku sebagai jaring pengaman mutlak,
kapan pun. Weiss & Kingsbury (1984) mencatat pentingnya batas seperti ini
dalam penerapan CAT operasional: kriteria presisi murni bisa menghasilkan
panjang tes yang sangat bervariasi (atau, sesuai §3, tidak pernah
terpenuhi) — batas keras menjamin sesi selalu berakhir dalam jumlah
item/waktu yang terprediksi.

## 5. Guard `allow_constrain_breaks`

`mirtCAT` mendukung constraint administrasi item (`ordered`/`unordered`
block — Fase 6, belum diimplementasikan crate ini). `allow_constrain_breaks
= false` (default) mencegah sesi berhenti **di tengah** blok yang belum
tuntas, walau kriteria presisi/`max_items`/`max_time` sudah terpenuhi —
alasannya, blok `ordered` merepresentasikan unit administrasi yang secara
psikometrik/konten harus disajikan utuh (mis. satu soal cerita dengan
beberapa sub-pertanyaan berurutan).

`stopping_rule::evaluate()` mengambil fakta ini lewat parameter
`in_unfinished_constraint_block: bool` (bukan struct constraint penuh —
lih. catatan deviasi desain di `src/stopping_rule/mod.rs`), karena sistem
constraint sungguhan adalah tanggung jawab Fase 6.

## 6. Hook `CustomStop`: menimpa TOTAL, bukan OR

Detail non-obvious yang perlu ditekankan: `customStop` R (`stop_now <-
customStop(...)`, baris terakhir `Update.stop_now`) **menimpa total** nilai
`stop_now` yang sudah dihitung dari SELURUH kondisi sebelumnya — bukan
digabung lewat `OR`. Ini berarti hook kustom bisa:

- **Membatalkan** keputusan berhenti (termasuk `max_items` sekalipun!) bila
  mengembalikan `false`.
- **Memaksakan** berhenti sekalipun presisi/`max_items`/`max_time` belum
  tercapai, bila mengembalikan `true`.

`stopping_rule::CustomStop` (trait objek `Fn(&PersonState) -> bool`)
mereplikasi semantik "menimpa total" ini persis (lih.
`docs/panduan/04-stopping-rule-overview.md` untuk contoh kode).

## Peta dokumen per-mekanisme

| Mekanisme | Teori | Panduan |
|---|---|---|
| `min_SEM` | `04-stopping-rule-min-sem.md` | `04-stopping-rule-min-sem.md` |
| `delta_thetas` | `04-stopping-rule-delta-thetas.md` | `04-stopping-rule-delta-thetas.md` |
| Klasifikasi `CI` | `04-stopping-rule-ci.md` | `04-stopping-rule-ci.md` |
| Klasifikasi `SPRT` | `04-stopping-rule-sprt.md` | `04-stopping-rule-sprt.md` |
| `max_items`/`max_time` | `04-stopping-rule-max-items-time.md` | `04-stopping-rule-max-items-time.md` |

## Referensi

- Wald, A. (1945). Sequential tests of statistical hypotheses. *Annals of
  Mathematical Statistics*, 16(2), 117–186.
- Weiss, D. J., & Kingsbury, G. G. (1984). Application of computerized
  adaptive testing to educational problems. *Journal of Educational
  Measurement*, 21(4), 361–375.
- van der Linden, W. J., & Glas, C. A. W. (Eds.) (2010). *Elements of
  Adaptive Testing*. Springer.
- Reckase, M. D. (2009). *Multidimensional Item Response Theory*, bab 7.
  Springer.
- Chalmers, R. P. (2012). mirt: A Multidimensional Item Response Theory
  Package for the R Environment. *Journal of Statistical Software*, 48(6),
  1–29.
