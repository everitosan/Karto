<script lang="ts">
  // Zonas de redimensionado para ventana sin decoración (`decorations: false`).
  // En Linux/algunos compositores, quitar la decoración nativa elimina el resize
  // por bordes con el mouse; estas franjas invisibles en los bordes/esquinas lo
  // reponen llamando a `startResizeDragging(direction)` de Tauri. Se montan a
  // nivel de app para cubrir todas las pantallas (workspace, welcome, unlock…).
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { windowState } from "./windowState.svelte";

  // `ResizeDirection` no se exporta desde el módulo; se replica el union local.
  type ResizeDirection =
    | "North"
    | "South"
    | "East"
    | "West"
    | "NorthEast"
    | "NorthWest"
    | "SouthEast"
    | "SouthWest";

  const win = getCurrentWindow();

  // 4 bordes + 4 esquinas. El orden dibuja las esquinas por encima (mayor z).
  const zones: { dir: ResizeDirection; cls: string }[] = [
    { dir: "North", cls: "n" },
    { dir: "South", cls: "s" },
    { dir: "West", cls: "w" },
    { dir: "East", cls: "e" },
    { dir: "NorthWest", cls: "nw" },
    { dir: "NorthEast", cls: "ne" },
    { dir: "SouthWest", cls: "sw" },
    { dir: "SouthEast", cls: "se" },
  ];

  function start(dir: ResizeDirection, e: PointerEvent) {
    // Solo botón primario; evita interferir con el arrastre de la ventana.
    if (e.button !== 0) return;
    e.preventDefault();
    void win.startResizeDragging(dir).catch(() => {});
  }
</script>

<!-- Maximizada: no hay bordes que redimensionar; se ocultan las franjas. -->
{#if !windowState.maximized}
  <div class="resize-zones" aria-hidden="true">
    {#each zones as z (z.cls)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="zone {z.cls}" onpointerdown={(e) => start(z.dir, e)}></div>
    {/each}
  </div>
{/if}

<style>
  .resize-zones {
    position: fixed;
    inset: 0;
    z-index: 9999;
    /* El contenedor no captura eventos; solo las franjas hijas. */
    pointer-events: none;
  }
  .zone {
    position: absolute;
    pointer-events: auto;
  }
  /* Grosor de las franjas de borde y esquinas. */
  .n,
  .s {
    left: 4px;
    right: 4px;
    height: 4px;
    cursor: ns-resize;
  }
  .n {
    top: 0;
  }
  .s {
    bottom: 0;
  }
  .w,
  .e {
    top: 4px;
    bottom: 4px;
    width: 4px;
    cursor: ew-resize;
  }
  .w {
    left: 0;
  }
  .e {
    right: 0;
  }
  .nw,
  .ne,
  .sw,
  .se {
    width: 8px;
    height: 8px;
  }
  .nw {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }
  .ne {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }
  .sw {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }
  .se {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
  }
</style>
