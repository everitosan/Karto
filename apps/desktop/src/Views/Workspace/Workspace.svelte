<script lang="ts">
  import { onMount } from "svelte";
  import type { VaultInfo } from "$domain/vault";
  import { vaultUseCases } from "$usecases/vault";
  // Componentes locales de esta vista (solo usados aquí) → viven junto al View.
  import TopBar from "./shell/TopBar.svelte";
  import ActivityRail from "./shell/ActivityRail.svelte";
  import DiagramsSection from "./sections/DiagramsSection.svelte";
  import ScriptsSection from "./sections/ScriptsSection.svelte";
  import SettingsSection from "./sections/SettingsSection.svelte";
  import ContextsModal from "./ContextsModal.svelte";
  import { sectionState } from "./section.svelte";
  import { appSettings, loadAppSettings } from "./appSettings.svelte";
  import { loadContexts } from "./networkContext.svelte";
  import { createIdleTimer } from "./autoLock";
  import { clipboardManager } from "./clipboard";

  interface Props {
    vault: VaultInfo;
    onLock: () => void;
    onClose: () => void;
  }

  let { vault, onLock, onClose }: Props = $props();

  let selectedMapId = $state<string | null>(null);
  // Vista del sidebar: fijado (empuja el canvas) o colapsado a una línea
  // (se revela como overlay al pasar el mouse, sin desplazar el canvas).
  let sidebarCollapsed = $state(false);
  let contextsOpen = $state(false);

  async function lock() {
    // Al bloquear, borra cualquier secreto que quedara en el portapapeles.
    await clipboardManager.clearNow();
    await vaultUseCases.lock();
    onLock();
  }

  // Cierra el vault por completo y regresa a la selección de vaults (Welcome).
  async function close() {
    await clipboardManager.clearNow();
    await vaultUseCases.close();
    onClose();
  }

  // --- Auto-bloqueo por inactividad ---
  // Un solo temporizador reiniciado por la actividad del usuario; el intervalo
  // reacciona a los ajustes (0 = desactivado).
  onMount(() => {
    void loadAppSettings();
    void loadContexts();

    let timer = createIdleTimer(0, () => void lock());
    const activity = () => timer.touch();
    const events = ["mousemove", "mousedown", "keydown", "wheel", "touchstart"];
    events.forEach((e) => window.addEventListener(e, activity, { passive: true }));

    // Recrea el temporizador cuando cambia el intervalo configurado.
    const applyInterval = $effect.root(() => {
      $effect(() => {
        timer.stop();
        timer = createIdleTimer(appSettings.autoLockMinutes * 60_000, () => void lock());
        timer.touch();
      });
    });

    return () => {
      events.forEach((e) => window.removeEventListener(e, activity));
      timer.stop();
      applyInterval();
    };
  });
</script>

<div class="workspace">
  <TopBar
    vaultPath={vault.path}
    collapsed={sidebarCollapsed}
    onToggleSidebar={() => (sidebarCollapsed = !sidebarCollapsed)}
    onLock={lock}
    onClose={close}
  />

  <ContextsModal open={contextsOpen} onClose={() => (contextsOpen = false)} />

  <div class="main">
    <ActivityRail />

    <div class="section-outlet">
      {#if sectionState.active === "diagrams"}
        <!-- Sección Diagramas: contexto (9) + árbol (8) + canvas (10). -->
        <DiagramsSection
          bind:selectedMapId
          collapsed={sidebarCollapsed}
          onManageContexts={() => (contextsOpen = true)}
        />
      {:else if sectionState.active === "scripts"}
        <ScriptsSection />
      {:else if sectionState.active === "config"}
        <SettingsSection />
      {/if}
    </div>
  </div>
</div>

<style>
  .workspace {
    height: 100%;
    display: flex;
    flex-direction: column;
  }
  .main {
    flex: 1;
    display: flex;
    min-height: 0;
  }
  .section-outlet {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
  }
</style>
