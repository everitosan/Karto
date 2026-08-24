<script lang="ts" module>
  import type { IconSvgElement } from "@hugeicons/svelte";

  /** Definición de un tab: id único, icono del trigger y etiqueta accesible. */
  export interface TabItem {
    id: string;
    icon: IconSvgElement;
    /** Texto para tooltip / aria-label (y opcionalmente visible junto al icono). */
    label: string;
  }
</script>

<script lang="ts">
  // Tabs genérico con triggers por icono. El contenido lo aporta el consumidor
  // vía snippet `children`, que recibe el id del tab activo para renderizar
  // condicionalmente. Reutilizable: no sabe nada de su contenido.
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  interface Props {
    tabs: TabItem[];
    /** Tab activo (bindable). Por defecto, el primero de la lista. */
    active?: string;
    /** Muestra la etiqueta junto al icono en cada trigger. */
    showLabels?: boolean;
    /** Contenido del panel; recibe el id del tab activo. */
    children: Snippet<[string]>;
  }

  let {
    tabs,
    active = $bindable(tabs[0]?.id),
    showLabels = false,
    children,
  }: Props = $props();

  // Si el tab activo deja de existir (p. ej. la lista cambia), cae al primero.
  $effect(() => {
    if (active !== undefined && !tabs.some((t) => t.id === active)) {
      active = tabs[0]?.id;
    }
  });
</script>

<div class="karto-tabs">
  <div class="karto-tabs__list" role="tablist">
    {#each tabs as tab (tab.id)}
      <button
        type="button"
        class="karto-tabs__trigger"
        class:active={active === tab.id}
        class:with-label={showLabels}
        role="tab"
        aria-selected={active === tab.id}
        title={tab.label}
        aria-label={tab.label}
        onclick={() => (active = tab.id)}
      >
        <Icon icon={tab.icon} size={16} />
        {#if showLabels}<span>{tab.label}</span>{/if}
      </button>
    {/each}
  </div>

  {#if active !== undefined}
    <div class="karto-tabs__panel" role="tabpanel">
      {@render children(active)}
    </div>
  {/if}
</div>

<style>
  .karto-tabs {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }
  .karto-tabs__list {
    display: flex;
    gap: 0.15rem;
    padding: 0.2rem;
    border: 1px solid var(--karto-color-border, #1e2633);
    border-radius: var(--karto-radius, 0.5rem);
    background: var(--karto-color-surface, #12161f);
  }
  .karto-tabs__trigger {
    flex: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.35rem;
    padding: 0.4rem;
    border: 0;
    border-radius: calc(var(--karto-radius, 0.5rem) - 0.15rem);
    background: transparent;
    color: var(--karto-color-text-muted, #8a93a6);
    cursor: pointer;
    font-family: var(--karto-font-body, inherit);
    font-size: 0.78rem;
  }
  .karto-tabs__trigger.with-label {
    justify-content: flex-start;
  }
  .karto-tabs__trigger:hover {
    color: var(--karto-color-text, #e6eaf0);
  }
  .karto-tabs__trigger.active {
    background: var(--karto-color-bg, #0b0f17);
    color: var(--karto-color-text, #e6eaf0);
  }
  .karto-tabs__panel {
    padding-top: 0.75rem;
    overflow-y: auto;
    min-height: 0;
  }
</style>
