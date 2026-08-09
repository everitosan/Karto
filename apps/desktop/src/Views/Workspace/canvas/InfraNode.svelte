<script lang="ts">
  // Nodo custom del canvas: icono por tipo, etiqueta y un resumen (IP visible).
  // Componente de nodo de Svelte Flow; recibe `data` y `selected` como props.
  import { Handle, Position, type NodeProps } from "@xyflow/svelte";
  import { Icon, TechIcon, resolveNodeIcon } from "@karto/ui";
  import type { NodeKind } from "$domain/infra";

  type InfraData = {
    kind: NodeKind;
    label: string;
    properties: Record<string, string>;
  };

  let { data, selected }: NodeProps & { data: InfraData } = $props();

  const icon = $derived(resolveNodeIcon(data.kind, data.properties));

  // Propiedad más representativa para el resumen de la caja.
  const summary = $derived(
    data.properties?.ip ??
      data.properties?.hostname ??
      data.properties?.host ??
      data.properties?.url_admin ??
      null,
  );
</script>

<div class="node {data.kind}" class:selected>
  <Handle type="target" position={Position.Left} />
  <div class="icon" class:brand={icon.type === "devicon"}>
    {#if icon.type === "devicon"}
      <TechIcon name={icon.name} size={22} />
    {:else}
      <Icon icon={icon.icon} size={20} />
    {/if}
  </div>
  <div class="body">
    <span class="label">{data.label}</span>
    {#if summary}
      <span class="summary">{summary}</span>
    {/if}
  </div>
  <Handle type="source" position={Position.Right} />
</div>

<style>
  .node {
    display: flex;
    align-items: center;
    gap: 0.55rem;
    min-width: 9rem;
    max-width: 15rem;
    padding: 0.55rem 0.7rem;
    background: var(--karto-color-surface, #131a26);
    border: 1px solid var(--karto-color-border, #334155);
    border-radius: 10px;
    color: var(--karto-color-text, #e2e8f0);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
  }
  .node.selected {
    border-color: var(--karto-color-accent, #11b245);
    box-shadow: 0 0 0 1px var(--karto-color-accent, #11b245);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 2rem;
    height: 2rem;
    flex-shrink: 0;
    border-radius: 8px;
    background: color-mix(in srgb, var(--karto-color-accent, #11b245) 18%, transparent);
    color: var(--karto-color-accent, #11b245);
  }
  /* Los logos de marca (Devicon) llevan su propio color: fondo neutro. */
  .icon.brand {
    background: color-mix(in srgb, #ffffff 8%, transparent);
  }
  .body {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .label {
    font-weight: 600;
    font-size: 0.9rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .summary {
    font-size: 0.75rem;
    opacity: 0.65;
    font-family: var(--karto-font-body, monospace);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
