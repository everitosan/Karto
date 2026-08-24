<script lang="ts">
  // Modal de export selectivo: contraseña nueva + qué contenido incluir.
  // Recibe cuántos nodos se exportan; delega el guardado en el padre.
  import { Modal, Button, Checkbox } from "@karto/ui";
  import PasswordField from "$components/PasswordField.svelte";
  import { m } from "$paraglide/messages.js";

  interface Props {
    open: boolean;
    nodeCount: number;
    onClose: () => void;
    onConfirm: (opts: {
      password: string;
      includeCredentials: boolean;
      includeFacts: boolean;
      includeIp: boolean;
      includeNotes: boolean;
    }) => Promise<void>;
  }

  let { open, nodeCount, onClose, onConfirm }: Props = $props();

  let password = $state("");
  let confirm = $state("");
  let includeCredentials = $state(false);
  let includeFacts = $state(true);
  let includeIp = $state(true);
  let includeNotes = $state(true);
  let busy = $state(false);
  let error = $state<string | null>(null);

  const canExport = $derived(password.length >= 8 && password === confirm && nodeCount > 0);

  async function submit() {
    error = null;
    if (password.length < 8) {
      error = m.export_error_short();
      return;
    }
    if (password !== confirm) {
      error = m.export_error_mismatch();
      return;
    }
    busy = true;
    try {
      await onConfirm({ password, includeCredentials, includeFacts, includeIp, includeNotes });
      password = confirm = "";
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = false;
    }
  }
</script>

<Modal {open} title={m.export_title()} {onClose}>
  <p class="lead">
    {@html nodeCount === 1 ? m.export_lead_one({ count: nodeCount }) : m.export_lead_other({ count: nodeCount })}
  </p>

  <div class="group">
    <span class="lbl">{m.export_content_label()}</span>
    <Checkbox bind:checked={includeCredentials}>{m.export_incl_credentials()}</Checkbox>
    <Checkbox bind:checked={includeIp}>{m.export_incl_ip()}</Checkbox>
    <Checkbox bind:checked={includeFacts}>{m.export_incl_facts()}</Checkbox>
    <Checkbox bind:checked={includeNotes}>{m.export_incl_notes()}</Checkbox>
  </div>

  <div class="group">
    <span class="lbl">{m.export_password_label()}</span>
    <PasswordField label={m.common_password()} bind:value={password} />
    <PasswordField label={m.welcome_password_confirm()} bind:value={confirm} />
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#snippet footer()}
    <Button variant="ghost" onclick={onClose} disabled={busy}>{m.common_cancel()}</Button>
    <Button onclick={submit} disabled={!canExport || busy}>
      {busy ? m.export_exporting() : m.export_submit()}
    </Button>
  {/snippet}
</Modal>

<style>
  .lead {
    font-size: 0.85rem;
    color: var(--karto-color-text-muted);
    margin: 0 0 1rem;
  }
  .lead :global(code) {
    font-size: 0.75rem;
  }
  .group {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem 0;
    border-top: 1px solid var(--karto-color-border);
  }
  .lbl {
    font-size: 0.8rem;
    color: var(--karto-color-text);
    margin-bottom: 0.1rem;
  }
  .error {
    color: #fca5a5;
    font-size: 0.85rem;
    margin: 0.5rem 0 0;
  }
</style>
