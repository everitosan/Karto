<script lang="ts">
  // Tab Seguridad: auto-bloqueo, limpieza de portapapeles y cambio de contraseña
  // maestra. Local a la sección Configuración.
  import { Button } from "@karto/ui";
  import PasswordField from "$components/PasswordField.svelte";
  import { vaultUseCases } from "$usecases/vault";
  import { VaultError } from "$domain/vault";
  import { appSettings, updateAppSetting } from "../../appSettings.svelte";
  import { m } from "$paraglide/messages.js";

  let autoLock = $state(appSettings.autoLockMinutes);
  let clipboardClear = $state(appSettings.clipboardClearSeconds);

  async function saveAutoLock() {
    const v = Math.max(0, Math.floor(Number(autoLock) || 0));
    autoLock = v;
    await updateAppSetting("autoLockMinutes", v);
  }

  async function saveClipboard() {
    const v = Math.max(0, Math.floor(Number(clipboardClear) || 0));
    clipboardClear = v;
    await updateAppSetting("clipboardClearSeconds", v);
  }

  let pwCurrent = $state("");
  let pwNew = $state("");
  let pwConfirm = $state("");
  let pwBusy = $state(false);
  let pwMessage = $state<{ kind: "ok" | "err"; text: string } | null>(null);

  async function changePassword() {
    pwMessage = null;
    if (pwNew.length < 1) {
      pwMessage = { kind: "err", text: m.sec_pw_empty() };
      return;
    }
    if (pwNew !== pwConfirm) {
      pwMessage = { kind: "err", text: m.export_error_mismatch() };
      return;
    }
    pwBusy = true;
    try {
      await vaultUseCases.rekey(pwCurrent, pwNew);
      pwCurrent = pwNew = pwConfirm = "";
      pwMessage = { kind: "ok", text: m.sec_pw_updated() };
    } catch (e) {
      const text =
        e instanceof VaultError && e.kind === "wrong-password"
          ? m.sec_pw_wrong()
          : m.sec_pw_error();
      pwMessage = { kind: "err", text };
    } finally {
      pwBusy = false;
    }
  }
</script>

<section class="group">
  <h4>{m.sec_lock_title()}</h4>
  <div class="lock-row">
    <label class="field">
      <span>{m.sec_autolock()}</span>
      <input type="number" min="0" bind:value={autoLock} onchange={saveAutoLock} onblur={saveAutoLock} />
    </label>
    <label class="field">
      <span>{m.sec_clipboard()}</span>
      <input type="number" min="0" bind:value={clipboardClear} onchange={saveClipboard} onblur={saveClipboard} />
    </label>
  </div>
</section>

<section class="group">
  <h4>{m.auth_password_master()}</h4>
  <div class="pw-grid">
    <div class="pw-field">
      <PasswordField label={m.sec_pw_current()} bind:value={pwCurrent} />
    </div>
    <div class="pw-field">
      <PasswordField label={m.sec_pw_new()} bind:value={pwNew} />
    </div>
    <div class="pw-field">
      <PasswordField label={m.sec_pw_confirm_new()} bind:value={pwConfirm} />
    </div>
    <div class="pw-actions">
      <Button variant="secondary" onclick={changePassword} disabled={pwBusy}>
        {pwBusy ? m.sec_changing() : m.sec_change_pw()}
      </Button>
    </div>
  </div>
  {#if pwMessage}
    <p class="msg" class:err={pwMessage.kind === "err"}>{pwMessage.text}</p>
  {/if}
</section>

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
  .lock-row {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--karto-space-4);
  }
  .lock-row .field {
    flex: 0 1 auto;
    min-width: 12rem;
    max-width: none;
    margin-bottom: 0;
  }
  .field {
    display: block;
    margin-bottom: 0.6rem;
    max-width: 32rem;
  }
  .field span {
    display: block;
    font-size: 0.8rem;
    margin-bottom: 0.25rem;
    color: var(--karto-color-text-muted);
  }
  .field input {
    width: 100%;
    padding: 0.4rem 0.5rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
  }
  .pw-grid {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    column-gap: var(--karto-space-4);
    row-gap: var(--karto-space-2);
    margin-bottom: 0.6rem;
  }
  .pw-field {
    flex: 1 1 12rem;
    min-width: 12rem;
  }
  .pw-actions {
    margin-left: auto;
    align-self: center;
    display: flex;
    align-items: center;
  }
  @media (max-width: 1200px) {
    /* Contraseña actual sola arriba; nueva + confirmación juntas; botón abajo. */
    .pw-field:first-child {
      flex-basis: 100%;
    }
    .pw-actions {
      flex-basis: 100%;
      align-self: auto;
      justify-content: flex-end;
    }
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
