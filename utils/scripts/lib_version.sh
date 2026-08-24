#!/usr/bin/env bash
# lib_version.sh — helpers de versión para Karto.
#
# Fuente de verdad: la clave `version` de apps/desktop/src-tauri/Cargo.toml.
# Al escribir una versión nueva se sincronizan los 4 sitios:
#   - Cargo.toml            (fuente de verdad)
#   - Cargo.lock            (entrada del paquete `karto`)
#   - tauri.conf.json       (top-level "version")
#   - apps/desktop/package.json (top-level "version")
#
# Este archivo está pensado para hacer `source`, no para ejecutarse directo.

set -euo pipefail

VERSION_LIB_ROOT="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"

CARGO_TOML="$VERSION_LIB_ROOT/apps/desktop/src-tauri/Cargo.toml"
CARGO_LOCK="$VERSION_LIB_ROOT/apps/desktop/src-tauri/Cargo.lock"
TAURI_CONF="$VERSION_LIB_ROOT/apps/desktop/src-tauri/tauri.conf.json"
DESKTOP_PKG="$VERSION_LIB_ROOT/apps/desktop/package.json"

die() { printf '\033[31m✗ %s\033[0m\n' "$*" >&2; exit 1; }
info() { printf '\033[36m→ %s\033[0m\n' "$*"; }
ok() { printf '\033[32m✓ %s\033[0m\n' "$*"; }

# Lee la versión actual desde Cargo.toml (primera línea `version = "..."`).
current_version() {
  grep -m1 -E '^version = ' "$CARGO_TOML" | sed -E 's/^version = "(.*)"/\1/'
}

validate_semver() {
  [[ "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-rc\.[0-9]+)?$ ]] \
    || die "Versión inválida: '$1' (esperado X.Y.Z o X.Y.Z-rc.N)"
}

is_rc()      { [[ "$1" == *-rc.* ]]; }
base_of()    { echo "${1%%-rc.*}"; }
rc_num_of()  { echo "${1##*-rc.}"; }

# bump <base X.Y.Z> <major|minor|patch>
bump() {
  local M m p
  IFS=. read -r M m p <<<"$1"
  case "$2" in
    major) echo "$((M+1)).0.0" ;;
    minor) echo "$M.$((m+1)).0" ;;
    patch) echo "$M.$m.$((p+1))" ;;
    *) die "Nivel de bump desconocido: $2" ;;
  esac
}

# Escribe la versión nueva en los 4 archivos. Ediciones quirúrgicas (una línea
# cada una) para no reformatear los JSON ni tocar otras versiones del lockfile.
write_version() {
  local v="$1"
  validate_semver "$v"
  sed -i -E '0,/^version = ".*"/s//version = "'"$v"'"/' "$CARGO_TOML"
  sed -i -E '/^name = "karto"$/{n;s/^version = ".*"/version = "'"$v"'"/;}' "$CARGO_LOCK"
  sed -i -E '0,/"version": ".*"/s//"version": "'"$v"'"/' "$TAURI_CONF"
  sed -i -E '0,/"version": ".*"/s//"version": "'"$v"'"/' "$DESKTOP_PKG"
  ok "Versión escrita: $v (Cargo.toml, Cargo.lock, tauri.conf.json, package.json)"
}

# Verifica que los 4 archivos declaren la misma versión (integridad).
assert_versions_synced() {
  local c t p
  c="$(current_version)"
  t="$(grep -m1 -E '"version":' "$TAURI_CONF" | sed -E 's/.*"version": "(.*)".*/\1/')"
  p="$(grep -m1 -E '"version":' "$DESKTOP_PKG" | sed -E 's/.*"version": "(.*)".*/\1/')"
  [[ "$c" == "$t" && "$c" == "$p" ]] \
    || die "Versiones desincronizadas: Cargo=$c tauri=$t pkg=$p. Corrige a mano antes de continuar."
}
