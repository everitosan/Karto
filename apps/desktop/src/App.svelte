<script lang="ts">
  import { onMount } from "svelte";
  import type { VaultInfo } from "$domain/vault";
  import { vaultUseCases } from "$usecases/vault";
  import Splash from "$views/Splash/Splash.svelte";
  import Welcome from "$views/Welcome/Welcome.svelte";
  import Unlock from "$views/Unlock/Unlock.svelte";
  import Workspace from "$views/Workspace/Workspace.svelte";
  import WindowResizeZones from "$components/WindowResizeZones.svelte";
  import { windowState, initWindowState } from "$components/windowState.svelte";

  const SPLASH_MIN_MS = 1600;

  let vault = $state<VaultInfo>({ path: null, status: "no-vault" });
  let ready = $state(false);
  let splashDone = $state(false);

  onMount(() => {
    // Splash con duración mínima para que no parpadee en arranques rápidos.
    const timer = setTimeout(() => (splashDone = true), SPLASH_MIN_MS);
    // Ventana sin decoración: sigue el estado maximizado para el marco redondeado.
    const stopWindow = initWindowState();

    vaultUseCases
      .status()
      .then((info) => (vault = info))
      .catch(() => {
        // Sin backend (p. ej. corriendo solo Vite): quedarse en bienvenida.
        vault = { path: null, status: "no-vault" };
      })
      .finally(() => (ready = true));

    return () => {
      clearTimeout(timer);
      stopWindow();
    };
  });

  const showSplash = $derived(!ready || !splashDone);

  function onVaultChange(next: VaultInfo) {
    vault = next;
  }

  // Cancelar el desbloqueo: cierra el vault en el backend (por si venía de
  // "Bloquear", que deja el path recordado) y vuelve a la selección de vaults.
  async function cancelUnlock() {
    await vaultUseCases.close().catch(() => {});
    vault = { path: null, status: "no-vault" };
  }
</script>

<!-- Marco de la app: fondo + esquinas redondeadas (rectas al maximizar/fullscreen,
     donde un radio dejaría huecos en las esquinas de la pantalla). -->
<div class="app-frame" class:maximized={windowState.maximized}>
  {#if showSplash}
    <Splash />
  {:else if vault.status === "unlocked"}
    <Workspace
      {vault}
      onLock={() => (vault = { ...vault, status: "locked" })}
      onClose={() => (vault = { path: null, status: "no-vault" })}
    />
  {:else if vault.status === "locked"}
    <Unlock path={vault.path} onUnlocked={onVaultChange} onCancel={cancelUnlock} />
  {:else}
    <Welcome onReady={onVaultChange} />
  {/if}

  <!-- Ventana sin decoración: repone el resize por bordes (app-wide). -->
  <WindowResizeZones />
</div>

<style>
  .app-frame {
    height: 100%;
    overflow: hidden;
    background: var(--karto-bg);
    background-attachment: fixed;
    border-radius: 10px;
    /* Borde sutil para separar la ventana del escritorio con transparencia. */
    border: 1px solid var(--karto-color-border);
  }
  .app-frame.maximized {
    border-radius: 0;
    border-color: transparent;
  }
</style>
