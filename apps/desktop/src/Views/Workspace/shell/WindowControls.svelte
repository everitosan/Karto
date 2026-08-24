<script lang="ts">
  // Controles de ventana custom (zona 3): minimizar, maximizar/restaurar, cerrar.
  // Requiere `decorations: false` en tauri.conf.json. Usa la API de ventana de
  // Tauri; fuera de Tauri (p. ej. Vite solo) degrada silenciosamente. El estado
  // maximizado viene del store compartido `windowState`.
  // Nota Fase 7: en macOS los controles van a la izquierda y con otro estilo;
  // este componente se parametrizará por SO ahí.
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { windowState } from "$components/windowState.svelte";

  const win = getCurrentWindow();
  const maximized = $derived(windowState.maximized);

  const minimize = () => void win.minimize().catch(() => {});
  const toggleMaximize = () => void win.toggleMaximize().catch(() => {});
  const close = () => void win.close().catch(() => {});
</script>

<div class="controls">
  <button class="ctl" title="Minimizar" aria-label="Minimizar" onclick={minimize}>
    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
      <line x1="2" y1="6" x2="10" y2="6" stroke="currentColor" stroke-width="1" />
    </svg>
  </button>

  <button
    class="ctl"
    title={maximized ? "Restaurar" : "Maximizar"}
    aria-label={maximized ? "Restaurar" : "Maximizar"}
    onclick={toggleMaximize}
  >
    {#if maximized}
      <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
        <rect x="2.5" y="3.5" width="6" height="6" fill="none" stroke="currentColor" stroke-width="1" />
        <path d="M4.5 3.5 V2.5 H9.5 V7.5 H8.5" fill="none" stroke="currentColor" stroke-width="1" />
      </svg>
    {:else}
      <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
        <rect x="2.5" y="2.5" width="7" height="7" fill="none" stroke="currentColor" stroke-width="1" />
      </svg>
    {/if}
  </button>

  <button class="ctl close" title="Cerrar" aria-label="Cerrar" onclick={close}>
    <svg viewBox="0 0 12 12" width="12" height="12" aria-hidden="true">
      <line x1="2.5" y1="2.5" x2="9.5" y2="9.5" stroke="currentColor" stroke-width="1" />
      <line x1="9.5" y1="2.5" x2="2.5" y2="9.5" stroke="currentColor" stroke-width="1" />
    </svg>
  </button>
</div>

<style>
  .controls {
    display: inline-flex;
    align-items: center;
    gap: 0.15rem;
    /* Los botones no forman parte de la zona de arrastre de la titlebar. */
    -webkit-app-region: no-drag;
  }
  .ctl {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 1.9rem;
    height: 1.9rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
  }
  .ctl:hover {
    background: var(--karto-color-surface);
    color: var(--karto-color-text);
  }
  .ctl.close:hover {
    background: #e11d48;
    color: #fff;
  }
</style>
