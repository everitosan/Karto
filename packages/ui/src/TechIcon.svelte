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
</script>

<i
  class="devicon-{name}"
  class:colored
  style="font-size: {size}px; line-height: 1;"
  aria-hidden="true"
></i>
