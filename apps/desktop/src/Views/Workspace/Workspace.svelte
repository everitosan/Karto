<script lang="ts">
  import { Button, Icon, Logo, icons } from "@karto/ui";
  import type { VaultInfo } from "$domain/vault";
  import { vaultUseCases } from "$usecases/vault";
  // Componentes locales de esta vista (solo usados aquí) → viven junto al View.
  import Sidebar from "./Sidebar.svelte";
  import Canvas from "./Canvas.svelte";

  interface Props {
    vault: VaultInfo;
    onLock: () => void;
  }

  let { vault, onLock }: Props = $props();

  let selectedMapId = $state<string | null>(null);
  // Vista del sidebar: fijado (empuja el canvas) o colapsado a una línea
  // (se revela como overlay al pasar el mouse, sin desplazar el canvas).
  let sidebarCollapsed = $state(false);

  async function lock() {
    await vaultUseCases.lock();
    onLock();
  }
</script>

<div class="workspace">
  <header>
    <button
      class="sidebar-toggle"
      class:active={!sidebarCollapsed}
      title={sidebarCollapsed ? "Fijar barra lateral" : "Colapsar barra lateral"}
      aria-pressed={!sidebarCollapsed}
      onclick={() => (sidebarCollapsed = !sidebarCollapsed)}
    >
      <Icon icon={icons.sidebar} size={18} />
    </button>
    <Logo variant="iso" size={22} />
    <span class="path">{vault.path}</span>
    <Button variant="ghost" onclick={lock}>
      <Icon icon={icons.lock} size={16} /> Bloquear
    </Button>
  </header>
  <div class="body" class:collapsed={sidebarCollapsed}>
    <div class="sidebar-host">
      <Sidebar bind:selectedMapId />
    </div>
    <div class="canvas-host">
      <Canvas mapId={selectedMapId} />
    </div>
  </div>
</div>

<style>
  .workspace {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  header {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 1rem;
    border-bottom: 1px solid var(--karto-color-border);
  }
  .sidebar-toggle {
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
  .sidebar-toggle:hover {
    background: var(--karto-color-surface);
    color: var(--karto-color-text);
  }
  .sidebar-toggle.active {
    color: var(--karto-color-accent);
  }
  .path {
    flex: 1;
    font-size: 0.8rem;
    opacity: 0.6;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .body {
    flex: 1;
    display: grid;
    grid-template-columns: 16rem 1fr;
    /* Fila explícita acotada a la altura disponible: si no, la fila implícita
       `auto` crece con el contenido (p. ej. la paleta de nodos) y desborda toda
       la pantalla en Y en vez de que cada panel haga scroll interno. */
    grid-template-rows: minmax(0, 1fr);
    min-height: 0;
    position: relative;
  }
  /* El host del sidebar ocupa la columna del grid cuando está fijado. */
  .sidebar-host {
    grid-column: 1;
    grid-row: 1;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
  }
  /* El canvas siempre en la 2ª columna: al colapsar (sidebar en `absolute`),
     el grid deja de reservarle sitio y el canvas gana casi todo el ancho. */
  .canvas-host {
    grid-column: 2;
    grid-row: 1;
    min-height: 0;
    min-width: 0;
  }

  /* --- Estado colapsado: la columna se reduce a una línea; el panel se
     revela como overlay al hacer hover, por encima del canvas. --- */
  .body.collapsed {
    grid-template-columns: 0.5rem 1fr;
  }
  .body.collapsed .sidebar-host {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 16rem;
    z-index: 20;
    /* Deja visible solo una línea a la izquierda. */
    transform: translateX(calc(-16rem + 0.5rem));
    transition: transform 0.18s ease, box-shadow 0.18s ease;
  }
  /* La línea visible (borde de acento sutil) que invita a pasar el mouse. */
  .body.collapsed .sidebar-host::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    right: 0;
    width: 0.5rem;
    background: var(--karto-color-surface);
    border-right: 2px solid var(--karto-color-accent);
    opacity: 0.7;
    transition: opacity 0.18s ease;
  }
  /* Hover sobre el host (incluye el panel desbordado) → se despliega. */
  .body.collapsed .sidebar-host:hover {
    transform: translateX(0);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
  .body.collapsed .sidebar-host:hover::after {
    opacity: 0;
  }
</style>
