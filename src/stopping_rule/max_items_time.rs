//! Batas keras `max_items`/`max_time` — dicek **independen** dari kriteria
//! presisi (`min_SEM`/`delta_thetas`/klasifikasi), berlaku juga bila
//! `n_answered < min_items`.
//!
//! Padanan `DesignClass.R:361-363`:
//! ```r
//! if(.Object@stage > 1L && nanswered == .Object@max_items) .Object@stop_now <- TRUE
//! if(.Object@max_time <= sum(person$item_time)) .Object@stop_now <- TRUE
//! ```
//! Rust memakai `>=` (bukan `==`) untuk `max_items` agar robust terhadap
//! pemanggil yang memeriksa setelah `n_answered` melampaui `max_items` --
//! hasilnya identik untuk penggunaan wajar (jumlah item terjawab tidak
//! pernah melompati `max_items` dalam loop administrasi normal).

use std::time::Duration;

pub(crate) fn max_items_reached(n_answered: usize, max_items: usize) -> bool {
    n_answered >= max_items
}

pub(crate) fn max_time_reached(elapsed: Duration, max_time: Option<Duration>) -> bool {
    match max_time {
        Some(limit) => elapsed >= limit,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_items_reached_at_or_beyond_limit() {
        assert!(!max_items_reached(4, 5));
        assert!(max_items_reached(5, 5));
        assert!(max_items_reached(6, 5));
    }

    #[test]
    fn max_time_none_never_reached() {
        assert!(!max_time_reached(Duration::from_secs(999_999), None));
    }

    #[test]
    fn max_time_reached_at_or_beyond_limit() {
        let limit = Duration::from_secs(60);
        assert!(!max_time_reached(Duration::from_secs(59), Some(limit)));
        assert!(max_time_reached(Duration::from_secs(60), Some(limit)));
        assert!(max_time_reached(Duration::from_secs(61), Some(limit)));
    }
}
