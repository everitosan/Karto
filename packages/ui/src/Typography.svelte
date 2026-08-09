<script lang="ts" module>
  export type TypographyVariant =
    | "display"
    | "h1"
    | "h2"
    | "h3"
    | "title"
    | "subtitle"
    | "body"
    | "body-sm"
    | "caption"
    | "label";

  export type TypographyColor = "default" | "muted" | "accent";

  // Etiqueta HTML por defecto para cada variante.
  const DEFAULT_TAG: Record<TypographyVariant, string> = {
    display: "h1",
    h1: "h1",
    h2: "h2",
    h3: "h3",
    title: "p",
    subtitle: "p",
    body: "p",
    "body-sm": "p",
    caption: "span",
    label: "span",
  };
</script>

<script lang="ts">
  import type { Snippet } from "svelte";

  interface Props {
    variant?: TypographyVariant;
    /** Sobrescribe la etiqueta HTML (p. ej. usar h2 con estilo de "title"). */
    as?: string;
    color?: TypographyColor;
    align?: "left" | "center" | "right";
    children: Snippet;
  }

  let {
    variant = "body",
    as = undefined,
    color = "default",
    align = "left",
    children,
  }: Props = $props();

  const tag = $derived(as ?? DEFAULT_TAG[variant]);
</script>

<svelte:element
  this={tag}
  class="karto-t karto-t--{variant} karto-t--{color}"
  style="text-align: {align};"
>
  {@render children()}
</svelte:element>

<style>
  .karto-t {
    margin: 0;
    font-family: var(--karto-font-body);
    color: var(--karto-color-text);
  }

  /* Colores */
  .karto-t--muted {
    color: var(--karto-color-text-muted);
  }
  .karto-t--accent {
    color: var(--karto-color-accent);
  }

  /* Títulos → Titillium Web */
  .karto-t--display,
  .karto-t--h1,
  .karto-t--h2,
  .karto-t--h3,
  .karto-t--title {
    font-family: var(--karto-font-title);
  }

  .karto-t--display {
    font-size: 3rem;
    font-weight: 700;
    line-height: 1.05;
    letter-spacing: -0.01em;
  }
  .karto-t--h1 {
    font-size: 2rem;
    font-weight: 700;
    line-height: 1.15;
  }
  .karto-t--h2 {
    font-size: 1.5rem;
    font-weight: 600;
    line-height: 1.2;
  }
  .karto-t--h3 {
    font-size: 1.25rem;
    font-weight: 600;
    line-height: 1.25;
  }
  .karto-t--title {
    font-size: 1.125rem;
    font-weight: 600;
    line-height: 1.3;
  }

  /* Texto → Ubuntu Sans */
  .karto-t--subtitle {
    font-size: 1rem;
    font-weight: 400;
    line-height: 1.5;
    color: var(--karto-color-text-muted);
  }
  .karto-t--body {
    font-size: 1rem;
    font-weight: 400;
    line-height: 1.55;
  }
  .karto-t--body-sm {
    font-size: 0.875rem;
    font-weight: 400;
    line-height: 1.5;
  }
  .karto-t--caption {
    font-size: 0.75rem;
    font-weight: 400;
    line-height: 1.4;
    color: var(--karto-color-text-muted);
  }
  .karto-t--label {
    font-size: 0.8rem;
    font-weight: 500;
    line-height: 1.4;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
</style>
