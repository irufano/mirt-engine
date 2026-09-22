//! Fase 6 — Content Balancing pada MCAT: menjaga proporsi kategori konten
//! item yang disajikan mendekati target yang ditentukan pengguna, beroperasi
//! **sebelum** exposure control (Fase 5) dalam pipeline seleksi item.
//!
//! Ground truth: `mirtCAT::Design$initialize()` (setup `content`/
//! `content_prop`) dan `mirtCAT::findNextCATItem()` (cabang
//! `design@use_content`), keduanya diperiksa langsung dari source terpasang
//! mirtCAT 1.14 (`getMethod("initialize", "Design")`,
//! `getFromNamespace("findNextCATItem", "mirtCAT")`) — bukan hanya dibaca
//! dari `PLAN.md`.
//!
//! **Temuan yang mengoreksi/memperjelas sketsa awal `PLAN.md` §Fase 6**:
//!
//! 1. **`content_prop` WAJIB menjumlah persis 1** (toleransi `1e-6`) —
//!    `Design$initialize()`: `if (!closeEnough(sum(design$content_prop)-1,
//!    -1e-6, 1e-6)) stop("content_prop does not sum to 1")`. Sketsa awal
//!    `PLAN.md` menduga ini fleksibel ("tidak wajib persis 1.0") — keliru,
//!    dikoreksi di sini: [`ContentBalancingConfig::new`] memvalidasi sum
//!    dengan toleransi yang sama.
//! 2. **Hanya `Criterion::Seq` yang dilarang**, BUKAN `Criterion::Random` —
//!    `Design$initialize()`: `if (use_content && criteria == "seq")
//!    stop("content designs are not supported for seq criteria")`.
//!    `findNextCATItem()` punya cabang content-balancing terpisah (duplikat
//!    kode, bukan bug) di dalam blok `criteria == "random"`, fungsional
//!    setara "sempitkan ke kategori prioritas, fallback ke pool penuh bila
//!    kategori itu habis" — sama seperti kriteria berskor. Direplikasi
//!    lewat [`ContentBalancingConfig::validate_with_criterion`].
//! 3. **Urutan kerja nyata `findNextCATItem()`** (kriteria berskor): `crit`/
//!    `index` (skor SELURUH kandidat belum dijawab) dihitung dulu, BARU
//!    disempitkan berdasar kategori, BARU exposure control jalan di atas
//!    hasil sempitan. Karena skor tiap kandidat independen dari kandidat
//!    lain, menyempitkan pool `ItemIndex` SEBELUM ranking (urutan
//!    [`narrow_candidates`] di modul ini) menghasilkan nilai numerik
//!    identik dengan menyempitkan SESUDAH ranking (urutan R) — dibuktikan
//!    lewat replay fixture R di `tests/integration_content_balancing.rs`
//!    (kecocokan EKSAK item demi item, bukan cuma keanggotaan set, karena
//!    skenario fixture deterministik penuh: kriteria `Drule` tanpa exposure
//!    control, tie-break "pertama" konsisten di kedua sisi).
//! 4. **`ItemMeta.content: Option<String>` (`src/bank.rs`) sudah disiapkan
//!    sebelumnya untuk fase ini**, tapi [`ContentBalancingConfig`] SENGAJA
//!    tetap decoupled dari `ItemBank` (pola sama seperti
//!    `exposure_control::ExposureControlConfig`, yang menyimpan `Vec<f64>`
//!    sendiri) — `category_per_item` dikonstruksi eksplisit oleh pemanggil
//!    (nanti `McatSession` Fase 8, dari `bank.meta(i).content`), supaya
//!    modul ini bisa diuji tanpa perlu `ItemBank` sungguhan.

use std::collections::{BTreeSet, HashMap};

use crate::error::MirtCatError;
use crate::item_selection::{Criterion, ItemIndex};

/// Kategori konten item — newtype `String` polos (bukan interning; jumlah
/// kategori konten pada praktiknya kecil/tetap, tidak ada dependency
/// interning di crate ini, menambahnya tidak proporsional untuk kasus ini).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContentCategory(pub String);

impl ContentCategory {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

impl From<&str> for ContentCategory {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl From<String> for ContentCategory {
    fn from(s: String) -> Self {
        Self(s)
    }
}

/// Konfigurasi content balancing.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContentBalancingConfig {
    /// Kategori tiap item di bank, diindeks per `ItemIndex` — panjang &
    /// kecocokan dengan bank sesungguhnya adalah tanggung jawab pemanggil
    /// (config ini tidak menyimpan referensi `ItemBank`, lih. catatan
    /// modul poin 4); `narrow_candidates` mengembalikan error eksplisit
    /// bila ada index di luar jangkauan saat dipanggil.
    category_per_item: Vec<ContentCategory>,
    /// Target proporsi tiap kategori — mencakup SELURUH kategori yang
    /// muncul di `category_per_item` (divalidasi di `new`), boleh berisi
    /// entri ekstra yang tidak dipakai (padanan `mirtCAT`: `content_prop`
    /// utuh divalidasi menjumlah 1 SEBELUM direindeks ke kategori yang
    /// benar-benar ada).
    target_prop: HashMap<ContentCategory, f64>,
    /// Urutan kategori deterministik (alfabetis, padanan urutan level
    /// `factor()` R) — dipakai tie-break "kategori pertama" saat beberapa
    /// kategori punya defisit sama besar, sama seperti `which.max` R.
    category_order: Vec<ContentCategory>,
    /// Aktifkan jejak debug (prinsip desain #7 `PLAN.md`). Default `false`.
    pub debug: bool,
}

impl ContentBalancingConfig {
    /// Validasi (lih. catatan modul poin 1 utk rasional toleransi sum):
    /// - `category_per_item` tidak boleh kosong.
    /// - `target_prop` tidak boleh berisi nilai negatif.
    /// - `target_prop` harus menjumlah `1.0` (toleransi `1e-6`) — padanan
    ///   persis `content_prop does not sum to 1` mirtCAT.
    /// - Setiap kategori yang muncul di `category_per_item` WAJIB punya
    ///   entri di `target_prop` — error eksplisit, BUKAN default diam-diam
    ///   ke `0.0` (deviasi sengaja dari opsi "(atau default 0)" di sketsa
    ///   awal `PLAN.md`, mencegah typo nama kategori bocor jadi target 0
    ///   yang tidak disengaja).
    pub fn new(
        category_per_item: Vec<ContentCategory>,
        target_prop: HashMap<ContentCategory, f64>,
        debug: bool,
    ) -> Result<Self, MirtCatError> {
        if category_per_item.is_empty() {
            return Err(MirtCatError::InvalidParameter(
                "ContentBalancingConfig::new: category_per_item kosong".into(),
            ));
        }

        if target_prop.values().any(|&p| p < 0.0) {
            return Err(MirtCatError::InvalidParameter(
                "ContentBalancingConfig::new: target_prop tidak boleh berisi nilai negatif".into(),
            ));
        }

        let sum: f64 = target_prop.values().sum();
        if (sum - 1.0).abs() > 1e-6 {
            return Err(MirtCatError::InvalidParameter(format!(
                "ContentBalancingConfig::new: target_prop harus menjumlah 1.0 (didapat {sum}) -- \
                 padanan validasi `content_prop does not sum to 1` mirtCAT"
            )));
        }

        let category_order: Vec<ContentCategory> = category_per_item
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        for cat in &category_order {
            if !target_prop.contains_key(cat) {
                return Err(MirtCatError::InvalidParameter(format!(
                    "ContentBalancingConfig::new: kategori '{}' muncul di category_per_item \
                     tapi tidak ada entri target_prop untuknya",
                    cat.0
                )));
            }
        }

        Ok(Self {
            category_per_item,
            target_prop,
            category_order,
            debug,
        })
    }

    /// Validasi larangan eksplisit `mirtCAT`: content balancing tidak
    /// kompatibel dengan `Criterion::Seq` (lih. catatan modul poin 2).
    /// `Criterion::Random` dan seluruh kriteria berskor DIDUKUNG.
    pub fn validate_with_criterion(&self, criterion: Criterion) -> Result<(), MirtCatError> {
        if criterion == Criterion::Seq {
            return Err(MirtCatError::ContentBalancingIncompatibleWithSeq);
        }
        Ok(())
    }
}

/// Jejak debug opsional dari [`narrow_candidates`] — hanya `Some` bila
/// `ContentBalancingConfig::debug == true`; tidak memengaruhi pool kandidat
/// yang dikembalikan (prinsip desain #7 `PLAN.md`).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContentBalancingDebug {
    /// Proporsi empiris tiap kategori dari `answered_items`, SELURUH
    /// kategori di `category_order` selalu muncul (0 bila belum pernah
    /// terjawab) — padanan `factor()` + `table()` R.
    pub empirical_prop: HashMap<ContentCategory, f64>,
    /// `target - empirical` tiap kategori, urutan evaluasi sama seperti
    /// `category_order`.
    pub deficits: HashMap<ContentCategory, f64>,
    /// Kategori yang pembatasannya BENAR-BENAR diterapkan pada pool yang
    /// dikembalikan — `None` bila `fallback_used == true` (tidak ada
    /// pembatasan aktif, pool dikembalikan utuh).
    pub chosen_category: Option<ContentCategory>,
    /// `true` bila kategori prioritas kehabisan kandidat di `candidate_pool`
    /// sehingga seleksi kembali ke pool penuh tanpa pembatasan.
    pub fallback_used: bool,
}

/// Sempitkan `candidate_pool` ke kategori konten yang paling kurang jatah
/// (`argmax(target - empirical)`, tie-break "kategori pertama" secara
/// alfabetis — persis `which.max` R pada `names(dif)` terurut `factor()`),
/// fallback ke `candidate_pool` UTUH bila kategori itu tidak punya kandidat
/// tersisa di `candidate_pool` (bukan error — padanan `mirtCAT`:
/// `if (sum(pick) > 0L) { index <- index[pick]; crit <- crit[pick] }`, tanpa
/// `else`, jadi `index`/`crit` tetap utuh bila `pick` semuanya `FALSE`).
///
/// `answered_items` dipakai menghitung proporsi empiris — kategori yang
/// belum pernah terjawab tetap terhitung `0.0`, bukan hilang dari
/// perhitungan (lih. catatan modul poin 3 & dokumentasi `PLAN.md` §Fase 6).
///
/// Error bila `candidate_pool` kosong (padanan "kehabisan item untuk
/// disajikan" — kondisi ini semestinya sudah dicegah pemanggil sebelum
/// pipeline sampai ke sini, sama seperti `rank_candidates`/
/// `select_with_exposure`), atau ada index di `answered_items`/
/// `candidate_pool` di luar jangkauan `category_per_item`.
pub fn narrow_candidates(
    config: &ContentBalancingConfig,
    answered_items: &[ItemIndex],
    candidate_pool: &[ItemIndex],
) -> Result<(Vec<ItemIndex>, Option<ContentBalancingDebug>), MirtCatError> {
    if candidate_pool.is_empty() {
        return Err(MirtCatError::InvalidParameter(
            "content_balancing::narrow_candidates: candidate_pool kosong -- tidak ada kandidat \
             untuk disempitkan"
                .into(),
        ));
    }

    let category_of = |idx: ItemIndex| -> Result<ContentCategory, MirtCatError> {
        config.category_per_item.get(idx).cloned().ok_or_else(|| {
            MirtCatError::InvalidParameter(format!(
                "content_balancing::narrow_candidates: index item {idx} di luar jangkauan \
                 category_per_item"
            ))
        })
    };

    let mut counts: HashMap<ContentCategory, usize> = config
        .category_order
        .iter()
        .map(|c| (c.clone(), 0usize))
        .collect();
    for &idx in answered_items {
        let cat = category_of(idx)?;
        *counts.get_mut(&cat).expect(
            "kategori item terjawab selalu ada di category_order (bagian category_per_item)",
        ) += 1;
    }

    let total_answered = answered_items.len() as f64;
    let empirical_prop: HashMap<ContentCategory, f64> = counts
        .into_iter()
        .map(|(cat, n)| {
            let p = if total_answered == 0.0 {
                0.0
            } else {
                n as f64 / total_answered
            };
            (cat, p)
        })
        .collect();

    let mut deficits: HashMap<ContentCategory, f64> =
        HashMap::with_capacity(config.category_order.len());
    let mut priority: Option<(ContentCategory, f64)> = None;
    for cat in &config.category_order {
        let target = config.target_prop[cat];
        let empirical = empirical_prop[cat];
        let deficit = target - empirical;
        deficits.insert(cat.clone(), deficit);
        priority = match priority {
            Some((_, best)) if best >= deficit => priority,
            _ => Some((cat.clone(), deficit)),
        };
    }
    let priority_category = priority.map(|(cat, _)| cat).ok_or_else(|| {
        MirtCatError::InvalidParameter(
            "content_balancing::narrow_candidates: category_order kosong (tidak seharusnya \
                 terjadi -- ContentBalancingConfig::new memvalidasi category_per_item non-kosong)"
                .into(),
        )
    })?;

    let mut narrowed = Vec::new();
    for &idx in candidate_pool {
        if category_of(idx)? == priority_category {
            narrowed.push(idx);
        }
    }

    let fallback_used = narrowed.is_empty();
    let (result, chosen_category) = if fallback_used {
        (candidate_pool.to_vec(), None)
    } else {
        (narrowed, Some(priority_category))
    };

    let debug = config.debug.then_some(ContentBalancingDebug {
        empirical_prop,
        deficits,
        chosen_category,
        fallback_used,
    });

    Ok((result, debug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bank::ItemMeta;
    use crate::irt::Dichotomous;
    use crate::item_selection::{argmax, rank_candidates, PersonState, SelectionConfig};
    use crate::{GroupParams, ItemBank};

    fn two_categories() -> (Vec<ContentCategory>, HashMap<ContentCategory, f64>) {
        // item 0,1,2 -> "Aljabar"; item 3,4 -> "Geometri".
        let categories = vec![
            ContentCategory::from("Aljabar"),
            ContentCategory::from("Aljabar"),
            ContentCategory::from("Aljabar"),
            ContentCategory::from("Geometri"),
            ContentCategory::from("Geometri"),
        ];
        let mut target = HashMap::new();
        target.insert(ContentCategory::from("Aljabar"), 0.5);
        target.insert(ContentCategory::from("Geometri"), 0.5);
        (categories, target)
    }

    #[test]
    fn new_rejects_empty_category_per_item() {
        let err = ContentBalancingConfig::new(vec![], HashMap::new(), false).unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn new_rejects_negative_target_prop() {
        let mut target = HashMap::new();
        target.insert(ContentCategory::from("A"), -0.5);
        target.insert(ContentCategory::from("B"), 1.5);
        let err = ContentBalancingConfig::new(
            vec![ContentCategory::from("A"), ContentCategory::from("B")],
            target,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn new_rejects_target_prop_not_summing_to_one() {
        let mut target = HashMap::new();
        target.insert(ContentCategory::from("A"), 0.3);
        target.insert(ContentCategory::from("B"), 0.3);
        let err = ContentBalancingConfig::new(
            vec![ContentCategory::from("A"), ContentCategory::from("B")],
            target,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn new_rejects_category_missing_target_prop_entry() {
        let mut target = HashMap::new();
        target.insert(ContentCategory::from("A"), 1.0);
        // "B" muncul di category_per_item tapi tidak ada di target_prop.
        let err = ContentBalancingConfig::new(
            vec![ContentCategory::from("A"), ContentCategory::from("B")],
            target,
            false,
        )
        .unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn validate_with_criterion_rejects_seq_but_allows_random() {
        let (categories, target) = two_categories();
        let config = ContentBalancingConfig::new(categories, target, false).unwrap();

        let err = config.validate_with_criterion(Criterion::Seq).unwrap_err();
        assert_eq!(err, MirtCatError::ContentBalancingIncompatibleWithSeq);

        assert!(config.validate_with_criterion(Criterion::Random).is_ok());
        assert!(config
            .validate_with_criterion(Criterion::DRule { prior: false })
            .is_ok());
    }

    #[test]
    fn unanswered_category_counts_as_zero_not_missing() {
        let (categories, target) = two_categories();
        let config = ContentBalancingConfig::new(categories, target, true).unwrap();

        // Belum ada item terjawab -- kedua kategori harus tetap 0.0, bukan
        // hilang dari perhitungan.
        let (_, debug) = narrow_candidates(&config, &[], &[0, 1, 2, 3, 4]).unwrap();
        let debug = debug.unwrap();
        assert_eq!(
            debug.empirical_prop.get(&ContentCategory::from("Aljabar")),
            Some(&0.0)
        );
        assert_eq!(
            debug.empirical_prop.get(&ContentCategory::from("Geometri")),
            Some(&0.0)
        );
    }

    #[test]
    fn category_with_largest_deficit_is_prioritized_among_three() {
        let categories = vec![
            ContentCategory::from("A"),
            ContentCategory::from("B"),
            ContentCategory::from("C"),
            ContentCategory::from("A"),
            ContentCategory::from("B"),
            ContentCategory::from("C"),
        ];
        let mut target = HashMap::new();
        target.insert(ContentCategory::from("A"), 0.2);
        target.insert(ContentCategory::from("B"), 0.3);
        target.insert(ContentCategory::from("C"), 0.5);
        let config = ContentBalancingConfig::new(categories, target, true).unwrap();

        // Sudah terjawab: 1x A, 1x B, 0x C -- empirical A=0.5, B=0.5, C=0.0.
        // Defisit: A=0.2-0.5=-0.3, B=0.3-0.5=-0.2, C=0.5-0.0=0.5 -- C menang
        // telak.
        let answered = [0usize, 1usize];
        let (narrowed, debug) = narrow_candidates(&config, &answered, &[2, 3, 4, 5]).unwrap();
        assert_eq!(narrowed, vec![2, 5]); // index kategori C di candidate_pool
        assert_eq!(
            debug.unwrap().chosen_category,
            Some(ContentCategory::from("C"))
        );
    }

    #[test]
    fn falls_back_to_full_pool_when_priority_category_has_no_candidates_left() {
        let (categories, target) = two_categories();
        let config = ContentBalancingConfig::new(categories, target, true).unwrap();

        // Belum ada terjawab -> defisit sama (0.5 vs 0.5) -> tie-break
        // alfabetis pertama -> "Aljabar" jadi prioritas. Tapi candidate_pool
        // hanya berisi item kategori "Geometri" (index 3, 4) -- harus
        // fallback ke pool penuh, BUKAN error.
        let (narrowed, debug) = narrow_candidates(&config, &[], &[3, 4]).unwrap();
        assert_eq!(narrowed, vec![3, 4]);
        let debug = debug.unwrap();
        assert!(debug.fallback_used);
        assert_eq!(debug.chosen_category, None);
    }

    #[test]
    fn debug_off_does_not_change_narrowed_pool_vs_debug_on() {
        let (categories, target) = two_categories();

        let config_off =
            ContentBalancingConfig::new(categories.clone(), target.clone(), false).unwrap();
        let (narrowed_off, debug_off) =
            narrow_candidates(&config_off, &[], &[0, 1, 2, 3, 4]).unwrap();
        assert!(debug_off.is_none());

        let config_on = ContentBalancingConfig::new(categories, target, true).unwrap();
        let (narrowed_on, debug_on) = narrow_candidates(&config_on, &[], &[0, 1, 2, 3, 4]).unwrap();
        assert!(debug_on.is_some());

        assert_eq!(narrowed_off, narrowed_on);
    }

    #[test]
    fn errors_on_empty_candidate_pool() {
        let (categories, target) = two_categories();
        let config = ContentBalancingConfig::new(categories, target, false).unwrap();
        let err = narrow_candidates(&config, &[], &[]).unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    #[test]
    fn errors_on_out_of_range_candidate_index() {
        let (categories, target) = two_categories();
        let config = ContentBalancingConfig::new(categories, target, false).unwrap();
        let err = narrow_candidates(&config, &[], &[999]).unwrap_err();
        assert!(matches!(err, MirtCatError::InvalidParameter(_)));
    }

    /// Uji komposisi (checklist `PLAN.md` §Fase 6: "urutan pipeline: hasil
    /// `narrow_candidates()` dipakai sebagai input exposure control") --
    /// membuktikan output `narrow_candidates()` bisa langsung dikonsumsi
    /// `item_selection::rank_candidates` + `argmax` tanpa penyesuaian.
    #[test]
    fn narrowed_pool_composes_directly_with_rank_candidates() {
        let mut bank = ItemBank::new();
        // item0/item1 kuat di dim1 ("Aljabar"), item2/item3 kuat di dim2
        // ("Geometri").
        bank.push(
            Box::new(Dichotomous::m2pl(vec![2.0, 0.1], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.8, 0.1], 0.0).unwrap()),
            ItemMeta::new(1),
        )
        .unwrap();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![0.1, 2.0], 0.0).unwrap()),
            ItemMeta::new(2),
        )
        .unwrap();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![0.1, 1.8], 0.0).unwrap()),
            ItemMeta::new(3),
        )
        .unwrap();

        let categories = vec![
            ContentCategory::from("Aljabar"),
            ContentCategory::from("Aljabar"),
            ContentCategory::from("Geometri"),
            ContentCategory::from("Geometri"),
        ];
        let mut target = HashMap::new();
        target.insert(ContentCategory::from("Aljabar"), 0.0);
        target.insert(ContentCategory::from("Geometri"), 1.0);
        let config = ContentBalancingConfig::new(categories, target, false).unwrap();

        let person = PersonState::new(2);
        let group = GroupParams::standard_normal(2);
        let sel_config = SelectionConfig {
            criterion: Criterion::DRule { prior: false },
            ..Default::default()
        };

        let candidates: Vec<ItemIndex> = (0..bank.len()).collect();
        let (narrowed, _) =
            narrow_candidates(&config, person.answered_items(), &candidates).unwrap();
        // Target Geometri=1.0 -> selalu prioritas -> narrowed cuma item
        // kategori Geometri (index 2, 3).
        assert_eq!(narrowed, vec![2, 3]);

        let scores = rank_candidates(&sel_config, &narrowed, &person, &bank, &group).unwrap();
        let chosen = argmax(&scores).unwrap();
        // Kandidat yang bisa terpilih HARUS tetap dalam pool yang sudah
        // disempitkan -- exposure control/item selection tidak pernah
        // "escape" balik ke item kategori lain.
        assert!(narrowed.contains(&chosen));
    }
}
