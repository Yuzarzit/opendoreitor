#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod adblock;
mod eco;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{
    webview::{PageLoadEvent, WebviewBuilder},
    window::WindowBuilder,
    Emitter, LogicalPosition, LogicalSize, Manager, WebviewUrl, Window, WindowEvent,
};

const INITIAL_URL: &str = "https://lite.duckduckgo.com/lite/";
const TOOLBAR_HEIGHT: f64 = 84.0;
/// Ancho del panel de historial/marcadores. Debe coincidir con #drawer en style.css.
const DRAWER_WIDTH: f64 = 360.0;
const WINDOW_W: f64 = 1200.0;
const WINDOW_H: f64 = 800.0;

static ADBLOCK: AtomicBool = AtomicBool::new(true);
static ECO: AtomicBool = AtomicBool::new(false);
static DARK: AtomicBool = AtomicBool::new(true);
static DRAWER: AtomicBool = AtomicBool::new(false);

fn current_script() -> String {
    adblock::build_injection_script(
        ADBLOCK.load(Ordering::Relaxed),
        ECO.load(Ordering::Relaxed),
        DARK.load(Ordering::Relaxed),
    )
}

fn content_webview(app: &tauri::AppHandle) -> Result<tauri::Webview, String> {
    app.get_webview("content")
        .ok_or_else(|| "el webview de contenido no existe".to_string())
}

fn reinject(app: &tauri::AppHandle) -> Result<(), String> {
    content_webview(app)?
        .eval(current_script())
        .map_err(|e| e.to_string())
}

/// Coloca la barra y el contenido. Se llama al arrancar, al redimensionar y
/// al abrir/cerrar el panel lateral.
///
/// - Barra: ocupa SOLO la franja superior (84 px)...
/// - ...salvo con el panel abierto: entonces ocupa toda la ventana y el
///   contenido se encoge para dejarle sitio a la derecha.
fn layout(window: &Window) -> tauri::Result<()> {
    let scale = window.scale_factor()?;
    let size = window.inner_size()?.to_logical::<f64>(scale);
    let (w, h) = (size.width, size.height);
    if w < 1.0 || h < 1.0 {
        return Ok(()); // ventana minimizada
    }
    let drawer = DRAWER.load(Ordering::Relaxed);

    if let Some(tb) = window.get_webview("toolbar") {
        let tb_h = if drawer { h } else { TOOLBAR_HEIGHT.min(h) };
        tb.set_position(LogicalPosition::new(0.0, 0.0))?;
        tb.set_size(LogicalSize::new(w, tb_h))?;
    }
    if let Some(ct) = window.get_webview("content") {
        let cw = if drawer { (w - DRAWER_WIDTH).max(120.0) } else { w };
        let ch = (h - TOOLBAR_HEIGHT).max(1.0);
        ct.set_position(LogicalPosition::new(0.0, TOOLBAR_HEIGHT))?;
        ct.set_size(LogicalSize::new(cw, ch))?;
    }
    Ok(())
}

#[tauri::command]
async fn navegar(app: tauri::AppHandle, url: String) -> Result<(), String> {
    let parsed: tauri::Url = url.trim().parse().map_err(|e| format!("URL invalida: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("solo se permiten URLs http/https".into());
    }
    content_webview(&app)?.navigate(parsed).map_err(|e| e.to_string())
}

#[tauri::command]
async fn atras(app: tauri::AppHandle) -> Result<(), String> {
    content_webview(&app)?.eval("history.back()").map_err(|e| e.to_string())
}

#[tauri::command]
async fn adelante(app: tauri::AppHandle) -> Result<(), String> {
    content_webview(&app)?.eval("history.forward()").map_err(|e| e.to_string())
}

#[tauri::command]
async fn recargar(app: tauri::AppHandle) -> Result<(), String> {
    content_webview(&app)?.eval("location.reload()").map_err(|e| e.to_string())
}

#[tauri::command]
async fn inicio(app: tauri::AppHandle) -> Result<(), String> {
    let u: tauri::Url = INITIAL_URL.parse().map_err(|e| format!("{e}"))?;
    content_webview(&app)?.navigate(u).map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_adblock(app: tauri::AppHandle, v: bool) -> Result<(), String> {
    ADBLOCK.store(v, Ordering::Relaxed);
    reinject(&app)
}

#[tauri::command]
async fn set_eco(app: tauri::AppHandle, v: bool) -> Result<(), String> {
    ECO.store(v, Ordering::Relaxed);
    let prioridad = eco::apply(v);
    reinject(&app)?;
    prioridad
}

#[tauri::command]
async fn set_theme(app: tauri::AppHandle, dark: bool) -> Result<(), String> {
    DARK.store(dark, Ordering::Relaxed);
    reinject(&app)
}

#[tauri::command]
async fn set_drawer(window: Window, open: bool) -> Result<(), String> {
    DRAWER.store(open, Ordering::Relaxed);
    layout(&window).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            navegar, atras, adelante, recargar, inicio,
            set_adblock, set_eco, set_theme, set_drawer
        ])
        .setup(|app| {
            let window = WindowBuilder::new(app, "main")
                .title("opendoreitor")
                .inner_size(WINDOW_W, WINDOW_H)
                .min_inner_size(720.0, 480.0)
                .build()?;

            // 1) Barra superior: UI local (ui/index.html).
            window.add_child(
                WebviewBuilder::new("toolbar", WebviewUrl::App("index.html".into())),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(WINDOW_W, TOOLBAR_HEIGHT),
            )?;

            // 2) Contenido web real. Se anade despues => queda por encima.
            let content_url: tauri::Url = INITIAL_URL.parse()?;
            window.add_child(
                WebviewBuilder::new("content", WebviewUrl::External(content_url))
                    // Corre al CREAR cada documento: los hooks de red llegan antes que
                    // los scripts de la pagina (no solo cuando termina de cargar).
                    .initialization_script(current_script())
                    .on_page_load(|wv, payload| {
                        let finished = matches!(payload.event(), PageLoadEvent::Finished);
                        // Mantiene sincronizada la barra de direcciones y el historial.
                        let _ = wv.emit_to(
                            "toolbar",
                            "page-load",
                            serde_json::json!({
                                "url": payload.url().as_str(),
                                "finished": finished
                            }),
                        );
                        if finished {
                            let _ = wv.eval(current_script());
                        }
                    }),
                LogicalPosition::new(0.0, TOOLBAR_HEIGHT),
                LogicalSize::new(WINDOW_W, WINDOW_H - TOOLBAR_HEIGHT),
            )?;

            layout(&window)?;

            // Sin auto_resize (escala proporcionalmente y descuadra la barra de 84 px):
            // recolocamos a mano en cada cambio de tamano.
            let handle = app.handle().clone();
            window.on_window_event(move |event| {
                if matches!(
                    event,
                    WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. }
                ) {
                    if let Some(w) = handle.get_window("main") {
                        let _ = layout(&w);
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error fatal en opendoreitor");
}
