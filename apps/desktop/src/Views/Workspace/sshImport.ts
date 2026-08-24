// Lógica pura de la importación SSH: destino de un host y detección de
// duplicados contra un diagrama existente. Sin dependencias de Svelte ni Tauri
// para poder probarla de forma aislada (vitest).
import type { ImportedHost, InfraNode } from "$domain/infra";

/** Host destino: `HostName` si existe, si no el propio alias. */
export function hostTarget(host: ImportedHost): string {
  return host.hostname ?? host.alias;
}

/**
 * Un host ya existe en el diagrama si algún nodo comparte su etiqueta (alias)
 * o su host destino (IP/hostname), comparando de forma laxa (minúsculas).
 */
export function isDuplicate(host: ImportedHost, nodes: InfraNode[]): boolean {
  const alias = host.alias.toLowerCase();
  const target = hostTarget(host).toLowerCase();
  return nodes.some((n) => {
    const label = n.label.toLowerCase();
    const hostname = (n.properties.hostname ?? "").toLowerCase();
    // La dirección ya no es una propiedad fija: vive por contexto en endpoints.
    const addresses = Object.values(n.endpoints ?? {}).map((a) => a.toLowerCase());
    return (
      label === alias ||
      label === target ||
      (hostname !== "" && (hostname === target || hostname === alias)) ||
      addresses.some((ip) => ip === target || ip === alias)
    );
  });
}
