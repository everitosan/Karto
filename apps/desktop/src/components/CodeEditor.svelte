<script lang="ts">
  // Editor de código (CodeMirror 6) con resaltado de shell. Envuelve el editor en
  // un componente controlado por `value` (bindable): los cambios del usuario se
  // reflejan en `value` y los cambios externos (cargar otro script) se empujan al
  // editor. El tema se funde con los tokens de color de la app (fondo transparente)
  // reutilizando solo el `highlightStyle` de one-dark para los colores de sintaxis.
  import { onMount } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { EditorState, Compartment, type Extension } from "@codemirror/state";
  import { StreamLanguage, syntaxHighlighting } from "@codemirror/language";
  import { shell } from "@codemirror/legacy-modes/mode/shell";
  import { sql } from "@codemirror/lang-sql";
  import { python } from "@codemirror/lang-python";
  import { javascript } from "@codemirror/lang-javascript";
  import { oneDarkHighlightStyle } from "@codemirror/theme-one-dark";

  /** Lenguaje del resaltado. `plain` = sin resaltado de lenguaje. */
  export type CodeLanguage = "shell" | "python" | "sql" | "javascript" | "plain";

  let { value = $bindable(""), language = "shell" }: {
    value?: string;
    language?: CodeLanguage;
  } = $props();

  let host: HTMLDivElement;
  let view: EditorView | undefined;

  // Extensión de lenguaje intercambiable en caliente (sin recrear el editor).
  const langCompartment = new Compartment();
  function languageExt(lang: CodeLanguage): Extension {
    switch (lang) {
      case "python":
        return python();
      case "sql":
        return sql();
      case "javascript":
        return javascript();
      case "plain":
        return [];
      default:
        return StreamLanguage.define(shell);
    }
  }

  // Tema que hereda los colores de la app; los colores de tokens vienen del
  // highlightStyle de one-dark.
  const appTheme = EditorView.theme(
    {
      "&": {
        height: "100%",
        backgroundColor: "transparent",
        color: "var(--karto-color-text)",
        fontSize: "0.85rem",
      },
      "&.cm-focused": { outline: "none" },
      ".cm-scroller": {
        fontFamily: "var(--karto-font-mono, monospace)",
        lineHeight: "1.5",
      },
      ".cm-content": { padding: "0.75rem 0" },
      ".cm-gutters": {
        backgroundColor: "transparent",
        color: "var(--karto-color-text-muted)",
        border: "none",
      },
      ".cm-activeLine": {
        backgroundColor: "color-mix(in srgb, var(--karto-color-accent) 8%, transparent)",
      },
      ".cm-activeLineGutter": { backgroundColor: "transparent" },
    },
    { dark: true },
  );

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({
        doc: value,
        extensions: [
          basicSetup,
          langCompartment.of(languageExt(language)),
          syntaxHighlighting(oneDarkHighlightStyle),
          appTheme,
          EditorView.updateListener.of((u) => {
            if (u.docChanged) {
              const doc = u.state.doc.toString();
              if (doc !== value) value = doc;
            }
          }),
        ],
      }),
    });
    return () => view?.destroy();
  });

  // Empuja cambios externos de `value` (p. ej. cargar otro script) al editor.
  $effect(() => {
    const v = value;
    if (view && v !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: v } });
    }
  });

  // Reconfigura el lenguaje al cambiar el intérprete del script.
  $effect(() => {
    view?.dispatch({ effects: langCompartment.reconfigure(languageExt(language)) });
  });
</script>

<div class="cm-host" bind:this={host}></div>

<style>
  .cm-host {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .cm-host :global(.cm-editor) {
    height: 100%;
  }
  .cm-host :global(.cm-scroller) {
    overflow: auto;
  }
</style>
