<script lang="ts">
  // Canvas de infraestructura (Fase 2). Provee el contexto de Svelte Flow y
  // monta el editor por diagrama. Componente local a la vista Workspace.
  import { SvelteFlowProvider } from "@xyflow/svelte";
  import FlowEditor from "./canvas/FlowEditor.svelte";
  import KeyOnboardingModal from "./canvas/KeyOnboardingModal.svelte";

  interface Props {
    mapId: string | null;
  }

  let { mapId }: Props = $props();
</script>

<section class="canvas">
  {#if mapId}
    <!-- key: remonta el editor (y recarga el grafo) al cambiar de diagrama. -->
    {#key mapId}
      <SvelteFlowProvider>
        <FlowEditor {mapId} />
      </SvelteFlowProvider>
    {/key}
  {:else}
    <div class="placeholder">
      Selecciona o crea un diagrama en la barra lateral.
    </div>
  {/if}

  <!-- Modal de onboarding de llave SSH: se abre desde el panel o el menú
       contextual vía el store compartido de connectFlow. -->
  <KeyOnboardingModal />
</section>

<style>
  .canvas {
    position: relative;
    min-height: 0;
    overflow: hidden;
    height: 100%;
  }
  .placeholder {
    height: 100%;
    display: grid;
    place-content: center;
    text-align: center;
    opacity: 0.5;
  }
</style>
