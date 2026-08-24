// Casos de uso del log de diagnóstico (soporte). Es config a nivel de **máquina**
// (no del vault): el nivel se guarda en el app_store y el archivo vive en el dir
// de datos de la app. `normalizeLevel` es puro y testeable sin backend.
import { bridge, type Bridge } from "./tauri";

export type LogLevel = "info" | "warning" | "error";

export const LOG_LEVELS: { value: LogLevel; label: string; hint: string }[] = [
  { value: "info", label: "Info", hint: "Información de acciones comunes." },
  { value: "warning", label: "Warning", hint: "Avisos y errores (por defecto)." },
  { value: "error", label: "Error", hint: "Solo errores." },
];

/** Sanea el nivel crudo del backend a un `LogLevel` válido (fallback Warning). */
export function normalizeLevel(raw: string | undefined | null): LogLevel {
  return raw === "info" || raw === "error" ? raw : "warning";
}

export function makeDiagnosticsUseCases(io: Bridge = bridge) {
  return {
    async getLevel(): Promise<LogLevel> {
      return normalizeLevel(await io.invoke<string>("log_level_get"));
    },
    async setLevel(level: LogLevel): Promise<void> {
      await io.invoke<void>("log_level_set", { level });
    },
    /** Ruta absoluta del archivo de log (para mostrar/copiar). */
    async logPath(): Promise<string> {
      return await io.invoke<string>("log_path_get");
    },
    /** Abre la carpeta del log en el gestor de archivos del sistema. */
    async openLogDir(): Promise<void> {
      await io.invoke<void>("open_log_dir");
    },
  };
}

export const diagnosticsUseCases = makeDiagnosticsUseCases();
