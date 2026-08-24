<script lang="ts">
  // Checkbox circular de Karto: aro con borde de acento que se rellena de verde
  // al marcarse. Envuelve un <input> nativo (accesible, bindable y con foco por
  // teclado) oculto visualmente; el círculo es la representación estilizada.
  import type { Snippet } from "svelte";

  interface Props {
    checked?: boolean;
    disabled?: boolean;
    /** Se invoca con el nuevo estado al cambiar (modo controlado). */
    onchange?: (checked: boolean) => void;
    /** Etiqueta opcional a la derecha del círculo. */
    children?: Snippet;
  }

  let {
    checked = $bindable(false),
    disabled = false,
    onchange,
    children,
  }: Props = $props();

  function handleChange(event: Event) {
    // Soporta tanto `bind:checked` (actualiza el bindable) como modo controlado
    // (el consumidor pasa `checked` de fuera y reacciona vía `onchange`).
    const next = (event.currentTarget as HTMLInputElement).checked;
    checked = next;
    onchange?.(next);
  }
</script>

<label class="karto-check" class:disabled>
  <input type="checkbox" {checked} {disabled} onchange={handleChange} />
  <span class="karto-check__circle" aria-hidden="true"></span>
  {#if children}<span class="karto-check__label">{@render children()}</span>{/if}
</label>

<style>
  .karto-check {
    display: inline-flex;
    align-items: flex-start;
    gap: 0.6rem;
    cursor: pointer;
  }
  .karto-check.disabled {
    opacity: 0.5;
    cursor: default;
  }

  /* Input nativo oculto visualmente pero accesible (foco/teclado/lectores). */
  .karto-check input {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
    white-space: nowrap;
    border: 0;
  }

  .karto-check__circle {
    flex: none;
    width: 1.15rem;
    height: 1.15rem;
    margin-top: 0.05rem;
    border-radius: 50%;
    border: 2px solid var(--karto-color-accent, #11b245);
    background: transparent;
    box-sizing: border-box;
    transition: background-color 0.15s ease, box-shadow 0.15s ease;
  }

  /* Marcado: el círculo se rellena de verde. */
  .karto-check input:checked + .karto-check__circle {
    background: var(--karto-color-accent, #11b245);
  }

  /* Anillo de foco por teclado. */
  .karto-check input:focus-visible + .karto-check__circle {
    box-shadow: 0 0 0 3px color-mix(in srgb, var(--karto-color-accent, #11b245) 35%, transparent);
  }

  .karto-check__label {
    font-family: var(--karto-font-body, inherit);
    color: var(--karto-color-text, #e6eaf0);
  }
</style>
