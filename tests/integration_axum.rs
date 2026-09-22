//! Test HTTP end-to-end untuk contoh Axum (Fase 8, `examples/08_axum_api/`).
//!
//! Memakai `router.oneshot()` (`tower::ServiceExt`) -- request/response
//! in-process langsung ke `axum::Router`, TANPA membuka socket TCP
//! sungguhan (lebih cepat & stabil untuk CI daripada `axum::serve` +
//! `reqwest` beneran). `app.rs` diimpor lewat `#[path]` dari folder contoh
//! supaya endpoint yang diuji di sini adalah PERSIS kode yang dipakai
//! `main.rs`, bukan duplikat.
//!
//! Seluruh file `#![cfg(feature = "serde")]` -- contoh Axum (dan modul
//! `app.rs` yang dipakai ulang di sini) TIDAK bisa compile tanpa fitur
//! `serde` (JSON body Axum & `McatSession` yang disimpan sebagai string
//! JSON keduanya wajib `Serialize`/`Deserialize`), sama seperti
//! `examples/08_axum_api` yang diberi `required-features = ["serde"]` di
//! `Cargo.toml`.

#![cfg(feature = "serde")]

#[path = "../examples/08_axum_api/app.rs"]
mod app;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use tower::ServiceExt;

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn get_request(uri: String) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

fn post_json_request(uri: String, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn full_session_via_http_reaches_stop_and_result_and_debug_are_consistent() {
    let state = app::build_state(true); // debug=true -- juga menguji endpoint /debug
    let router = app::build_router(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    let session_id = body["session_id"].as_str().unwrap().to_string();
    let mut next_item = body["next_item"].as_u64().unwrap() as usize;

    // Hasil belum bermakna sebelum sesi berhenti -> 409.
    let response = router
        .clone()
        .oneshot(get_request(format!("/session/{session_id}/result")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let mut stop_now = false;
    let mut guard = 0;
    while !stop_now {
        guard += 1;
        assert!(
            guard < app::N_ITEMS + 1,
            "sesi tidak pernah berhenti dalam {} langkah -- kemungkinan bug pipeline",
            app::N_ITEMS
        );

        let response = router
            .clone()
            .oneshot(post_json_request(
                format!("/session/{session_id}/answer"),
                json!({ "item": next_item, "response": 1 }),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        stop_now = body["stop_now"].as_bool().unwrap();
        if !stop_now {
            next_item = body["next_item"].as_u64().unwrap() as usize;
        } else {
            assert!(body["next_item"].is_null());
        }
    }

    let response = router
        .clone()
        .oneshot(get_request(format!("/session/{session_id}/result")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let result = body_json(response).await;
    assert!(!result["items_answered"].as_array().unwrap().is_empty());
    assert_eq!(
        result["items_answered"].as_array().unwrap().len(),
        result["thetas_history"].as_array().unwrap().len()
    );

    let response = router
        .oneshot(get_request(format!("/session/{session_id}/debug")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let debug_log = body_json(response).await;
    assert!(!debug_log.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn debug_endpoint_is_404_when_session_config_debug_is_false() {
    let state = app::build_state(false);
    let router = app::build_router(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(response).await;
    let session_id = body["session_id"].as_str().unwrap().to_string();

    let response = router
        .oneshot(get_request(format!("/session/{session_id}/debug")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn unknown_session_id_returns_404_on_every_endpoint() {
    let state = app::build_state(false);
    let router = app::build_router(state);

    for uri in [
        "/session/does-not-exist/result".to_string(),
        "/session/does-not-exist/debug".to_string(),
    ] {
        let response = router.clone().oneshot(get_request(uri)).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    let response = router
        .oneshot(post_json_request(
            "/session/does-not-exist/answer".to_string(),
            json!({ "item": 0, "response": 1 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn answering_an_item_that_was_not_requested_returns_400() {
    let state = app::build_state(false);
    let router = app::build_router(state);

    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = body_json(response).await;
    let session_id = body["session_id"].as_str().unwrap().to_string();
    let next_item = body["next_item"].as_u64().unwrap() as usize;
    let wrong_item = if next_item == 0 { 1 } else { 0 };

    let response = router
        .oneshot(post_json_request(
            format!("/session/{session_id}/answer"),
            json!({ "item": wrong_item, "response": 1 }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
