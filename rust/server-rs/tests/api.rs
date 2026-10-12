//! Tests de integración: réplica de server/tests/api.test.ts (los 8 tests).

use axum::body::Body;
use axum::http::{Method, Request, StatusCode, header};
use axum::response::Response;
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use distroforge_server::{build_router, db::open_db};

/// Nueva app con BD en memoria y QEMU inexistente (como los tests TS).
fn app() -> Router {
    build_router(open_db(":memory:"), "qemu-inexistente")
}

async fn send(
    app: &Router,
    method: Method,
    uri: &str,
    body: Option<Value>,
    cookie: Option<&str>,
) -> Response {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, format!("df_session={cookie}"));
    }
    let (builder, body) = match body {
        Some(payload) => (
            builder.header(header::CONTENT_TYPE, "application/json"),
            Body::from(payload.to_string()),
        ),
        None => (builder, Body::empty()),
    };
    app.clone()
        .oneshot(builder.body(body).expect("request válida"))
        .await
        .expect("oneshot")
}

async fn body_json(resp: Response) -> Value {
    let bytes = resp.into_body().collect().await.expect("body").to_bytes();
    serde_json::from_slice(&bytes).expect("json válido")
}

/// Valor de la cookie df_session de la respuesta (como `r.cookies` en TS).
fn df_session(resp: &Response) -> Option<String> {
    for raw in resp.headers().get_all(header::SET_COOKIE) {
        let raw = raw.to_str().ok()?;
        if let Some(value) = raw.strip_prefix("df_session=") {
            let value = value.split(';').next()?;
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

async fn register(app: &Router) -> Response {
    send(
        app,
        Method::POST,
        "/api/register",
        Some(json!({ "name": "tester", "password": "password123" })),
        None,
    )
    .await
}

// ---------- auth ----------

#[tokio::test]
async fn registra_loguea_y_devuelve_usuario_actual() {
    let app = app();
    let r = register(&app).await;
    assert_eq!(r.status(), StatusCode::OK);
    let cookie = df_session(&r);
    assert!(cookie.is_some());

    let me = send(&app, Method::GET, "/api/me", None, cookie.as_deref()).await;
    let me = body_json(me).await;
    assert!(me["user"]["id"].is_i64());
    assert_eq!(me["user"]["name"], "tester");
    assert_eq!(me["user"].as_object().map(|o| o.len()), Some(2));

    // también vale Authorization: Bearer <token>
    let login = body_json(r).await;
    let token = login["token"].as_str().expect("token");
    let me = send(
        &app,
        Method::GET,
        "/api/me",
        None,
        None,
    )
    .await;
    // (sin credenciales: null)
    assert_eq!(body_json(me).await["user"], Value::Null);
    let req = Request::builder()
        .method(Method::GET)
        .uri("/api/me")
        .header(header::AUTHORIZATION, format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let me = app.oneshot(req).await.unwrap();
    assert_eq!(body_json(me).await["user"]["name"], "tester");
}

#[tokio::test]
async fn rechaza_nombres_invalidos_passwords_cortos_y_duplicados() {
    let app = app();
    let corto = send(
        &app,
        Method::POST,
        "/api/register",
        Some(json!({ "name": "x", "password": "password123" })),
        None,
    )
    .await;
    assert_eq!(corto.status(), StatusCode::BAD_REQUEST);

    let pass_corta = send(
        &app,
        Method::POST,
        "/api/register",
        Some(json!({ "name": "buen", "password": "corta" })),
        None,
    )
    .await;
    assert_eq!(pass_corta.status(), StatusCode::BAD_REQUEST);

    assert_eq!(register(&app).await.status(), StatusCode::OK);
    let duplicado = register(&app).await;
    assert_eq!(duplicado.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn login_falla_con_credenciales_equivocadas_y_logout_cierra_sesion() {
    let app = app();
    register(&app).await;

    let bad = send(
        &app,
        Method::POST,
        "/api/login",
        Some(json!({ "name": "tester", "password": "incorrecta" })),
        None,
    )
    .await;
    assert_eq!(bad.status(), StatusCode::UNAUTHORIZED);

    let ok = send(
        &app,
        Method::POST,
        "/api/login",
        Some(json!({ "name": "tester", "password": "password123" })),
        None,
    )
    .await;
    assert_eq!(ok.status(), StatusCode::OK);
    let cookie = df_session(&ok).expect("cookie de login");

    let out = send(&app, Method::POST, "/api/logout", None, Some(&cookie)).await;
    assert_eq!(out.status(), StatusCode::OK);

    let me = send(&app, Method::GET, "/api/me", None, Some(&cookie)).await;
    assert_eq!(body_json(me).await["user"], Value::Null);
}

// ---------- galería con búsqueda server-side ----------

async fn login_cookie(app: &Router) -> String {
    let r = send(
        app,
        Method::POST,
        "/api/register",
        Some(json!({ "name": "autor", "password": "password123" })),
        None,
    )
    .await;
    df_session(&r).expect("cookie")
}

#[tokio::test]
async fn publica_packs_auth_y_los_busca_por_texto() {
    let app = app();
    let cookie = login_cookie(&app).await;

    let mk = |name: &str, desc: &str, packages: Value| {
        json!({ "name": name, "desc": desc, "json": { "name": name, "packages": packages } })
    };
    let gaming = send(
        &app,
        Method::POST,
        "/api/packs",
        Some(mk("Gaming total", "steam y lutris", json!(["steam", "lutris"]))),
        Some(&cookie),
    )
    .await;
    assert_eq!(gaming.status(), StatusCode::OK);

    let nas = send(
        &app,
        Method::POST,
        "/api/packs",
        Some(mk("Servidor NAS", "samba y nfs", json!(["samba", "nfs-kernel-server"]))),
        Some(&cookie),
    )
    .await;
    assert_eq!(nas.status(), StatusCode::OK);

    let roto = send(
        &app,
        Method::POST,
        "/api/packs",
        Some(json!({ "name": "roto", "desc": "sin packages", "json": {} })),
        Some(&cookie),
    )
    .await;
    assert_eq!(roto.status(), StatusCode::BAD_REQUEST);

    let q = send(&app, Method::GET, "/api/packs?query=steam", None, None).await;
    let q = body_json(q).await;
    let names: Vec<&str> = q["packs"]
        .as_array()
        .expect("packs array")
        .iter()
        .map(|p| p["name"].as_str().expect("name str"))
        .collect();
    assert!(names.contains(&"Gaming total"));
    assert!(!names.contains(&"Servidor NAS"));

    let all = send(&app, Method::GET, "/api/packs", None, None).await;
    let all = body_json(all).await;
    assert_eq!(all["packs"].as_array().map(Vec::len), Some(2));
}

#[tokio::test]
async fn sin_login_no_se_puede_publicar_ni_valorar() {
    let app = app();
    let packs = send(
        &app,
        Method::POST,
        "/api/packs",
        Some(json!({ "name": "x", "json": { "packages": [] } })),
        None,
    )
    .await;
    assert_eq!(packs.status(), StatusCode::UNAUTHORIZED);

    let rating = send(
        &app,
        Method::POST,
        "/api/rating",
        Some(json!({ "packId": 1, "stars": 5 })),
        None,
    )
    .await;
    assert_eq!(rating.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn valora_packs_y_el_promedio_aparece_en_la_busqueda() {
    let app = app();
    let cookie = login_cookie(&app).await;

    let r = send(
        &app,
        Method::POST,
        "/api/packs",
        Some(json!({ "name": "media", "desc": "video", "json": { "packages": ["vlc"] } })),
        Some(&cookie),
    )
    .await;
    let id = body_json(r).await["id"].as_i64().expect("id");

    for stars in [4, 5] {
        let r = send(
            &app,
            Method::POST,
            "/api/rating",
            Some(json!({ "packId": id, "stars": stars })),
            Some(&cookie),
        )
        .await;
        assert_eq!(r.status(), StatusCode::OK);
    }

    let q = send(&app, Method::GET, "/api/packs?query=video", None, None).await;
    let q = body_json(q).await;
    let rating = &q["packs"][0]["rating"];
    assert_eq!(rating["avg"].as_f64(), Some(5.0));
    assert_eq!(rating["n"].as_i64(), Some(1));
}

// ---------- test drive ----------

#[tokio::test]
async fn available_informa_de_qemu_ausente_y_start_devuelve_503_sin_qemu() {
    let app = app();
    let r = send(&app, Method::GET, "/api/testdrive/available", None, None).await;
    assert_eq!(body_json(r).await["available"], json!(false));

    let reg = register(&app).await;
    let cookie = df_session(&reg).expect("cookie");

    let start = send(
        &app,
        Method::POST,
        "/api/testdrive",
        Some(json!({ "imagePath": "/tmp/noexiste.iso" })),
        Some(&cookie),
    )
    .await;
    assert_eq!(start.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test]
async fn sin_login_o_con_ruta_relativa_rechaza() {
    let app = app();
    let no_login = send(
        &app,
        Method::POST,
        "/api/testdrive",
        Some(json!({ "imagePath": "/x" })),
        None,
    )
    .await;
    assert_eq!(no_login.status(), StatusCode::UNAUTHORIZED);

    let reg = register(&app).await;
    let cookie = df_session(&reg).expect("cookie");

    let relativa = send(
        &app,
        Method::POST,
        "/api/testdrive",
        Some(json!({ "imagePath": "relativa.iso" })),
        Some(&cookie),
    )
    .await;
    assert_eq!(relativa.status(), StatusCode::BAD_REQUEST);
}
