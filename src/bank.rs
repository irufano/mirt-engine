//! Kumpulan item terkalibrasi (`ItemBank`) beserta metadata per-item dan
//! parameter populasi (`GroupParams`) — padanan gabungan `mirtCAT`
//! `TestClass`/ekstraksi `gp$gmeans`/`gp$gcov` dari model terkalibrasi.

use crate::error::MirtCatError;
use crate::irt::ItemResponseModel;
use crate::theta::{InfoMatrix, ThetaVector};

/// Metadata per-item yang tidak berkaitan dengan model probabilitas: id
/// item, dan kategori konten (dipakai content balancing, Fase 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemMeta {
    /// Id item (indeks/identitas eksternal, terpisah dari indeks posisi di
    /// `ItemBank`).
    pub id: usize,
    /// Kategori konten item, `None` bila tidak dipakai (content balancing
    /// nonaktif).
    pub content: Option<String>,
}

impl ItemMeta {
    /// `ItemMeta` tanpa kategori konten.
    pub fn new(id: usize) -> Self {
        Self { id, content: None }
    }

    /// `ItemMeta` dengan kategori konten (dipakai content balancing).
    pub fn with_content(id: usize, content: impl Into<String>) -> Self {
        Self {
            id,
            content: Some(content.into()),
        }
    }
}

/// Kumpulan item respons `Box<dyn ItemResponseModel>` + metadata,
/// mensyaratkan seluruh item punya `nfact` yang sama (satu model
/// multidimensional yang konsisten).
pub struct ItemBank {
    items: Vec<Box<dyn ItemResponseModel>>,
    meta: Vec<ItemMeta>,
    nfact: Option<usize>,
}

impl ItemBank {
    /// Bank kosong (belum ada item, `nfact()` masih `None`).
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            meta: Vec::new(),
            nfact: None,
        }
    }

    /// Tambahkan item ke bank. Error bila `nfact` item tidak konsisten
    /// dengan item yang sudah ada di bank.
    pub fn push(
        &mut self,
        item: Box<dyn ItemResponseModel>,
        meta: ItemMeta,
    ) -> Result<(), MirtCatError> {
        match self.nfact {
            None => self.nfact = Some(item.nfact()),
            Some(expected) if expected != item.nfact() => {
                return Err(MirtCatError::DimensionMismatch {
                    expected,
                    actual: item.nfact(),
                    context: "ItemBank::push: nfact item tidak konsisten dengan bank",
                });
            }
            Some(_) => {}
        }
        self.items.push(item);
        self.meta.push(meta);
        Ok(())
    }

    /// Jumlah item di bank.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `true` bila bank belum berisi item.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `nfact` bank (sama untuk semua item), `None` bila bank kosong.
    pub fn nfact(&self) -> Option<usize> {
        self.nfact
    }

    /// Item pada posisi `index`, `None` bila di luar jangkauan.
    pub fn item(&self, index: usize) -> Option<&dyn ItemResponseModel> {
        self.items.get(index).map(|b| b.as_ref())
    }

    /// Metadata item pada posisi `index`, `None` bila di luar jangkauan.
    pub fn meta(&self, index: usize) -> Option<&ItemMeta> {
        self.meta.get(index)
    }

    /// Iterator atas seluruh item di bank, terurut posisi.
    pub fn iter(&self) -> impl Iterator<Item = &dyn ItemResponseModel> {
        self.items.iter().map(|b| b.as_ref())
    }
}

impl Default for ItemBank {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameter populasi model terkalibrasi: rata-rata (`gmeans`) dan matriks
/// kovarians (`gcov`) prior multivariat `N(gmeans, gcov)` — sumber theta
/// awal & SE awal di Fase 1.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupParams {
    gmeans: ThetaVector,
    gcov: InfoMatrix,
}

impl GroupParams {
    /// Error bila dimensi `gmeans` dan `gcov` tidak cocok.
    pub fn new(gmeans: ThetaVector, gcov: InfoMatrix) -> Result<Self, MirtCatError> {
        if gmeans.nfact() != gcov.nfact() {
            return Err(MirtCatError::DimensionMismatch {
                expected: gmeans.nfact(),
                actual: gcov.nfact(),
                context: "GroupParams::new: gmeans vs gcov",
            });
        }
        Ok(Self { gmeans, gcov })
    }

    /// `GroupParams` standar `N(0, I)` sepanjang `nfact` — dipakai bila
    /// model terkalibrasi tidak menyediakan parameter populasi eksplisit.
    pub fn standard_normal(nfact: usize) -> Self {
        Self {
            gmeans: ThetaVector::zeros(nfact),
            gcov: InfoMatrix::identity(nfact),
        }
    }

    /// `nfact` populasi (dari `gmeans`/`gcov`, sudah dijamin konsisten).
    pub fn nfact(&self) -> usize {
        self.gmeans.nfact()
    }

    /// Rata-rata populasi.
    pub fn gmeans(&self) -> &ThetaVector {
        &self.gmeans
    }

    /// Matriks kovarians populasi.
    pub fn gcov(&self) -> &InfoMatrix {
        &self.gcov
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irt::Dichotomous;

    #[test]
    fn item_bank_push_and_access() {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0, 0.5], 0.0).unwrap()),
            ItemMeta::with_content(0, "aljabar"),
        )
        .unwrap();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![0.8, 1.2], -0.3).unwrap()),
            ItemMeta::new(1),
        )
        .unwrap();

        assert_eq!(bank.len(), 2);
        assert_eq!(bank.nfact(), Some(2));
        assert_eq!(bank.meta(0).unwrap().content.as_deref(), Some("aljabar"));
        assert!(bank.item(1).is_some());
        assert!(bank.item(2).is_none());
    }

    #[test]
    fn item_bank_rejects_inconsistent_nfact() {
        let mut bank = ItemBank::new();
        bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0, 0.5], 0.0).unwrap()),
            ItemMeta::new(0),
        )
        .unwrap();
        let err = bank.push(
            Box::new(Dichotomous::m2pl(vec![1.0], 0.0).unwrap()),
            ItemMeta::new(1),
        );
        assert!(err.is_err());
        assert_eq!(bank.len(), 1); // item kedua tidak ikut masuk
    }

    #[test]
    fn group_params_dimension_mismatch_errors() {
        let gmeans = ThetaVector::zeros(3);
        let gcov = InfoMatrix::identity(2);
        assert!(GroupParams::new(gmeans, gcov).is_err());
    }

    #[test]
    fn group_params_standard_normal() {
        let gp = GroupParams::standard_normal(3);
        assert_eq!(gp.nfact(), 3);
        assert_eq!(gp.gmeans().as_slice(), &[0.0, 0.0, 0.0]);
        assert_eq!(gp.gcov().get(0, 0), 1.0);
        assert_eq!(gp.gcov().get(0, 1), 0.0);
    }
}
