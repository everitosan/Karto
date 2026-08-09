<script lang="ts">
  // Modal genérico: fondo oscurecido + panel centrado. Cierra con Escape, clic
  // en el fondo o el botón ✕. El contenido y el pie son snippets del consumidor.
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";
  import { icons } from "./icons";

  interface Props {
    open: boolean;
    title?: string;
    /** Ancho máximo del panel (CSS length). */
    width?: string;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
  }

  let { open, title, width = "26rem", onClose, children, footer }: Props = $props();

  function onKeydown(e: KeyboardEvent) {
    if (open && e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="karto-modal-backdrop" onclick={onClose}>
    <div
      class="karto-modal"
      style="max-width: {width}"
      role="dialog"
      aria-modal="true"
      aria-label={title}
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
    >
      <header class="karto-modal__head">
        <h3>{title}</h3>
        <button class="karto-modal__close" type="button" title="Cerrar" onclick={onClose}>
          <Icon icon={icons.close} size={18} />
        </button>
      </header>

      <div class="karto-modal__body">
        {@render children()}
      </div>

      {#if footer}
        <footer class="karto-modal__foot">
          {@render footer()}
        </footer>
      {/if}
    </div>
  </div>
{/if}

<style>
  .karto-modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 1rem;
    background: rgba(0, 0, 0, 0.55);
    backdrop-filter: blur(2px);
  }
  .karto-modal {
    width: 100%;
    max-height: calc(100vh - 2rem);
    display: flex;
    flex-direction: column;
    background: var(--karto-color-bg, #0b0f17);
    border: 1px solid var(--karto-color-border, #1e2633);
    border-radius: calc(var(--karto-radius, 0.5rem) * 1.5);
    box-shadow: 0 1.5rem 3rem rgba(0, 0, 0, 0.5);
    color: var(--karto-color-text, #e6eaf0);
  }
  .karto-modal__head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 1rem;
    padding: 1rem 1.25rem;
    border-bottom: 1px solid var(--karto-color-border, #1e2633);
  }
  .karto-modal__head h3 {
    margin: 0;
    font-family: var(--karto-font-title, inherit);
    font-size: 1.05rem;
    font-weight: 600;
  }
  .karto-modal__close {
    display: inline-flex;
    padding: 0.3rem;
    border: 0;
    border-radius: var(--karto-radius, 0.5rem);
    background: transparent;
    color: inherit;
    cursor: pointer;
    opacity: 0.7;
  }
  .karto-modal__close:hover {
    background: var(--karto-color-surface, #12161f);
    opacity: 1;
  }
  .karto-modal__body {
    padding: 1.25rem;
    overflow-y: auto;
  }
  .karto-modal__foot {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    padding: 0.85rem 1.25rem;
    border-top: 1px solid var(--karto-color-border, #1e2633);
  }
</style>
