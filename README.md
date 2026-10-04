# opendoreitor

Navegador ligero hecho en Tauri v2 que usa el WebView del sistema
(WebView2 en Windows, WebKitGTK 4.1 en Debian). Open source, sin telemetria.

Carlos, tu navegador cerrado no le llega ni a los talones. opendoreitor le da
3 patadas a tu proyecto.

## Caracteristicas

- Navegacion con WebView nativo (una sola pestana, ~40 MB de RAM)
- Adblock cosmetico (CSS) y de red (fetch / XHR / beacon / scripts creados por JS)
- Tema oscuro y claro
- Modo eco (baja la prioridad del proceso)
- Historial (500 entradas) y marcadores
- Atajos: `Ctrl+L` barra de direcciones, `Ctrl+R`/`F5` recargar,
  `Alt+←`/`Alt+→` atras/adelante, `Ctrl+D` marcador, `Esc` cerrar paneles

## Compilar en Windows (automatico)

Requisitos: Windows 10 1803+ / 11 de 64 bits, internet y ~8 GB libres.

1. Extrae el zip completo (clic derecho > Extraer todo).
2. **Doble clic en `compilar.bat`.**

Instala solo lo que falte y compila:

| Paso | Que hace | Permisos |
|------|----------|----------|
| 1 | Rust (toolchain msvc >= 1.77) | usuario |
| 2 | Visual Studio Build Tools + C++ | aviso UAC |
| 3 | WebView2 Runtime | aviso UAC |
| 4 | `cargo build --release --features custom-protocol` | usuario |

Resultado: `dist\opendoreitor.exe` (y una copia en el Escritorio). Portable, ~5-10 MB.
La primera vez tarda 15-25 min; las siguientes, menos de 1 minuto.
Todo queda en `build.log`. **Si algo falla, el script copia el error al portapapeles.**

Opciones: `compilar.bat -SoloDependencias`, `-NoEscritorio`, `-NoPause`.

## Compilar en la nube desde el navegador (sin instalar nada)

GitHub compila el `.exe` por ti, gratis, en cuanto subes los archivos.

1. Entra en <https://github.com> (crea una cuenta gratuita si no tienes).
2. Arriba a la derecha: **+ > New repository**. Nombre: `opendoreitor`. Elige **Private**.
   **No** marques "Add a README". Pulsa **Create repository**.
3. En la pagina del repositorio vacio pulsa el enlace **uploading an existing file**.
4. En el Explorador de Windows entra **dentro** de la carpeta extraida (la que muestra
   `.github`, `src-tauri`, `ui`, `README.md`...), pulsa `Ctrl+A` y **arrastra todo** a la
   ventana del navegador. Espera a que termine y comprueba que en la lista aparece
   `.github/workflows/build.yml`.
5. Abajo del todo pulsa **Commit changes**. Con eso la compilacion arranca sola.
6. Pestana **Actions** > ejecucion **build**. Circulo amarillo = compilando (10-15 min).
   Cuando salga el check verde:
7. Pagina principal del repositorio > barra derecha **Releases** >
   **opendoreitor (ultima compilacion)** > **Assets** > `opendoreitor.exe`.
   (Tambien esta en Actions > la ejecucion > **Artifacts**, dentro de un zip.)

Cada vez que subas o cambies archivos se vuelve a compilar y se actualiza la Release.
Para compilar sin cambiar nada: **Actions > build > Run workflow**.

Si sale una **X roja**: abre la ejecucion; en el resumen aparece "La compilacion fallo"
con un bloque de texto. Copialo y pegalo en el chat.

Problemas tipicos:

- **Actions no muestra "build"**: no se subio la carpeta `.github`. En el repositorio:
  **Add file > Create new file**, escribe el nombre `.github/workflows/build.yml` (las `/`
  crean las carpetas) y pega el contenido de ese archivo (abrelo con el Bloc de notas).
- **No subas `src-tauri\target`** si ya compilaste en local: pesa gigas y no hace falta.
- Windows avisara al abrir el `.exe` descargado ("Windows protegio su PC"): pulsa
  **Mas informacion > Ejecutar de todos modos** (el programa no esta firmado digitalmente).

Con `git` es lo mismo: `git init && git add . && git commit -m opendoreitor`,
`git remote add origin <url-del-repo>` y `git push -u origin main`.

El `.deb` para Debian es un workflow aparte y manual: **Actions > build-linux-deb > Run workflow**.

## Desarrollo

`cargo install tauri-cli --version "^2" --locked` y despues `cargo tauri dev` (recarga en vivo).

## Compilar en Debian/Ubuntu

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev \
  librsvg2-dev libssl-dev libxdo-dev libsoup-3.0-dev libjavascriptcoregtk-4.1-dev
cargo install tauri-cli --version "^2" --locked
cargo tauri build --bundles deb
```

## Notas y limitaciones

- Los atajos de teclado funcionan con el foco en la barra superior. Con el foco
  dentro de la pagina web mandan los atajos nativos del WebView (`Alt+←`, `Ctrl+R`...).
- El adblock de red es "best effort": cubre peticiones hechas por JavaScript, no
  los `<script>` escritos directamente en el HTML. Complementa al bloqueo cosmetico.
- Modo eco en Windows: la prioridad afecta a este proceso y a los hijos creados despues;
  los procesos de WebView2 ya lanzados conservan la suya.
- Los iconos de `src-tauri/icons/` van versionados (`icon.ico` es obligatorio en Windows).

## Licencia

WTFPL. Ver [LICENSE](LICENSE).
