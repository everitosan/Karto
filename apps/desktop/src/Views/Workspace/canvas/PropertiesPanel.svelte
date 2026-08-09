<script lang="ts">
  // Panel lateral de propiedades del nodo seleccionado: etiqueta, propiedades
  // clave/valor y credenciales (secreto oculto por defecto, botón copiar).
  import { Icon, TechIcon, icons, resolveNodeIcon } from "@karto/ui";
  import type { Credential, NodeKind } from "$domain/infra";
  import { NODE_KIND_LABELS, NODE_CATALOG } from "$domain/infra";
  import { workspaceUseCases as uc } from "$usecases/workspace";
  import CredentialModal, { type CredentialDraft } from "./CredentialModal.svelte";

  interface Props {
    nodeId: string;
    kind: NodeKind;
    label: string;
    properties: Record<string, string>;
    onLabel: (label: string) => void;
    onProperties: (properties: Record<string, string>) => void;
    onClose: () => void;
    onDeleteNode: () => void;
  }

  let {
    nodeId,
    kind,
    label,
    properties,
    onLabel,
    onProperties,
    onClose,
    onDeleteNode,
  }: Props = $props();

  // Especificación de propiedades sugeridas del tipo (del catálogo compartido).
  const spec = $derived(NODE_CATALOG[kind]);

  // Valores de las propiedades del catálogo (por su `key`).
  let values = $state<Record<string, string>>({});
  // Propiedades libres extra (fuera del catálogo del tipo).
  let extraPairs = $state<{ key: string; value: string }[]>([]);
  // Propiedades internas (prefijo `_`, p. ej. tamaño de zona `_w`/`_h`): no se
  // editan en el panel pero deben conservarse al guardar.
  let hiddenProps = $state<Record<string, string>>({});

  $effect(() => {
    // Recalcula cuando cambia el nodo seleccionado.
    void nodeId;
    const specKeys = new Set(NODE_CATALOG[kind].properties.map((p) => p.key));
    values = Object.fromEntries(
      NODE_CATALOG[kind].properties.map((p) => [p.key, properties[p.key] ?? ""]),
    );
    extraPairs = Object.entries(properties)
      .filter(([k]) => !specKeys.has(k) && !k.startsWith("_"))
      .map(([key, value]) => ({ key, value }));
    hiddenProps = Object.fromEntries(
      Object.entries(properties).filter(([k]) => k.startsWith("_")),
    );
  });

  // Icono en vivo (refleja el gestor/framework elegido mientras se edita).
  const headerIcon = $derived(resolveNodeIcon(kind, values));

  function commitProperties() {
    const out: Record<string, string> = { ...hiddenProps };
    for (const [key, value] of Object.entries(values)) {
      if (value.trim()) out[key] = value;
    }
    for (const { key, value } of extraPairs) {
      const k = key.trim();
      if (k && value.trim()) out[k] = value;
    }
    onProperties(out);
  }

  function addPair() {
    extraPairs = [...extraPairs, { key: "", value: "" }];
  }

  // --- Credenciales ---
  let credentials = $state<Credential[]>([]);
  let revealed = $state<Record<string, string>>({});

  // Modal de agregar/editar. `modalInitial` es la copia semilla del formulario.
  let modalOpen = $state(false);
  let modalInitial = $state<CredentialDraft>(emptyDraft());

  function emptyDraft(): CredentialDraft {
    return {
      id: null,
      kind: "ssh",
      username: "",
      secret: "",
      port: "",
      keyPath: "",
      isDefault: credentials.length === 0,
      options: "",
    };
  }

  $effect(() => {
    void nodeId;
    revealed = {};
    modalOpen = false;
    uc.listCredentials(nodeId).then((list) => (credentials = list));
  });

  async function reloadCredentials() {
    credentials = await uc.listCredentials(nodeId);
  }

  function openNew() {
    modalInitial = emptyDraft();
    modalOpen = true;
  }

  // Carga una credencial existente. Revela el secreto para reponerlo (si no, al
  // guardar se sobreescribiría con vacío).
  async function openEdit(cred: Credential) {
    const secret = await uc.revealCredential(cred.id);
    modalInitial = {
      id: cred.id,
      kind: cred.kind,
      username: cred.username ?? "",
      secret: secret ?? "",
      port: cred.port ? String(cred.port) : "",
      keyPath: cred.keyPath ?? "",
      isDefault: cred.isDefault,
      options: cred.options ?? "",
    };
    modalOpen = true;
  }

  async function saveCredential(form: CredentialDraft) {
    await uc.upsertCredential({
      id: form.id,
      nodeId,
      kind: form.kind,
      username: form.username || null,
      secret: form.secret || null,
      port: form.port ? Number(form.port) : null,
      keyPath: form.keyPath || null,
      isDefault: form.isDefault,
      options: form.options.trim() || null,
    });
    modalOpen = false;
    await reloadCredentials();
  }

  async function makeDefault(cred: Credential) {
    await uc.upsertCredential({
      id: cred.id,
      nodeId,
      kind: cred.kind,
      username: cred.username,
      port: cred.port,
      keyPath: cred.keyPath,
      isDefault: true,
    });
    await reloadCredentials();
  }

  async function removeCredential(cred: Credential) {
    if (!confirm(`¿Eliminar la credencial ${cred.kind}?`)) return;
    await uc.deleteCredential(cred.id);
    await reloadCredentials();
  }

  async function toggleReveal(cred: Credential) {
    if (revealed[cred.id] !== undefined) {
      const { [cred.id]: _, ...rest } = revealed;
      revealed = rest;
      return;
    }
    const secret = await uc.revealCredential(cred.id);
    revealed = { ...revealed, [cred.id]: secret ?? "" };
  }

  async function copySecret(cred: Credential) {
    const secret = revealed[cred.id] ?? (await uc.revealCredential(cred.id)) ?? "";
    try {
      await navigator.clipboard.writeText(secret);
    } catch {
      // El portapapeles puede no estar disponible fuera de la app empaquetada.
    }
  }

  // Lanza la conexión (SSH/web/VNC). `credentialId = null` ⇒ usa la predeterminada.
  // Acción explícita por botón (no por doble click).
  async function connect(credentialId: string | null = null) {
    try {
      await uc.connectNode(nodeId, credentialId);
    } catch (e) {
      console.error("connect_node falló", e);
    }
  }

</script>

<aside class="panel">
  <header>
    {#if headerIcon.type === "devicon"}
      <TechIcon name={headerIcon.name} size={20} />
    {:else}
      <Icon icon={headerIcon.icon} size={18} />
    {/if}
    <span class="kind">{NODE_KIND_LABELS[kind]}</span>
    <button class="icon-btn" title="Cerrar" onclick={onClose}>
      <Icon icon={icons.close} size={16} />
    </button>
  </header>

  <label class="field">
    <span>Etiqueta</span>
    <input
      value={label}
      onchange={(e) => onLabel((e.target as HTMLInputElement).value)}
    />
  </label>

  {#if spec.connectable}
    <button
      class="connect"
      title="Conectar con la credencial predeterminada"
      disabled={credentials.length === 0}
      onclick={() => connect()}
    >
      <Icon icon={icons.connect} size={15} /> Conectar
    </button>
  {/if}

  <section>
    <div class="section-head">
      <h4>Propiedades</h4>
      <button class="icon-btn" title="Añadir propiedad" onclick={addPair}>
        <Icon icon={icons.add} size={15} />
      </button>
    </div>
    {#each spec.properties as p (p.key)}
      <label class="field">
        <span>{p.label}</span>
        {#if p.type === "select"}
          <select bind:value={values[p.key]} onchange={commitProperties}>
            <option value="">—</option>
            {#each p.options ?? [] as opt (opt.value)}
              <option value={opt.value}>{opt.label}</option>
            {/each}
          </select>
        {:else}
          <input
            placeholder={p.placeholder ?? ""}
            bind:value={values[p.key]}
            onblur={commitProperties}
          />
        {/if}
      </label>
    {/each}

    {#if extraPairs.length > 0}
      <h4 class="extra-head">Otras</h4>
      {#each extraPairs as pair, i (i)}
        <div class="pair">
          <input class="key" placeholder="clave" bind:value={pair.key} onblur={commitProperties} />
          <input class="value" placeholder="valor" bind:value={pair.value} onblur={commitProperties} />
        </div>
      {/each}
    {/if}
  </section>

  <section>
    <div class="section-head">
      <h4>Credenciales</h4>
      <button class="icon-btn" title="Añadir credencial" onclick={openNew}>
        <Icon icon={icons.add} size={15} />
      </button>
    </div>

    {#each credentials as cred (cred.id)}
      <div class="cred">
        <div class="cred-top">
          <span class="badge">{cred.kind.toUpperCase()}</span>
          {#if cred.isDefault}<span class="default">por defecto</span>{/if}
          <span class="user">{cred.username ?? "—"}{cred.port ? `:${cred.port}` : ""}</span>
        </div>
        <div class="cred-secret">
          <code>{revealed[cred.id] !== undefined ? (revealed[cred.id] || "(vacío)") : "••••••••"}</code>
          <button class="icon-btn" title="Mostrar/ocultar" onclick={() => toggleReveal(cred)}>
            <Icon icon={revealed[cred.id] !== undefined ? icons.eyeOff : icons.eye} size={15} />
          </button>
          <button class="icon-btn" title="Copiar" onclick={() => copySecret(cred)}>
            <Icon icon={icons.copy} size={15} />
          </button>
          <button class="icon-btn" title="Editar" onclick={() => openEdit(cred)}>
            <Icon icon={icons.edit} size={15} />
          </button>
          {#if spec.connectable}
            <button class="icon-btn" title="Conectar con esta credencial" onclick={() => connect(cred.id)}>
              <Icon icon={icons.connect} size={15} />
            </button>
          {/if}
          {#if !cred.isDefault}
            <button class="icon-btn" title="Marcar por defecto" onclick={() => makeDefault(cred)}>
              <Icon icon={icons.key} size={15} />
            </button>
          {/if}
          <button class="icon-btn" title="Eliminar" onclick={() => removeCredential(cred)}>
            <Icon icon={icons.delete} size={15} />
          </button>
        </div>
      </div>
    {/each}
    {#if credentials.length === 0}
      <p class="muted">Sin credenciales.</p>
    {/if}
  </section>

  <button class="delete-node" onclick={onDeleteNode}>
    <Icon icon={icons.delete} size={14} /> Eliminar nodo
  </button>
</aside>

<CredentialModal
  open={modalOpen}
  initial={modalInitial}
  onSave={saveCredential}
  onClose={() => (modalOpen = false)}
/>

<style>
  .panel {
    width: 18rem;
    border-left: 1px solid var(--karto-color-border);
    padding: 0.75rem;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 1rem;
    background: var(--karto-color-bg);
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .kind {
    flex: 1;
    font-weight: 600;
  }
  h4 {
    margin: 0 0 0.4rem;
    font-size: 0.75rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    opacity: 0.65;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--karto-space-2);
    font-size: 0.75rem;
    opacity: 0.9;
  }
  /* Separación entre campos consecutivos de una sección. */
  section .field + .field {
    margin-top: var(--karto-space-3);
  }
  input,
  select {
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    padding: 0.35rem 0.5rem;
    font-family: var(--karto-font-body);
    font-size: 0.85rem;
    width: 100%;
  }
  input:focus,
  select:focus {
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
    background-position: right 0.5rem center;
  }
  option {
    background: var(--karto-color-bg, #0f1520);
    color: var(--karto-color-text);
  }
  .extra-head {
    margin-top: 0.5rem;
  }
  .connect {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.4rem;
    padding: 0.45rem 0.5rem;
    border: 1px solid var(--karto-color-accent);
    border-radius: var(--karto-radius);
    background: color-mix(in srgb, var(--karto-color-accent) 16%, transparent);
    color: var(--karto-color-text);
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }
  .connect:hover:not(:disabled) {
    background: color-mix(in srgb, var(--karto-color-accent) 28%, transparent);
  }
  .connect:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .pair {
    display: grid;
    grid-template-columns: 1fr 1.4fr;
    gap: 0.35rem;
    margin-bottom: 0.35rem;
  }
  .delete-node {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.35rem 0.5rem;
    border: 1px solid color-mix(in srgb, #f87171 40%, transparent);
    border-radius: var(--karto-radius);
    background: transparent;
    color: #f87171;
    font-size: 0.8rem;
    cursor: pointer;
    justify-content: center;
    margin-top: auto;
  }
  .delete-node:hover {
    background: color-mix(in srgb, #f87171 12%, transparent);
  }
  .icon-btn {
    display: inline-flex;
    padding: 0.25rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: inherit;
    cursor: pointer;
    opacity: 0.75;
  }
  .icon-btn:hover {
    background: var(--karto-color-surface);
    opacity: 1;
  }
  .cred {
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    padding: 0.5rem;
    margin-bottom: 0.5rem;
  }
  .cred-top {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    margin-bottom: 0.4rem;
    font-size: 0.8rem;
  }
  .badge {
    font-size: 0.65rem;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    background: var(--karto-color-surface);
    letter-spacing: 0.03em;
  }
  .default {
    font-size: 0.65rem;
    color: var(--karto-color-accent);
  }
  .user {
    margin-left: auto;
    opacity: 0.75;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cred-secret {
    display: flex;
    align-items: center;
    gap: 0.15rem;
  }
  .cred-secret code {
    flex: 1;
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .muted {
    font-size: 0.8rem;
    opacity: 0.5;
  }
</style>
