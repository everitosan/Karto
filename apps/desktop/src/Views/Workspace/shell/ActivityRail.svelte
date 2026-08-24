<script lang="ts">
  // Barra vertical izquierda (zonas 4, 5, 6). Conmuta la sección activa del
  // workspace. Diagramas y Scripts arriba; Configuración abajo (spacer flexible).
  import { Icon, icons } from "@karto/ui";
  import { sectionState, setSection, type Section } from "../section.svelte";
  import { m } from "$paraglide/messages.js";

  const top: { id: Section; icon: keyof typeof icons; label: string }[] = [
    { id: "diagrams", icon: "diagram", label: m.nav_diagrams() },
    { id: "scripts", icon: "script", label: m.nav_scripts() },
  ];
</script>

<nav class="rail" aria-label={m.nav_sections()}>
  <div class="group">
    {#each top as item (item.id)}
      <button
        class="rail-btn"
        class:active={sectionState.active === item.id}
        title={item.label}
        aria-current={sectionState.active === item.id ? "page" : undefined}
        onclick={() => setSection(item.id)}
      >
        <Icon icon={icons[item.icon]} size={20} />
      </button>
    {/each}
  </div>

  <div class="group bottom">
    <button
      class="rail-btn"
      class:active={sectionState.active === "config"}
      title={m.nav_config()}
      aria-current={sectionState.active === "config" ? "page" : undefined}
      onclick={() => setSection("config")}
    >
      <Icon icon={icons.settings} size={20} />
    </button>
  </div>
</nav>

<style>
  .rail {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    width: 3rem;
    padding: 0.5rem 0;
    border-right: 1px solid var(--karto-color-border);
    background: var(--karto-color-bg, transparent);
  }
  .group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }
  .rail-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 2.25rem;
    height: 2.25rem;
    border: 0;
    border-radius: var(--karto-radius);
    background: transparent;
    color: var(--karto-color-text-muted);
    cursor: pointer;
    position: relative;
  }
  .rail-btn:hover {
    background: var(--karto-color-surface);
    color: var(--karto-color-text);
  }
  .rail-btn.active {
    color: var(--karto-color-accent);
  }
  /* Indicador de sección activa: barra de acento a la izquierda. */
  .rail-btn.active::before {
    content: "";
    position: absolute;
    left: -0.5rem;
    top: 0.4rem;
    bottom: 0.4rem;
    width: 2px;
    border-radius: 1px;
    background: var(--karto-color-accent);
  }
</style>
