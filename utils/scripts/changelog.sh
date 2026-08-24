#!/usr/bin/env bash
# changelog.sh — lectura/rotación del CHANGELOG.md (formato "Keep a Changelog").
#
# Uso como librería (source):
#   extract_unreleased            -> imprime el cuerpo de "## [Unreleased]"
#   extract_section <X.Y.Z>       -> imprime el cuerpo de "## [X.Y.Z] - ..."
#   rotate_changelog <ver> <fecha>-> cierra Unreleased en una sección versionada
#
# Uso como CLI (lo usa el workflow de CI):
#   changelog.sh unreleased
#   changelog.sh section <X.Y.Z>

set -euo pipefail

CHANGELOG_ROOT="$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
CHANGELOG="$CHANGELOG_ROOT/CHANGELOG.md"

# Cuerpo entre "## [Unreleased]" y el siguiente encabezado "## [".
extract_unreleased() {
  awk '
    /^## \[Unreleased\]/ { f=1; next }
    /^## \[/            { f=0 }
    f                   { print }
  ' "$CHANGELOG" | sed -E '/^[[:space:]]*$/d'
}

# Cuerpo de una sección versionada concreta.
extract_section() {
  local v="$1"
  awk -v v="$v" '
    $0 ~ ("^## \\[" v "\\]") { f=1; next }
    /^## \[/                 { if (f) exit }
    f                        { print }
  ' "$CHANGELOG" | sed -E '/^[[:space:]]*$/d'
}

# Convierte "## [Unreleased]" en "## [ver] - fecha" y deja una Unreleased vacía
# encima. El cuerpo acumulado pasa a la nueva sección versionada.
rotate_changelog() {
  local v="$1" d="$2" tmp
  tmp="$(mktemp)"
  awk -v v="$v" -v d="$d" '
    /^## \[Unreleased\]/ && !done {
      print "## [Unreleased]";
      print "";
      print "## [" v "] - " d;
      done=1; next
    }
    { print }
  ' "$CHANGELOG" > "$tmp"
  mv "$tmp" "$CHANGELOG"
}

# Dispatch sólo cuando se ejecuta directamente (no al hacer source).
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
  case "${1:-}" in
    unreleased) extract_unreleased ;;
    section)    extract_section "${2:?falta la versión}" ;;
    *) echo "uso: changelog.sh {unreleased|section <X.Y.Z>}" >&2; exit 2 ;;
  esac
fi
