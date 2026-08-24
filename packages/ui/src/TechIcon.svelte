<script lang="ts">
  // Icono de marca (tecnología) vía Devicon a color. `name` es la clase base
  // sin el prefijo `devicon-`, p. ej. "postgresql-plain".
  import "devicon/devicon.min.css";

  interface Props {
    name: string;
    size?: number;
    colored?: boolean;
  }

  let { name, size = 20, colored = true }: Props = $props();

  // Marcas cuyo color oficial en Devicon es negro/casi negro: sobre superficies
  // oscuras se pierden. Para ellas ignoramos `.colored` y dejamos que hereden el
  // color de texto actual (currentColor), quedando nítidas en cualquier tema.
  const DARK_BRANDS = new Set([
    "nextjs",
    "github",
    "flask",
    "express",
    "vercel",
    "prisma",
    "threejs",
    "socketio",
    "rollup",
    "unrealengine",
  ]);

  // Token de marca = parte antes del primer guion ("nextjs-plain" -> "nextjs").
  const brand = $derived(name.split("-")[0]);
  const useColored = $derived(colored && !DARK_BRANDS.has(brand));
</script>

<i
  class="devicon-{name}"
  class:colored={useColored}
  style="font-size: {size}px; line-height: 1;"
  aria-hidden="true"
></i>
