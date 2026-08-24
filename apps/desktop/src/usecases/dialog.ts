// Casos de uso para elegir rutas de archivo del vault mediante los diálogos nativos.
import { open, save } from "@tauri-apps/plugin-dialog";

const FILTERS = [{ name: "Vault de Karto", extensions: ["karto"] }];

export async function pickNewVaultPath(defaultPath?: string): Promise<string | null> {
  const path = await save({ title: "Crear vault", defaultPath, filters: FILTERS });
  return path ?? null;
}

export async function pickBackupPath(): Promise<string | null> {
  const path = await save({ title: "Guardar copia de respaldo cifrada", filters: FILTERS });
  return path ?? null;
}

export async function pickSubsetExportPath(suggestedName?: string): Promise<string | null> {
  const path = await save({
    title: "Exportar selección a un vault nuevo",
    defaultPath: suggestedName ? `${suggestedName}.karto` : undefined,
    filters: FILTERS,
  });
  return path ?? null;
}

export async function pickExistingVaultPath(): Promise<string | null> {
  const path = await open({
    title: "Abrir vault",
    multiple: false,
    directory: false,
    filters: FILTERS,
  });
  return typeof path === "string" ? path : null;
}

export async function pickExportImagePath(
  format: "png" | "svg",
  suggestedName?: string,
): Promise<string | null> {
  const path = await save({
    title: "Exportar diagrama",
    defaultPath: suggestedName ? `${suggestedName}.${format}` : undefined,
    filters: [{ name: format.toUpperCase(), extensions: [format] }],
  });
  return path ?? null;
}

export async function pickSshKeyPath(): Promise<string | null> {
  const path = await open({
    title: "Elegir llave SSH",
    multiple: false,
    directory: false,
  });
  return typeof path === "string" ? path : null;
}
