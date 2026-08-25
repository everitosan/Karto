<script lang="ts">
  // Selector de descargas. Recibe el release horneado en build (base sólida,
  // sin spinner) y, al montar, refresca en segundo plano contra la API de
  // GitHub por si hubiera uno más nuevo (enfoque híbrido). Resalta el SO del
  // visitante y ofrece un instalador por sistema, marcando los que aún no
  // existen como "próximamente".
  import { onMount } from "svelte";
  import {
    detectOS,
    fetchReleaseModel,
    isNewer,
    OSES,
    type ReleaseModel,
    type OS,
  } from "../lib/releases";

  interface Labels {
    versionLabel: string;
    prerelease: string;
    yourSystem: string;
    comingSoon: string;
    viewAll: string;
    errorTitle: string;
    errorCta: string;
    osNames: Record<OS, string>;
    formatHints: Record<string, string>;
  }

  let { initial, labels }: { initial: ReleaseModel | null; labels: Labels } =
    $props();

  let release = $state<ReleaseModel | null>(initial);
  let os = $state<OS | null>(null);

  onMount(() => {
    os = detectOS(navigator);
    // Refresco híbrido: si GitHub tiene algo más nuevo, lo adoptamos.
    fetchReleaseModel().then((fresh) => {
      if (isNewer(release, fresh)) release = fresh;
    });
  });

  // El SO detectado se muestra primero; el resto conserva su orden.
  const ordered = $derived.by<OS[]>(() =>
    os ? [os, ...OSES.filter((o) => o !== os)] : [...OSES],
  );

  function fmtSize(bytes: number): string {
    if (!bytes) return "";
    const mb = bytes / (1024 * 1024);
    return `${mb.toFixed(mb >= 100 ? 0 : 1)} MB`;
  }

  function fmtDate(iso: string): string {
    if (!iso) return "";
    const d = new Date(iso);
    return Number.isNaN(d.getTime())
      ? ""
      : d.toLocaleDateString(undefined, {
          year: "numeric",
          month: "short",
          day: "numeric",
        });
  }
</script>

{#if !release}
  <div class="panel panel--error">
    <p>{labels.errorTitle}</p>
    <a
      class="all-link"
      href="https://github.com/everitosan/Karto/releases"
      data-track="download_error_github">{labels.errorCta}</a
    >
  </div>
{:else}
  <div class="panel">
    <div class="panel-head">
      <span class="version">{labels.versionLabel} {release.tag}</span>
      {#if release.prerelease}
        <span class="tag-rc">{labels.prerelease}</span>
      {/if}
      {#if fmtDate(release.publishedAt)}
        <span class="pubdate">· {fmtDate(release.publishedAt)}</span>
      {/if}
    </div>

    <div class="os-grid">
      {#each ordered as osKey (osKey)}
        {@const assets = release.byOS[osKey]}
        <section class="os-col" class:is-mine={os === osKey}>
          <header class="os-col-head">
            <h3>{labels.osNames[osKey]}</h3>
            {#if os === osKey}
              <span class="chip-mine">{labels.yourSystem}</span>
            {/if}
          </header>

          {#if assets.length}
            <ul class="asset-list">
              {#each assets as a (a.url)}
                <li>
                  <a
                    class="asset"
                    href={a.url}
                    data-track="download_asset"
                    data-os={a.os}
                    data-format={a.format}
                  >
                    <span class="asset-main">
                      <span class="asset-format">{a.format}</span>
                      {#if a.arch}<span class="asset-arch">{a.arch}</span>{/if}
                    </span>
                    <span class="asset-meta">
                      {#if labels.formatHints[a.format]}
                        <span class="asset-hint">{labels.formatHints[a.format]}</span>
                      {/if}
                      {#if fmtSize(a.size)}
                        <span class="asset-size">{fmtSize(a.size)}</span>
                      {/if}
                    </span>
                  </a>
                </li>
              {/each}
            </ul>
          {:else}
            <p class="soon">{labels.comingSoon}</p>
          {/if}
        </section>
      {/each}
    </div>

    <a class="all-link" href={release.htmlUrl} data-track="download_view_all"
      >{labels.viewAll}</a
    >
  </div>
{/if}

<style>
  .panel {
    width: 100%;
    max-width: 42rem;
    margin: var(--karto-space-4) auto 0;
    text-align: left;
  }
  .panel--error {
    text-align: center;
    color: var(--karto-color-text-muted);
  }

  .panel-head {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: var(--karto-space-2);
    margin-bottom: var(--karto-space-5);
    font-size: 0.85rem;
  }
  .version {
    font-family: var(--karto-font-title);
    font-weight: 600;
    color: var(--karto-color-text);
  }
  .tag-rc {
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: #854d0e;
    background: #fef9c3;
    padding: 0.1em 0.55em;
    border-radius: 99px;
  }
  .pubdate {
    color: var(--karto-color-text-muted);
  }

  .os-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--karto-space-4);
  }
  @media (max-width: 40rem) {
    .os-grid {
      grid-template-columns: 1fr;
    }
  }

  .os-col {
    border: 1px solid var(--karto-color-border);
    border-radius: calc(var(--karto-radius) * 1.5);
    background: rgba(18, 22, 31, 0.4);
    padding: var(--karto-space-4);
    display: flex;
    flex-direction: column;
    gap: var(--karto-space-3);
  }
  .os-col.is-mine {
    border-color: rgba(17, 178, 69, 0.45);
    box-shadow: 0 0 24px rgba(17, 178, 69, 0.1);
  }
  .os-col-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--karto-space-2);
  }
  .os-col-head h3 {
    font-family: var(--karto-font-title);
    font-weight: 600;
    font-size: 1rem;
    margin: 0;
  }
  .chip-mine {
    font-size: 0.62rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--karto-color-accent);
    border: 1px solid rgba(17, 178, 69, 0.4);
    border-radius: 99px;
    padding: 0.1em 0.5em;
    white-space: nowrap;
  }

  .asset-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: var(--karto-space-2);
  }
  .asset {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: var(--karto-space-3);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    background: var(--karto-color-surface);
    text-decoration: none;
    color: var(--karto-color-text);
    transition: border-color 0.15s, background 0.15s;
  }
  .asset:hover {
    border-color: var(--karto-color-accent);
    background: #1a2130;
  }
  .asset-main {
    display: flex;
    align-items: baseline;
    gap: var(--karto-space-2);
  }
  .asset-format {
    font-weight: 600;
  }
  .asset-arch {
    font-size: 0.7rem;
    color: var(--karto-color-text-muted);
  }
  .asset-meta {
    display: flex;
    justify-content: space-between;
    gap: var(--karto-space-2);
    font-size: 0.75rem;
    color: var(--karto-color-text-muted);
  }
  .soon {
    margin: 0;
    font-size: 0.85rem;
    color: var(--karto-color-text-muted);
    opacity: 0.75;
  }

  .all-link {
    display: block;
    text-align: center;
    margin-top: var(--karto-space-5);
    font-size: 0.85rem;
    color: var(--karto-color-text-muted);
    text-decoration: none;
    border-bottom: none;
  }
  .all-link:hover {
    color: var(--karto-color-accent);
  }
</style>
