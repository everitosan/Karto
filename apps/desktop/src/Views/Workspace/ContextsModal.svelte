<script lang="ts">
  // Gestión de contextos de acceso (puntos de vista de red): crear, renombrar y
  // eliminar. Local a la vista Workspace. El catálogo vive en el vault; el
  // contexto activo se elige desde el selector de la barra.
  import { Modal, Button, Icon, icons } from "@karto/ui";
  import {
    networkContext,
    createContext,
    renameContext,
    deleteContext,
  } from "./networkContext.svelte";
  import { m } from "$paraglide/messages.js";

  interface Props {
    open: boolean;
    onClose: () => void;
  }

  let { open, onClose }: Props = $props();

  let newName = $state("");

  async function add() {
    const name = newName.trim();
    if (!name) return;
    await createContext(name);
    newName = "";
  }

  async function commitRename(id: string, value: string) {
    const name = value.trim();
    if (!name) return;
    const current = networkContext.contexts.find((c) => c.id === id);
    if (current && current.name !== name) await renameContext(id, name);
  }

  async function remove(id: string, name: string) {
    if (networkContext.contexts.length <= 1) {
      alert(m.ctx_min_one());
      return;
    }
    if (!confirm(m.ctx_delete_confirm({ name })))
      return;
    await deleteContext(id);
  }
</script>

<Modal {open} title={m.ctx_title()} width="28rem" {onClose}>
  <p class="hint">
    {m.ctx_hint()}
  </p>

  <ul class="list">
    {#each networkContext.contexts as ctx (ctx.id)}
      <li>
        <input
          class="name"
          value={ctx.name}
          onblur={(e) => commitRename(ctx.id, (e.target as HTMLInputElement).value)}
        />
        <button
          class="icon-btn danger"
          title={m.ctx_delete_title()}
          onclick={() => remove(ctx.id, ctx.name)}
        >
          <Icon icon={icons.delete} size={15} />
        </button>
      </li>
    {/each}
  </ul>

  <div class="add">
    <input
      placeholder={m.ctx_new_placeholder()}
      bind:value={newName}
      onkeydown={(e) => e.key === "Enter" && add()}
    />
    <Button variant="secondary" onclick={add} disabled={!newName.trim()}>
      <Icon icon={icons.add} size={15} /> {m.common_add()}
    </Button>
  </div>

  {#snippet footer()}
    <Button onclick={onClose}>{m.common_close()}</Button>
  {/snippet}
</Modal>

<style>
  .hint {
    font-size: 0.8rem;
    color: var(--karto-color-text-muted);
    margin: 0 0 0.9rem;
  }
  .list {
    list-style: none;
    margin: 0 0 0.9rem;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .list li {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .name {
    flex: 1;
  }
  input {
    padding: 0.4rem 0.5rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    font-size: 0.85rem;
  }
  input:focus {
    outline: none;
    border-color: var(--karto-color-accent);
  }
  .add {
    display: flex;
    gap: 0.4rem;
  }
  .add input {
    flex: 1;
  }
  .icon-btn {
    display: inline-flex;
    padding: 0.35rem;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    background: transparent;
    color: inherit;
    cursor: pointer;
    opacity: 0.8;
  }
  .icon-btn:hover {
    opacity: 1;
  }
  .icon-btn.danger:hover {
    color: #f87171;
    border-color: color-mix(in srgb, #f87171 40%, transparent);
  }
</style>
