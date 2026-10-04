@echo off
rem Doble clic: instala lo que falte (Rust, Build Tools C++, WebView2) y compila el .exe.
cd /d "%~dp0"
if not exist "build.ps1" (
  echo.
  echo Falta build.ps1: parece que ejecutas esto DENTRO del zip.
  echo Clic derecho sobre el zip y "Extraer todo...", luego ejecuta compilar.bat desde la carpeta extraida.
  echo.
  pause
  exit /b 1
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build.ps1" %*
