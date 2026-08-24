<script lang="ts">
  // Modal de export selectivo: contraseña nueva + qué contenido incluir.
  // Recibe cuántos nodos se exportan; delega el guardado en el padre.
  import { Modal, Button, Checkbox } from "@karto/ui";
  import PasswordField from "$components/PasswordField.svelte";

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
      error = "La contraseña debe tener al menos 8 caracteres.";
      return;
    }
    if (password !== confirm) {
      error = "La confirmación no coincide.";
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

<Modal {open} title="Exportar selección" {onClose}>
  <p class="lead">
    Se exportarán <strong>{nodeCount}</strong>
    {nodeCount === 1 ? "nodo" : "nodos"} y las conexiones entre ellos a un
    <code>.karto</code> nuevo, cifrado con una contraseña propia (para compartir sin revelar tu
    contraseña maestra).
  </p>

  <div class="group">
    <span class="lbl">Contenido a incluir</span>
    <Checkbox bind:checked={includeCredentials}>Credenciales (usuario, secreto, llave)</Checkbox>
    <Checkbox bind:checked={includeIp}>Direcciones / IP por contexto</Checkbox>
    <Checkbox bind:checked={includeFacts}>Metadata del equipo (SO, kernel, recursos…)</Checkbox>
    <Checkbox bind:checked={includeNotes}>Notas</Checkbox>
  </div>

  <div class="group">
    <span class="lbl">Contraseña del archivo exportado</span>
    <PasswordField label="Contraseña" bind:value={password} />
    <PasswordField label="Confirmar contraseña" bind:value={confirm} />
  </div>

  {#if error}<p class="error">{error}</p>{/if}

  {#snippet footer()}
    <Button variant="ghost" onclick={onClose} disabled={busy}>Cancelar</Button>
    <Button onclick={submit} disabled={!canExport || busy}>
      {busy ? "Exportando…" : "Exportar…"}
    </Button>
  {/snippet}
</Modal>

<style>
  .lead {
    font-size: 0.85rem;
    color: var(--karto-color-text-muted);
    margin: 0 0 1rem;
  }
  .lead code {
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
