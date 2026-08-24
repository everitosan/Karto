// Casos de uso para elegir rutas de archivo del vault mediante los diálogos nativos.
import { open, save } from "@tauri-apps/plugin-dialog";
import { m } from "$paraglide/messages.js";

const FILTERS = [{ name: m.dialog_vault_filter(), extensions: ["karto"] }];

export async function pickNewVaultPath(defaultPath?: string): Promise<string | null> {
  const path = await save({ title: m.dialog_create_vault(), defaultPath, filters: FILTERS });
  return path ?? null;
}

export async function pickBackupPath(): Promise<string | null> {
  const path = await save({ title: m.dialog_save_backup(), filters: FILTERS });
  return path ?? null;
}

export async function pickSubsetExportPath(suggestedName?: string): Promise<string | null> {
  const path = await save({
    title: m.dialog_export_subset(),
    defaultPath: suggestedName ? `${suggestedName}.karto` : undefined,
    filters: FILTERS,
  });
  return path ?? null;
}

export async function pickExistingVaultPath(): Promise<string | null> {
  const path = await open({
    title: m.dialog_open_vault(),
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
    title: m.flow_export_title(),
    defaultPath: suggestedName ? `${suggestedName}.${format}` : undefined,
    filters: [{ name: format.toUpperCase(), extensions: [format] }],
  });
  return path ?? null;
}

export async function pickSshKeyPath(): Promise<string | null> {
  const path = await open({
    title: m.dialog_pick_ssh_key(),
    multiple: false,
    directory: false,
  });
  return typeof path === "string" ? path : null;
}
