<script lang="ts">
  // Modal para agregar/editar una credencial de un nodo. Mantiene una copia
  // local del formulario; el guardado real (upsert) lo hace el consumidor.
  import { Modal, Button } from "@karto/ui";
  import type { CredentialKind } from "$domain/infra";

  export interface CredentialDraft {
    id: string | null;
    kind: CredentialKind;
    username: string;
    secret: string;
    port: string;
    keyPath: string;
    isDefault: boolean;
    /** Opciones SSH extra, una por línea (se prefijan con `-o` al conectar). */
    options: string;
  }

  interface Props {
    open: boolean;
    initial: CredentialDraft;
    onSave: (draft: CredentialDraft) => void | Promise<void>;
    onClose: () => void;
  }

  let { open, initial, onSave, onClose }: Props = $props();

  const credKinds: CredentialKind[] = ["ssh", "rdp", "vnc", "web", "db"];

  // Copia de trabajo: se resiembra desde `initial` cada vez que se abre el modal
  // (el contenido solo se muestra con el modal abierto, así que el valor inicial
  // de placeholder nunca llega a verse).
  let form = $state<CredentialDraft>({
    id: null,
    kind: "ssh",
    username: "",
    secret: "",
    port: "",
    keyPath: "",
    isDefault: false,
    options: "",
  });
  let saving = $state(false);
  $effect(() => {
    if (open) {
      form = { ...initial };
      saving = false;
    }
  });

  const editing = $derived(initial.id !== null);
  // SSH/VNC/RDP/DB usan puerto y credenciales; web usa usuario/clave del panel.
  const showKeyPath = $derived(form.kind === "ssh");

  async function save() {
    saving = true;
    try {
      await onSave({ ...form });
    } finally {
      saving = false;
    }
  }
</script>

<Modal {open} title={editing ? "Editar credencial" : "Agregar credencial"} {onClose}>
  <div class="form">
    <label class="field">
      <span>Tipo</span>
      <select bind:value={form.kind}>
        {#each credKinds as k (k)}<option value={k}>{k.toUpperCase()}</option>{/each}
      </select>
    </label>

    <label class="field">
      <span>Usuario</span>
      <input placeholder="usuario" bind:value={form.username} />
    </label>

    <label class="field">
      <span>Secreto</span>
      <input placeholder="contraseña / passphrase" type="password" bind:value={form.secret} />
    </label>

    <label class="field">
      <span>Puerto</span>
      <input placeholder="p. ej. 22" inputmode="numeric" bind:value={form.port} />
    </label>

    {#if showKeyPath}
      <label class="field">
        <span>Ruta de la llave</span>
        <input placeholder="~/.ssh/id_ed25519 (opcional)" bind:value={form.keyPath} />
      </label>

      <label class="field">
        <span>Opciones SSH extra</span>
        <textarea
          rows="3"
          placeholder={"Una por línea, p. ej.:\nServerAliveInterval=60\nConnectTimeout=10\nProxyJump bastion"}
          bind:value={form.options}
        ></textarea>
        <small class="hint">Cada línea se pasa como <code>-o &lt;opción&gt;</code> a ssh.</small>
      </label>
    {/if}

    <label class="checkbox">
      <input type="checkbox" bind:checked={form.isDefault} />
      Usar como predeterminada
    </label>
  </div>

  {#snippet footer()}
    <Button variant="secondary" onclick={onClose}>Cancelar</Button>
    <Button variant="primary" disabled={saving} onclick={save}>
      {saving ? "Guardando…" : "Guardar"}
    </Button>
  {/snippet}
</Modal>

<style>
  .form {
    display: flex;
    flex-direction: column;
    gap: 0.85rem;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--karto-space-2);
    font-size: 0.78rem;
    opacity: 0.9;
  }
  input,
  select,
  textarea {
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    padding: 0.45rem 0.6rem;
    font-family: var(--karto-font-body);
    font-size: 0.9rem;
    width: 100%;
  }
  textarea {
    resize: vertical;
    font-family: var(--karto-font-mono, monospace);
  }
  input:focus,
  select:focus,
  textarea:focus {
    outline: none;
    border-color: var(--karto-color-accent);
  }
  /* El <select> nativo ignora el fondo oscuro sin appearance:none; y su
     desplegable sale claro sin color-scheme. */
  select {
    appearance: none;
    -webkit-appearance: none;
    color-scheme: dark;
    cursor: pointer;
    padding-right: 1.6rem;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%238a93a6' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'><polyline points='6 9 12 15 18 9'/></svg>");
    background-repeat: no-repeat;
    background-position: right 0.6rem center;
  }
  option {
    background: var(--karto-color-bg, #0f1520);
    color: var(--karto-color-text);
  }
  .hint {
    font-size: 0.7rem;
    opacity: 0.6;
  }
  .hint code {
    font-family: var(--karto-font-mono, monospace);
  }
  .checkbox {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  .checkbox input {
    width: auto;
  }
</style>
