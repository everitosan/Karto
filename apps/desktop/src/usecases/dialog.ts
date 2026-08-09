// Casos de uso para elegir rutas de archivo del vault mediante los diálogos nativos.
import { open, save } from "@tauri-apps/plugin-dialog";

const FILTERS = [{ name: "Vault de Karto", extensions: ["karto"] }];

export async function pickNewVaultPath(): Promise<string | null> {
  const path = await save({ title: "Crear vault", filters: FILTERS });
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
