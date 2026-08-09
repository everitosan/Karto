<script lang="ts">
  import type { Snippet } from "svelte";

  type Tone = "neutral" | "success" | "warning" | "danger";

  interface Props {
    tone?: Tone;
    children: Snippet;
  }

  let { tone = "neutral", children }: Props = $props();
</script>

<span class="karto-badge karto-badge--{tone}">{@render children()}</span>

<style>
  .karto-badge {
    display: inline-flex;
    align-items: center;
    padding: 0.125rem 0.5rem;
    border-radius: 9999px;
    font-size: 0.75rem;
    font-weight: 600;
    line-height: 1.5;
  }
  .karto-badge--neutral {
    background: #e5e7eb;
    color: #374151;
  }
  .karto-badge--success {
    background: #dcfce7;
    color: #166534;
  }
  .karto-badge--warning {
    background: #fef9c3;
    color: #854d0e;
  }
  .karto-badge--danger {
    background: #fee2e2;
    color: #991b1b;
  }
</style>
