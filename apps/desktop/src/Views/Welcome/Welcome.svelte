<script lang="ts">
  import { onMount } from "svelte";
  import { Button, Icon, icons, Logo, Typography } from "@karto/ui";
  import type { VaultInfo } from "$domain/vault";
  import { vaultUseCases } from "$usecases/vault";
  import { recentsUseCases, type RecentVault } from "$usecases/recents";
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
  let recents = $state<RecentVault[]>([]);

  const passwordsMatch = $derived(password.length >= 8 && password === confirm);

  onMount(() => {
    // Sin backend (p. ej. solo Vite) la llamada falla → lista vacía, sin ruido.
    recentsUseCases
      .list()
      .then((r) => (recents = r))
      .catch(() => (recents = []));
  });

  /** Nombre de archivo para mostrar la entrada reciente sin la ruta completa. */
  function fileName(path: string): string {
    return path.split(/[\\/]/).pop() ?? path;
  }

  async function createVault() {
    error = null;
    if (!passwordsMatch) {
      error = "La contraseña debe tener al menos 8 caracteres y coincidir.";
      return;
    }
    const dir = await recentsUseCases.defaultVaultDir().catch(() => "");
    const suggested = dir ? `${dir}/karto.karto` : undefined;
    const path = await pickNewVaultPath(suggested);
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

  // Un clic en un reciente va directo a desbloquear ese archivo.
  function openRecent(path: string) {
    error = null;
    onReady({ path, status: "locked" });
  }

  async function forgetRecent(path: string) {
    recents = await recentsUseCases.forget(path).catch(() => recents);
  }
</script>

<main class="welcome">
  <div class="card">
    <div class="brand"><Logo variant="full" size={40} /></div>
    <Typography variant="subtitle">
      Todo tu universo de infraestructura en un solo mapa cifrado.
    </Typography>

    {#if mode === "choose"}
      {#if recents.length > 0}
        <ul class="recents">
          {#each recents as recent (recent.path)}
            <li>
              <button class="recent" type="button" onclick={() => openRecent(recent.path)}>
                <Icon icon={icons.lock} size={16} />
                <span class="recent-text">
                  <span class="recent-name">{fileName(recent.path)}</span>
                  <span class="recent-path">{recent.path}</span>
                </span>
              </button>
              <button
                class="forget"
                type="button"
                title="Quitar de recientes"
                aria-label="Quitar de recientes"
                onclick={() => forgetRecent(recent.path)}
              >
                ✕
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="actions">
        <Button variant="secondary" onclick={openExisting}>Abrir vault existente</Button>
        <Button onclick={() => (mode = "create")}>Crear vault nuevo</Button>
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
  .recents {
    list-style: none;
    margin: 1rem 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .recents li {
    display: flex;
    align-items: center;
    gap: 0.25rem;
  }
  .recent {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0.6rem;
    background: transparent;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    cursor: pointer;
    text-align: left;
  }
  .recent:hover {
    border-color: var(--karto-color-accent);
  }
  .recent-text {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .recent-name {
    font-size: 0.9rem;
  }
  .recent-path {
    font-size: 0.72rem;
    color: var(--karto-color-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .forget {
    flex: none;
    padding: 0.35rem 0.5rem;
    background: transparent;
    border: none;
    color: var(--karto-color-text-muted);
    cursor: pointer;
    border-radius: var(--karto-radius);
  }
  .forget:hover {
    color: #fca5a5;
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
