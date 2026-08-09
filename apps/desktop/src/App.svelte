<script lang="ts">
  import { onMount } from "svelte";
  import type { VaultInfo } from "$domain/vault";
  import { vaultUseCases } from "$usecases/vault";
  import Splash from "$views/Splash/Splash.svelte";
  import Welcome from "$views/Welcome/Welcome.svelte";
  import Unlock from "$views/Unlock/Unlock.svelte";
  import Workspace from "$views/Workspace/Workspace.svelte";

  const SPLASH_MIN_MS = 1600;

  let vault = $state<VaultInfo>({ path: null, status: "no-vault" });
  let ready = $state(false);
  let splashDone = $state(false);

  onMount(() => {
    // Splash con duración mínima para que no parpadee en arranques rápidos.
    const timer = setTimeout(() => (splashDone = true), SPLASH_MIN_MS);

    vaultUseCases
      .status()
      .then((info) => (vault = info))
      .catch(() => {
        // Sin backend (p. ej. corriendo solo Vite): quedarse en bienvenida.
        vault = { path: null, status: "no-vault" };
      })
      .finally(() => (ready = true));

    return () => clearTimeout(timer);
  });

  const showSplash = $derived(!ready || !splashDone);

  function onVaultChange(next: VaultInfo) {
    vault = next;
  }
</script>

{#if showSplash}
  <Splash />
{:else if vault.status === "unlocked"}
  <Workspace {vault} onLock={() => (vault = { ...vault, status: "locked" })} />
{:else if vault.status === "locked"}
  <Unlock path={vault.path} onUnlocked={onVaultChange} />
{:else}
  <Welcome onReady={onVaultChange} />
{/if}
