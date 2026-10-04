//! Adblock minimalista: cosmetico por CSS + bloqueo de trackers por JS.
//!
//! El script generado es IDEMPOTENTE: se puede evaluar muchas veces (al cargar
//! la pagina y cada vez que cambia un ajuste) y siempre deja la pagina en el
//! estado correcto, anadiendo o quitando sus <style> segun los ajustes.

pub const COSMETIC_CSS: &str = r#"
.adsbygoogle,ins.adsbygoogle,
div[id^="google_ads"],div[id^="aswift_"],div[id^="div-gpt-ad"],
iframe[id^="google_ads_iframe"],iframe[src*="doubleclick.net"],
iframe[src*="googlesyndication.com"],iframe[src*="adservice.google"],
[class*=" ad-"],[class*=" ads-"],[class*="advert"],[id*="advert"],
[class*="sponsored"],[class*="taboola"],[class*="outbrain"],
[class*="mgid"],[class*="revcontent"],[class*="propellerads"],
[class*="popads"],[class*="adcash"],[id^="taboola-"],
.taboola,.trc_related_container,.OUTBRAIN,
[data-ad],[data-ads],[data-ad-slot],[data-ad-client],
div[class*="banner-ad"],div[id*="banner-ad"],
.promoted,.promo-ad,.sponsored-post,.sponsored-content
{display:none!important;visibility:hidden!important;height:0!important;width:0!important}
"#;

pub const TRACKER_DOMAINS: &[&str] = &[
    "doubleclick.net",
    "googlesyndication.com",
    "googleadservices.com",
    "google-analytics.com",
    "googletagmanager.com",
    "googletagservices.com",
    "adservice.google.com",
    "scorecardresearch.com",
    "quantserve.com",
    "taboola.com",
    "outbrain.com",
    "criteo.com",
    "criteo.net",
    "adnxs.com",
    "rubiconproject.com",
    "pubmatic.com",
    "openx.net",
    "casalemedia.com",
    "sharethrough.com",
    "mgid.com",
    "revcontent.com",
    "propellerads.com",
    "popads.net",
    "adcash.com",
    "connect.facebook.net",
];

/// Plantilla JS. Los marcadores __X__ se sustituyen en `build_injection_script`.
const JS_TEMPLATE: &str = r##"(function () {
  'use strict';
  var C = window.__OPD || (window.__OPD = {});
  C.adblock = __ADBLOCK__;
  C.eco = __ECO__;
  C.dark = __DARK__;

  var COSMETIC = __CSS__;
  var BLOCKED = __DOMAINS__;
  var ECO_CSS = '*,*::before,*::after{animation-play-state:paused!important;transition:none!important;will-change:auto!important}';

  // Compara por hostname (no por subcadena) para evitar falsos positivos.
  function isBlocked(u) {
    if (!C.adblock) return false;
    try {
      var h = new URL(String(u), location.href).hostname.toLowerCase();
      for (var i = 0; i < BLOCKED.length; i++) {
        var d = BLOCKED[i];
        if (h === d || h.slice(-(d.length + 1)) === '.' + d) return true;
      }
    } catch (e) {}
    return false;
  }

  // Los hooks de red se instalan UNA sola vez por pagina y consultan C.adblock en vivo.
  if (!C.hooked) {
    C.hooked = true;

    try {
      var _fetch = window.fetch;
      if (typeof _fetch === 'function') {
        window.fetch = function (input, init) {
          var u = (typeof input === 'string') ? input : ((input && input.url) || String(input));
          if (isBlocked(u)) return Promise.reject(new TypeError('Failed to fetch (bloqueado)'));
          return _fetch.apply(this, arguments);
        };
      }
    } catch (e) {}

    try {
      var XP = window.XMLHttpRequest && window.XMLHttpRequest.prototype;
      if (XP) {
        var _open = XP.open, _send = XP.send;
        XP.open = function (m, u) {
          try { this.__opdBlk = isBlocked(u); } catch (e) {}
          return _open.apply(this, arguments);
        };
        XP.send = function () {
          if (this.__opdBlk) {
            var x = this;
            // Simula un fallo de red en vez de dejar la peticion colgada para siempre.
            setTimeout(function () { try { x.dispatchEvent(new Event('error')); } catch (e) {} }, 0);
            return;
          }
          return _send.apply(this, arguments);
        };
      }
    } catch (e) {}

    try {
      if (navigator.sendBeacon) {
        var _sb = navigator.sendBeacon.bind(navigator);
        navigator.sendBeacon = function (u, d) { return isBlocked(u) ? false : _sb(u, d); };
      }
    } catch (e) {}

    // <script src>, <img src>, <iframe src> creados por JS
    try {
      [window.HTMLScriptElement, window.HTMLImageElement, window.HTMLIFrameElement].forEach(function (K) {
        if (!K) return;
        var desc = Object.getOwnPropertyDescriptor(K.prototype, 'src');
        if (!desc || !desc.set) return;
        Object.defineProperty(K.prototype, 'src', {
          configurable: true,
          enumerable: desc.enumerable,
          get: desc.get,
          set: function (v) { if (isBlocked(v)) return; desc.set.call(this, v); }
        });
      });
      var _sa = Element.prototype.setAttribute;
      Element.prototype.setAttribute = function (n, v) {
        try {
          if (typeof n === 'string' && n.toLowerCase() === 'src' && isBlocked(v)) {
            var t = this.tagName;
            if (t === 'SCRIPT' || t === 'IMG' || t === 'IFRAME') return;
          }
        } catch (e) {}
        return _sa.apply(this, arguments);
      };
    } catch (e) {}
  }

  // Anade el <style> con ese id, o lo quita si css esta vacio.
  function setStyle(id, css) {
    var s = document.getElementById(id);
    if (!css) { if (s && s.parentNode) s.parentNode.removeChild(s); return; }
    if (!s) { s = document.createElement('style'); s.id = id; }
    if (s.textContent !== css) s.textContent = css;
    var root = document.head || document.documentElement;
    if (root && s.parentNode !== root) root.appendChild(s);
  }

  function applyStyles() {
    try {
      setStyle('opd-adblock', C.adblock ? COSMETIC : '');
      setStyle('opd-eco', C.eco ? ECO_CSS : '');
      setStyle('opd-theme', 'html{color-scheme:' + (C.dark ? 'dark' : 'light') + '!important}');
    } catch (e) {}
  }

  C.applyStyles = applyStyles;
  applyStyles();
  // Si el script corre antes de que exista el documento, reintenta al estar listo.
  if (!document.documentElement) {
    document.addEventListener('DOMContentLoaded', function () { C.applyStyles(); }, { once: true });
  }
})();
"##;

pub fn build_injection_script(adblock_enabled: bool, eco_enabled: bool, dark: bool) -> String {
    let css = serde_json::to_string(COSMETIC_CSS).unwrap_or_else(|_| "\"\"".into());
    let domains = serde_json::to_string(TRACKER_DOMAINS).unwrap_or_else(|_| "[]".into());
    let b = |v: bool| if v { "true" } else { "false" };

    JS_TEMPLATE
        .replace("__ADBLOCK__", b(adblock_enabled))
        .replace("__ECO__", b(eco_enabled))
        .replace("__DARK__", b(dark))
        .replace("__DOMAINS__", &domains)
        .replace("__CSS__", &css)
}
