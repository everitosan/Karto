<script lang="ts">
  // Tab "Acerca de": branding del autor (evesan), versión, enlaces externos,
  // apoyo (donaciones) y datos legales. Los enlaces se abren en el navegador
  // del sistema vía backend (esquemas http/https).
  import { Logo, Icon, TechIcon, icons } from "@karto/ui";
  import { getVersion } from "@tauri-apps/api/app";
  import { aboutUseCases } from "$usecases/about";
  import { REPO_URL, DONATE_URL } from "$config/links";
  import { m } from "$paraglide/messages.js";

  type LinkItem =
    | { kind: "tech"; name: string; label: string; url: string }
    | { kind: "icon"; icon: typeof icons.link; label: string; url: string };

  const LINKS: LinkItem[] = [
    {
      kind: "tech",
      name: "github-original",
      label: "GitHub",
      url: "https://github.com/everitosan",
    },
    {
      kind: "tech",
      name: "linkedin-plain",
      label: "LinkedIn",
      url: "https://www.linkedin.com/in/everitosan/",
    },
    {
      kind: "icon",
      icon: icons.link,
      label: "evesan.rocks",
      url: "https://evesan.rocks/",
    },
    {
      kind: "icon",
      icon: icons.bug,
      label: m.about_report_issue(),
      url: `${REPO_URL}/issues`,
    },
  ];

  let version = $state("");
  let error = $state<string | null>(null);

  // Versión real de la app (Tauri); best-effort, no bloquea la tab.
  $effect(() => {
    void getVersion()
      .then((v) => (version = v))
      .catch(() => {});
  });

  async function open(url: string) {
    error = null;
    try {
      await aboutUseCases.openExternalUrl(url);
    } catch {
      error = m.about_open_error();
    }
  }
</script>

<div class="about">
<section class="group">
  <div class="brand">
    <Logo variant="full" size={40} />
    {#if version}
      <span class="version">v{version}</span>
    {/if}
    <p class="by">
      {m.about_made_by()} <strong>evesan</strong>
    </p>
  </div>
</section>

<section class="group support">
  <h4>{m.about_support_title()}</h4>
  <p class="hint">
    {m.about_opensource_before()}<button
      class="linktext"
      onclick={() => open(`${REPO_URL}/blob/main/LICENSE`)}>{m.about_license_link()}</button
    >{m.about_opensource_after()}
  </p>
  <p class="hint"> {m.about_support_hint()} </p>
  <button class="coffee" onclick={() => open(DONATE_URL)} title={DONATE_URL}>
    <Icon icon={icons.coffee} size={18} />
    <span>{m.topbar_donate()}</span>
  </button>
</section>


<section class="group bottom">
  <h4>{m.about_links()}</h4>
  <div class="links">
    {#each LINKS as item (item.url)}
      <button class="link" onclick={() => open(item.url)} title={item.url}>
        {#if item.kind === "tech"}
          <TechIcon name={item.name} size={20} colored={false} />
        {:else}
          <Icon icon={item.icon} size={18} />
        {/if}
        <span class="link-label">{item.label}</span>
        <Icon icon={icons.chevron} size={14} />
      </button>
    {/each}
  </div>
  {#if error}
    <p class="msg err">{error}</p>
  {/if}
</section>

<section class="group legal">
  <p class="legal-line muted">© {new Date().getFullYear()} evesan</p>
</section>
</div>

<style>
  .about {
    display: flex;
    flex-direction: column;
    min-height: 100%;
  }
  .bottom {
    margin-top: auto;
  }
  .group {
    padding: 0.75rem 0 1.25rem;
  }
  .group + .group {
    border-top: 1px solid var(--karto-color-border);
  }
  .brand {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.4rem;
    text-align: center;
  }
  .version {
    font-size: 0.75rem;
    color: var(--karto-color-text-muted);
    padding: 0.1rem 0.4rem;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
  }
  .by {
    font-size: 0.85rem;
    color: var(--karto-color-text-muted);
    margin: 0;
  }
  .by strong {
    color: var(--karto-color-text);
  }
  h4 {
    margin: 0 0 0.6rem;
    font-size: 0.9rem;
    color: var(--karto-color-text);
  }
  .hint {
    font-size: 0.8rem;
    color: var(--karto-color-text-muted);
    margin: 0 0 0.75rem;
    /* max-width: 30rem; */
  }
  .support {
    display: flex;
    flex-direction: column;
    /* align-items: center; */
    /* text-align: center; */
  }
  .coffee {
    position: relative;
    overflow: hidden;
    isolation: isolate;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.6rem 1.1rem;
    background: var(--karto-color-accent);
    border: none;
    border-radius: var(--karto-radius);
    color: #fff;
    font-size: 0.85rem;
    font-weight: 600;
    cursor: pointer;
  }
  /* Contenido siempre por encima de las capas de partículas. */
  .coffee > span,
  .coffee :global(svg) {
    position: relative;
    z-index: 1;
  }
  /* Dos campos de estrellas tenues que derivan a distinta velocidad
     (efecto parallax espacial) y titilan suavemente. */
  .coffee::before,
  .coffee::after {
    content: "";
    position: absolute;
    inset: -100%;
    pointer-events: none;
    z-index: 0;
    background-repeat: repeat;
    background-size: 120px 120px;
  }
  .coffee::before {
    background-image:
      radial-gradient(1.2px 1.2px at 20% 30%, rgba(4, 52, 21, 0.95), transparent),
      radial-gradient(1px 1px at 70% 60%, rgba(4, 52, 21, 0.8), transparent),
      radial-gradient(1.5px 1.5px at 45% 85%, rgba(4, 52, 21, 0.7), transparent),
      radial-gradient(1px 1px at 90% 20%, rgba(4, 52, 21, 0.9), transparent);
    opacity: 0.95;
    animation:
      drift 14s linear infinite,
      twinkle 3.5s ease-in-out infinite alternate;
  }
  .coffee::after {
    background-image:
      radial-gradient(1px 1px at 15% 65%, rgba(4, 52, 21, 0.8), transparent),
      radial-gradient(1px 1px at 55% 25%, rgba(4, 52, 21, 0.7), transparent),
      radial-gradient(1.3px 1.3px at 80% 75%, rgba(4, 52, 21, 0.85), transparent);
    opacity: 0.7;
    animation:
      drift 26s linear infinite reverse,
      twinkle 5s ease-in-out infinite alternate;
  }
  .coffee:hover {
    filter: brightness(1.08);
  }
  @keyframes drift {
    from {
      transform: translate3d(0, 0, 0);
    }
    to {
      transform: translate3d(-120px, -60px, 0);
    }
  }
  @keyframes twinkle {
    from {
      opacity: 0.6;
    }
    to {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .coffee::before,
    .coffee::after {
      animation: none;
    }
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 0.5rem;
  }
  .link {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.6rem 0.75rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    cursor: pointer;
    text-align: left;
  }
  .link:hover {
    border-color: var(--karto-color-text-muted);
  }
  .link-label {
    font-size: 0.85rem;
  }
  .legal {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.25rem;
  }
  .legal-line {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.78rem;
    color: var(--karto-color-text-muted);
    margin: 0;
  }
  .legal-line.muted {
    color: var(--karto-color-text-muted);
    opacity: 0.75;
  }
  .linktext {
    background: none;
    border: none;
    padding: 0;
    color: var(--karto-color-accent);
    font-size: 0.78rem;
    cursor: pointer;
  }
  .linktext:hover {
    text-decoration: underline;
  }
  .msg {
    font-size: 0.8rem;
    margin: 0.5rem 0 0;
    color: var(--karto-color-accent);
  }
  .msg.err {
    color: #ff6b6b;
  }
</style>
