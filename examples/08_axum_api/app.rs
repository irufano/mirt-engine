//! Modul reusable (bank sintetis, `SessionConfig`, dan `Router` Axum) --
//! dipakai `main.rs` (server sungguhan) DAN `tests/integration_axum.rs`
//! (lewat `#[path]`, supaya test HTTP end-to-end tidak menduplikasi logika
//! endpoint). Lih. `docs/panduan/08-integrasi-axum.md` untuk penjelasan
//! desain penuh.
//!
//! **Pola "stateless-friendly" yang ditunjukkan di sini**: `AppState.sessions`
//! menyimpan `McatSession` dalam bentuk STRING JSON HASIL SERIALISASI
//! (bukan struct langsung) -- tiap request deserialize dari string, mutasi,
//! serialize balik, simpan. Ini SENGAJA mensimulasikan apa yang terjadi
//! dengan Redis/DB sungguhan (yang cuma bisa menyimpan bytes/string), walau
//! backing store contoh ini cuma `HashMap` in-memory. Produksi sebaiknya
//! ganti `sessions` dengan store eksternal (Redis/DB) yang persisten lintas
//! restart & banyak instance server -- `HashMap` in-memory hilang begitu
//! proses berhenti dan tidak dibagi antar-instance saat di-scale horizontal.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use axum::routing::{get, post};
use axum::Router;
use rand::rngs::StdRng;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};

use mirtcat_engine::irt::Dichotomous;
use mirtcat_engine::item_selection::Criterion;
use mirtcat_engine::stopping_rule::StoppingConfig;
use mirtcat_engine::{
    EstimationConfig, GroupParams, ItemBank, ItemMeta, McatSession, Method, SelectionConfig,
    SessionConfig,
};

pub const NFACT: usize = 2;
pub const N_ITEMS: usize = 12;

/// Bank sintetis 2 dimensi, 12 item -- pola sama seperti contoh
/// `examples/07_final_result.rs`, diperkecil karena fokus contoh ini adalah
/// orkestrasi HTTP, bukan kedalaman psikometri.
pub fn build_bank() -> ItemBank {
    let mut bank = ItemBank::new();
    for i in 0..N_ITEMS {
        let dim_strong = i % NFACT;
        let mut a = vec![0.3; NFACT];
        a[dim_strong] = 1.4 + 0.1 * (i / NFACT) as f64;
        let d = 0.2 - 0.1 * (i as f64 % 4.0);
        bank.push(Box::new(Dichotomous::m2pl(a, d).unwrap()), ItemMeta::new(i))
            .unwrap();
    }
    bank
}

pub fn build_group() -> GroupParams {
    GroupParams::standard_normal(NFACT)
}

/// `debug`: diteruskan ke `SessionConfig::debug` -- `true` mengaktifkan
/// endpoint `GET /session/:id/debug` (404 bila `false`, lih. `get_debug`).
pub fn build_config(debug: bool) -> SessionConfig {
    SessionConfig {
        selection: SelectionConfig {
            criterion: Criterion::DRule { prior: true },
            ..Default::default()
        },
        estimation: EstimationConfig {
            method: Method::Map,
            ..Default::default()
        },
        stopping: StoppingConfig {
            min_items: 4,
            max_items: N_ITEMS,
            min_sem: vec![0.5],
            ..Default::default()
        },
        start_item: 0,
        debug,
        ..Default::default()
    }
}

pub struct AppState {
    pub bank: ItemBank,
    pub group: GroupParams,
    pub config: SessionConfig,
    /// `session_id -> McatSession` hasil `serde_json::to_string` -- lih.
    /// catatan modul soal kenapa disimpan sebagai string, bukan struct.
    pub sessions: Mutex<HashMap<String, String>>,
    next_id: AtomicU64,
}

pub fn build_state(debug: bool) -> Arc<AppState> {
    Arc::new(AppState {
        bank: build_bank(),
        group: build_group(),
        config: build_config(debug),
        sessions: Mutex::new(HashMap::new()),
        next_id: AtomicU64::new(1),
    })
}

pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/session", post(create_session))
        .route("/session/{id}/answer", post(submit_answer))
        .route("/session/{id}/result", get(get_result))
        .route("/session/{id}/debug", get(get_debug))
        .with_state(state)
}

fn error_response(status: StatusCode, message: impl Into<String>) -> axum::response::Response {
    (status, message.into()).into_response()
}

/// Ambil JSON sesi tersimpan, atau `404` bila `id` tidak dikenal. Error
/// dikembalikan sebagai `(StatusCode, &str)` kecil (bukan
/// `axum::response::Response` langsung) -- `Response` cukup besar
/// (clippy::result_large_err) untuk tipe `Err` di fungsi yang sering
/// dipanggil ini.
fn load_session_json(state: &AppState, id: &str) -> Result<String, (StatusCode, &'static str)> {
    state
        .sessions
        .lock()
        .expect("mutex sessions tidak pernah poisoned di contoh ini")
        .get(id)
        .cloned()
        .ok_or((StatusCode::NOT_FOUND, "sesi tidak ditemukan"))
}

fn save_session(state: &AppState, id: &str, session: &McatSession) {
    let json = serde_json::to_string(session)
        .expect("McatSession selalu bisa diserialisasi (feature serde)");
    state
        .sessions
        .lock()
        .expect("mutex sessions tidak pernah poisoned di contoh ini")
        .insert(id.to_string(), json);
}

#[derive(Serialize)]
struct CreateSessionResponse {
    session_id: String,
    next_item: usize,
}

async fn create_session(State(state): State<Arc<AppState>>) -> axum::response::Response {
    let mut session = match McatSession::new(state.config.clone(), &state.bank, &state.group, 0) {
        Ok(s) => s,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let mut rng = StdRng::from_entropy();
    let next_item = match session.next_item(&state.bank, &state.group, &mut rng) {
        Ok(item) => item,
        Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    let id = state.next_id.fetch_add(1, Ordering::Relaxed).to_string();
    save_session(&state, &id, &session);

    Json(CreateSessionResponse {
        session_id: id,
        next_item,
    })
    .into_response()
}

#[derive(Deserialize)]
struct AnswerRequest {
    item: usize,
    response: usize,
}

#[derive(Serialize)]
struct AnswerResponse {
    stop_now: bool,
    next_item: Option<usize>,
}

async fn submit_answer(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<AnswerRequest>,
) -> axum::response::Response {
    let json = match load_session_json(&state, &id) {
        Ok(j) => j,
        Err(e) => return e.into_response(),
    };
    let mut session: McatSession =
        serde_json::from_str(&json).expect("sesi tersimpan selalu JSON McatSession valid");

    let decision =
        match session.submit_response(body.item, body.response, &state.bank, &state.group, None) {
            Ok(d) => d,
            Err(e) => return error_response(StatusCode::BAD_REQUEST, e.to_string()),
        };

    let next_item = if decision.stop_now {
        None
    } else {
        let mut rng = StdRng::from_entropy();
        match session.next_item(&state.bank, &state.group, &mut rng) {
            Ok(item) => Some(item),
            Err(e) => return error_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        }
    };

    save_session(&state, &id, &session);

    Json(AnswerResponse {
        stop_now: decision.stop_now,
        next_item,
    })
    .into_response()
}

/// `409 Conflict` bila sesi belum berhenti (`McatSession::is_finished() ==
/// false`) -- hasil akhir hanya bermakna final setelah stopping rule
/// benar-benar memutuskan berhenti.
async fn get_result(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::response::Response {
    let json = match load_session_json(&state, &id) {
        Ok(j) => j,
        Err(e) => return e.into_response(),
    };
    let session: McatSession =
        serde_json::from_str(&json).expect("sesi tersimpan selalu JSON McatSession valid");

    if !session.is_finished() {
        return error_response(StatusCode::CONFLICT, "sesi belum selesai");
    }

    Json(session.finalize()).into_response()
}

/// `404` bila `SessionConfig::debug == false` (sesuai desain
/// `McatSession::debug_log()` -- selalu kosong, tidak ada gunanya
/// diekspos) ATAU sesi tidak ditemukan.
async fn get_debug(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> axum::response::Response {
    if !state.config.debug {
        return StatusCode::NOT_FOUND.into_response();
    }
    let json = match load_session_json(&state, &id) {
        Ok(j) => j,
        Err(e) => return e.into_response(),
    };
    let session: McatSession =
        serde_json::from_str(&json).expect("sesi tersimpan selalu JSON McatSession valid");

    Json(session.debug_log().to_vec()).into_response()
}
