<script lang="ts">
  // Renderiza una cadena Markdown como HTML saneado, con estilos tipográficos
  // consistentes de Karto. Reutilizable: solo recibe el texto fuente.
  import { renderMarkdown } from "./markdown";

  interface Props {
    source: string;
    /** Texto a mostrar cuando `source` está vacío. */
    empty?: string;
  }

  let { source, empty = "" }: Props = $props();

  const html = $derived(renderMarkdown(source));
  const isEmpty = $derived(!source?.trim());
</script>

{#if isEmpty && empty}
  <p class="karto-md karto-md--empty">{empty}</p>
{:else}
  <!-- eslint-disable-next-line svelte/no-at-html-tags -- saneado en renderMarkdown -->
  <div class="karto-md">{@html html}</div>
{/if}

<style>
  .karto-md {
    font-size: 0.85rem;
    line-height: 1.5;
    color: var(--karto-color-text, #e6eaf0);
    word-break: break-word;
  }
  .karto-md--empty {
    opacity: 0.5;
    margin: 0;
  }
  .karto-md :global(h1),
  .karto-md :global(h2),
  .karto-md :global(h3),
  .karto-md :global(h4) {
    margin: 0.6em 0 0.3em;
    line-height: 1.25;
  }
  .karto-md :global(h1) { font-size: 1.15rem; }
  .karto-md :global(h2) { font-size: 1.05rem; }
  .karto-md :global(h3) { font-size: 0.95rem; }
  .karto-md :global(h4) { font-size: 0.88rem; }
  .karto-md :global(p),
  .karto-md :global(ul),
  .karto-md :global(ol),
  .karto-md :global(blockquote),
  .karto-md :global(pre),
  .karto-md :global(table) {
    margin: 0.4em 0;
  }
  .karto-md :global(:first-child) { margin-top: 0; }
  .karto-md :global(:last-child) { margin-bottom: 0; }
  .karto-md :global(ul),
  .karto-md :global(ol) {
    padding-left: 1.25em;
  }
  .karto-md :global(a) {
    color: var(--karto-color-accent, #4ade80);
    text-decoration: underline;
  }
  .karto-md :global(code) {
    font-family: var(--karto-font-mono, ui-monospace, monospace);
    font-size: 0.82em;
    background: var(--karto-color-surface, #12161f);
    padding: 0.1em 0.35em;
    border-radius: 4px;
  }
  .karto-md :global(pre) {
    background: var(--karto-color-surface, #12161f);
    padding: 0.6em 0.75em;
    border-radius: var(--karto-radius, 0.5rem);
    overflow-x: auto;
  }
  .karto-md :global(pre code) {
    background: transparent;
    padding: 0;
  }
  .karto-md :global(blockquote) {
    border-left: 3px solid var(--karto-color-border, #1e2633);
    padding-left: 0.75em;
    color: var(--karto-color-text-muted, #8a93a6);
  }
  .karto-md :global(table) {
    border-collapse: collapse;
    width: 100%;
  }
  .karto-md :global(th),
  .karto-md :global(td) {
    border: 1px solid var(--karto-color-border, #1e2633);
    padding: 0.25em 0.5em;
    text-align: left;
  }
  .karto-md :global(hr) {
    border: 0;
    border-top: 1px solid var(--karto-color-border, #1e2633);
    margin: 0.8em 0;
  }
  .karto-md :global(img) {
    max-width: 100%;
    border-radius: var(--karto-radius, 0.5rem);
  }
</style>
