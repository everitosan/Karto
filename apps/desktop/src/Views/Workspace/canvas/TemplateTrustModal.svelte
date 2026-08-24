<script lang="ts">
  // Diálogo que aparece al conectar cuando el vault abierto (normalmente importado
  // de un tercero) trae una plantilla de conexión personalizada que ejecutaría un
  // comando de shell. Se muestra el comando y se pide confirmación explícita antes
  // de ejecutarlo, evitando ejecución de código silenciosa desde un vault ajeno.
  import { Modal, Button } from "@karto/ui";
  import {
    templateConfirm,
    confirmTemplateTrust,
    cancelTemplateTrust,
  } from "./connectFlow.svelte";
  import { m } from "$paraglide/messages.js";

  const open = $derived(templateConfirm.pending !== null);
  const command = $derived(templateConfirm.pending?.command ?? "");

  let busy = $state(false);
  async function proceed() {
    busy = true;
    try {
      await confirmTemplateTrust();
    } finally {
      busy = false;
    }
  }
</script>

<Modal {open} title={m.trust_title()} width="34rem" onClose={cancelTemplateTrust}>
  <p class="intro">
    {@html m.trust_intro()}
  </p>

  <pre class="cmd">{command}</pre>

  <p class="note">
    {m.trust_note()}
  </p>

  {#snippet footer()}
    <Button variant="ghost" onclick={cancelTemplateTrust} disabled={busy}>{m.common_cancel()}</Button>
    <Button onclick={proceed} disabled={busy}>
      {busy ? m.trust_connecting() : m.trust_confirm()}
    </Button>
  {/snippet}
</Modal>

<style>
  .intro {
    margin: 0 0 0.75rem;
    line-height: 1.45;
  }
  .cmd {
    margin: 0 0 0.75rem;
    padding: 0.6rem 0.7rem;
    background: var(--karto-color-surface-2, #0d1420);
    border: 1px solid var(--karto-color-border, #334155);
    border-radius: 8px;
    font-family: var(--karto-font-mono, monospace);
    font-size: 0.85rem;
    white-space: pre-wrap;
    word-break: break-all;
    color: var(--karto-color-text, #e2e8f0);
  }
  .note {
    margin: 0;
    font-size: 0.85rem;
    color: var(--karto-color-text-muted, #94a3b8);
  }
</style>
