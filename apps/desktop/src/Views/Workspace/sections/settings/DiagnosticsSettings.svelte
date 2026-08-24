<script lang="ts">
  // Tab Diagnóstico: nivel del log de soporte + acceso al archivo. Config a nivel
  // de máquina (no del vault): se guarda en el app_store vía comandos Tauri.
  import { Button, Icon, icons } from "@karto/ui";
  import {
    diagnosticsUseCases,
    LOG_LEVELS,
    type LogLevel,
  } from "$usecases/diagnostics";

  let level = $state<LogLevel>("warning");
  let logPath = $state("");
  let message = $state<{ kind: "ok" | "err"; text: string } | null>(null);
  let savingLevel = $state(false);

  // Carga inicial: nivel activo + ruta del log (best-effort, no bloquea la tab).
  $effect(() => {
    void (async () => {
      try {
        level = await diagnosticsUseCases.getLevel();
        logPath = await diagnosticsUseCases.logPath();
      } catch {
        message = { kind: "err", text: "No se pudo leer la configuración del log." };
      }
    })();
  });

  async function changeLevel(next: LogLevel) {
    if (next === level) return;
    const prev = level;
    level = next;
    savingLevel = true;
    message = null;
    try {
      await diagnosticsUseCases.setLevel(next);
      message = { kind: "ok", text: "Nivel del log actualizado." };
    } catch {
      level = prev; // revierte si falla el guardado
      message = { kind: "err", text: "No se pudo cambiar el nivel del log." };
    } finally {
      savingLevel = false;
    }
  }

  async function openFolder() {
    message = null;
    try {
      await diagnosticsUseCases.openLogDir();
    } catch {
      message = { kind: "err", text: "No se pudo abrir la carpeta del log." };
    }
  }

  async function copyPath() {
    if (!logPath) return;
    message = null;
    try {
      await navigator.clipboard.writeText(logPath);
      message = { kind: "ok", text: "Ruta copiada al portapapeles." };
    } catch {
      message = { kind: "err", text: "No se pudo copiar la ruta." };
    }
  }
</script>

<section class="group">
  <h4>Nivel de registro</h4>
  <p class="hint">
    Controla cuánto detalle se guarda en el log de soporte. <strong>Warning</strong> (por defecto)
    registra avisos y errores; <strong>Info</strong> añade más detalle; <strong>Error</strong> deja solo
    los fallos.
  </p>
  <div class="levels" role="radiogroup" aria-label="Nivel de registro">
    {#each LOG_LEVELS as opt (opt.value)}
      <button
        class="level"
        class:active={level === opt.value}
        role="radio"
        aria-checked={level === opt.value}
        disabled={savingLevel}
        onclick={() => changeLevel(opt.value)}
      >
        <span class="level-label">{opt.label}</span>
        <span class="level-hint">{opt.hint}</span>
      </button>
    {/each}
  </div>
</section>

<section class="group">
  <h4>Archivo de log</h4>
  <p class="hint">
    Si tienes un problema, comparte este archivo con soporte. No contiene contraseñas ni direcciones
    (IP/host): solo la información necesaria para identificar el fallo.
  </p>
  {#if logPath}
    <div class="path-wrap">
      <code class="path" title={logPath}>{logPath}</code>
      <button class="path-copy" onclick={copyPath} title="Copiar ruta" aria-label="Copiar ruta">
        <Icon icon={icons.copy} size={15} />
      </button>
    </div>
  {/if}
  <div class="actions">
    <Button variant="secondary" onclick={openFolder}>Abrir carpeta</Button>
  </div>
</section>

{#if message}
  <p class="msg" class:err={message.kind === "err"}>{message.text}</p>
{/if}

<style>
  .group {
    padding: 0.75rem 0 1.25rem;
  }
  .group + .group {
    border-top: 1px solid var(--karto-color-border);
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
  }
  .levels {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }
  .level {
    flex: 1 1 12rem;
    text-align: left;
    padding: 0.6rem 0.75rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
    cursor: pointer;
  }
  .level:hover:not(:disabled) {
    border-color: var(--karto-color-text-muted);
  }
  .level:disabled {
    opacity: 0.6;
    cursor: default;
  }
  .level.active {
    border-color: var(--karto-color-accent);
    box-shadow: inset 0 0 0 1px var(--karto-color-accent);
  }
  .level-label {
    display: block;
    font-size: 0.85rem;
    font-weight: 600;
  }
  .level-hint {
    display: block;
    font-size: 0.72rem;
    color: var(--karto-color-text-muted);
    margin-top: 0.15rem;
  }
  .path-wrap {
    position: relative;
    margin-bottom: 0.6rem;
  }
  .path {
    display: block;
    font-size: 0.75rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    padding: 0.4rem 0.5rem;
    padding-right: 2.25rem;
    overflow-x: auto;
    white-space: nowrap;
    color: var(--karto-color-text-muted);
  }
  .path-copy {
    position: absolute;
    top: 1px;
    right: 1px;
    bottom: 1px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 2rem;
    padding: 0;
    border: none;
    border-left: 1px solid var(--karto-color-border);
    border-radius: 0 var(--karto-radius) var(--karto-radius) 0;
    background: var(--karto-color-surface);
    color: var(--karto-color-text-muted);
    cursor: pointer;
  }
  .path-copy:hover {
    color: var(--karto-color-text);
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
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
