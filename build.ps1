<#
  opendoreitor - instala TODO lo necesario y compila el .exe portable.

  Uso normal: doble clic en compilar.bat

  Parametros (opcionales):
    -SoloDependencias   instala lo que falte pero no compila
    -NoEscritorio       no copia el .exe al Escritorio
    -NoPause            no espera Enter al terminar

  Pasos (solo instala lo que no encuentra):
    1. Rust (toolchain msvc, version >= 1.77)    -> sin Administrador
    2. Visual Studio Build Tools + C++           -> aviso UAC
    3. WebView2 Runtime                          -> aviso UAC
    4. Compila: cargo build --release --features custom-protocol
       (equivale a "cargo tauri build --no-bundle", sin necesitar la CLI de Tauri)
  El resultado se copia a dist\ y al Escritorio.
#>
param(
    [switch]$SoloDependencias,
    [switch]$NoEscritorio,
    [switch]$NoPause
)

Set-Location $PSScriptRoot
$ErrorActionPreference = "Continue"
$log = Join-Path $PSScriptRoot "build.log"
$tmp = Join-Path $env:TEMP "opendoreitor-setup"
New-Item -ItemType Directory -Force $tmp | Out-Null
$exitCode = 0
$transcribiendo = $false
try { Start-Transcript -Path $log -Force | Out-Null; $transcribiendo = $true } catch { }

# ---------------------------------------------------------------- utilidades
function Step($n, $texto) { Write-Host ""; Write-Host "[$n/4] $texto" -ForegroundColor Cyan }
function Ok($texto)   { Write-Host "  OK  $texto" -ForegroundColor Green }
function Info($texto) { Write-Host "  ... $texto" }
function Have($cmd)   { return [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }

function Refresh-Path {
    $env:Path = [Environment]::GetEnvironmentVariable("Path", "Machine") + ";" +
                [Environment]::GetEnvironmentVariable("Path", "User") + ";" +
                (Join-Path $env:USERPROFILE ".cargo\bin")
}

# Ejecuta un programa mostrando (y registrando en build.log) TODA su salida,
# incluida la de error, y devuelve su codigo de salida.
function Exec($exe, $argv) {
    & $exe @argv 2>&1 | ForEach-Object { "$_" } | Out-Host
    return $LASTEXITCODE
}

function Download($url, $dest) {
    [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
    $old = $ProgressPreference
    $ProgressPreference = "SilentlyContinue"
    try {
        Info "Descargando $url"
        Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing
    } catch {
        throw "No se pudo descargar $url`nComprueba tu conexion a internet. Detalle: $($_.Exception.Message)"
    } finally {
        $ProgressPreference = $old
    }
}

# Ejecuta un instalador con permisos de administrador (aparece el aviso UAC).
function Run-Admin($file, $argList) {
    try {
        $p = Start-Process -FilePath $file -ArgumentList $argList -Verb RunAs -Wait -PassThru -ErrorAction Stop
    } catch {
        throw "No se concedieron permisos de administrador (UAC). Vuelve a ejecutar y pulsa 'Si'. Si tu PC no te deja ser administrador, usa la compilacion en la nube (ver README)."
    }
    return $p.ExitCode
}

function Has-VCTools {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $p = & $vswhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    return [bool]$p
}

function Has-WebView2 {
    $guid = "{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
    $keys = @(
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$guid",
        "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$guid",
        "HKCU:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$guid"
    )
    foreach ($k in $keys) {
        if (Test-Path $k) {
            $v = (Get-ItemProperty $k -ErrorAction SilentlyContinue).pv
            if ($v -and $v -ne "0.0.0.0") { return $true }
        }
    }
    return $false
}

# ---------------------------------------------------------------- proceso
try {
    Write-Host "=== opendoreitor: instalacion automatica + compilacion ===" -ForegroundColor White
    Write-Host "Carpeta: $PSScriptRoot"

    if (-not (Test-Path "src-tauri\Cargo.toml")) {
        throw "No encuentro src-tauri\Cargo.toml. Extrae el zip COMPLETO (clic derecho > Extraer todo) y ejecuta compilar.bat desde la carpeta extraida."
    }
    if ($PSScriptRoot -match "[^\x00-\x7F]") {
        Write-Host "AVISO: la ruta tiene acentos o letras especiales. Si falla el enlazado, mueve la carpeta a C:\opendoreitor." -ForegroundColor Yellow
    }

    # ---- 1) Rust -----------------------------------------------------------
    Step 1 "Rust"
    $arch = if ($env:PROCESSOR_ARCHITECTURE -eq "ARM64") { "aarch64" } else { "x86_64" }
    Refresh-Path
    if (-not (Have cargo) -or -not (Have rustc)) {
        $rustup = Join-Path $tmp "rustup-init.exe"
        Download "https://win.rustup.rs/$arch" $rustup
        Info "Instalando Rust (stable, msvc)..."
        $code = Exec $rustup @("-y", "--default-toolchain", "stable", "--profile", "minimal", "--default-host", "$arch-pc-windows-msvc")
        if ($code -ne 0) { throw "rustup-init fallo (codigo $code)" }
        Refresh-Path
    }
    if (-not (Have cargo)) { throw "Rust se instalo pero cargo no aparece en el PATH. Cierra esta ventana y ejecuta compilar.bat de nuevo." }

    # Tauri necesita el toolchain msvc (el gnu no sirve en Windows)
    $hostInfo = (& rustc -vV 2>&1 | ForEach-Object { "$_" }) -join "`n"
    if ($hostInfo -notmatch "host: \S*msvc") {
        Info "El toolchain por defecto no es msvc; cambiando..."
        $null = Exec "rustup" @("default", "stable-$arch-pc-windows-msvc")
    }
    # Version minima 1.77
    $m = [regex]::Match(((& rustc --version) -join " "), "rustc (\d+)\.(\d+)")
    if ($m.Success -and [int]$m.Groups[1].Value -eq 1 -and [int]$m.Groups[2].Value -lt 77) {
        Info "Rust demasiado antiguo; actualizando..."
        $null = Exec "rustup" @("update", "stable")
    }
    $null = Exec "rustup" @("component", "add", "rustfmt", "clippy")
    Ok ((& rustc --version) -join " ")

    # ---- 2) Visual Studio Build Tools (C++) ----------------------------------------
    Step 2 "Visual Studio Build Tools (C++)"
    if (Has-VCTools) {
        Ok "C++ (VC.Tools.x86.x64) ya instalado"
    } else {
        $vs = Join-Path $tmp "vs_BuildTools.exe"
        Download "https://aka.ms/vs/17/release/vs_BuildTools.exe" $vs
        Info "Instalando Build Tools con C++ (5-15 min, ~6 GB). Acepta el aviso de administrador."
        $code = Run-Admin $vs @("--quiet", "--wait", "--norestart", "--add", "Microsoft.VisualStudio.Workload.VCTools", "--includeRecommended")
        if ($code -ne 0 -and $code -ne 3010) { throw "El instalador de Build Tools termino con codigo $code" }
        if ($code -eq 3010) { Write-Host "  AVISO: Windows pide reiniciar. Si la compilacion falla, reinicia y ejecuta compilar.bat otra vez." -ForegroundColor Yellow }
        if (-not (Has-VCTools)) { throw "Build Tools no aparece como instalado. Reinicia Windows y ejecuta compilar.bat otra vez." }
        Ok "Build Tools instalados"
    }

    # ---- 3) WebView2 -----------------------------------------------------------------------
    Step 3 "WebView2 Runtime"
    if (Has-WebView2) {
        Ok "WebView2 ya instalado"
    } else {
        $wv = Join-Path $tmp "MicrosoftEdgeWebview2Setup.exe"
        Download "https://go.microsoft.com/fwlink/p/?LinkId=2124703" $wv
        $code = Run-Admin $wv @("/silent", "/install")
        if ($code -ne 0) { Write-Host "  AVISO: WebView2 devolvio codigo $code (no impide compilar, pero el .exe lo necesita para abrirse)." -ForegroundColor Yellow }
        else { Ok "WebView2 instalado" }
    }

    if ($SoloDependencias) {
        Write-Host ""
        Write-Host "Dependencias listas. Ejecuta compilar.bat para generar el .exe." -ForegroundColor Green
    } else {
        # ---- 4) Compilar ----------------------------------------------------------------------
        Step 4 "Compilando (la primera vez tarda 5-15 min; no cierres la ventana)"
        if (-not (Test-Path "src-tauri\icons\icon.ico")) {
            throw "Falta src-tauri\icons\icon.ico (obligatorio en Windows). Extrae de nuevo el zip completo."
        }
        $code = Exec "cargo" @("build", "--release", "--features", "custom-protocol", "--manifest-path", "src-tauri\Cargo.toml")
        if ($code -ne 0) { throw "La compilacion fallo (codigo $code)." }

        $targetDir = if ($env:CARGO_TARGET_DIR) { $env:CARGO_TARGET_DIR } else { Join-Path $PSScriptRoot "src-tauri\target" }
        $exe = Join-Path $targetDir "release\opendoreitor.exe"
        if (-not (Test-Path $exe)) { throw "Compilo bien pero no encuentro $exe" }

        New-Item -ItemType Directory -Force dist | Out-Null
        Copy-Item $exe "dist\opendoreitor.exe" -Force
        $mb = [math]::Round((Get-Item $exe).Length / 1MB, 1)
        Write-Host ""
        Write-Host "LISTO: $PSScriptRoot\dist\opendoreitor.exe  ($mb MB)" -ForegroundColor Green

        if (-not $NoEscritorio) {
            $desk = [Environment]::GetFolderPath("Desktop")
            if ($desk -and (Test-Path $desk)) {
                $dest = Join-Path $desk "opendoreitor.exe"
                Copy-Item $exe $dest -Force
                Write-Host "Copia en el Escritorio: $dest" -ForegroundColor Green
                Start-Process explorer.exe -ArgumentList "/select,`"$dest`""
            }
        }
    }
} catch {
    $exitCode = 1
    Write-Host ""
    Write-Host "ERROR: $($_.Exception.Message)" -ForegroundColor Red
    Write-Host "Registro completo en: $log" -ForegroundColor Red
} finally {
    if ($transcribiendo) { try { Stop-Transcript | Out-Null } catch { } }
}

# Si algo fallo, deja el error en el portapapeles para pegarlo directamente en el chat.
if ($exitCode -ne 0) {
    try {
        $cola = Get-Content $log -Tail 80 -ErrorAction Stop
        Set-Clipboard -Value ($cola -join "`r`n")
        Write-Host "He copiado las ultimas lineas del error al portapapeles: pegalas (Ctrl+V) en el chat." -ForegroundColor Yellow
    } catch { }
}

if (-not $NoPause) { Write-Host ""; Read-Host "Pulsa Enter para cerrar" | Out-Null }
exit $exitCode
