# install_clients.ps1 — instala los clientes externos que Karto invoca en runtime.
#
# Equivalente para Windows de install_clients.sh. Karto no empaqueta clientes:
# lanza las herramientas del sistema. Este script detecta cuáles faltan en el
# PATH y las instala con winget o, cuando no hay paquete decente, con el zip
# oficial del proveedor en %LOCALAPPDATA%\Karto\tools (y ajusta el PATH de usuario).
#
# La lista refleja lo que el backend busca de verdad:
#   - detect_tools()   (usecases/diagnostics.rs)
#   - build_db_command (usecases/scripts.rs)
#
# Nota: el soporte de runtime en Windows está en curso (docs/specs/windows-adapt.md).
# Este script deja las herramientas listas; lo que Karto ya sabe lanzar en Windows
# lo marca ese documento.
#
# Uso:
#   .\install_clients.ps1                  # instala todo lo que falte (pregunta antes)
#   .\install_clients.ps1 -Check           # solo reporta, no instala
#   .\install_clients.ps1 -Only pg,redis   # instala solo esos grupos
#   .\install_clients.ps1 -Yes             # sin confirmación (CI / desatendido)
#   .\install_clients.ps1 -List            # lista los grupos disponibles
#
# Variables de entorno:
#   KARTO_MONGOSH_VERSION   fija la versión de mongosh en vez de tomar la última

[CmdletBinding()]
param(
  [switch]$Check,
  [switch]$Yes,
  [string[]]$Only,
  [switch]$List
)

$ErrorActionPreference = 'Stop'
# PowerShell 5.1 negocia TLS 1.0 por defecto: sin esto fallan las descargas.
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Write-Die  { param($m) Write-Host "x $m" -ForegroundColor Red;    exit 1 }
function Write-Info { param($m) Write-Host "-> $m" -ForegroundColor Cyan }
function Write-Ok   { param($m) Write-Host "OK $m" -ForegroundColor Green }
function Write-Warn { param($m) Write-Host "!  $m" -ForegroundColor Yellow }

$ToolsRoot = Join-Path $env:LOCALAPPDATA 'Karto\tools'

# --- Utilidades --------------------------------------------------------------

function Test-Admin {
  $id = [Security.Principal.WindowsIdentity]::GetCurrent()
  (New-Object Security.Principal.WindowsPrincipal($id)).IsInRole(
    [Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Test-Bin { param([string]$Name) [bool](Get-Command $Name -ErrorAction SilentlyContinue) }

# Añade un directorio al PATH del usuario y a la sesión actual (idempotente).
function Add-ToUserPath {
  param([string]$Dir)
  $cur = [Environment]::GetEnvironmentVariable('Path', 'User')
  if (($cur -split ';') -notcontains $Dir) {
    if ([string]::IsNullOrEmpty($cur)) { $new = $Dir } else { $new = "$cur;$Dir" }
    [Environment]::SetEnvironmentVariable('Path', $new, 'User')
    Write-Info "PATH de usuario += $Dir"
  }
  if (($env:Path -split ';') -notcontains $Dir) { $env:Path = "$env:Path;$Dir" }
}

function Install-Winget {
  param([string]$Id)
  if (-not (Test-Bin 'winget')) {
    throw "Falta winget (App Installer). Instálalo desde Microsoft Store y reintenta."
  }
  Write-Info "winget install $Id"
  & winget install --id $Id --exact --source winget `
      --accept-package-agreements --accept-source-agreements --silent | Out-Host
  # 0 = ok; -1978335189 = ya instalado. Cualquier otro código es fallo real.
  if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne -1978335189) {
    throw "winget devolvió $LASTEXITCODE al instalar $Id"
  }
}

# Descarga un zip y lo extrae en $ToolsRoot\$Name, dejando en el PATH el
# directorio que contiene $Exe (los zips varían: a veces hay carpeta intermedia).
function Install-PortableZip {
  param([string]$Name, [string]$Url, [string]$Exe)
  $dest = Join-Path $ToolsRoot $Name
  $tmp  = Join-Path ([IO.Path]::GetTempPath()) ("karto-$Name-" + [guid]::NewGuid().ToString('N') + '.zip')
  Write-Info "Descargando $Name desde $Url"
  Invoke-WebRequest -Uri $Url -OutFile $tmp -UseBasicParsing
  if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
  New-Item -ItemType Directory -Path $dest -Force | Out-Null
  Expand-Archive -Path $tmp -DestinationPath $dest -Force
  Remove-Item $tmp -Force
  $found = Get-ChildItem $dest -Recurse -Filter $Exe -File | Select-Object -First 1
  if (-not $found) { throw "El zip de $Name no contiene $Exe" }
  Add-ToUserPath $found.Directory.FullName
}

# Busca el subdirectorio bin de un producto instalado bajo Archivos de programa
# y lo mete al PATH (los instaladores de PostgreSQL/MariaDB no lo hacen).
function Add-ProductBinToPath {
  param([string]$Parent, [string]$Filter, [string]$Exe)
  # $Parent vacío = buscar directamente bajo Archivos de programa (MariaDB 12.3\bin).
  foreach ($root in @($env:ProgramFiles, ${env:ProgramFiles(x86)})) {
    if (-not $root) { continue }
    if ($Parent) { $base = Join-Path $root $Parent } else { $base = $root }
    if (-not (Test-Path $base)) { continue }
    $dirs = Get-ChildItem $base -Directory -Filter $Filter -ErrorAction SilentlyContinue |
              Sort-Object Name -Descending
    foreach ($d in $dirs) {
      $bin = Join-Path $d.FullName 'bin'
      if (Test-Path (Join-Path $bin $Exe)) { Add-ToUserPath $bin; return $true }
    }
  }
  return $false
}

# Última versión publicada en un repo de GitHub (sin token: basta para un release).
function Get-LatestGitHubRelease {
  param([string]$Repo)
  (Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" `
     -Headers @{ 'User-Agent' = 'karto-install-clients' }).tag_name -replace '^v', ''
}

# --- Instaladores por grupo --------------------------------------------------

function Install-Ssh {
  if (-not (Test-Admin)) {
    throw "El cliente OpenSSH es una capacidad de Windows: reabre PowerShell como administrador."
  }
  $cap = Get-WindowsCapability -Online -Name 'OpenSSH.Client*' | Select-Object -First 1
  if (-not $cap) { throw "Esta edición de Windows no ofrece la capacidad OpenSSH.Client." }
  Add-WindowsCapability -Online -Name $cap.Name | Out-Null
  Add-ToUserPath (Join-Path $env:SystemRoot 'System32\OpenSSH')
  Write-Warn "ssh-copy-id no existe en Windows; el aprovisionamiento de llaves usa otra ruta (ver docs/specs/windows-adapt.md)."
}

function Install-Psql {
  Install-Winget 'PostgreSQL.PostgreSQL.17'
  if (-not (Add-ProductBinToPath 'PostgreSQL' '*' 'psql.exe')) {
    Write-Warn "psql se instaló pero no encontré su carpeta bin; añádela al PATH a mano."
  }
  Write-Warn "El paquete de PostgreSQL incluye el servidor; Karto solo usa psql."
}

function Install-Mysql {
  Install-Winget 'MariaDB.Server'
  if (-not (Add-ProductBinToPath '' 'MariaDB*' 'mysql.exe')) {
    Write-Warn "mysql se instaló pero no encontré su carpeta bin; añádela al PATH a mano."
  }
  Write-Warn "MariaDB incluye el servidor; Karto solo usa el cliente mysql (compatible con MySQL)."
}

function Install-Mongosh {
  $ver = $env:KARTO_MONGOSH_VERSION
  if (-not $ver) { $ver = Get-LatestGitHubRelease 'mongodb-js/mongosh' }
  if (-not $ver) { throw "No pude resolver la versión de mongosh (fija KARTO_MONGOSH_VERSION)." }
  Install-PortableZip 'mongosh' "https://downloads.mongodb.com/compass/mongosh-$ver-win32-x64.zip" 'mongosh.exe'
}

function Install-RedisCli {
  # Redis no publica binarios de Windows. redis-windows sigue las versiones
  # actuales (8.x) y su zip "sin servicio" trae redis-cli.exe con sus DLLs al
  # lado. Importa que sea >= 6: Karto pasa el secreto por REDISCLI_AUTH, que las
  # portabilidades antiguas (el fork 5.0 de tporadowski) no entienden.
  $rel = Invoke-RestMethod -Uri 'https://api.github.com/repos/redis-windows/redis-windows/releases/latest' `
           -Headers @{ 'User-Agent' = 'karto-install-clients' }
  $asset = $rel.assets | Where-Object { $_.name -like '*-msys2.zip' } | Select-Object -First 1
  if (-not $asset) {
    $asset = $rel.assets | Where-Object { $_.name -like '*-cygwin.zip' } | Select-Object -First 1
  }
  if (-not $asset) { throw "No encontré un zip de redis-cli en el último release de redis-windows." }
  Install-PortableZip 'redis' $asset.browser_download_url 'redis-cli.exe'
}

function Install-VncViewer {
  # RealVNC Viewer es de los pocos que registran el esquema vnc://, que es como
  # Karto abre la conexión (build_vnc -> start vnc://host:puerto).
  Install-Winget 'RealVNC.VNCViewer'
}

function Install-Terminal { Install-Winget 'Microsoft.WindowsTerminal' }

# --- Catálogo ----------------------------------------------------------------
# Mode: all = hacen falta todos los binarios · any = basta con uno.
$Catalog = @(
  @{ Name='ssh';      Mode='all'; Bins=@('ssh');        Desc='SSH: conectar y ejecutar scripts';                Install=${function:Install-Ssh} }
  @{ Name='terminal'; Mode='any'; Bins=@('wt');         Desc='Windows Terminal (terminal preferida de Karto)';  Install=${function:Install-Terminal} }
  @{ Name='vnc';      Mode='any'; Bins=@('vncviewer');  Desc='Visor VNC registrado para vnc://';                Install=${function:Install-VncViewer} }
  @{ Name='pg';       Mode='all'; Bins=@('psql');       Desc='PostgreSQL: cliente psql';                        Install=${function:Install-Psql} }
  @{ Name='mysql';    Mode='all'; Bins=@('mysql');      Desc='MySQL / MariaDB: cliente mysql';                  Install=${function:Install-Mysql} }
  @{ Name='mongo';    Mode='all'; Bins=@('mongosh');    Desc='MongoDB: cliente mongosh';                        Install=${function:Install-Mongosh} }
  @{ Name='redis';    Mode='all'; Bins=@('redis-cli');  Desc='Redis: cliente redis-cli';                        Install=${function:Install-RedisCli} }
)

function Get-Group { param([string]$Name) $Catalog | Where-Object { $_.Name -eq $Name } | Select-Object -First 1 }

# Devuelve los binarios del grupo hallados en el PATH.
function Get-FoundBins { param($G) @($G.Bins | Where-Object { Test-Bin $_ }) }

function Test-GroupSatisfied {
  param($G)
  $found = Get-FoundBins $G
  if ($G.Mode -eq 'any') { return $found.Count -gt 0 }
  return $found.Count -eq $G.Bins.Count
}

function Show-Group {
  param($G)
  if (Test-GroupSatisfied $G) {
    Write-Host ("  OK   {0,-9} {1}" -f $G.Name, ((Get-FoundBins $G) -join ' ')) -ForegroundColor Green
    return $true
  }
  Write-Host ("  ..   {0,-9} falta - {1}" -f $G.Name, $G.Desc) -ForegroundColor Yellow
  return $false
}

# --- CLI ---------------------------------------------------------------------

if ($List) {
  foreach ($g in $Catalog) { "  {0,-9} {1}" -f $g.Name, $g.Desc }
  exit 0
}

# Se acepta tanto `-Only pg,redis` (array) como `-Only "pg,redis"`, que es lo que
# llega cuando el script se invoca con `powershell -File`.
if ($Only) { $selected = @($Only -split ',' | Where-Object { $_ }) }
else       { $selected = @($Catalog | ForEach-Object { $_.Name }) }
foreach ($n in $selected) {
  if (-not (Get-Group $n)) { Write-Die "Grupo desconocido: $n (usa -List)" }
}

Write-Info "Windows $([Environment]::OSVersion.Version) - admin: $(Test-Admin)"
Write-Host ""

# `web` lo cubre el propio SO: build_open_url usa `cmd /C start`, siempre presente.
Write-Host "  OK   web       lo provee el sistema (cmd /C start)" -ForegroundColor Green

$missing = @()
foreach ($n in $selected) {
  $g = Get-Group $n
  if (-not (Show-Group $g)) { $missing += $g }
}
Write-Host ""

if ($missing.Count -eq 0) { Write-Ok "Todo listo: no falta ningún cliente."; exit 0 }

if ($Check) {
  Write-Info ("Faltan {0} grupo(s): {1}" -f $missing.Count, (($missing | ForEach-Object { $_.Name }) -join ' '))
  exit 1
}

Write-Info ("Se instalarán: {0}" -f (($missing | ForEach-Object { $_.Name }) -join ' '))
if ($missing.Name -contains 'ssh' -and -not (Test-Admin)) {
  Write-Warn "El grupo ssh necesita PowerShell como administrador; los demás no."
}
if (-not $Yes) {
  $answer = Read-Host "  ¿Continuar? [S/n]"
  if ($answer -and $answer -notmatch '^[sSyY]$') { Write-Die "Cancelado." }
}

# --- Instalación -------------------------------------------------------------
$failed = @()
foreach ($g in $missing) {
  Write-Host ""
  Write-Info "Instalando $($g.Name) - $($g.Desc)"
  try { & $g.Install; Write-Ok "$($g.Name) listo" }
  catch { Write-Warn "$($g.Name) falló: $($_.Exception.Message)"; $failed += $g.Name }
}

# --- Verificación ------------------------------------------------------------
Write-Host ""
Write-Info "Verificación final (lo mismo que Karto registra en su log al arrancar):"
foreach ($n in $selected) {
  $g = Get-Group $n
  if (Test-GroupSatisfied $g) {
    Write-Host ("  OK   {0,-9} {1}" -f $g.Name, ((Get-FoundBins $g) -join ' ')) -ForegroundColor Green
  } else {
    Write-Host ("  X    {0,-9} sigue faltando" -f $g.Name) -ForegroundColor Red
  }
}

if ($failed.Count -gt 0) { Write-Host ""; Write-Die ("Falló la instalación de: {0}" -f ($failed -join ' ')) }
Write-Host ""
Write-Ok "Clientes de Karto instalados."
Write-Warn "Abre una terminal nueva para que los cambios de PATH surtan efecto."
