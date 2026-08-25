<script lang="ts">
  // CTA inteligente del hero. Sobre el modelo horneado en build, detecta el SO
  // del visitante al montar y apunta al instalador que le corresponde; si no
  // hay descarga para su SO (o no se pudo detectar), cae a la sección de
  // descargas. Sin red propia: el refresco vive en el panel de descargas.
  import { onMount } from "svelte";
  import { Button } from "@karto/ui";
  import { detectOS, hasDownloads, type ReleaseModel, type OS } from "../lib/releases";

  interface Labels {
    download: string; // "Descargar para"
    fallback: string; // "Ver descargas"
    osNames: Record<OS, string>;
  }

  let { release, labels }: { release: ReleaseModel | null; labels: Labels } =
    $props();

  let os = $state<OS | null>(null);
  onMount(() => {
    os = detectOS(navigator);
  });

  const target = $derived.by(() => {
    if (os && release && hasDownloads(release, os)) {
      return {
        href: release.byOS[os][0].url,
        label: `${labels.download} ${labels.osNames[os]}`,
      };
    }
    return { href: "#descargar", label: labels.fallback };
  });

  function go() {
    location.assign(target.href);
  }
</script>

<span class="track" data-track="download_hero">
  <Button variant="primary" onclick={go}>{target.label}</Button>
</span>

<style>
  .track {
    display: contents;
  }
</style>
