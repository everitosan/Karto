<script lang="ts">
  // Titlebar de la app (zonas 1, 2, 7 del rediseño). Componente local a Workspace.
  // El selector de contexto vive en la sección Diagramas (R2) y los ajustes en la
  // sección Config (R3), ambos accesibles desde el ActivityRail. La zona 3
  // (controles de ventana) se añade en R4.
  import { Icon, icons } from "@karto/ui";
  import WindowControls from "./WindowControls.svelte";
  import { aboutUseCases } from "$usecases/about";
  import { DONATE_URL } from "$config/links";
  import { m } from "$paraglide/messages.js";

  interface Props {
    vaultPath: string | null;
    collapsed: boolean;
    onToggleSidebar: () => void;
    onLock: () => void;
    onClose: () => void;
  }

  let {
    vaultPath,
    collapsed,
    onToggleSidebar,
    onLock,
    onClose,
  }: Props = $props();
</script>

<!-- data-tauri-drag-region: la barra arrastra la ventana (y doble clic
     maximiza/restaura). Solo aplica a los elementos con el atributo, así que
     los botones/controles interactivos siguen funcionando como tales. -->
<header data-tauri-drag-region>
  <!-- Zona 1 (izquierda): solo el trigger de la barra lateral. -->
  <div class="section section-left" data-tauri-drag-region>
    <button
      class="icon-btn"
      class:active={!collapsed}
      title={collapsed ? m.topbar_sidebar_pin() : m.topbar_sidebar_collapse()}
      aria-pressed={!collapsed}
      onclick={onToggleSidebar}
    >
      <Icon icon={icons.sidebar} size={18} />
    </button>
  </div>

  <!-- Zona 2 (centro): archivo abierto con su botón de cerrar a la izquierda. -->
  <div class="section section-center">
    <button class="icon-btn" title={m.topbar_close_vault()} onclick={onClose}>
      <Icon icon={icons.closeSquare} size={18} />
    </button>
    <span class="path">{vaultPath}</span>
  </div>

  <!-- Zona 3 (derecha): donar, bloquear y controles de ventana. -->
  <div class="section section-right">
    <button
      class="icon-btn coffee"
      title={m.topbar_donate()}
      onclick={() => aboutUseCases.openExternalUrl(DONATE_URL)}
    >
      <Icon icon={icons.coffee} size={18} />
    </button>
    <button class="icon-btn" title={m.topbar_lock()} aria-label={m.topbar_lock()} onclick={onLock}>
      <Icon icon={icons.lock} size={18} />
    </button>
    <WindowControls />
  </div>
</header>

<style>
  header {
    display: flex;
    align-items: center;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--karto-color-border);
  }
  .section {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .section-left {
    flex: 1;
    justify-content: flex-start;
  }
  .section-center {
    flex: 0 1 auto;
    min-width: 0;
    justify-content: center;
  }
  .section-right {
    flex: 1;
    justify-content: flex-end;
  }
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.3rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--karto-color-surface);
    color: var(--karto-color-text);
  }
  .icon-btn.active {
    color: var(--karto-color-accent);
  }
  .icon-btn.coffee:hover {
    color: var(--karto-color-accent);
  }
  .path {
    flex: 0 1 auto;
    min-width: 0;
    font-size: 0.8rem;
    opacity: 0.6;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .spacer {
    flex: 1;
    align-self: stretch;
  }
</style>
