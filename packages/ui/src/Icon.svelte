<script lang="ts">
  // Wrapper sobre HugeIcons (free) con los defaults de Karto: hereda el color
  // del texto (currentColor) y un grosor de trazo consistente.
  import { HugeiconsIcon } from "@hugeicons/svelte";
  import type { IconSvgElement } from "@hugeicons/svelte";

  interface Props {
    icon: IconSvgElement;
    size?: number;
    strokeWidth?: number;
    color?: string;
    class?: string;
  }

  let {
    icon,
    size = 20,
    strokeWidth = 1.8,
    color = "currentColor",
    class: className = "",
  }: Props = $props();
</script>

<HugeiconsIcon {icon} {size} {color} {strokeWidth} class={className} />
