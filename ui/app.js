(() => {
  "use strict";
  const $ = (s) => document.querySelector(s);

  if (!window.__TAURI__) {
    document.body.textContent = "Esta interfaz solo funciona dentro de opendoreitor (Tauri).";
    return;
  }
  const invoke = window.__TAURI__.core.invoke;
  const listen = window.__TAURI__.event.listen;

  const INITIAL_URL = "https://lite.duckduckgo.com/lite/";

  const url       = $("#url");
  const btnBack   = $("#back");
  const btnFwd    = $("#fwd");
  const btnReload = $("#reload");
  const btnHome   = $("#home");
  const btnGo     = $("#go");
  const btnStar   = $("#star");
  const btnMenu   = $("#menu");
  const panel     = $("#panel");
  const drawer    = $("#drawer");
  const drawerTitle = $("#drawerTitle");
  const drawerList  = $("#drawerList");
  const btnTheme  = $("#btnTheme");
  const btnEco    = $("#btnEco");
  const btnAdblock= $("#btnAdblock");
  const btnClear  = $("#btnClear");

  const store = {
    get(k, def) { try { const v = localStorage.getItem("opd." + k); return v === null ? def : JSON.parse(v); } catch { return def; } },
    set(k, v)   { try { localStorage.setItem("opd." + k, JSON.stringify(v)); } catch {} }
  };

  let theme   = store.get("theme",   "dark");
  let eco     = store.get("eco",     false);
  let adblock = store.get("adblock", true);
  let history = store.get("history", []);
  let marks   = store.get("marks",   []);
  let currentUrl = INITIAL_URL;
  let drawerKind = null; // "history" | "marks" | null

  const logErr = (e) => console.error(e);

  function applyTheme() {
    document.documentElement.dataset.theme = theme;
    btnTheme.textContent = theme === "dark" ? "Tema oscuro" : "Tema claro";
    store.set("theme", theme);
    invoke("set_theme", { dark: theme === "dark" }).catch(logErr);
  }

  // --- Direccion o busqueda ---------------------------------------------------
  function esUrl(t) {
    if (/\s/.test(t)) return false;
    if (/^https?:\/\//i.test(t)) return true;
    if (/^localhost(:\d+)?([/?#].*)?$/i.test(t)) return true;
    if (/^\d{1,3}(\.\d{1,3}){3}(:\d+)?([/?#].*)?$/.test(t)) return true;
    return /^[\w-]+(\.[\w-]+)*\.[a-z]{2,}(:\d+)?([/?#].*)?$/i.test(t);
  }

  function construirBusqueda(q) {
    return "https://lite.duckduckgo.com/lite/?q=" + encodeURIComponent(q);
  }

  function marcarError() {
    url.classList.add("err");
    setTimeout(() => url.classList.remove("err"), 1200);
  }

  async function navegar(texto) {
    texto = (texto || "").trim();
    if (!texto) return;
    const target = esUrl(texto)
      ? (/^https?:\/\//i.test(texto) ? texto : "https://" + texto)
      : construirBusqueda(texto);
    try {
      await invoke("navegar", { url: target });
      url.value = target;
    } catch (e) {
      logErr(e);
      marcarError();
    }
  }

  // --- Historial y marcadores --------------------------------------------------
  function pushHistory(u) {
    if (!u || u.startsWith("about:") || u.startsWith("data:")) return;
    if (history[0] && history[0].u === u) return;
    history.unshift({ u, t: new Date().toISOString() });
    history = history.slice(0, 500);
    store.set("history", history);
  }

  function updateStar() {
    const marked = marks.some((m) => m.u === currentUrl);
    btnStar.textContent = marked ? "\u2605" : "\u2606";
    btnStar.style.color = marked ? "#ffcb45" : "";
  }

  // Rust avisa en cada carga de pagina (clics en enlaces, redirecciones, etc.)
  listen("page-load", (e) => {
    const p = e.payload || {};
    if (!p.url) return;
    currentUrl = p.url;
    if (document.activeElement !== url) url.value = p.url;
    if (p.finished) {
      pushHistory(p.url);
      if (drawerKind === "history") abrirDrawer("Historial", "history");
    }
    updateStar();
  }).catch(logErr);

  // --- Botones ------------------------------------------------------------------
  btnBack.onclick   = () => invoke("atras").catch(logErr);
  btnFwd.onclick    = () => invoke("adelante").catch(logErr);
  btnReload.onclick = () => invoke("recargar").catch(logErr);
  btnHome.onclick   = () => { invoke("inicio").catch(logErr); url.value = ""; };
  btnGo.onclick     = () => navegar(url.value);

  url.addEventListener("keydown", (e) => {
    if (e.key === "Enter") { navegar(url.value); url.blur(); }
  });

  document.addEventListener("keydown", (e) => {
    const ctrl = e.ctrlKey || e.metaKey;
    const k = e.key.toLowerCase();
    if (ctrl && k === "l") { e.preventDefault(); url.focus(); url.select(); }
    else if ((ctrl && k === "r") || e.key === "F5") { e.preventDefault(); invoke("recargar").catch(logErr); }
    else if (ctrl && k === "d") { e.preventDefault(); btnStar.click(); }
    else if (e.altKey && e.key === "ArrowLeft")  { e.preventDefault(); invoke("atras").catch(logErr); }
    else if (e.altKey && e.key === "ArrowRight") { e.preventDefault(); invoke("adelante").catch(logErr); }
    else if (e.key === "Escape") { cerrarDrawer(); panel.classList.add("hidden"); }
  });

  btnMenu.onclick = () => panel.classList.toggle("hidden");
  btnTheme.onclick = () => { theme = theme === "dark" ? "light" : "dark"; applyTheme(); };

  btnEco.onclick = async () => {
    eco = !eco;
    btnEco.textContent = "Modo eco: " + (eco ? "on" : "off");
    store.set("eco", eco);
    try { await invoke("set_eco", { v: eco }); } catch (e) { logErr(e); }
  };

  btnAdblock.onclick = async () => {
    adblock = !adblock;
    btnAdblock.textContent = "Adblock: " + (adblock ? "on" : "off");
    store.set("adblock", adblock);
    try { await invoke("set_adblock", { v: adblock }); } catch (e) { logErr(e); }
  };

  $("#btnHist").onclick  = () => abrirDrawer("Historial", "history");
  $("#btnMarks").onclick = () => abrirDrawer("Marcadores", "marks");

  // "Borrar datos" pide confirmacion con un segundo clic (confirm() no es fiable en WebKitGTK)
  let clearArmed = false, clearTimer = null;
  function disarmClear() {
    clearArmed = false;
    clearTimeout(clearTimer);
    btnClear.textContent = "Borrar datos";
  }
  btnClear.onclick = () => {
    if (!clearArmed) {
      clearArmed = true;
      btnClear.textContent = "Seguro? Clic otra vez";
      clearTimer = setTimeout(disarmClear, 3000);
      return;
    }
    disarmClear();
    history = []; marks = [];
    store.set("history", []); store.set("marks", []);
    updateStar();
    if (drawerKind) abrirDrawer(drawerKind === "history" ? "Historial" : "Marcadores", drawerKind);
  };

  // --- Panel lateral ---------------------------------------------------------------
  async function abrirDrawer(titulo, tipo) {
    drawerKind = tipo;
    drawerTitle.textContent = titulo;
    drawerList.innerHTML = "";
    const items = tipo === "history" ? history : marks;

    if (!items.length) {
      const li = document.createElement("li");
      li.className = "empty";
      li.textContent = tipo === "history" ? "Sin historial todavia." : "Sin marcadores.";
      drawerList.appendChild(li);
    } else {
      for (const it of items) {
        const li = document.createElement("li");
        const t = document.createElement("span");
        t.className = "t";
        t.textContent = it.u;
        t.title = it.u;
        t.onclick = () => { navegar(it.u); cerrarDrawer(); };

        const x = document.createElement("button");
        x.className = "x";
        x.textContent = "\u2715";
        x.title = "Quitar";
        x.onclick = (e) => {
          e.stopPropagation();
          if (tipo === "history") { history = history.filter((h) => h !== it); store.set("history", history); }
          else                    { marks   = marks.filter((m) => m !== it);   store.set("marks",   marks); updateStar(); }
          abrirDrawer(titulo, tipo);
        };

        li.appendChild(t);
        li.appendChild(x);
        drawerList.appendChild(li);
      }
    }

    // Primero Rust hace sitio (barra a pantalla completa + contenido mas estrecho)...
    try { await invoke("set_drawer", { open: true }); } catch (e) { logErr(e); }
    // ...y despues se muestra, para que no tape ni parpadee.
    drawer.classList.remove("hidden");
  }

  function cerrarDrawer() {
    if (!drawerKind) return;
    drawerKind = null;
    drawer.classList.add("hidden");
    invoke("set_drawer", { open: false }).catch(logErr);
  }

  $("#drawerClose").onclick = cerrarDrawer;

  btnStar.onclick = () => {
    if (!currentUrl) return;
    const idx = marks.findIndex((m) => m.u === currentUrl);
    if (idx >= 0) marks.splice(idx, 1);
    else marks.unshift({ u: currentUrl, t: new Date().toISOString() });
    store.set("marks", marks);
    updateStar();
    if (drawerKind === "marks") abrirDrawer("Marcadores", "marks");
  };

  // --- Arranque ------------------------------------------------------------------------
  btnEco.textContent = "Modo eco: " + (eco ? "on" : "off");
  btnAdblock.textContent = "Adblock: " + (adblock ? "on" : "off");
  applyTheme();
  updateStar();

  invoke("set_eco",     { v: eco }).catch(logErr);
  invoke("set_adblock", { v: adblock }).catch(logErr);
})();
