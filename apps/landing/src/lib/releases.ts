// Lógica pura (sin DOM ni Astro) para leer los releases de Karto desde la API
// de GitHub, clasificar cada asset por sistema operativo y elegir el más nuevo.
// Se usa en dos sitios: en build (Astro hornea el resultado en el HTML) y en el
// cliente (refresco en segundo plano). Al ser puro, es fácil de testear.

export const RELEASES_API =
  "https://api.github.com/repos/everitosan/Karto/releases";

export type OS = "linux" | "mac" | "windows";

export const OSES: OS[] = ["linux", "mac", "windows"];

export interface DownloadAsset {
  name: string;
  url: string;
  os: OS;
  /** Formato legible del instalador: "AppImage", "deb", "dmg", "msi"… */
  format: string;
  /** Arquitectura si se puede inferir del nombre; si no, null. */
  arch: "x64" | "arm64" | null;
  /** Tamaño en bytes (0 si la API no lo dio). */
  size: number;
}

export interface ReleaseModel {
  /** Etiqueta del tag, p. ej. "v0.1.0-rc.1". */
  tag: string;
  /** Versión sin la "v" inicial, p. ej. "0.1.0-rc.1". */
  version: string;
  /** Nombre del release (cae al tag si viene vacío). */
  name: string;
  prerelease: boolean;
  /** ISO date de publicación. */
  publishedAt: string;
  /** Página del release en GitHub. */
  htmlUrl: string;
  /** Assets descargables, ya filtrados y clasificados. */
  assets: DownloadAsset[];
  /** Los mismos assets agrupados por SO, para pintar por columnas. */
  byOS: Record<OS, DownloadAsset[]>;
}

// ── Clasificación de assets ──────────────────────────────────────────────
// Mapea un nombre de archivo (salida típica de Tauri) a SO + formato. Devuelve
// null para artefactos que no son instalables por el usuario (firmas .sig, el
// manifiesto latest.json del updater, etc.).

interface AssetKind {
  os: OS;
  format: string;
}

function classifyKind(lower: string): AssetKind | null {
  // Artefactos internos del updater / firmas: no se ofrecen al usuario.
  if (lower.endsWith(".sig") || lower.endsWith(".json")) return null;

  if (lower.endsWith(".appimage")) return { os: "linux", format: "AppImage" };
  if (lower.endsWith(".deb")) return { os: "linux", format: "deb" };
  if (lower.endsWith(".rpm")) return { os: "linux", format: "rpm" };
  if (lower.endsWith(".flatpak")) return { os: "linux", format: "Flatpak" };

  if (lower.endsWith(".dmg")) return { os: "mac", format: "dmg" };
  // El .app.tar.gz es el bundle del updater de macOS, no una descarga manual.
  if (lower.endsWith(".app.tar.gz") || lower.endsWith(".app.zip")) return null;

  if (lower.endsWith(".msi")) return { os: "windows", format: "msi" };
  if (lower.endsWith(".exe")) return { os: "windows", format: "exe" };

  return null;
}

function detectArch(lower: string): DownloadAsset["arch"] {
  if (/aarch64|arm64/.test(lower)) return "arm64";
  if (/x86_64|amd64|x64/.test(lower)) return "x64";
  return null;
}

export function classifyAsset(
  name: string,
  url: string,
  size = 0,
): DownloadAsset | null {
  const lower = name.toLowerCase();
  const kind = classifyKind(lower);
  if (!kind) return null;
  return {
    name,
    url,
    os: kind.os,
    format: kind.format,
    arch: detectArch(lower),
    size,
  };
}

// ── Normalización del release ────────────────────────────────────────────

/** Forma mínima del JSON de GitHub que consumimos (tolerante a campos extra). */
interface RawAsset {
  name?: string;
  browser_download_url?: string;
  size?: number;
}
interface RawRelease {
  tag_name?: string;
  name?: string | null;
  prerelease?: boolean;
  draft?: boolean;
  published_at?: string;
  html_url?: string;
  assets?: RawAsset[];
}

function emptyByOS(): Record<OS, DownloadAsset[]> {
  return { linux: [], mac: [], windows: [] };
}

export function normalizeRelease(raw: RawRelease): ReleaseModel | null {
  const tag = raw.tag_name;
  if (!tag) return null;

  const assets: DownloadAsset[] = [];
  for (const a of raw.assets ?? []) {
    if (!a?.name || !a.browser_download_url) continue;
    const asset = classifyAsset(a.name, a.browser_download_url, a.size ?? 0);
    if (asset) assets.push(asset);
  }

  const byOS = emptyByOS();
  for (const asset of assets) byOS[asset.os].push(asset);

  return {
    tag,
    version: tag.replace(/^v/, ""),
    name: raw.name || tag,
    prerelease: Boolean(raw.prerelease),
    publishedAt: raw.published_at ?? "",
    htmlUrl: raw.html_url ?? `https://github.com/everitosan/Karto/releases`,
    assets,
    byOS,
  };
}

/**
 * Elige el release a mostrar de la lista cruda de GitHub. Descarta drafts y,
 * por defecto, incluye prereleases (queremos mostrar la rc actual). El más
 * nuevo se decide por fecha de publicación.
 */
export function selectRelease(
  rawList: unknown,
  opts: { includePrerelease?: boolean } = {},
): ReleaseModel | null {
  if (!Array.isArray(rawList)) return null;
  const includePrerelease = opts.includePrerelease ?? true;

  const candidates = (rawList as RawRelease[])
    .filter((r) => r && !r.draft)
    .filter((r) => includePrerelease || !r.prerelease);

  // Más reciente primero por published_at (fallback: orden de la API).
  candidates.sort((a, b) => {
    const ta = Date.parse(a.published_at ?? "") || 0;
    const tb = Date.parse(b.published_at ?? "") || 0;
    return tb - ta;
  });

  for (const raw of candidates) {
    const model = normalizeRelease(raw);
    // Nos quedamos con el primero que traiga al menos un asset descargable.
    if (model && model.assets.length > 0) return model;
  }
  // Si ninguno tiene assets, devolvemos el más nuevo igualmente (por metadatos).
  return candidates.length ? normalizeRelease(candidates[0]) : null;
}

/** Trae y normaliza el release desde GitHub. Devuelve null ante cualquier fallo. */
export async function fetchReleaseModel(
  fetchImpl: typeof fetch = fetch,
  opts: { includePrerelease?: boolean } = {},
): Promise<ReleaseModel | null> {
  try {
    const res = await fetchImpl(RELEASES_API, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!res.ok) return null;
    const data = await res.json();
    return selectRelease(data, opts);
  } catch {
    return null;
  }
}

// ── Detección de SO del visitante ────────────────────────────────────────

interface NavLike {
  userAgent?: string;
  platform?: string;
  // navigator.userAgentData?.platform (Chromium)
  userAgentData?: { platform?: string };
}

/** Best-effort del SO del visitante a partir de navigator. null si no se sabe. */
export function detectOS(nav: NavLike | undefined | null): OS | null {
  if (!nav) return null;
  const hints = [
    nav.userAgentData?.platform,
    nav.platform,
    nav.userAgent,
  ]
    .filter(Boolean)
    .join(" ")
    .toLowerCase();

  if (!hints) return null;
  // Android es Linux-kernel pero no es un target de escritorio: sin match.
  if (/android|iphone|ipad|ipod/.test(hints)) return null;
  if (/win/.test(hints)) return "windows";
  if (/mac|darwin|os x/.test(hints)) return "mac";
  if (/linux|x11|ubuntu|fedora|debian/.test(hints)) return "linux";
  return null;
}

/** ¿Hay algún asset disponible para ese SO? */
export function hasDownloads(model: ReleaseModel | null, os: OS): boolean {
  return Boolean(model && model.byOS[os].length > 0);
}

/** Compara dos modelos; true si `next` es un release distinto/más nuevo que `current`. */
export function isNewer(
  current: ReleaseModel | null,
  next: ReleaseModel | null,
): boolean {
  if (!next) return false;
  if (!current) return true;
  if (next.tag === current.tag) return false;
  const tc = Date.parse(current.publishedAt) || 0;
  const tn = Date.parse(next.publishedAt) || 0;
  // Si no hay fechas fiables, considera "más nuevo" cualquier tag distinto.
  return tn === 0 || tc === 0 ? true : tn > tc;
}
