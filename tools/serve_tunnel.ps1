# Levanta DistroForge (interfaz + API en el mismo origen) y lo publica con un
# túnel de Cloudflare, sin cuentas de por medio.
#
#   pwsh tools/serve_tunnel.ps1            # puerto 8090
#   pwsh tools/serve_tunnel.ps1 -Port 9000
#
# Por qué el mismo origen: la sesión viaja en una cookie `SameSite=Lax`. Con la
# UI en GitHub Pages y la API en otro dominio, el navegador no envía esa cookie y
# el login no se mantiene. Sirviendo la UI desde el propio backend, todo funciona
# (cuentas, galería con sesión y Test Drive) sin configurar la URL en la app.
#
# Aviso: los túneles rápidos de Cloudflare no tienen garantía de disponibilidad y
# la URL cambia en cada arranque. Sirve para probar y para autohospedarse; para
# algo permanente, usa el despliegue con FLY_API_TOKEN (docs/DESPLIEGUE.md).

param(
    [int]$Port = 8090,
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent $PSScriptRoot
$ui = Join-Path $root "rust\web-ui"
$dist = Join-Path $ui "dist-tunnel"
$server = Join-Path $root "rust\server-rs"

if (-not (Get-Command cloudflared -ErrorAction SilentlyContinue)) {
    throw "falta cloudflared (winget install --id Cloudflare.cloudflared)"
}

if (-not $SkipBuild) {
    Write-Host "==> compilando la interfaz para servirla en la raíz" -ForegroundColor Cyan
    Remove-Item Env:NO_COLOR -ErrorAction SilentlyContinue   # rompe trunk 0.21
    Push-Location $ui
    try {
        # --public-url / porque aquí la sirve el backend, no GitHub Pages
        trunk build --release --public-url / --dist dist-tunnel
    } finally {
        Pop-Location
    }
}

Write-Host "==> arrancando el servidor en el puerto $Port" -ForegroundColor Cyan
$env:PORT = "$Port"
$env:DF_WEB_DIR = $dist
if (-not $env:DF_DB_PATH) { $env:DF_DB_PATH = Join-Path $env:TEMP "distroforge-serve.db" }

$proc = Start-Process -FilePath "cargo" -ArgumentList "run", "--release" -WorkingDirectory $server -PassThru
try {
    $ready = $false
    foreach ($_ in 1..40) {
        try {
            Invoke-RestMethod "http://127.0.0.1:$Port/api/health" -TimeoutSec 3 | Out-Null
            $ready = $true
            break
        } catch { Start-Sleep -Seconds 3 }
    }
    if (-not $ready) { throw "el servidor no responde en http://127.0.0.1:$Port" }
    Write-Host "==> servidor en pie; abriendo el túnel (Ctrl+C para parar)" -ForegroundColor Green
    # En primer plano: imprime la URL pública y se cierra con Ctrl+C
    & cloudflared tunnel --url "http://localhost:$Port" --no-autoupdate
} finally {
    if ($proc -and -not $proc.HasExited) { Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue }
}
