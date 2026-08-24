<script lang="ts">
  // Tab Respaldos: copia de respaldo cifrada del vault.
  import { Button } from "@karto/ui";
  import { vaultUseCases } from "$usecases/vault";
  import { pickBackupPath } from "$usecases/dialog";

  let backupBusy = $state(false);
  let backupMessage = $state<{ kind: "ok" | "err"; text: string } | null>(null);

  async function exportBackup() {
    backupMessage = null;
    const dest = await pickBackupPath();
    if (!dest) return;
    backupBusy = true;
    try {
      await vaultUseCases.exportBackup(dest);
      backupMessage = { kind: "ok", text: "Copia guardada correctamente." };
    } catch {
      backupMessage = { kind: "err", text: "No se pudo escribir la copia (¿el archivo ya existe?)." };
    } finally {
      backupBusy = false;
    }
  }
</script>

<section class="group">
  <h4>Copia de respaldo cifrada</h4>
  <div class="row">
    <p class="hint">
      Genera un archivo <code>.karto</code> cifrado con la contraseña actual. Guárdalo en un lugar
      seguro: si olvidas la contraseña, los datos son irrecuperables.
    </p>
    <Button variant="secondary" onclick={exportBackup} disabled={backupBusy}>
      {backupBusy ? "Exportando…" : "Exportar copia cifrada…"}
    </Button>
  </div>
  {#if backupMessage}
    <p class="msg" class:err={backupMessage.kind === "err"}>{backupMessage.text}</p>
  {/if}
</section>

<style>
  .group {
    padding: 0.75rem 0;
  }
  h4 {
    margin: 0 0 0.6rem;
    font-size: 0.9rem;
    color: var(--karto-color-text);
  }
  .row {
    display: flex;
    align-items: center;
    gap: 1.5rem;
  }
  .row .hint {
    flex: 1;
    min-width: 0;
    margin: 0;
  }
  .row :global(button) {
    flex: none;
    white-space: nowrap;
  }
  .hint {
    font-size: 0.8rem;
    color: var(--karto-color-text-muted);
    margin: 0 0 0.5rem;
  }
  .hint code {
    font-size: 0.72rem;
    background: var(--karto-color-surface);
    padding: 0 0.2rem;
    border-radius: 3px;
  }
  .msg {
    font-size: 0.8rem;
    margin: 0 0 0.5rem;
    color: var(--karto-color-accent);
  }
  .msg.err {
    color: #ff6b6b;
  }
</style>
