// Sección activa del workspace: qué muestra el área principal bajo la titlebar.
// Es estado local de la UI (no viaja en el vault); se persiste en localStorage
// para recuperar la última sección al reabrir. La barra vertical (ActivityRail)
// conmuta entre estas secciones.
export type Section = "diagrams" | "scripts" | "config";

const STORAGE_KEY = "karto.activeSection";
const DEFAULT: Section = "diagrams";

function isSection(v: string | null): v is Section {
  return v === "diagrams" || v === "scripts" || v === "config";
}

function read(): Section {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return isSection(v) ? v : DEFAULT;
  } catch {
    return DEFAULT;
  }
}

export const sectionState = $state<{ active: Section }>({ active: read() });

export function setSection(next: Section): void {
  sectionState.active = next;
  try {
    localStorage.setItem(STORAGE_KEY, next);
  } catch {
    // localStorage no disponible: la sección vive solo en memoria esta sesión.
  }
}
