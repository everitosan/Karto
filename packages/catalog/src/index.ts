// @karto/catalog — vocabulario compartido del catálogo de tipos de nodo
// (categorías, tipos, propiedades sugeridas e iconos). Framework-agnóstico: no
// contiene componentes Svelte, solo datos y descriptores de icono; los
// componentes que los pintan (Icon/TechIcon) viven en @karto/ui.
export type { NodeKind, NodeCategory } from "./types";
export {
  NODE_CATALOG,
  NODE_CATEGORIES,
  NODE_KINDS,
  NODE_KIND_LABELS,
  CATEGORY_LABELS,
  nodeTypeIcon,
  nodesByCategory,
  resolveNodeIcon,
} from "./catalog";
export type {
  NodeSpec,
  PropertySpec,
  PropertyOption,
  ResolvedNodeIcon,
} from "./catalog";
