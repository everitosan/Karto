<script lang="ts">
  import type { Snippet } from "svelte";

  type Variant = "primary" | "secondary" | "ghost" | "danger";

  interface Props {
    variant?: Variant;
    disabled?: boolean;
    type?: "button" | "submit" | "reset";
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  }

  let {
    variant = "primary",
    disabled = false,
    type = "button",
    onclick,
    children,
  }: Props = $props();
</script>

<button class="karto-btn karto-btn--{variant}" {type} {disabled} {onclick}>
  {@render children()}
</button>

<style>
  .karto-btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.5rem 1rem;
    border: 1px solid transparent;
    border-radius: var(--karto-radius, 0.5rem);
    font-family: var(--karto-font-body, inherit);
    font-size: 1rem;
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.15s ease, opacity 0.15s ease;
  }
  .karto-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .karto-btn--primary {
    background: var(--karto-color-accent, #11b245);
    color: #06130a;
  }
  .karto-btn--primary:hover:not(:disabled) {
    background: var(--karto-color-accent-hover, #0e9a3b);
  }
  .karto-btn--secondary {
    background: var(--karto-color-surface, #12161f);
    color: var(--karto-color-text, #e6eaf0);
    border-color: var(--karto-color-border, #1e2633);
  }
  .karto-btn--secondary:hover:not(:disabled) {
    background: #1a2130;
  }
  .karto-btn--ghost {
    background: transparent;
    color: var(--karto-color-accent, #11b245);
    border-color: currentColor;
  }
  .karto-btn--danger {
    background: #dc2626;
    color: #fff;
  }
  .karto-btn--danger:hover:not(:disabled) {
    background: #b91c1c;
  }
</style>
