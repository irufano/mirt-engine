//! `seq` — baseline non-adaptif: item bank disajikan berurutan
//! (item ke-`n` = item bank ke-`n`). Tidak memakai info Fisher/probabilitas
//! sama sekali.

/// Pilih index kandidat terkecil dari daftar `candidates` (index `ItemBank`
/// yang belum dijawab). Panic bila `candidates` kosong — caller
/// (`select_next_item`) sudah memvalidasi non-kosong lebih dulu.
pub(crate) fn select(candidates: &[usize]) -> usize {
    *candidates
        .iter()
        .min()
        .expect("seq::select: candidates tidak boleh kosong")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picks_smallest_candidate_index() {
        assert_eq!(select(&[4, 1, 7, 2]), 1);
    }

    #[test]
    fn single_candidate_is_returned() {
        assert_eq!(select(&[3]), 3);
    }
}
