<script lang="ts">
  // Tab Atajos: referencia de atajos de teclado agrupados por sección (contexto).
  // Son comportamientos reales de la app; aquí solo se documentan.
  interface Shortcut {
    keys: string[];
    action: string;
  }
  interface SectionShortcuts {
    section: string;
    shortcuts: Shortcut[];
  }

  const SECTIONS: SectionShortcuts[] = [
    {
      section: "Diagrama",
      shortcuts: [
        { keys: ["Supr", "Retroceso"], action: "Eliminar el nodo o la conexión seleccionada" },
        { keys: ["Esc"], action: "Cerrar el menú contextual" },
      ],
    },
  ];
</script>

<div class="atajos">
  {#each SECTIONS as group (group.section)}
    <section class="group">
      <h4>{group.section}</h4>
      <ul class="list">
        {#each group.shortcuts as sc (sc.action)}
          <li class="row">
            <span class="keys">
              {#each sc.keys as k, i (k)}
                {#if i > 0}<span class="sep">o</span>{/if}
                <kbd>{k}</kbd>
              {/each}
            </span>
            <span class="action">{sc.action}</span>
          </li>
        {/each}
      </ul>
    </section>
  {/each}
</div>

<style>
  .group {
    padding: 0.75rem 0;
  }
  h4 {
    margin: 0 0 0.6rem;
    font-size: 0.9rem;
    color: var(--karto-color-text);
  }
  .list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 1rem;
  }
  .keys {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    flex: none;
    min-width: 12rem;
  }
  kbd {
    font-family: var(--karto-font-mono, monospace);
    font-size: 0.72rem;
    padding: 0.1rem 0.4rem;
    border: 1px solid var(--karto-color-border);
    border-bottom-width: 2px;
    border-radius: 4px;
    background: var(--karto-color-surface);
    color: var(--karto-color-text);
  }
  .sep {
    font-size: 0.7rem;
    color: var(--karto-color-text-muted);
  }
  .action {
    font-size: 0.85rem;
    color: var(--karto-color-text-muted);
  }
</style>
