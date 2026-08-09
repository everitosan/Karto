<script lang="ts">
  import { Button, Logo, Typography } from "@karto/ui";
  import type { VaultInfo } from "$domain/vault";
  import { vaultUseCases } from "$usecases/vault";
  import { pickNewVaultPath, pickExistingVaultPath } from "$usecases/dialog";
  import PasswordField from "$components/PasswordField.svelte";

  interface Props {
    onReady: (info: VaultInfo) => void;
  }

  let { onReady }: Props = $props();

  let mode = $state<"choose" | "create">("choose");
  let password = $state("");
  let confirm = $state("");
  let error = $state<string | null>(null);
  let busy = $state(false);

  const passwordsMatch = $derived(password.length >= 8 && password === confirm);

  async function createVault() {
    error = null;
    if (!passwordsMatch) {
      error = "La contraseña debe tener al menos 8 caracteres y coincidir.";
      return;
    }
    const path = await pickNewVaultPath();
    if (!path) return;
    busy = true;
    try {
      onReady(await vaultUseCases.create({ path, password }));
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }

  async function openExisting() {
    error = null;
    const path = await pickExistingVaultPath();
    if (!path) return;
    // Delegamos el desbloqueo a la vista Unlock reportando estado "locked".
    onReady({ path, status: "locked" });
  }
</script>

<main class="welcome">
  <div class="card">
    <div class="brand"><Logo variant="full" size={40} /></div>
    <Typography variant="subtitle">
      Mapea tu infraestructura. Conéctate con un doble click.
    </Typography>

    {#if mode === "choose"}
      <div class="actions">
        <Button onclick={() => (mode = "create")}>Crear vault nuevo</Button>
        <Button variant="secondary" onclick={openExisting}>Abrir vault existente</Button>
      </div>
    {:else}
      <form onsubmit={(e) => (e.preventDefault(), createVault())}>
        <PasswordField label="Contraseña maestra" bind:value={password} />
        <PasswordField label="Confirmar contraseña" bind:value={confirm} />
        <p class="hint">
          Si la olvidas, los datos son irrecuperables (cifrado real). Haz respaldos.
        </p>
        {#if error}<p class="error">{error}</p>{/if}
        <div class="actions">
          <Button type="submit" disabled={!passwordsMatch || busy}>
            {busy ? "Creando…" : "Crear y cifrar"}
          </Button>
          <Button variant="ghost" onclick={() => (mode = "choose")}>Volver</Button>
        </div>
      </form>
    {/if}
  </div>
</main>

<style>
  .welcome {
    height: 100%;
    display: grid;
    place-items: center;
  }
  .card {
    width: min(28rem, 90vw);
    padding: 2rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: 1rem;
  }
  .brand {
    margin-bottom: 0.75rem;
  }
  .actions {
    display: flex;
    gap: 0.75rem;
    margin-top: 1rem;
    flex-wrap: wrap;
  }
  .hint {
    font-size: 0.8rem;
    opacity: 0.6;
  }
  .error {
    color: #fca5a5;
    font-size: 0.85rem;
  }
</style>
