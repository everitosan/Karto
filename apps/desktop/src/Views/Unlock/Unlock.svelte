<script lang="ts">
  import { Button, Icon, icons } from "@karto/ui";
  import type { VaultInfo } from "$domain/vault";
  import { vaultUseCases } from "$usecases/vault";
  import PasswordField from "$components/PasswordField.svelte";

  interface Props {
    path: string | null;
    onUnlocked: (info: VaultInfo) => void;
  }

  let { path, onUnlocked }: Props = $props();

  let password = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);

  const fileName = $derived(path?.split(/[\\/]/).pop() ?? "vault");

  async function unlock() {
    if (!path) return;
    error = null;
    busy = true;
    try {
      onUnlocked(await vaultUseCases.unlock({ path, password }));
    } catch (e) {
      error =
        e instanceof Error && (e as { kind?: string }).kind === "wrong-password"
          ? "Contraseña incorrecta."
          : e instanceof Error
            ? e.message
            : String(e);
      password = "";
    } finally {
      busy = false;
    }
  }
</script>

<main class="unlock">
  <form class="card" onsubmit={(e) => (e.preventDefault(), unlock())}>
    <h1>Desbloquear</h1>
    <p class="file"><Icon icon={icons.lock} size={16} /> {fileName}</p>
    <PasswordField label="Contraseña maestra" bind:value={password} autofocus />
    {#if error}<p class="error">{error}</p>{/if}
    <Button type="submit" disabled={busy || password.length === 0}>
      {busy ? "Desbloqueando…" : "Desbloquear"}
    </Button>
  </form>
</main>

<style>
  .unlock {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .card {
    width: min(24rem, 90vw);
    padding: 2rem;
    background: var(--karto-color-surface);
    border-radius: 1rem;
  }
  h1 {
    margin: 0 0 0.25rem;
  }
  .file {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin: 0 0 1.5rem;
    color: var(--karto-color-text-muted);
    font-size: 0.9rem;
  }
  .error {
    color: #fca5a5;
    font-size: 0.85rem;
  }
</style>
