#!/usr/bin/env bash
# deploy_rc.sh — publica un Release Candidate.
#
#   - Si la versión actual es estable (X.Y.Z): pregunta el nivel de bump
#     (patch/minor/major) y arranca en -rc.1  ->  X.Y'.Z'-rc.1
#   - Si la versión actual ya es un RC:        incrementa el número -> -rc.(N+1)
#
# NO consume el bloque [Unreleased] del CHANGELOG: los RC siguen acumulando notas.
# Al final: commit, tag anotado y push (dispara el workflow de release en CI).

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

# --- Cálculo de la versión sugerida -----------------------------------------
if is_rc "$cur"; then
  base="$(base_of "$cur")"
  n="$(rc_num_of "$cur")"
  suggested="$base-rc.$((n + 1))"
  info "Versión actual: $cur (RC). Siguiente RC sugerido: $suggested"
else
  info "Versión actual: $cur (estable). Se inicia un ciclo de RC nuevo."
  echo "  ¿Nivel de salto?  [1] minor (por defecto)  [2] patch  [3] major"
  read -r -p "  Elige 1/2/3 [1]: " lvl
  case "${lvl:-1}" in
    1|"") level=minor ;;
    2)    level=patch ;;
    3)    level=major ;;
    *)    die "Opción inválida: $lvl" ;;
  esac
  base="$(bump "$cur" "$level")"
  suggested="$base-rc.1"
  info "Salto '$level' → base $base → RC sugerido: $suggested"
fi

read -r -p "Versión del RC a publicar [$suggested]: " input
target="${input:-$suggested}"
validate_semver "$target"
is_rc "$target" || die "'$target' no es un RC (debe terminar en -rc.N)."

tag="v$target"
git rev-parse "$tag" >/dev/null 2>&1 && die "El tag $tag ya existe."

# --- Notas (sin consumir Unreleased) ----------------------------------------
notes_file="$(mktemp)"
trap 'rm -f "$notes_file"' EXIT
{
  echo "Karto $tag (release candidate)"
  echo
  extract_unreleased
} > "$notes_file"

echo
info "Se hará:"
echo "  • versión $cur → $target en los 4 archivos"
echo "  • commit 'chore(release): $tag'"
echo "  • tag anotado $tag + push a origin/$branch (dispara CI)"
echo
read -r -p "¿Continuar? [y/N]: " go
[[ "$go" =~ ^[yY]$ ]] || die "Cancelado."

# --- Ejecución ---------------------------------------------------------------
write_version "$target"
git add -A
git commit -m "chore(release): $tag"
git tag -a "$tag" -F "$notes_file"
git push origin "$branch"
git push origin "$tag"
ok "RC $tag publicado. CI compilará y creará el prerelease en GitHub."
