<script lang="ts">
  // Modal para agregar/editar una credencial de un nodo. Mantiene una copia
  // local del formulario; el guardado real (upsert) lo hace el consumidor.
  import { Modal, Button, Checkbox, FilePicker } from "@karto/ui";
  import type { CredentialKind } from "$domain/infra";
  import { pickSshKeyPath } from "$usecases/dialog";
  import { m } from "$paraglide/messages.js";

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
  // Web abre la URL del nodo: el puerto de la credencial no se usa (va en la URL).
  const showPort = $derived(form.kind !== "web");

  async function save() {
    saving = true;
    try {
      await onSave({ ...form });
    } finally {
      saving = false;
    }
  }

  // Ajusta la altura del textarea a su contenido: se re-mide cada vez que
  // cambia `value` (incluido al resembrar el form al abrir).
  function autosize(node: HTMLTextAreaElement, _value: string) {
    const fit = () => {
      node.style.height = "auto";
      node.style.height = `${node.scrollHeight}px`;
    };
    fit();
    return { update: fit };
  }
</script>

<Modal {open} title={editing ? m.cred_edit_title() : m.cred_add_title()} {onClose}>
  <div class="form">
    <label class="field span-2">
      <span>{m.cred_type()}</span>
      <select bind:value={form.kind}>
        {#each credKinds as k (k)}<option value={k}>{k.toUpperCase()}</option>{/each}
      </select>
    </label>

    <label class="field" class:span-2={!showPort}>
      <span>{m.cred_user()}</span>
      <input placeholder={m.cred_user_placeholder()} bind:value={form.username} />
    </label>

    {#if showPort}
      <label class="field">
        <span>{m.cred_port()}</span>
        <input placeholder={m.cred_port_placeholder()} inputmode="numeric" bind:value={form.port} />
      </label>
    {/if}

    <label class="field" class:span-2={!showKeyPath}>
      <span>{m.common_password()}</span>
      <input placeholder={m.cred_secret_placeholder()} type="password" bind:value={form.secret} />
    </label>

    {#if showKeyPath}
      <label class="field">
        <span>{m.cred_key_path()}</span>
        <FilePicker
          bind:value={form.keyPath}
          placeholder={m.cred_key_placeholder()}
          onBrowse={pickSshKeyPath}
        />
      </label>

      <label class="field span-2">
        <span>{m.cred_ssh_options()}</span>
        <textarea
          rows="3"
          placeholder={m.cred_ssh_options_placeholder()}
          bind:value={form.options}
          use:autosize={form.options}
        ></textarea>
        <small class="hint">{@html m.cred_ssh_options_hint()}</small>
      </label>
    {/if}

    <div class="checkbox span-2">
      <Checkbox bind:checked={form.isDefault}>{m.cred_use_default()}</Checkbox>
    </div>
  </div>

  {#snippet footer()}
    <div class="footer-actions">
      <Button variant="secondary" onclick={onClose}>{m.common_cancel()}</Button>
      <Button variant="primary" disabled={saving} onclick={save}>
        {saving ? m.common_saving() : m.common_accept()}
      </Button>
    </div>
  {/snippet}
</Modal>

<style>
  .form {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.85rem;
    width: 100%;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--karto-space-2);
    font-size: 0.78rem;
    opacity: 0.9;
    min-width: 0;
  }
  .span-2 {
    grid-column: 1 / -1;
  }
  .footer-actions {
    display: flex;
    justify-content: space-between;
    width: 100%;
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
    resize: none;
    overflow: hidden;
    min-height: 0;
    padding-block: calc(0.45rem + var(--karto-space-1));
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
  .hint :global(code) {
    font-family: var(--karto-font-mono, monospace);
  }
  .checkbox {
    font-size: 0.85rem;
  }
</style>
