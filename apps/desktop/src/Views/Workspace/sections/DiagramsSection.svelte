<script lang="ts">
  // Sección Diagramas: panel izquierdo (contexto 9 + árbol 8) y canvas (10).
  // Reúne el layout que antes vivía inline en Workspace, incluida la lógica de
  // colapso overlay-on-hover del panel lateral.
  import Sidebar from "../Sidebar.svelte";
  import Canvas from "../Canvas.svelte";
  import ContextBar from "../ContextBar.svelte";
  import GlobalSearch from "../GlobalSearch.svelte";
  import type { SearchHit } from "$domain/infra";
  import { requestFocus } from "../focusNode.svelte";

  interface Props {
    selectedMapId: string | null;
    collapsed: boolean;
    onManageContexts: () => void;
  }

  let {
    selectedMapId = $bindable(null),
    collapsed,
    onManageContexts,
  }: Props = $props();

  // Al elegir un resultado de la búsqueda: abrir su diagrama y pedir enfocar el
  // nodo (lo consume el FlowEditor del mapa destino tras cargar el grafo).
  function onPickSearch(hit: SearchHit) {
    selectedMapId = hit.mapId;
    requestFocus(hit.mapId, hit.nodeId);
  }
</script>

<div class="body" class:collapsed>
  <div class="sidebar-host">
    <ContextBar {onManageContexts} />
    <GlobalSearch onPick={onPickSearch} />
    <div class="tree-host">
      <Sidebar bind:selectedMapId />
    </div>
  </div>
  <div class="canvas-host">
    <Canvas mapId={selectedMapId} />
  </div>
</div>

<style>
  .body {
    flex: 1;
    display: grid;
    grid-template-columns: 16rem 1fr;
    /* Fila explícita acotada a la altura disponible: si no, la fila implícita
       `auto` crece con el contenido (p. ej. la paleta de nodos) y desborda toda
       la pantalla en Y en vez de que cada panel haga scroll interno. */
    grid-template-rows: minmax(0, 1fr);
    min-height: 0;
    min-width: 0;
    position: relative;
    /* Recorta el panel colapsado (posición absoluta) a los límites de `.body`,
       para que al deslizarse fuera no se pinte sobre el ActivityRail. */
    overflow: hidden;
  }
  /* El host del sidebar ocupa la columna del grid cuando está fijado. Columna
     flex: contexto fijo arriba, árbol con scroll debajo. */
  .sidebar-host {
    grid-column: 1;
    grid-row: 1;
    min-height: 0;
    min-width: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    /* El divisor vertical vive aquí (no en `.sidebar`) para que cubra toda la
       altura: ContextBar y GlobalSearch quedan por encima del árbol y, si no,
       el borde solo se pintaría en el tramo del árbol y faltaría arriba. */
    border-right: 1px solid var(--karto-color-border);
  }
  .tree-host {
    flex: 1;
    min-height: 0;
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

  /* --- Estado colapsado: el panel se oculta por completo (sin franja visible);
     el canvas ocupa todo el ancho. Se revela con el botón de la barra superior. --- */
  .body.collapsed {
    grid-template-columns: 0 1fr;
  }
  .body.collapsed .sidebar-host {
    position: absolute;
    left: 0;
    top: 0;
    bottom: 0;
    width: 16rem;
    z-index: 20;
    /* Fuera de pantalla por completo; el botón (zona 1) lo trae de vuelta. */
    transform: translateX(-100%);
    transition: transform 0.18s ease;
    background: var(--karto-color-bg, #0f1520);
  }
</style>
