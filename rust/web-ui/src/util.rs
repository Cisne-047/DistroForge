//! Interop con el navegador: localStorage, alertas, descargas, lectura de
//! ficheros, clipboard, fetch y reloj del mock.

use std::collections::BTreeMap;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

#[wasm_bindgen]
extern "C" {
    /// Función definida en index.html (fflate): genera y descarga el zip.
    #[wasm_bindgen(js_namespace = window, js_name = distroforgeDownloadZip)]
    fn download_zip_js(name: &str, files_json: &str);
}

/// Genera y descarga el zip del proyecto. `false` si fflate no está cargado.
pub fn download_zip(name: &str, files: &BTreeMap<String, String>) -> bool {
    let has = js_sys::Reflect::has(
        &js_sys::global(),
        &JsValue::from_str("distroforgeDownloadZip"),
    )
    .unwrap_or(false);
    if !has {
        alert("No se pudo cargar fflate (script CDN): el zip no puede generarse sin conexión.");
        return false;
    }
    let json = serde_json::to_string(files).unwrap_or_else(|_| "{}".to_string());
    download_zip_js(name, &json);
    true
}

pub fn window() -> web_sys::Window {
    web_sys::window().expect("sin window")
}

pub fn alert(msg: &str) {
    let _ = window().alert_with_message(msg);
}

pub fn ls_get(key: &str) -> Option<String> {
    let ls = window().local_storage().ok()??;
    ls.get_item(key).ok()?
}

pub fn ls_set(key: &str, value: &str) {
    if let Ok(Some(ls)) = window().local_storage() {
        let _ = ls.set_item(key, value);
    }
}

/// Descarga un fichero de texto generado en memoria (Blob + <a download>).
pub fn download_text_file(filename: &str, content: &str) {
    let arr = js_sys::Array::from_iter([JsValue::from_str(content)]);
    let blob = web_sys::Blob::new_with_str_sequence(&arr).expect("blob");
    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("object url");
    let doc = window().document().expect("sin document");
    let a = doc
        .create_element("a")
        .expect("createElement")
        .unchecked_into::<web_sys::HtmlAnchorElement>();
    a.set_href(&url);
    a.set_download(filename);
    let body = doc.body().expect("sin body");
    let _ = body.append_child(&a);
    a.click();
    let _ = body.remove_child(&a);
    web_sys::Url::revoke_object_url(&url).expect("revoke");
}

/// Fichero elegido en un `<input type=file>` (y limpia el input, como el TS).
pub fn file_from_input_event(ev: &web_sys::Event) -> Option<web_sys::File> {
    let input = ev.target()?.unchecked_into::<web_sys::HtmlInputElement>();
    let f = input.files().and_then(|fl| fl.get(0));
    if f.is_some() {
        input.set_value("");
    }
    f
}

/// Lee un fichero como data URL (FileReader); el callback recibe el resultado.
pub fn read_as_data_url(file: &web_sys::File, cb: impl FnOnce(String) + 'static) {
    let blob = gloo_file::Blob::from(file.clone());
    let fut = gloo_file::futures::read_as_data_url(&blob);
    wasm_bindgen_futures::spawn_local(async move {
        match fut.await {
            Ok(s) => cb(s),
            Err(e) => alert(&format!("No se pudo leer el fichero: {e:?}")),
        }
    });
}

/// Lee un fichero como texto UTF-8.
pub fn read_as_text(file: &web_sys::File, cb: impl FnOnce(String) + 'static) {
    let blob = gloo_file::Blob::from(file.clone());
    let fut = gloo_file::futures::read_as_text(&blob);
    wasm_bindgen_futures::spawn_local(async move {
        match fut.await {
            Ok(s) => cb(s),
            Err(e) => alert(&format!("No se pudo leer el fichero: {e:?}")),
        }
    });
}

/// `event.target.value` de un input de texto.
pub fn input_val(ev: &web_sys::Event) -> String {
    ev.target()
        .map(|t| t.unchecked_into::<web_sys::HtmlInputElement>().value())
        .unwrap_or_default()
}

/// `event.target.value` de un select.
pub fn sel_val(ev: &web_sys::Event) -> String {
    ev.target()
        .map(|t| t.unchecked_into::<web_sys::HtmlSelectElement>().value())
        .unwrap_or_default()
}

/// `event.target.checked` de un checkbox.
pub fn checked_val(ev: &web_sys::Event) -> bool {
    ev.target()
        .map(|t| t.unchecked_into::<web_sys::HtmlInputElement>().checked())
        .unwrap_or(false)
}

/// Hora HH:MM (reloj del mock de escritorio).
pub fn hhmm() -> String {
    let d = js_sys::Date::new_0();
    format!("{:02}:{:02}", d.get_hours(), d.get_minutes())
}

/// Fecha estilo `toDateString()` (Last login del mock TTY).
pub fn date_string() -> String {
    js_sys::Date::new_0()
        .to_date_string()
        .as_string()
        .unwrap_or_default()
}

/// Copia al portapapeles (navigator.clipboard). Requiere contexto seguro.
pub async fn copy_to_clipboard(text: &str) -> bool {
    let cb = window().navigator().clipboard();
    wasm_bindgen_futures::JsFuture::from(cb.write_text(text))
        .await
        .is_ok()
}

/// `location.origin + pathname + "#" + encoded` (URL para compartir).
pub fn share_url(encoded: &str) -> String {
    let loc = window().location();
    let origin = loc.origin().unwrap_or_default();
    let pathname = loc.pathname().unwrap_or_default();
    format!("{origin}{pathname}#{encoded}")
}

/// GET de texto (galería, packs por URL).
pub async fn fetch_text(url: &str) -> Result<String, String> {
    let res = gloo_net::http::Request::get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = res.status();
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}"));
    }
    res.text().await.map_err(|e| e.to_string())
}
