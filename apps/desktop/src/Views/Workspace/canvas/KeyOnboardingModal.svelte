<script lang="ts">
  // Modal que aparece al conectar por contraseña a un nodo SSH: ofrece cambiar a
  // acceso por llave (más seguro). Los dos últimos checkboxes dependen del
  // primero. Lee/actualiza el estado compartido en `connectFlow.svelte.ts`.
  import { Modal, Button, Checkbox } from "@karto/ui";
  import { onboarding, confirmOnboarding, cancelOnboarding } from "./connectFlow.svelte";

  let registerKey = $state(true);
  let setDefaultKey = $state(true);
  let storeInVault = $state(false);

  // Resiembra las opciones cada vez que se abre para una credencial nueva.
  let lastPending: unknown = null;
  $effect(() => {
    if (onboarding.pending && onboarding.pending !== lastPending) {
      lastPending = onboarding.pending;
      registerKey = true;
      setDefaultKey = true;
      storeInVault = false;
    }
  });

  const open = $derived(onboarding.pending !== null);
  const host = $derived(onboarding.pending?.credential.username ?? "");

  async function proceed() {
    await confirmOnboarding({
      registerKey,
      setDefaultKey: registerKey && setDefaultKey,
      storeInVault: registerKey && storeInVault,
    });
  }
</script>

<Modal {open} title="Conexión SSH por contraseña" width="30rem" onClose={cancelOnboarding}>
  <p class="intro">
    Esta credencial{host ? ` (${host})` : ""} se conecta por contraseña. Puedes
    cambiar a <strong>acceso por llave</strong>, más seguro: tecleas la contraseña
    una sola vez, se copia la llave al servidor y la conexión continúa con la llave.
  </p>

  <div class="check">
    <Checkbox bind:checked={registerKey}>
      <span class="label">
        <strong>Registrar una llave</strong> para conexiones seguras
        <small>Genera una llave ed25519 y la copia al servidor (ssh-copy-id).</small>
      </span>
    </Checkbox>
  </div>

  <div class="check">
    <Checkbox bind:checked={setDefaultKey} disabled={!registerKey}>
      <span class="label">
        Usar la llave como <strong>conexión predeterminada</strong>
        <small>Las próximas conexiones usarán la llave (la contraseña queda de respaldo).</small>
      </span>
    </Checkbox>
  </div>

  <div class="check">
    <Checkbox bind:checked={storeInVault} disabled={!registerKey}>
      <span class="label">
        Guardar la llave <strong>en el vault del diagrama</strong>
        <small>La llave privada viaja cifrada con el .karto (portable entre equipos).</small>
      </span>
    </Checkbox>
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={cancelOnboarding} disabled={onboarding.busy}>Cancelar</Button>
    <Button onclick={proceed} disabled={onboarding.busy}>
      {onboarding.busy ? "Procesando…" : registerKey ? "Configurar y conectar" : "Conectar con contraseña"}
    </Button>
  {/snippet}
</Modal>

<style>
  .intro {
    margin: 0 0 1rem;
    font-size: 0.85rem;
    color: var(--karto-color-text-muted);
    line-height: 1.4;
  }
  .check {
    padding: 0.5rem 0;
  }
  .label {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-size: 0.88rem;
    color: var(--karto-color-text);
  }
  .label small {
    font-size: 0.78rem;
    color: var(--karto-color-text-muted);
  }
</style>
