<script lang="ts">
  // Pantalla de arranque: logo completo centrado sobre el fondo degradado.
  import { Logo } from "@karto/ui";
</script>

<main class="splash">
  <div class="mark">
    <Logo variant="full" size={64} />
    <p class="by">by evesan</p>
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
    display: grid;
    justify-items: center;
    gap: 0.75rem;
    animation: karto-fade 600ms ease both;
  }
  .by {
    margin: 0;
    font-size: 0.8rem;
    letter-spacing: 0.08em;
    color: var(--karto-color-text-muted);
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
