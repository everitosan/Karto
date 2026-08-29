#!/usr/bin/env bash
# install_clients.sh — instala los clientes externos que Karto invoca en runtime.
#
# Karto no empaqueta clientes: lanza las herramientas del sistema. Este script
# detecta cuáles faltan en el PATH y las instala con el gestor de paquetes nativo
# (apt / dnf / pacman / zypper / brew). Es idempotente: lo ya presente se omite.
#
# La lista refleja lo que el backend busca de verdad:
#   - detect_tools()   (usecases/diagnostics.rs)
#   - LINUX_TERMINALS  (usecases/connections.rs)
#   - build_db_command (usecases/scripts.rs)
#
# Uso:
#   ./install_clients.sh                 # instala todo lo que falte (pregunta antes)
#   ./install_clients.sh --check         # solo reporta, no instala
#   ./install_clients.sh --only pg,redis # instala solo esos grupos
#   ./install_clients.sh --yes           # sin confirmación (CI / desatendido)
#   ./install_clients.sh --list          # lista los grupos disponibles
#
# Variables de entorno:
#   KARTO_TERMINAL          paquete de terminal a instalar si no hay ninguna (por defecto: xterm)
#   KARTO_MONGOSH_VERSION   fija la versión de mongosh al caer al tarball oficial

set -euo pipefail

die()  { printf '\033[31m✗ %s\033[0m\n' "$*" >&2; exit 1; }
info() { printf '\033[36m→ %s\033[0m\n' "$*"; }
ok()   { printf '\033[32m✓ %s\033[0m\n' "$*"; }
warn() { printf '\033[33m! %s\033[0m\n' "$*" >&2; }

TERMINAL_PKG="${KARTO_TERMINAL:-xterm}"

# --- Catálogo de grupos ------------------------------------------------------
# Formato: modo|binarios|apt|dnf|pacman|zypper|brew
#   modo      all = hacen falta todos los binarios · any = basta con uno
#   binarios  lo que Karto busca en el PATH
#   paquetes  '-' = lo provee el propio SO · '@x' = instalador especial
KARTO_GROUPS=(ssh web terminal vnc pg mysql mongo redis)

spec() {
  case "$1" in
    ssh)      echo "all|ssh ssh-copy-id|openssh-client|openssh-clients|openssh|openssh-clients|-" ;;
    web)      echo "all|xdg-open|xdg-utils|xdg-utils|xdg-utils|xdg-utils|-" ;;
    terminal) echo "any|x-terminal-emulator gnome-terminal konsole kitty alacritty xterm|$TERMINAL_PKG|$TERMINAL_PKG|$TERMINAL_PKG|$TERMINAL_PKG|-" ;;
    vnc)      echo "any|remmina vinagre xtigervncviewer vncviewer|remmina remmina-plugin-vnc|remmina|remmina|remmina|-" ;;
    pg)       echo "all|psql|postgresql-client|postgresql|postgresql|postgresql|@brew_keg:libpq" ;;
    mysql)    echo "all|mysql|default-mysql-client|mysql|mariadb-clients|mariadb-client|@brew_keg:mysql-client" ;;
    mongo)    echo "all|mongosh|@mongosh|@mongosh|@mongosh|@mongosh|mongosh" ;;
    redis)    echo "all|redis-cli|redis-tools|redis|redis|redis|redis" ;;
  esac
}

desc() {
  case "$1" in
    ssh)      echo "SSH: conectar, ejecutar scripts y aprovisionar llaves" ;;
    web)      echo "Abrir URLs de administración web (y el esquema vnc://)" ;;
    terminal) echo "Emulador de terminal donde Karto abre la sesión SSH" ;;
    vnc)      echo "Visor VNC registrado para vnc://" ;;
    pg)       echo "PostgreSQL: cliente psql" ;;
    mysql)    echo "MySQL / MariaDB: cliente mysql" ;;
    mongo)    echo "MongoDB: cliente mongosh" ;;
    redis)    echo "Redis: cliente redis-cli" ;;
  esac
}

field() { echo "$1" | cut -d'|' -f"$2"; }

# --- Entorno -----------------------------------------------------------------
PM=""
PM_INDEX=0   # columna de `spec` con el paquete de este gestor

detect_pm() {
  if [[ "$(uname -s)" == "Darwin" ]]; then
    command -v brew >/dev/null 2>&1 \
      || die "Falta Homebrew. Instálalo desde https://brew.sh y vuelve a correr esto."
    PM=brew; PM_INDEX=7; return
  fi
  for c in apt-get dnf zypper pacman; do
    if command -v "$c" >/dev/null 2>&1; then
      PM="$c"
      case "$c" in
        apt-get) PM_INDEX=3 ;;
        dnf)     PM_INDEX=4 ;;
        pacman)  PM_INDEX=5 ;;
        zypper)  PM_INDEX=6 ;;
      esac
      return
    fi
  done
  die "No se reconoció el gestor de paquetes (se esperaba apt/dnf/pacman/zypper/brew)."
}

SUDO=""
setup_sudo() {
  [[ "$PM" == "brew" ]] && return          # brew nunca con sudo
  [[ "${EUID:-$(id -u)}" -eq 0 ]] && return
  command -v sudo >/dev/null 2>&1 || die "Hace falta root o sudo para instalar paquetes."
  SUDO=sudo
}

APT_UPDATED=0
pm_install() {
  case "$PM" in
    apt-get)
      if [[ $APT_UPDATED -eq 0 ]]; then $SUDO apt-get update -qq; APT_UPDATED=1; fi
      $SUDO apt-get install -y "$@" ;;
    dnf)    $SUDO dnf install -y "$@" ;;
    pacman) $SUDO pacman -S --needed --noconfirm "$@" ;;
    zypper) $SUDO zypper --non-interactive install "$@" ;;
    brew)   brew install "$@" ;;
  esac
}

# --- Instaladores especiales -------------------------------------------------

# Fórmula keg-only de brew (libpq, mysql-client): instala y enlaza los binarios,
# que si no quedan fuera del PATH.
brew_keg() {
  brew install "$1"
  brew link --force --overwrite "$1"
}

# mongosh no está en los repos de las distros: se toma del repo oficial de
# MongoDB y, si la distro no está en su matriz, del tarball oficial.
install_mongosh() {
  if [[ "$PM" == "apt-get" ]] && mongosh_apt_repo; then return; fi
  if [[ "$PM" == "dnf" ]] && mongosh_dnf_repo; then return; fi
  install_mongosh_tarball
}

# Escribe el repo apt de MongoDB. Devuelve 1 si la distro no está soportada
# (el caller cae al tarball).
mongosh_apt_repo() {
  local id codename url
  # shellcheck disable=SC1091
  . /etc/os-release
  id="${ID:-}"
  codename="${VERSION_CODENAME:-}"
  # Las derivadas (Mint, Pop!_OS…) declaran su base en UBUNTU_CODENAME.
  if [[ -n "${UBUNTU_CODENAME:-}" ]]; then id=ubuntu; codename="$UBUNTU_CODENAME"; fi
  case "$id:$codename" in
    ubuntu:jammy|ubuntu:noble)
      url="https://repo.mongodb.org/apt/ubuntu $codename/mongodb-org/8.0 multiverse" ;;
    debian:bullseye|debian:bookworm)
      url="https://repo.mongodb.org/apt/debian $codename/mongodb-org/8.0 main" ;;
    *)
      warn "El repo apt de MongoDB no cubre '$id $codename'; uso el tarball oficial."
      return 1 ;;
  esac
  info "Añadiendo el repositorio oficial de MongoDB ($id $codename)…"
  curl -fsSL https://www.mongodb.org/static/pgp/server-8.0.asc \
    | $SUDO gpg --dearmor -o /usr/share/keyrings/mongodb-server-8.0.gpg
  echo "deb [ signed-by=/usr/share/keyrings/mongodb-server-8.0.gpg ] $url" \
    | $SUDO tee /etc/apt/sources.list.d/mongodb-org-8.0.list >/dev/null
  APT_UPDATED=0   # fuerza el `apt-get update` que trae el índice recién añadido
  pm_install mongodb-mongosh
}

mongosh_dnf_repo() {
  info "Añadiendo el repositorio oficial de MongoDB (dnf)…"
  $SUDO tee /etc/yum.repos.d/mongodb-org-8.0.repo >/dev/null <<'REPO'
[mongodb-org-8.0]
name=MongoDB Repository
baseurl=https://repo.mongodb.org/yum/redhat/$releasever/mongodb-org/8.0/x86_64/
gpgcheck=1
enabled=1
gpgkey=https://pgp.mongodb.com/server-8.0.asc
REPO
  pm_install mongodb-mongosh
}

# Respaldo para distros fuera de la matriz de los repos de MongoDB. Solo Linux:
# en macOS mongosh viene de brew y no se llega aquí (además allí el artefacto es
# un .zip, no un .tgz).
install_mongosh_tarball() {
  local ver arch url tmp
  ver="${KARTO_MONGOSH_VERSION:-}"
  if [[ -z "$ver" ]]; then
    ver="$(curl -fsSL https://api.github.com/repos/mongodb-js/mongosh/releases/latest \
      | sed -n 's/.*"tag_name": *"v\{0,1\}\([^"]*\)".*/\1/p' | head -1)"
  fi
  [[ -n "$ver" ]] || die "No pude resolver la versión de mongosh (fija KARTO_MONGOSH_VERSION)."
  case "$(uname -m)" in
    x86_64|amd64)  arch=x64 ;;
    aarch64|arm64) arch=arm64 ;;
    *) die "Arquitectura sin build oficial de mongosh: $(uname -m)" ;;
  esac
  url="https://downloads.mongodb.com/compass/mongosh-$ver-linux-$arch.tgz"
  info "Descargando mongosh $ver (linux-$arch)…"
  tmp="$(mktemp -d)"
  curl -fL "$url" -o "$tmp/mongosh.tgz"
  tar -xzf "$tmp/mongosh.tgz" -C "$tmp"
  # El tarball trae bin/mongosh y su librería de cifrado al lado: van juntos.
  $SUDO install -m 0755 "$tmp"/mongosh-*/bin/* /usr/local/bin/
  rm -rf "$tmp"
  ok "mongosh instalado en /usr/local/bin"
}

# --- Detección ---------------------------------------------------------------
has() { command -v "$1" >/dev/null 2>&1; }

# ¿El grupo ya está satisfecho? Imprime en stdout los binarios hallados y
# devuelve 0/1 según el modo (all = todos, any = al menos uno).
group_satisfied() {
  local s mode bins found=""
  s="$(spec "$1")"; mode="$(field "$s" 1)"; bins="$(field "$s" 2)"
  for b in $bins; do
    if has "$b"; then found="$found $b"; fi
  done
  echo "${found# }"
  if [[ "$mode" == "any" ]]; then
    [[ -n "$found" ]]
  else
    for b in $bins; do has "$b" || return 1; done
  fi
}

# ¿Este grupo lo cubre el propio SO con este gestor? (macOS: ssh, open, VNC…)
provided_by_os() { [[ "$(field "$(spec "$1")" "$PM_INDEX")" == "-" ]]; }

install_group() {
  local pkgs
  pkgs="$(field "$(spec "$1")" "$PM_INDEX")"
  case "$pkgs" in
    -)           ok "$1: lo provee el sistema, nada que instalar" ;;
    @mongosh)    install_mongosh ;;
    @brew_keg:*) brew_keg "${pkgs#@brew_keg:}" ;;
    *)           pm_install $pkgs ;;   # sin comillas: el campo puede traer varios paquetes
  esac
}

# Línea de estado de un grupo, compartida por el plan y la verificación final.
report_group() {
  local found
  if found="$(group_satisfied "$1")"; then
    printf '  \033[32m✓\033[0m %-9s %s\n' "$1" "$found"
  elif provided_by_os "$1"; then
    printf '  \033[32m✓\033[0m %-9s lo provee el sistema\n' "$1"
  else
    return 1
  fi
}

# --- CLI ---------------------------------------------------------------------
MODE=install
ASSUME_YES=0
SELECTED=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --check|-n) MODE=check ;;
    --yes|-y)   ASSUME_YES=1 ;;
    --only)     shift; IFS=',' read -r -a SELECTED <<< "${1:-}" ;;
    --list)
      for g in "${KARTO_GROUPS[@]}"; do printf '  %-9s %s\n' "$g" "$(desc "$g")"; done
      exit 0 ;;
    --help|-h)  sed -n '2,23p' "$0" | sed 's/^#\{1,\} \{0,1\}//'; exit 0 ;;
    *)          die "Opción desconocida: $1 (usa --help)" ;;
  esac
  shift
done

if [[ ${#SELECTED[@]} -eq 0 ]]; then SELECTED=("${KARTO_GROUPS[@]}"); fi
for g in "${SELECTED[@]}"; do
  [[ -n "$(spec "$g")" ]] || die "Grupo desconocido: $g (usa --list)"
done

# --- Plan --------------------------------------------------------------------
detect_pm
info "Gestor de paquetes: $PM"
echo

MISSING=()
for g in "${SELECTED[@]}"; do
  if ! report_group "$g"; then
    printf '  \033[33m·\033[0m %-9s falta — %s\n' "$g" "$(desc "$g")"
    MISSING+=("$g")
  fi
done
echo

if [[ ${#MISSING[@]} -eq 0 ]]; then
  ok "Todo listo: no falta ningún cliente."
  exit 0
fi

if [[ "$MODE" == "check" ]]; then
  info "Faltan ${#MISSING[@]} grupo(s): ${MISSING[*]}"
  exit 1
fi

info "Se instalarán: ${MISSING[*]}"
for g in "${MISSING[@]}"; do
  if [[ "$g" == "mongo" && "$PM" != "brew" ]]; then
    warn "mongo añade el repositorio oficial de MongoDB (o baja su tarball oficial)."
  fi
done

if [[ $ASSUME_YES -eq 0 ]]; then
  read -r -p "  ¿Continuar? [S/n]: " answer
  case "${answer:-s}" in
    s|S|y|Y|"") ;;
    *) die "Cancelado." ;;
  esac
fi

setup_sudo

# --- Instalación -------------------------------------------------------------
FAILED=()
for g in "${MISSING[@]}"; do
  echo
  info "Instalando $g — $(desc "$g")"
  if install_group "$g"; then ok "$g listo"; else warn "$g falló"; FAILED+=("$g"); fi
done

# --- Verificación ------------------------------------------------------------
echo
info "Verificación final (lo mismo que Karto registra en su log al arrancar):"
for g in "${SELECTED[@]}"; do
  report_group "$g" || printf '  \033[31m✗\033[0m %-9s sigue faltando\n' "$g"
done

if [[ ${#FAILED[@]} -gt 0 ]]; then
  echo
  die "Falló la instalación de: ${FAILED[*]}"
fi
echo
ok "Clientes de Karto instalados."
