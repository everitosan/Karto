<script lang="ts">
  // Menú contextual (click derecho) de un nodo: conectar con la credencial
  // predeterminada o con una concreta, abrir propiedades o eliminar. Se
  // posiciona en las coordenadas del click y cierra al hacer clic fuera / Escape.
  import { Icon, icons } from "@karto/ui";
  import type { Credential, NodeKind } from "$domain/infra";
  import { NODE_CATALOG } from "$domain/infra";
  import { workspaceUseCases as uc } from "$usecases/workspace";
  import { requestConnect } from "./connectFlow.svelte";
  import { checkNode } from "./nodeHealth.svelte";

  interface Props {
    x: number;
    y: number;
    nodeId: string;
    kind: NodeKind;
    /** ¿El nodo tiene objetivo sondeable (hostname/URL/endpoint)? */
    canProbe: boolean;
    onDelete: () => void;
    onClose: () => void;
  }

  let { x, y, nodeId, kind, canProbe, onDelete, onClose }: Props = $props();

  const connectable = $derived(NODE_CATALOG[kind]?.connectable ?? false);

  let credentials = $state<Credential[]>([]);
  $effect(() => {
    if (connectable) {
      uc.listCredentials(nodeId).then((list) => (credentials = list));
    }
  });

  // Etiqueta legible de la credencial para el ítem del menú.
  function credLabel(c: Credential): string {
    const who = c.username ? ` · ${c.username}` : "";
    return `${c.kind.toUpperCase()}${who}${c.isDefault ? " (por defecto)" : ""}`;
  }

  function connect(credentialId: string | null) {
    // `requestConnect` decide de forma síncrona si conecta directo o abre el
    // modal de onboarding (guardado en el store compartido), así que podemos
    // cerrar el menú a continuación sin perder la acción.
    requestConnect(nodeId, credentialId, credentials).catch((e: unknown) => {
      console.error("connect falló", e);
      const msg = typeof e === "string" ? e : e instanceof Error ? e.message : "No se pudo conectar";
      alert(msg);
    });
    onClose();
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") onClose();
  }
</script>

<svelte:window onkeydown={onKeydown} />

<!-- Capa transparente para cerrar al hacer clic fuera. -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="backdrop" onclick={onClose} oncontextmenu={(e) => { e.preventDefault(); onClose(); }}></div>

<div class="menu" style="left: {x}px; top: {y}px" role="menu">
  {#if connectable}
    {#if credentials.length > 0}
      <button class="item accent" role="menuitem" onclick={() => connect(null)}>
        <Icon icon={icons.connect} size={15} /> Conectar
      </button>
      <div class="sep"></div>
      {#each credentials as c (c.id)}
        <button class="item" role="menuitem" onclick={() => connect(c.id)}>
          <Icon icon={icons.terminal} size={15} /> {credLabel(c)}
        </button>
      {/each}
    {:else}
      <span class="empty">Sin credenciales</span>
    {/if}
    <div class="sep"></div>
  {/if}

  {#if kind !== "zone" && canProbe}
    <button class="item" role="menuitem" onclick={() => { void checkNode(nodeId); onClose(); }}>
      <Icon icon={icons.connect} size={15} /> Comprobar estado
    </button>
    <div class="sep"></div>
  {/if}

  <button class="item danger" role="menuitem" onclick={() => { onDelete(); onClose(); }}>
    <Icon icon={icons.delete} size={15} /> Eliminar nodo
  </button>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 40;
  }
  .menu {
    position: fixed;
    z-index: 41;
    min-width: 12rem;
    padding: 0.25rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    background: var(--karto-color-bg);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    box-shadow: 0 8px 28px rgba(0, 0, 0, 0.45);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.5rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text);
    font-family: var(--karto-font-body);
    font-size: 0.85rem;
    text-align: left;
    cursor: pointer;
    white-space: nowrap;
  }
  .item:hover {
    background: var(--karto-color-surface);
  }
  .item.accent {
    color: var(--karto-color-accent);
  }
  .item.danger {
    color: #f87171;
  }
  .sep {
    height: 1px;
    margin: 0.2rem 0.25rem;
    background: var(--karto-color-border);
  }
  .empty {
    padding: 0.4rem 0.5rem;
    font-size: 0.8rem;
    opacity: 0.5;
  }
</style>
