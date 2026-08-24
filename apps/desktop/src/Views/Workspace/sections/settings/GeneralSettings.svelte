<script lang="ts">
  // Tab General: preferencias de interfaz a nivel de máquina. Por ahora, el
  // idioma. La elección la persiste Paraglide en localStorage (estrategia
  // `localStorage`); por defecto la app sigue al idioma del sistema operativo
  // (ver `$i18n/detect`). Cambiar idioma recarga la ventana para repintar toda la
  // UI en el nuevo idioma (acción poco frecuente, recarga instantánea y robusta).
  import { getLocale, setLocale, locales, type Locale } from "$paraglide/runtime.js";
  import { m } from "$paraglide/messages.js";

  // Endónimos: cada idioma se muestra en su propio nombre, sin traducir.
  const NATIVE_NAMES: Record<Locale, string> = {
    es: "Español",
    en: "English",
  };

  let current = $state<Locale>(getLocale());

  function choose(next: Locale) {
    if (next === current) return;
    current = next;
    setLocale(next); // persiste en localStorage y recarga la ventana
  }
</script>

<section class="group">
  <h4>{m.settings_language()}</h4>
  <p class="hint">{m.settings_language_hint()}</p>
  <div class="langs" role="radiogroup" aria-label={m.settings_language()}>
    {#each locales as loc (loc)}
      <button
        class="lang"
        class:active={current === loc}
        role="radio"
        aria-checked={current === loc}
        onclick={() => choose(loc)}
      >
        <span class="lang-label">{NATIVE_NAMES[loc]}</span>
        <span class="lang-code">{loc.toUpperCase()}</span>
      </button>
    {/each}
  </div>
</section>

<style>
  .group {
    padding: 0.75rem 0 1.25rem;
  }
  h4 {
    margin: 0 0 0.6rem;
    font-size: 0.9rem;
    color: var(--karto-color-text);
  }
  .hint {
    font-size: 0.8rem;
    color: var(--karto-color-text-muted);
    margin: 0 0 0.75rem;
  }
  .langs {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .lang {
    flex: 0 1 10rem;
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 0.5rem;
    text-align: left;
    padding: 0.6rem 0.75rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    cursor: pointer;
  }
  .lang:hover {
    border-color: var(--karto-color-text-muted);
  }
  .lang.active {
    border-color: var(--karto-color-accent);
    box-shadow: inset 0 0 0 1px var(--karto-color-accent);
  }
  .lang-label {
    font-size: 0.85rem;
    font-weight: 600;
  }
  .lang-code {
    font-size: 0.7rem;
    color: var(--karto-color-text-muted);
  }
</style>
