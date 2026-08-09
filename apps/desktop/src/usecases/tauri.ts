// Envoltorio delgado sobre la API de Tauri para poder mockearla en pruebas.
import { invoke } from "@tauri-apps/api/core";

export const bridge = { invoke };

export type Bridge = typeof bridge;
