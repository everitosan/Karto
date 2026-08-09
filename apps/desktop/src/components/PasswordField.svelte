<script lang="ts">
  // Componente de presentación compartido entre las vistas Welcome y Unlock.
  import { Icon, icons } from "@karto/ui";

  interface Props {
    label: string;
    value: string;
    autofocus?: boolean;
  }

  let { label, value = $bindable(""), autofocus = false }: Props = $props();
  let reveal = $state(false);
</script>

<label class="field">
  <span>{label}</span>
  <div class="row">
    <!-- svelte-ignore a11y_autofocus -->
    <input
      type={reveal ? "text" : "password"}
      bind:value
      {autofocus}
      autocomplete="off"
      spellcheck="false"
    />
    <button type="button" onclick={() => (reveal = !reveal)} aria-label="Mostrar/ocultar">
      <Icon icon={reveal ? icons.eyeOff : icons.eye} size={18} />
    </button>
  </div>
</label>

<style>
  .field {
    display: block;
    margin-bottom: 0.75rem;
  }
  .field span {
    display: block;
    font-size: 0.8rem;
    margin-bottom: 0.25rem;
    opacity: 0.8;
  }
  .row {
    display: flex;
    gap: 0.5rem;
  }
  input {
    flex: 1;
    padding: 0.5rem 0.75rem;
    border-radius: var(--karto-radius);
    border: 1px solid var(--karto-color-border);
    background: #0b0f16;
    color: inherit;
    font-family: var(--karto-font-body);
    font-size: 1rem;
  }
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0 0.75rem;
    border-radius: var(--karto-radius);
    border: 1px solid var(--karto-color-border);
    background: #0b0f16;
    color: var(--karto-color-text-muted);
    cursor: pointer;
  }
  button:hover {
    color: var(--karto-color-text);
  }
</style>
