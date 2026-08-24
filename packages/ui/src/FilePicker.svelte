<script lang="ts">
  // Selector de archivo: muestra la ruta elegida en un input de solo lectura y,
  // al hacer clic (en el input o en el botón), lanza el explorador vía `onBrowse`.
  // El componente es presentacional: no conoce Tauri; el consumidor implementa
  // `onBrowse` (p. ej. con el diálogo nativo) y devuelve la ruta o null.
  import Icon from "./Icon.svelte";
  import { icons } from "./icons";

  interface Props {
    /** Ruta seleccionada (bindable). */
    value?: string;
    placeholder?: string;
    disabled?: boolean;
    /** Lanza el explorador y devuelve la ruta elegida (o null si se cancela). */
    onBrowse: () => string | null | Promise<string | null>;
  }

  let {
    value = $bindable(""),
    placeholder = "",
    disabled = false,
    onBrowse,
  }: Props = $props();

  let busy = $state(false);

  async function browse() {
    if (disabled || busy) return;
    busy = true;
    try {
      const path = await onBrowse();
      if (path) value = path;
    } finally {
      busy = false;
    }
  }
</script>

<div class="karto-file" class:disabled>
  <input
    type="text"
    readonly
    {placeholder}
    {value}
    {disabled}
    onclick={browse}
    title={value || placeholder}
  />
  <button type="button" {disabled} onclick={browse} title="Examinar…" aria-label="Examinar">
    <Icon icon={icons.folder} size={16} />
  </button>
</div>

<style>
  .karto-file {
    display: flex;
    gap: var(--karto-space-2, 8px);
    min-width: 0;
  }
  .karto-file.disabled {
    opacity: 0.5;
  }

  input {
    flex: 1;
    min-width: 0;
    background: var(--karto-color-surface, #12161f);
    border: 1px solid var(--karto-color-border, #1e2633);
    border-radius: var(--karto-radius, 0.5rem);
    color: var(--karto-color-text, #e6eaf0);
    padding: 0.45rem 0.6rem;
    font-family: var(--karto-font-body, inherit);
    font-size: 0.9rem;
    cursor: pointer;
    text-overflow: ellipsis;
  }
  input:focus {
    outline: none;
    border-color: var(--karto-color-accent, #11b245);
  }

  button {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 0.6rem;
    background: var(--karto-color-surface, #12161f);
    border: 1px solid var(--karto-color-border, #1e2633);
    border-radius: var(--karto-radius, 0.5rem);
    color: inherit;
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    border-color: var(--karto-color-accent, #11b245);
  }
  button:disabled,
  input:disabled {
    cursor: default;
  }
</style>
