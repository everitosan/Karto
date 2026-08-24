<script lang="ts">
  // Sección Configuración (página completa, zona 6). Shell de tabs a ancho
  // completo; cada tab es un componente local en `settings/`.
  import AboutSettings from "./settings/AboutSettings.svelte";
  import SecuritySettings from "./settings/SecuritySettings.svelte";
  import BackupSettings from "./settings/BackupSettings.svelte";
  import TemplateSettings from "./settings/TemplateSettings.svelte";
  import ShortcutSettings from "./settings/ShortcutSettings.svelte";
  import DiagnosticsSettings from "./settings/DiagnosticsSettings.svelte";

  type Tab = "about" | "security" | "backups" | "templates" | "shortcuts" | "diagnostics";
  const TABS: { id: Tab; label: string }[] = [
    { id: "about", label: "Acerca de" },
    { id: "security", label: "Seguridad" },
    { id: "backups", label: "Respaldos" },
    { id: "templates", label: "Plantillas" },
    { id: "shortcuts", label: "Atajos" },
    { id: "diagnostics", label: "Diagnóstico" },
  ];

  let active = $state<Tab>("about");
</script>

<div class="settings">
  <header class="head">
    <h2>Configuración</h2>
    <div class="tabs" role="tablist">
      {#each TABS as tab (tab.id)}
        <button
          class="tab"
          class:active={active === tab.id}
          role="tab"
          aria-selected={active === tab.id}
          onclick={() => (active = tab.id)}
        >
          {tab.label}
        </button>
      {/each}
    </div>
  </header>

  <div class="panel">
    {#if active === "about"}
      <AboutSettings />
    {:else if active === "security"}
      <SecuritySettings />
    {:else if active === "backups"}
      <BackupSettings />
    {:else if active === "templates"}
      <TemplateSettings />
    {:else if active === "shortcuts"}
      <ShortcutSettings />
    {:else if active === "diagnostics"}
      <DiagnosticsSettings />
    {/if}
  </div>
</div>

<style>
  .settings {
    flex: 1;
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .head {
    padding: 1.25rem 1.5rem 0;
  }
  h2 {
    margin: 0 0 0.75rem;
    font-family: var(--karto-font-title, inherit);
    color: var(--karto-color-text);
  }
  .tabs {
    display: flex;
    gap: 0.25rem;
    border-bottom: 1px solid var(--karto-color-border);
  }
  .tab {
    padding: 0.5rem 0.9rem;
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -1px;
    color: var(--karto-color-text-muted);
    cursor: pointer;
    font-size: 0.85rem;
  }
  .tab:hover {
    color: var(--karto-color-text);
  }
  .tab.active {
    color: var(--karto-color-text);
    border-bottom-color: var(--karto-color-accent);
  }
  .panel {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 1rem 1.5rem 2rem;
  }
</style>
