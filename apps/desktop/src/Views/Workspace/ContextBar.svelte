<script lang="ts">
  // Selector de contexto de red activo (zona 9). Vive en la sección Diagramas,
  // sobre el árbol de carpetas. El contexto determina qué dirección de cada nodo
  // se usa al conectar (oficina/VPN/…); cambiarlo reetiqueta todo el diagrama.
  // La gestión (alta/renombrar/borrar) sigue en el modal `ContextsModal`.
  import { Icon, icons } from "@karto/ui";
  import { networkContext, setActiveContext } from "./networkContext.svelte";
  import { m } from "$paraglide/messages.js";

  interface Props {
    onManageContexts: () => void;
  }

  let { onManageContexts }: Props = $props();
</script>

{#if networkContext.contexts.length > 0}
  <div class="context" title={m.contextbar_active()}>
    <Icon icon={icons.link} size={15} />
    <select
      value={networkContext.activeId}
      onchange={(e) => setActiveContext((e.target as HTMLSelectElement).value)}
    >
      {#each networkContext.contexts as ctx (ctx.id)}
        <option value={ctx.id}>{ctx.name}</option>
      {/each}
    </select>
    <button class="icon-btn" title={m.contextbar_manage()} onclick={onManageContexts}>
      <Icon icon={icons.settings} size={15} />
    </button>
  </div>
{/if}

<style>
  .context {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.35rem 0.5rem;
    border-bottom: 1px solid var(--karto-color-border);
    color: var(--karto-color-text-muted);
  }
  .context select {
    flex: 1;
    min-width: 0;
    appearance: none;
    -webkit-appearance: none;
    color-scheme: dark;
    background: transparent;
    border: 0;
    color: var(--karto-color-text);
    font-family: var(--karto-font-body);
    font-size: 0.8rem;
    cursor: pointer;
    padding: 0.1rem 0.2rem;
  }
  .context select:focus {
    outline: none;
  }
  .context option {
    background: var(--karto-color-bg, #0f1520);
    color: var(--karto-color-text);
  }
  .icon-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0.3rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
  }
  .icon-btn:hover {
    background: var(--karto-color-surface);
    color: var(--karto-color-text);
  }
</style>
