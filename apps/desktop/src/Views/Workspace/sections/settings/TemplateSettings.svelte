<script lang="ts">
  // Tab Plantillas: biblioteca de comandos de conexión (a nivel de máquina) y
  // overrides ligados al vault (viajan con el `.karto`).
  import { onMount } from "svelte";
  import { Button } from "@karto/ui";
  import { templatesUseCases, type Template, type VaultTemplate } from "$usecases/templates";
  import { m } from "$paraglide/messages.js";

  let templates = $state<Template[]>([]);
  let linked = $state<VaultTemplate[]>([]);
  let tplName = $state("");
  let tplConnection = $state("ssh");
  let tplCommand = $state("ssh -i {key} -p {port} {userhost}");

  async function load() {
    templates = await templatesUseCases.list().catch(() => []);
    linked = await templatesUseCases.vaultList().catch(() => []);
  }

  onMount(load);

  function linkedFor(connection: string): VaultTemplate | undefined {
    return linked.find((l) => l.connection === connection);
  }

  async function addTemplate() {
    if (!tplName.trim() || !tplCommand.trim()) return;
    await templatesUseCases.upsert({
      name: tplName.trim(),
      connection: tplConnection,
      command: tplCommand.trim(),
    });
    tplName = "";
    await load();
  }

  async function removeTemplate(id: string) {
    await templatesUseCases.remove(id);
    await load();
  }

  async function linkTemplate(id: string) {
    await templatesUseCases.linkToVault(id);
    await load();
  }

  async function unlinkTemplate(connection: string) {
    await templatesUseCases.unlink(connection);
    await load();
  }
</script>

<section class="group">
  <p class="hint">
    {m.tpl_hint_placeholders()}: <code>{"{host}"}</code> <code>{"{port}"}</code>
    <code>{"{user}"}</code> <code>{"{key}"}</code> <code>{"{userhost}"}</code>. {@html m.tpl_hint_rest()}
  </p>

  <ul class="tpl-list">
    {#each templates as tpl (tpl.id)}
      {@const link = linkedFor(tpl.connection)}
      {@const isLinked = link?.command === tpl.command}
      <li class="tpl">
        <div class="tpl-text">
          <span class="tpl-name">
            {tpl.name}
            <span class="tag">{tpl.connection}</span>
            {#if isLinked}<span class="tag linked">{m.tpl_linked_tag()}</span>{/if}
          </span>
          <code class="tpl-cmd">{tpl.command}</code>
        </div>
        <div class="tpl-actions">
          <Button variant="secondary" onclick={() => linkTemplate(tpl.id)}>
            {isLinked ? m.tpl_replace() : m.tpl_link()}
          </Button>
          {#if !tpl.isDefault}
            <button class="link-btn" onclick={() => removeTemplate(tpl.id)}>{m.common_delete()}</button>
          {/if}
        </div>
      </li>
    {/each}
  </ul>

  {#if linked.length > 0}
    <h4>{m.tpl_linked_heading()}</h4>
    <ul class="tpl-list">
      {#each linked as l (l.connection)}
        <li class="tpl">
          <div class="tpl-text">
            <span class="tpl-name"><span class="tag">{l.connection}</span></span>
            <code class="tpl-cmd">{l.command}</code>
          </div>
          <button class="link-btn" onclick={() => unlinkTemplate(l.connection)}>{m.tpl_unlink()}</button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="tpl-form">
    <input class="tpl-input" placeholder={m.common_name()} bind:value={tplName} />
    <select class="tpl-input sel" bind:value={tplConnection}>
      <option value="ssh">ssh</option>
      <option value="vnc">vnc</option>
      <option value="web">web</option>
      <option value="rdp">rdp</option>
    </select>
    <input class="tpl-input cmd" placeholder={m.tpl_command_placeholder()} bind:value={tplCommand} />
    <Button onclick={addTemplate} disabled={!tplName.trim() || !tplCommand.trim()}>{m.common_add()}</Button>
  </div>
</section>

<style>
  .group {
    padding: 0.75rem 0;
  }
  h4 {
    margin: 1rem 0 0.5rem;
    font-size: 0.9rem;
    color: var(--karto-color-text);
  }
  .hint {
    font-size: 0.8rem;
    color: var(--karto-color-text-muted);
    margin: 0 0 0.75rem;
    max-width: 48rem;
  }
  .hint :global(code) {
    font-size: 0.72rem;
    background: var(--karto-color-surface);
    padding: 0 0.2rem;
    border-radius: 3px;
  }
  .tpl-list {
    list-style: none;
    margin: 0.5rem 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .tpl {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.5rem 0.6rem;
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
  }
  .tpl-text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .tpl-name {
    font-size: 0.85rem;
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .tpl-cmd {
    font-size: 0.72rem;
    color: var(--karto-color-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tag {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.03em;
    padding: 0.05rem 0.35rem;
    border-radius: 999px;
    background: var(--karto-color-surface);
    color: var(--karto-color-text-muted);
  }
  .tag.linked {
    background: var(--karto-color-accent);
    color: #04140a;
  }
  .tpl-actions {
    display: flex;
    align-items: center;
    gap: 0.4rem;
  }
  .link-btn {
    background: none;
    border: none;
    color: var(--karto-color-text-muted);
    cursor: pointer;
    font-size: 0.78rem;
    padding: 0.2rem 0.3rem;
  }
  .link-btn:hover {
    color: #ff6b6b;
  }
  .tpl-form {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    margin-top: 0.75rem;
  }
  .tpl-input {
    padding: 0.4rem 0.5rem;
    background: var(--karto-color-surface);
    border: 1px solid var(--karto-color-border);
    border-radius: var(--karto-radius);
    color: var(--karto-color-text);
  }
  .tpl-input.cmd {
    flex: 1;
    min-width: 12rem;
    font-family: var(--karto-font-mono, monospace);
    font-size: 0.8rem;
  }
  .tpl-input.sel {
    flex: none;
  }
</style>
