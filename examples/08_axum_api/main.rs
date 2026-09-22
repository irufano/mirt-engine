//! Fase 8 — contoh nyata `mirtcat-engine` dipasang di web API (Axum),
//! membuktikan engine ini "bisa dipakai" di luar test/CLI, bukan cuma
//! pustaka murni fungsi. Lih. `docs/panduan/08-integrasi-axum.md` untuk
//! penjelasan desain penuh (pola state antar-request, in-memory vs
//! Redis/DB) dan `app.rs` untuk implementasi endpoint.
//!
//! Jalankan:
//! ```sh
//! cargo run --example 08_axum_api --features serde
//! ```
//!
//! Uji manual (di terminal lain, server default di `127.0.0.1:3000`):
//! ```sh
//! # 1. Mulai sesi -- balasan berisi session_id & item pertama (start_item).
//! curl -s -X POST localhost:3000/session
//! # {"session_id":"1","next_item":0}
//!
//! # 2. Jawab item yang diminta (kategori 0/1 untuk item dikotomus), ulangi
//! #    dengan `item` = `next_item` dari balasan sebelumnya sampai
//! #    `stop_now == true`.
//! curl -s -X POST localhost:3000/session/1/answer \
//!   -H 'content-type: application/json' -d '{"item":0,"response":1}'
//! # {"stop_now":false,"next_item":7}
//!
//! # 3. Setelah stop_now == true, ambil hasil akhir (McatResult).
//! curl -s localhost:3000/session/1/result
//!
//! # 4. Jejak debug (404 bila SessionConfig::debug == false -- lih.
//! #    app::build_config).
//! curl -s localhost:3000/session/1/debug
//! ```

#[path = "app.rs"]
mod app;

#[tokio::main]
async fn main() {
    let state = app::build_state(false);
    let router = app::build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("gagal bind ke 127.0.0.1:3000");
    println!("mirtcat-engine contoh Axum (Fase 8) berjalan di http://127.0.0.1:3000");
    println!("lih. komentar atas file ini untuk contoh perintah curl.");
    axum::serve(listener, router)
        .await
        .expect("server Axum berhenti tak terduga");
}
