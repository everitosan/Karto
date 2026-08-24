// Capa de traducción del catálogo de nodos.
//
// `@karto/catalog` es dato agnóstico de idioma (ids estables de categoría y tipo,
// iconos, claves de propiedad). Las etiquetas *visibles* de categorías y tipos se
// resuelven aquí vía Paraglide; la label que trae el catálogo queda solo como
// respaldo si aún no existe traducción para ese id. Así el idioma vive en la app,
// no en el paquete compartido.
import {
  NODE_CATALOG,
  NODE_CATEGORIES,
  NODE_KINDS,
  CATEGORY_LABELS,
  type NodeCategory,
  type NodeKind,
} from "@karto/catalog";
import { m } from "$paraglide/messages.js";

// `m` está tipado con claves fijas; para indexar por id lo tratamos como mapa de
// funciones y caemos al respaldo del catálogo cuando la clave no existe.
const messages = m as unknown as Record<string, (() => string) | undefined>;

/** Etiqueta traducida de un tipo de nodo (respaldo: la label del catálogo). */
export function nodeKindLabel(kind: NodeKind): string {
  return messages[`nodeKind_${kind}`]?.() ?? NODE_CATALOG[kind]?.label ?? kind;
}

/**
 * Texto buscable de un tipo de nodo: su etiqueta más los valores y las etiquetas
 * de las opciones de sus selects. Permite hallar un tipo por una de sus marcas
 * internas (p. ej. buscar "firebase" y que aparezca el nodo de base de datos).
 */
export function nodeSearchText(kind: NodeKind): string {
  const parts = [nodeKindLabel(kind)];
  for (const p of NODE_CATALOG[kind]?.properties ?? []) {
    for (const o of p.options ?? []) {
      parts.push(o.value, catalogText(o.label));
    }
  }
  return parts.join(" ").toLowerCase();
}

/** Etiqueta traducida de una categoría (respaldo: la label del catálogo). */
export function categoryLabel(category: NodeCategory): string {
  return messages[`category_${category}`]?.() ?? CATEGORY_LABELS[category] ?? category;
}

/**
 * Resuelve un token de texto del catálogo (label/placeholder de propiedad, o
 * label de opción) vía Paraglide. Los slugs `cp_`/`cph_`/`co_` traen traducción;
 * los nombres propios (PostgreSQL, nginx…) no son claves y caen al literal.
 */
export function catalogText(token: string | undefined): string {
  if (!token) return "";
  return messages[token]?.() ?? token;
}

/** Como `nodesByCategory()` del catálogo, pero con las etiquetas ya traducidas. */
export function nodeGroups(): {
  category: NodeCategory;
  label: string;
  kinds: NodeKind[];
}[] {
  return NODE_CATEGORIES.map((category) => ({
    category,
    label: categoryLabel(category),
    kinds: NODE_KINDS.filter((k) => NODE_CATALOG[k].category === category),
  })).filter((g) => g.kinds.length > 0);
}
