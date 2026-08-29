<script lang="ts">
  // Modal que aparece al conectar cuando el vault no puede llevarse la
  // credencial. Dos casos, con explicación distinta:
  //   - `password`: SSH sin llave -> ofrecer acceso por llave (más seguro).
  //   - `local-key`: ya hay llave, pero sólo existe en este equipo -> ofrecer
  //     una propia de Karto, instalada usando la del usuario, que así nunca
  //     entra en el vault.
  // Los dos últimos checkboxes dependen del primero.
  import { Modal, Button, Checkbox } from "@karto/ui";
  import { onboarding, confirmOnboarding, cancelOnboarding } from "./connectFlow.svelte";
  import { m } from "$paraglide/messages.js";

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
      // En `local-key` guardar la llave en el vault ES el objetivo (que el
      // .karto sea portable), así que viene marcado.
      storeInVault = onboarding.pending.reason === "local-key";
    }
  });

  const open = $derived(onboarding.pending !== null);
  const host = $derived(onboarding.pending?.credential.username ?? "");
  const localKey = $derived(onboarding.pending?.reason === "local-key");
  const hostSuffix = $derived(host ? ` (${host})` : "");

  async function proceed() {
    await confirmOnboarding({
      registerKey,
      setDefaultKey: registerKey && setDefaultKey,
      storeInVault: registerKey && storeInVault,
    });
  }
</script>

<Modal
  {open}
  title={localKey ? m.keyob_title_localkey() : m.keyob_title()}
  width="30rem"
  onClose={cancelOnboarding}
>
  <p class="intro">
    {#if localKey}
      {m.keyob_intro_localkey_before({ host: hostSuffix })}<strong
        >{m.keyob_intro_localkey_strong()}</strong
      >{m.keyob_intro_localkey_after()}
    {:else}
      {m.keyob_intro_before({ host: hostSuffix })}<strong>{m.keyob_intro_strong()}</strong>{m.keyob_intro_after()}
    {/if}
  </p>

  <div class="check">
    <Checkbox bind:checked={registerKey}>
      <span class="label">
        <strong>{localKey ? m.keyob_register_title_localkey() : m.keyob_register_title()}</strong
        >{localKey ? "" : m.keyob_register_rest()}
        <small>{localKey ? m.keyob_register_hint_localkey() : m.keyob_register_hint()}</small>
      </span>
    </Checkbox>
  </div>

  <div class="check">
    <Checkbox bind:checked={setDefaultKey} disabled={!registerKey}>
      <span class="label">
        {m.keyob_default_before()}<strong>{m.keyob_default_strong()}</strong>
        <small>{m.keyob_default_hint()}</small>
      </span>
    </Checkbox>
  </div>

  <div class="check">
    <Checkbox bind:checked={storeInVault} disabled={!registerKey}>
      <span class="label">
        {m.keyob_store_before()}<strong>{m.keyob_store_strong()}</strong>
        <small>{m.keyob_store_hint()}</small>
      </span>
    </Checkbox>
  </div>

  {#snippet footer()}
    <Button variant="ghost" onclick={cancelOnboarding} disabled={onboarding.busy}>{m.common_cancel()}</Button>
    <Button onclick={proceed} disabled={onboarding.busy}>
      {onboarding.busy
        ? m.keyob_processing()
        : registerKey
          ? m.keyob_setup_connect()
          : localKey
            ? m.keyob_connect_asis()
            : m.keyob_connect_password()}
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
