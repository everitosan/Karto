<script lang="ts">
  // Pantalla de arranque: logo completo centrado sobre el fondo degradado.
  import { Logo } from "@karto/ui";
</script>

<main class="splash">
  <div class="mark">
    <Logo variant="full" size={64} />
  </div>
</main>

<style>
  .splash {
    height: 100%;
    display: grid;
    place-items: center;
    background: var(--karto-bg);
  }
  .mark {
    animation: karto-fade 600ms ease both;
  }
  @keyframes karto-fade {
    from {
      opacity: 0;
      transform: translateY(6px);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .mark {
      animation: none;
    }
  }
</style>
