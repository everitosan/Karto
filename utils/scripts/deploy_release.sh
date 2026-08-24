#!/usr/bin/env bash
# deploy_release.sh — promueve el RC actual a release estable.
#
#   X.Y.Z-rc.N  ->  X.Y.Z   (se quita el sufijo -rc)
#
# Requiere que la versión actual sea un RC. Rota el CHANGELOG: cierra
# [Unreleased] en una sección "## [X.Y.Z] - fecha". Al final: commit, tag
# anotado y push (dispara el workflow de release, marcado como estable).

set -euo pipefail
DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lib_version.sh
source "$DIR/lib_version.sh"
# shellcheck source=changelog.sh
source "$DIR/changelog.sh"

# --- Guardas -----------------------------------------------------------------
[[ -z "$(git status --porcelain)" ]] || die "Árbol git sucio. Haz commit o stash antes de publicar."
branch="$(git rev-parse --abbrev-ref HEAD)"
[[ "$branch" == "main" ]] || die "Debes estar en 'main' (estás en '$branch')."
assert_versions_synced

cur="$(current_version)"
is_rc "$cur" || die "La versión actual ($cur) no es un RC. Ejecuta 'make deploy-rc-app' primero."

target="$(base_of "$cur")"
validate_semver "$target"
tag="v$target"
git rev-parse "$tag" >/dev/null 2>&1 && die "El tag $tag ya existe."

today="$(date +%F)"

echo
info "Se hará:"
echo "  • promoción $cur → $target (release estable) en los 4 archivos"
echo "  • CHANGELOG: cerrar [Unreleased] → [$target] - $today"
echo "  • commit 'chore(release): $tag'"
echo "  • tag anotado $tag + push a origin/$branch (dispara CI)"
echo
read -r -p "¿Continuar? [y/N]: " go
[[ "$go" =~ ^[yY]$ ]] || die "Cancelado."

# --- Ejecución ---------------------------------------------------------------
write_version "$target"
rotate_changelog "$target" "$today"

notes_file="$(mktemp)"
trap 'rm -f "$notes_file"' EXIT
{
  echo "Karto $tag"
  echo
  extract_section "$target"
} > "$notes_file"

git add -A
git commit -m "chore(release): $tag"
git tag -a "$tag" -F "$notes_file"
git push origin "$branch"
git push origin "$tag"
ok "Release $tag publicado. CI compilará y creará el release estable en GitHub."
