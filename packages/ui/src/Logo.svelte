<script lang="ts">
  // Logo de Karto reconstruido desde /docs (isotipo.svg + Logotipo.svg).
  // Todo usa `currentColor`, así que el prop `color` tiñe iso y texto a la vez.
  interface Props {
    /** Alto del isotipo en px (en `full` también escala el texto). */
    size?: number;
    variant?: "full" | "iso";
    color?: string;
    /** Etiqueta accesible. */
    title?: string;
  }

  let {
    size = 48,
    variant = "full",
    color = "var(--karto-color-accent)",
    title = "Karto",
  }: Props = $props();
</script>

{#snippet iso()}
  <svg
    class="karto-logo__iso"
    viewBox="0 0 103 103"
    fill="none"
    role="img"
    aria-label={title}
  >
    <path
      d="M94.416748046875,51.5C94.416748046875,75.202880859375,75.20263671875,94.416748046875,51.5,94.416748046875C27.79736328125,94.416748046875,8.5833740234375,75.202880859375,8.5833740234375,51.5C8.5833740234375,27.797119140625,27.79736328125,8.5833740234375,51.5,8.5833740234375C75.20263671875,8.5833740234375,94.416748046875,27.797119140625,94.416748046875,51.5"
      fill="none"
      stroke="currentColor"
      stroke-width="9"
      stroke-linejoin="round"
      stroke-linecap="round"
    />
    <path
      d="M51.5,94.416748046875C65.722412109375,94.416748046875,77.25,75.202880859375,77.25,51.5C77.25,27.797119140625,65.722412109375,8.5833740234375,51.5,8.5833740234375M42.9166259765625,62.229248046875C42.9166259765625,65.7845458984375,40.0343017578125,68.666748046875,36.4791259765625,68.666748046875C32.924072265625,68.666748046875,30.0416259765625,65.7845458984375,30.0416259765625,62.229248046875C30.0416259765625,58.673828125,32.924072265625,55.791748046875,36.4791259765625,55.791748046875C40.0343017578125,55.791748046875,42.9166259765625,58.673828125,42.9166259765625,62.229248046875M47.744873046875,30.041748046875L47.208251953125,30.041748046875M48.28125,30.041748046875C48.28125,30.6341552734375,47.80126953125,31.1146240234375,47.208251953125,31.1146240234375C46.615234375,31.1146240234375,46.135498046875,30.6341552734375,46.135498046875,30.041748046875C46.135498046875,29.44921875,46.615234375,28.96875,47.208251953125,28.96875C47.80126953125,28.96875,48.28125,29.44921875,48.28125,30.041748046875"
      fill="none"
      stroke="currentColor"
      stroke-width="4.5"
      stroke-linejoin="round"
      stroke-linecap="round"
    />
  </svg>
{/snippet}

{#if variant === "iso"}
  <span class="karto-logo karto-logo--iso" style="--logo:{size}px; color:{color};">
    {@render iso()}
  </span>
{:else}
  <span class="karto-logo karto-logo--full" style="--logo:{size}px; color:{color};">
    {@render iso()}
    <span class="karto-logo__word">
      <span class="karto-logo__title">| {title}</span>
      <span class="karto-logo__subtitle">Map your infra</span>
    </span>
  </span>
{/if}

<style>
  .karto-logo {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--logo) * 0.14);
    line-height: 1;
  }
  .karto-logo__iso {
    width: var(--logo);
    height: var(--logo);
    flex: 0 0 auto;
    display: block;
  }
  .karto-logo__word {
    display: inline-flex;
    flex-direction: column;
    align-items: flex-end;
  }
  .karto-logo__title {
    font-family: var(--karto-font-title);
    font-weight: 400;
    font-size: calc(var(--logo) * 0.72);
    line-height: 1;
    white-space: nowrap;
  }
  .karto-logo__subtitle {
    font-family: var(--karto-font-body);
    font-weight: 400;
    font-size: calc(var(--logo) * 0.155);
    letter-spacing: -0.02em;
    margin-top: calc(var(--logo) * 0.06);
  }
</style>
