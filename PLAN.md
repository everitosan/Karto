# Karto — Plan de desarrollo

Mapeo visual de infraestructura en un canvas, con inventario cifrado de cada elemento
(IPs, hostnames, credenciales, puertos, apps), organización por carpetas de proyecto/ambiente,
conexión directa a los equipos con doble click, y compilación nativa multiplataforma.

## Stack

| Capa | Tecnología | Notas |
|---|---|---|
| Shell de app | Tauri 2 | Binarios nativos Linux/Windows/macOS |
| Backend | Rust | Único dueño de la clave de cifrado, del acceso a datos y del lanzamiento de conexiones |
| Base de datos | SQLite + SQLCipher | `rusqlite` con feature `bundled-sqlcipher-vendored-openssl` (estático, sin deps del sistema) |
| Derivación de clave | Argon2id | Contraseña maestra → clave AES-256; `zeroize` para limpiar memoria |
| Frontend | Svelte 5 + TypeScript + Vite | |
| Canvas | Svelte Flow (`@xyflow/svelte`) | Nodos custom, edges etiquetados, minimapa, serialización JSON |
| Import SSH | crate `ssh2-config` | Parseo de `~/.ssh/config` |

## Estructura del monorepo

Monorepo con **Turborepo + pnpm workspaces**. Reglas: Clean Architecture, módulos pequeños
y testeables; en el front, `Views/` para pantallas, `components/` para lo compartido, y los
componentes locales dentro del directorio de su View; lógica de negocio fuera de la UI
(`domain/` + `usecases/`).

```
karto/
├── apps/
│   ├── desktop/            # App Tauri 2 + Svelte 5 (producto principal)
│   │   ├── src/
│   │   │   ├── Views/          # pantallas (con sus componentes locales)
│   │   │   ├── components/     # componentes de presentación compartidos entre vistas
│   │   │   ├── domain/         # entidades y tipos de negocio
│   │   │   └── usecases/       # lógica que invoca los comandos Tauri
│   │   └── src-tauri/          # backend Rust (Cargo, dispara turbo como tarea)
│   │       └── src/
│   │           ├── vault/          # cifrado, Argon2, apertura/desbloqueo (módulo testeable)
│   │           ├── db/             # migraciones y repositorios SQLCipher
│   │           ├── connections/    # lanzamiento SSH/VNC/RDP/web
│   │           └── ssh_import/     # parseo de ~/.ssh/config
│   ├── storybook/          # Storybook que documenta @karto/ui
│   └── landing/            # sitio de presentación (Astro)
├── packages/
│   └── ui/                 # @karto/ui — componentes Svelte compartidos (desktop, storybook, landing)
├── turbo.json
├── pnpm-workspace.yaml
└── package.json
```

## Principios de seguridad

- El frontend **nunca** ve la contraseña maestra ni la clave derivada; solo invoca comandos Tauri.
- La clave vive en memoria del proceso Rust, envuelta en `zeroize::Zeroizing`.
- Vault portable: un solo archivo `.karto` (SQLite cifrada) que se puede respaldar/mover.
- Contraseña olvidada = datos irrecuperables (cifrado real). Mitigación: exports/backups cifrados.
- Auto-bloqueo por inactividad; limpieza de portapapeles N segundos tras copiar una credencial.
- Al lanzar conexiones, los secretos nunca se pasan como argumentos de línea de comandos visibles
  en `ps` (ver sección de conexiones).

## Organización: carpetas y diagramas

Árbol de carpetas anidadas a profundidad libre, tipo explorador de archivos, en un sidebar:

```
📁 Proyecto E-commerce
  📁 Producción
    🗺️ Capa web
    🗺️ Capa app
    🗺️ Capa datos
  📁 Staging
    🗺️ Diagrama general
  📁 Desarrollo
📁 Proyecto interno
  🗺️ Red oficina
```

- Ambos patrones soportados: `Proyecto > Ambiente > Diagramas` o `Proyecto > Diagrama por ambiente`;
  la jerarquía es libre, el usuario decide la convención.
- Drag & drop para mover diagramas/carpetas; renombrar y colorear carpetas.
- (Post-MVP) Un mismo equipo podría aparecer en varios diagramas referenciando el mismo registro,
  para no duplicar credenciales entre ambientes que comparten, p. ej., un firewall.

## Conexión con doble click (funcionalidad central)

Doble click sobre un nodo lanza la conexión con sus credenciales del vault, sin teclear nada:

- **SSH** — abre la terminal por defecto del sistema con la sesión ya autenticándose:
  - Con llave: `ssh -i <ruta_llave> user@host -p puerto` (la ruta de la llave se guarda en el vault;
    si la llave tiene passphrase se puede integrar con `ssh-agent`).
  - Con contraseña: la terminal abre `ssh` y el usuario **teclea la contraseña** interactivamente
    (se descartó `sshpass` por dependencia externa + licencia GPL; el secreto no se intermedia).
  - Detección de terminal por SO: Linux (x-terminal-emulator / gnome-terminal / konsole / kitty…,
    configurable), macOS (Terminal.app / iTerm2), Windows (Windows Terminal / cmd).
- **VNC** — lanza el cliente VNC configurado pasando host/puerto y la contraseña por archivo
  de password (formato del cliente, p. ej. `vncpasswd`) o stdin según el cliente.
- **RDP** — Windows: genera `.rdp` temporal + `cmdkey` para la credencial; Linux: `xfreerdp`
  con `/from-stdin`; macOS: cliente configurado.
- **Web/admin** (routers, paneles) — abre el navegador en la URL de administración; usuario y
  contraseña quedan a un click de copiar (con limpieza automática del portapapeles).
- Cada nodo puede tener varias credenciales/métodos; doble click usa el marcado como
  predeterminado, click derecho ofrece el menú completo ("Conectar por SSH", "Abrir panel web"…).
- Los comandos de lanzamiento son configurables por el usuario (plantillas), para soportar
  cualquier terminal/cliente no contemplado.

## Modelo de datos (esquema inicial)

- `folders` — id, parent_id (nullable, auto-referencia para anidación), nombre, color, orden.
- `maps` — id, folder_id (nullable = raíz), nombre, viewport, orden.
- `nodes` — id, map_id, tipo (server/router/database/firewall/cdn/generic), posición x/y, etiqueta.
- `node_properties` — clave/valor tipado por nodo (ip, hostname, proveedor, puertos, url_admin, notas…).
- `credentials` — node_id, tipo (ssh/rdp/vnc/web/db), usuario, secreto, puerto, ruta_llave,
  es_default, extras (JSON).
- `edges` — source, target, etiqueta (protocolo/puerto), estilo.
- `launch_templates` — plantillas de comando por tipo de conexión y SO (con defaults sensatos).
- `meta` — versión de esquema, parámetros Argon2, salt.

El secreto ya está protegido por el cifrado de página completa de SQLCipher;
no hace falta cifrado por campo adicional en el MVP.

## Estado de implementación

**Branding del autor — splash + "Acerca de" (2026-08-23):**

- ✅ **Splash**: bajo el logo completo aparece `by evesan` (texto atenuado, misma animación de entrada).
- ✅ **Configuración**: nueva **primera tab "Acerca de"** (`sections/settings/AboutSettings.svelte`, activa por
  defecto) con el logo, "Hecho por evesan" y enlaces a GitHub (`everitosan`), LinkedIn (`everitosan`) y
  `evesan.rocks`. Iconos de marca vía Devicon (`github-original`, `linkedin-plain`) + icono `link`.
- ✅ **Backend**: comando `open_external_url` (`usecases/connections::open_external_url`) que abre enlaces en el
  navegador del sistema reutilizando `build_open_url` (`xdg-open`/`open`/`start`). **Solo acepta http/https**
  (rechaza `file:`, `javascript:`, etc.); sin vault ni credenciales. Usecase frontend `usecases/about.ts`.
  Test `external_url_rejects_non_http_schemes`.

**Log de diagnóstico para soporte (2026-08-23):**

- ✅ Nuevo `usecases/diagnostics.rs`: escribe warnings en `karto.log` dentro del **dir de datos de la app**
  (`~/.local/share/app.karto.desktop/karto.log`, junto a `app.db`) para que el usuario pueda **compartir el
  archivo** al reportar un problema. Formato de una línea por evento: `TS WARN [component] event k="v" …`
  (timestamp ISO-8601 UTC sin dependencias externas). Rotación simple a `karto.log.1` al superar ~1 MB.
- ✅ **Privacidad**: nunca se registran secretos ni direcciones. Los llamadores pasan solo campos no
  sensibles (nodeId, kind, engine, program, código de salida, mensaje de error propio) y `sanitize` redacta
  como red de seguridad IPs y tokens `usuario@host`. La salida de scripts (stderr) **no** se registra.
- ✅ **Puntos instrumentados**: conexión que no arranca (`connection/launch_failed`, `db_launch_failed`,
  `url_open_failed`), cliente de BD ausente en el PATH (`db_client_missing`), y scripts que no pudieron
  arrancar (`script/target_error`) o terminaron con error (`script/target_failed` con código de salida).
  `warn` es best-effort: si el IO falla, nunca rompe la operación.
- ✅ **Encabezado de sesión al arrancar** (`record_startup`, comando `setup`): escribe **siempre** (sin
  filtrar por nivel) una línea `app/startup` con datos NO sensibles del equipo — versión de la app, SO,
  arquitectura, escritorio (`XDG_CURRENT_DESKTOP`), locale (`LANG`), nivel de log activo y **clientes/
  terminales detectados en el PATH** (ssh, xdg-open, psql/mysql/mongosh/redis-cli, gnome-terminal/konsole…).
  Saber qué falta explica de un vistazo muchos "no se pudo lanzar/conectar". Sin host/IP/usuario/secretos.
- ✅ **Nivel configurable** (`info`|`warning`|`error`, default Warning): umbral global de proceso
  (`AtomicU8`) leído en cada evento; se persiste en el **app_store** (migración v7, tabla `app_config` KV a
  nivel de máquina, NO en el vault) y se aplica al arranque en `setup`. Comandos `log_level_get/set`,
  `log_path_get`, `open_log_dir`.
- ✅ **Frontend**: nueva **tab "Diagnóstico"** en Configuración (`DiagnosticsSettings.svelte`): selector de
  nivel (3 opciones tipo radio), ruta del log (con "Copiar ruta" y "Abrir carpeta" → gestor de archivos del
  SO). Usecase `usecases/diagnostics.ts` (`normalizeLevel` puro + comandos).
- **Verificación**: **122 cargo** (+8 del módulo diagnostics, incl. smoke test que escribe el encabezado de
  arranque en un dir temporal y lo verifica), **48 vitest** (+4), `svelte-check` limpio, `vite build` OK,
  backend build limpio (el único warning, `SCHEMA_VERSION`, es preexistente). **No verificado en vivo**
  (sin display): repasar la tab, el "Abrir carpeta" y el encabezado de arranque real en el equipo.


**Export selectivo de nodos a un vault nuevo (2026-08-22):**

- ✅ Exportar un **subconjunto** (los nodos **seleccionados en el lienzo**) y las **aristas entre ellos**
  a un `.karto` nuevo, cifrado con **contraseña propia** (para compartir sin revelar la maestra). Caso de
  uso: de a-b-c-d-e-f exportar solo b-c-d.
- ✅ **Contenido opt-in** (checkboxes): credenciales, IP/direcciones por contexto (`node_endpoints`),
  metadata del equipo/facts (`os,recursos,kernel,arch,uptime,virt`) y notas (`notas`). La **identidad**
  (etiqueta, tipo, posición, hostname y propiedades genéricas) siempre viaja.
- ✅ **Backend** `usecases/export_subset.rs`: núcleo `copy_subset` (puro sobre dos conexiones migradas →
  testeable sin SQLCipher) que copia nodos (parent_id solo si el padre también se exportó), propiedades
  filtradas, endpoints (+ sus contextos), credenciales (con secreto, el `.karto` va cifrado) y solo las
  aristas con ambos extremos en la selección, todo en una transacción. `VaultService::export_subset` crea
  el destino cifrado con `store.create` (migrado) y borra el archivo si la copia falla. Comando
  `vault_export_subset`. **+4 tests cargo (86)**.
- ✅ **Frontend**: `vaultUseCases.exportSubset`, `pickSubsetExportPath`, `SubsetExportModal` (checkboxes +
  contraseña con confirmación) y botón **"Exportar selección (N)"** en el `Panel` del canvas (activo con
  ≥1 nodo seleccionado). `selectedNodeIds` derivado de `flowNodes[].selected`.
- ✅ **Modo de interacción tipo Figma** en el canvas (habilita la selección múltiple para el export):
  `selectionOnDrag` + `panOnDrag={false}` (cursor normal, arrastrar = recuadro de selección) +
  `panActivationKey="Space"` (Espacio + arrastrar = paneo) + `selectionMode=Partial` (toca ≠ encierra).
- **Verificación**: 86 cargo, 39 vitest, `svelte-check` limpio, `vite build` OK. **No verificado en vivo**
  (sin display): repasar el flujo real de selección múltiple + apertura del `.karto` exportado en otra sesión.


**Configuración con tabs + atajos por sección (2026-08-22):**

- ✅ `SettingsSection` pasó de una vista con todo apilado (columna centrada de 34rem) a un **shell de
  tabs a ancho completo**: **Seguridad** (auto-bloqueo, portapapeles, contraseña maestra), **Respaldos**
  (export cifrado), **Plantillas** (biblioteca + ligado al vault) y **Atajos**. Cada tab es un componente
  local en `sections/settings/` (`SecuritySettings`, `BackupSettings`, `TemplateSettings`, `ShortcutSettings`).
- ✅ **Atajos = teclado por sección** (decisión: se descartó el modelo de "shortcuts-alias" editables).
  Se **retiró** por completo la tabla `shortcuts` del app_store (migración v3 `DROP TABLE IF EXISTS`) y
  sus comandos/usecase. La pestaña Atajos es una **referencia read-only agrupada por sección** (hoy
  "Diagrama": Supr/Retroceso = eliminar, doble clic, Esc).
- ✅ **Borrado en el diagrama con Supr y Retroceso**: `SvelteFlow` usa `deleteKey={["Delete","Backspace"]}`
  (antes solo Backspace por defecto; la prop correcta en `@xyflow/svelte` 1.6 es `deleteKey`, no
  `deleteKeyCode`). Reusa el `onDelete` ya existente (borra nodos y aristas seleccionados).
- **Verificación**: 82 cargo, 40 vitest, `svelte-check` limpio (fix a11y `role=tablist` en `<div>`),
  `vite build` OK.


**Estado de app en SQLite sin cifrar — recientes + shortcuts (2026-08-22):**

- ✅ **Motivación**: hasta ahora el path del vault solo vivía en memoria de sesión → cada arranque caía
  en Welcome a re-elegir el archivo. Se añade **estado de app a nivel de equipo** (NO secreto: recientes,
  shortcuts) fuera del vault. Decisión de diseño: la ruta del vault **no es un control de seguridad** (lo
  es el cifrado); se prioriza descubribilidad y respaldo fácil.
- ✅ **SQLite sin cifrar** (no un JSON) pensando en un **futuro CLI**: GUI y CLI son procesos distintos
  que pueden escribir a la vez; SQLite da locking + transacciones (evita carreras/corrupción) y consultas
  parciales. **WAL** para concurrencia. La **ruta se resuelve por env** (`$XDG_DATA_HOME|~/.local/share`
  + `app.karto.desktop` → `app.db`), **independiente del runtime de Tauri**, para que un CLI apunte al
  mismo archivo. mac/Windows convencionales → Fase 7.
- ✅ **Backend** `usecases/app_store.rs`: conexión + migraciones propias (`user_version` independiente del
  vault; v1 = `recent_vaults` + `shortcuts` con 2 defaults sembrados `new`/`open`). Ops testeables sobre
  `&Connection` (in-memory): `remember` (upsert + recorte a 10), `list_recents`, `forget`, `prune_missing`
  (predicado inyectable), `list_shortcuts`. **Migración del `recent.json` heredado** (`import_legacy_recents`,
  una vez; renombra a `.migrated`). Comandos `recents_list` (purga de paso), `recents_forget`,
  `shortcut_list`, `default_vault_dir`. `lib.rs` registra el reciente tras `vault_create`/`vault_unlock`
  (best-effort). Reemplaza al `app_config.rs`/JSON anterior.
- ✅ **Frontend**: usecases `recents.ts` y `shortcuts.ts` (puentes inyectables). **Welcome** lista los
  recientes (clic → desbloquear; ✕ para olvidar) y al crear pre-rellena el diálogo con la ruta sugerida
  (`home/karto.karto`, editable). `pickNewVaultPath(defaultPath?)`. Arranque = Welcome con recientes.
- **Verificación**: **78 tests cargo** (app_store: seed/upsert/trim/forget+prune/import-legacy), **36
  vitest** (+recents/shortcuts forwarding), `svelte-check` limpio, `vite build` OK, sin warnings clippy nuevos.
- Hecho: tablas `templates` (v2) y `scripts` (v4) en el app_store. **Portabilidad decidida**: las
  plantillas se ligan al vault (portables); los **scripts son solo globales de máquina** (no viajan con el
  `.karto`). Ojo: `launch_templates` **ya existe** en el esquema del vault (migración 0). UI de shortcuts pendiente.

Actualizado: 2026-08-16.

**Direcciones contextuales — IP por contexto de acceso (2026-08-16):**

- ✅ **Motivación**: la `ip` de un nodo era un atributo fijo que se rompía al cambiar de sitio o
  entrar por VPN (una IP privada depende de *desde dónde* te conectas). Se separa **identidad**
  (hostname/FQDN, estable) de **localizador** (dirección, contextual).
- ✅ **Modelo** (migración **v6**): `access_contexts` (puntos de vista de red: Oficina/VPN/…) y
  `node_endpoints(node_id, context_id, address)`. La migración crea el contexto `default`
  ("Principal") y mueve la `ip` existente de `node_properties` a un endpoint de ese contexto
  (sin pérdida); `hostname` queda como respaldo estable al resolver.
- ✅ **Backend**: usecase `contexts.rs` (CRUD, borrado en cascada de endpoints), `node_set_endpoints`
  y `graph_load` extendido (Node gana `endpoints`). `connections::resolve`/`connect_node` y
  `ssh_provision::provision` reciben `context_id`: eligen el endpoint del contexto activo y caen a
  `hostname` si no hay. Comandos Tauri `context_list/create/rename/delete`, `node_set_endpoints`.
- ✅ **Contexto activo = estado local** (no viaja en el vault): store `networkContext.svelte.ts`
  con el id en `localStorage`, para no errar al abrir el mismo vault desde otra red. Al cambiarlo,
  **todo el diagrama** re-muestra y conecta por la dirección de ese contexto.
- ✅ **UI**: selector de contexto en la barra + `ContextsModal` (gestión); `PropertiesPanel` con
  sección "Direcciones por contexto" (una entrada por contexto, resalta el activo); `InfraNode`
  resume la dirección del contexto activo. Catálogo (`@karto/ui`) sin la propiedad `ip` (ahora es
  endpoint); dedup del import SSH mira endpoints.
- **Verificación**: **56 tests cargo**, **31 vitest**, `svelte-check` limpio.

**Fase 5 — Aprovisionamiento de llave SSH (2026-08-15):**

- ✅ **Onboarding de llave** para pasar de contraseña a llave: `usecases/ssh_provision.rs`
  genera una llave **ed25519 sin passphrase** con `ssh-keygen` (una por credencial, en
  `~/.ssh/karto/`; reusa la existente) y **encadena en una sola terminal**
  `ssh-copy-id … && ssh -i <llave> …` (`onboarding_script`, pura): el usuario teclea la contraseña
  **una sola vez**, se copia la pública y la sesión continúa ya con la llave. Karto no intermedia el
  secreto. Reusa detección de terminal + `hold_line`/`to_shell_line`/`ssh_inner_command` de `connections`.
- ✅ **Modal en la UI** (`KeyOnboardingModal`, sustituye al `confirm` nativo) con 3 checkboxes
  (los 2 últimos dependientes del primero): (1) registrar llave; (2) usar la llave como conexión
  **predeterminada** (fija `key_path`, conservando la contraseña como respaldo); (3) guardar la
  **privada en el vault** (migración **v5** `credentials.private_key`, cifrada y portable con el
  `.karto`). Estado compartido `connectFlow.svelte.ts` (`requestConnect`/`confirmOnboarding`) para
  que **panel y menú contextual** disparen el mismo modal, montado una vez en `Canvas`.
- ✅ **Materialización**: al conectar en otro equipo, si la credencial trae la privada en el vault y
  el archivo no existe en disco, `resolve` la escribe con permisos **0600** (`materialize_key`).
- ✅ Comando Tauri `ssh_provision_key(node_id, credential_id, set_default_key, store_in_vault)`;
  usecase front `provisionSshKey`. Helpers puros `pickCredential`/`needsKeyOnboarding` (front).
- **Verificación**: **50 tests cargo** (`keygen_command`, `copy_id_inner_command`, `onboarding_script`),
  **31 vitest** (`pickCredential`, `needsKeyOnboarding`), `cargo clippy`/`svelte-check` limpios,
  `vite build` OK.
- Nota: llave sin passphrase (decisión: conveniencia); el `key_path`/`private_key` se registran de
  forma optimista (el éxito real de `ssh-copy-id` ocurre en su terminal); si la copia falla, `ssh`
  reintenta contraseña. mac/Windows → Fase 7.

**Fase 5 — Endurecimiento (núcleo, 2026-08-15):**

- ✅ **Preferencias en el vault**: migración v4 (tabla `settings` clave/valor dentro del vault
  cifrado, portable con el `.karto`). Usecase `settings::{get_all,set}` + comandos `settings_get`/
  `settings_set`. Front: `usecases/settings.ts` (`settingsFromMap` puro + defaults saneados) y store
  reactivo `Views/Workspace/appSettings.svelte.ts` compartido por la vista.
- ✅ **Auto-bloqueo por inactividad** (configurable, 0 = off): helper puro `autoLock.ts`
  (`createIdleTimer`, temporizadores inyectables) cableado en `Workspace.svelte` — escucha
  actividad (mouse/teclado/wheel/touch) y recrea el temporizador al cambiar el intervalo; al
  vencer llama a `lock()`.
- ✅ **Limpieza automática del portapapeles** (configurable, 0 = nunca): `clipboard.ts`
  (`createClipboardManager`, escritura/temporizadores inyectables, recuerda lo último copiado).
  `PropertiesPanel.copySecret` copia con auto-limpieza; al bloquear el vault se limpia de inmediato.
- ✅ **Cambio de contraseña maestra**: `VaultService::rekey` verifica la contraseña actual
  (abre una conexión efímera) y re-cifra en sitio con `PRAGMA rekey`; comando `vault_rekey`.
- ✅ **Backup cifrado**: `VaultService::export` usa `VACUUM INTO` (copia cifrada con la clave
  actual, falla si el destino existe); comando `vault_export` + diálogo `pickBackupPath`.
- ✅ **UI**: `SettingsModal` (local a Workspace) con ajustes, cambio de contraseña y export; botón
  de ajustes en la cabecera. Icono `settings` añadido a `@karto/ui`.
- **Verificación**: **46 tests cargo** (rekey real con SQLCipher, export, settings), **24 vitest**
  (idle timer, clipboard manager, `settingsFromMap`), `cargo clippy`/`svelte-check` limpios,
  `vite build` OK.
- Decisiones: **Argon2id aplazado** (se mantiene la KDF nativa de SQLCipher); **builds/firma
  multiplataforma** se agrupan en la Fase 7 (regla Linux-first).

**Fase 4 — Importación SSH (2026-08-09):**

- ✅ **Backend**: el parseo de `~/.ssh/config` (`parse_hosts` + comando `ssh_import_preview`) ya
  existía. Nuevo `import_hosts(conn, map_id, hosts)` en `usecases/ssh_import.rs`: crea un nodo
  `server` por host con auto-layout en **rejilla** (`grid_layout`, función pura testeable; 4 cols,
  220×140), fija la propiedad `hostname` (con `host_target`: `HostName` o el alias como fallback) y
  una **credencial SSH predeterminada** (usuario, puerto, ruta de llave del config; secreto vacío —
  la contraseña se teclea al conectar). `ImportedHost` ahora deriva `Deserialize`/`Clone` en
  `camelCase`. Comando Tauri `ssh_import_hosts(map_id, hosts)` registrado. **+4 tests cargo (41
  totales)**: `grid_layout`, `host_target`, e `import_hosts` (nodos + credencial default + fallback
  de host).
- ✅ **Frontend**: helper puro `Views/Workspace/sshImport.ts` (`hostTarget`, `isDuplicate` — casa por
  alias/hostname/ip contra los nodos del diagrama destino, laxo en minúsculas) con **6 tests vitest
  (14 totales)**. Casos de uso `sshImportPreview`/`sshImportHosts` en `usecases/workspace.ts`. Nuevo
  **`SshImportModal`** (local a Workspace): lista los hosts con checkbox, muestra host/usuario/puerto/
  llave, elige **diagrama destino** (existente o crear uno nuevo) y **marca duplicados** (arrancan sin
  seleccionar; se recalculan al cambiar de destino). Botón de import en la cabecera del `Sidebar`;
  tras importar refresca el árbol y **fuerza el remonte del canvas** (null → tick → id) para que
  recargue el grafo aunque el destino ya estuviera abierto. `svelte-check` limpio, `vite build` OK.
- Nota: sin auto-layout tipo dagre (rejilla simple es suficiente para el MVP); el usuario reordena
  luego en el canvas.

**Fase 4 — Selector de origen + descubrimiento recursivo (2026-08-09):**

- ✅ **Descubrimiento recursivo** bajo `~/.ssh`: `discover_in(dir)` recorre el árbol (incluye
  `config.d/**`) y devuelve `CandidateFile { path, name, hostCount }` de cada archivo que **parece**
  config SSH (`looks_like_ssh_config`: alguna línea `Host`/`HostName`/`Match` fuera de comentarios) y
  contiene ≥1 host. Descarta por nombre llaves/known_hosts/sockets (`is_probably_not_config`) y por
  tamaño (>512 KB) o no-UTF8. Comando `ssh_import_candidates`. `discover_candidates()` usa `ssh_dir()`.
- ✅ **Parseo por ruta** (`parse_file` + comando `ssh_import_parse_file`) para sugerencias y archivos
  soltados. **+4 tests cargo (44 totales)**: heurística de contenido, descarte por nombre, y
  `discover_in` sobre un directorio temporal (config + config.d, ignorando id_ed25519/known_hosts/txt).
- ✅ **Modal rediseñado** (`SshImportModal`) en **dos etapas**: (1) **origen** — lista de sugerencias
  con nombre/ruta/nº de hosts, **zona de drop** (vía `getCurrentWebview().onDragDropEvent` de Tauri:
  el file-drop del SO entrega rutas reales, no HTML5) y botón **"Elegir archivo…"** (diálogo nativo);
  (2) **vista previa** del archivo elegido con el selector de destino y el marcado de duplicados ya
  existentes. El botón del sidebar abre este modal (ya no lee `~/.ssh/config` por defecto). Casos de
  uso `sshImportCandidates`/`sshImportParseFile`. `svelte-check` limpio, `vite build` OK, 14 vitest.

Actualizado: 2026-08-08.

**Opciones SSH extra por credencial (2026-08-08):**

- ✅ **Migración v2** (`ALTER TABLE credentials ADD COLUMN options`): cada credencial guarda opciones
  SSH extra como texto libre, **una por línea** (p. ej. `ServerAliveInterval=60`, `ConnectTimeout=10`,
  `ProxyJump bastion`). El vault existente migra solo.
- ✅ Al conectar, cada línea se inyecta como `-o <opción>` **antes del destino** (un argv por opción,
  así valores con espacios no rompen). `parse_ssh_options` recorta espacios y descarta líneas vacías y
  comentarios (`#`). Web/VNC no las usan. Cubre keepalive, timeouts, ProxyJump, IdentitiesOnly, etc.,
  sin que Karto tenga que conocer cada flag.
- ✅ Cableado full-stack: `Credential.options`/`CredentialInput.options` (Rust + TS), comando
  `credential_upsert` y `resolve`. En el **modal** de credencial hay un `textarea` "Opciones SSH extra"
  (solo para SSH) con ayuda inline. **35 tests cargo** (inyección `-o`, parseo, round-trip en `resolve`
  y `credential_list`), **8 vitest**, `cargo clippy`/`svelte-check` limpios, `vite build` OK.
- Nota: para flags que no son `-o KEY=VALUE` (p. ej. `-D 1080`), quedará la vía de **plantillas de
  comando** configurables (`launch_templates`, tabla ya en el esquema) — pendiente.

**Catálogo de nodos por categorías + iconos por propiedad (2026-08-08):**

- ✅ Catálogo compartido en `@karto/ui` (`catalog.ts`): 11 categorías (Red, Seguridad, Identidad,
  Cómputo, Aplicación, Datos, Almacenamiento, Mensajería, Observabilidad, Externo, Cliente) y ~40
  tipos de nodo con etiqueta, categoría, `connectable`, propiedades sugeridas (texto/`select`) e
  icono de respaldo (HugeIcons). Un único tipo **Base de datos** con `modelo`/`gestor`/`versión`
  (Redis/Elastic/Influx entran como modelos). `NodeKind`/`NodeCategory` centralizados aquí; el
  dominio del desktop los re-exporta. El backend Rust no cambia (`kind` es String libre).
- ✅ **Iconos por propiedad vía Devicon a color**: componente `TechIcon` (icon-font empaquetada
  offline) + `resolveNodeIcon(kind, properties)` que muestra el logo del gestor/framework elegido
  (Postgres, React, Docker, Kafka…) con **fallback** al icono del tipo. Clases Devicon verificadas
  contra el paquete para no dejar iconos en blanco.
- ✅ UI cableada: `NodePalette` agrupa por categoría; `InfraNode` pinta el icono de marca (fondo
  neutro para no teñirlo); `PropertiesPanel` genera los campos del tipo (selects de gestor/framework
  actualizan el icono en vivo) y conserva propiedades libres extra. `svelte-check` limpio (ui +
  desktop), `vite build` OK, 8 tests vitest verdes.
- Prueba visual de iconos a color en `docs/devicon-preview.html`.
- Devicon: se deja el empaquetado con **woff** (decisión del usuario; no se optimiza por ahora).
- ✅ **Agrupadores visuales** (categoría "Agrupadores", tipo `zone`): rectángulo de fondo
  redimensionable (`NodeResizer`) con etiqueta, `tipo` (VPC/subred/zona/región/DC), `cidr` y color;
  `ZoneNode.svelte` + tipo `zone` en `FlowEditor` (zIndex bajo + `elevateNodesOnSelect=false` para
  quedar detrás). Tamaño en `properties._w`/`_h` (propiedades internas `_*` ocultas y preservadas en
  el panel), sin cambios de esquema. **Puramente visual: sin parent/child** (mover la zona no arrastra
  los nodos) — la contención real (parentId) quedaría como mejora futura. `svelte-check`/`build`/tests OK.

**UI: modal de credenciales (2026-08-08):**

- ✅ Nuevo componente genérico **`Modal`** en `@karto/ui` (fondo oscurecido, panel centrado, cierre con
  Escape / clic fuera / ✕, `children` y `footer` como snippets). Exportado + story en Storybook.
- ✅ **`CredentialModal`** (local a `Views/Workspace/canvas/`): formulario de agregar/editar credencial
  dentro del `Modal` (tipo, usuario, secreto, puerto, ruta de llave solo para SSH, predeterminada). El
  `PropertiesPanel` abre el modal (botón + y lápiz de editar) en vez del formulario inline anterior; al
  editar revela el secreto para reponerlo. `svelte-check` limpio, `vite build` OK.
- ✅ Botón **"Conectar"** en el panel (usa la credencial predeterminada) + botón de conectar por
  credencial en la lista, reusando `connectNode` → `connect_node` (`usecases/connections.rs`). Se
  muestran solo en tipos `connectable` del catálogo. Lo que se descartó fue **el gesto de doble
  click**, no los botones (acción explícita, ver decisión de UX). `svelte-check` limpio, `vite build` OK.

**Fase 3 — Conexión (núcleo Linux) (2026-08-08):**

- ✅ Backend `usecases/connections.rs` reescrito con tres responsabilidades separadas y testeables:
  (1) **armado puro** de comandos por tipo/SO (`ssh_inner_command`, `build_ssh`, `build_open_url`,
  `build_vnc`, `plan`), (2) **resolución desde el vault** (`resolve`: deriva host de las propiedades
  del nodo —`ip` y si no `hostname`—, URL de `url_admin`/`url`, y carga la credencial elegida o la
  predeterminada con su secreto), y (3) **lanzamiento** (`connect_node`, con efecto: arranca el proceso).
- ✅ **Detección de terminal en Linux**: tabla `LINUX_TERMINALS` en orden de preferencia
  (x-terminal-emulator, gnome-terminal, konsole, kitty, alacritty, xterm) con su flag de ejecución;
  `detect_terminal` es inyectable (predicado `exists`) y `program_in_path` escanea el `PATH` sin deps.
- ✅ **SSH con llave** (`ssh -i <llave> -p <puerto> user@host`) y **SSH con contraseña interactiva**:
  la terminal abre `ssh` y el usuario teclea la contraseña. Se evaluó `sshpass` (`-e` y como sidecar
  de Tauri) pero se descartó por la dependencia externa y la licencia GPL; Karto no intermedia el
  secreto al conectar (se guarda en el vault solo para copiar/documentar).
- ✅ **Web/admin**: abre la URL con `xdg-open`/`open`/`cmd start` según SO.
- ✅ **VNC**: abre el cliente por defecto del equipo vía `vnc://[user@]host:puerto` con el abridor
  del SO (`build_vnc` reusa `build_open_url`); contraseña manual. Inyección automática (sidecar
  TigerVNC + `VNC_PASSWORD`) diferida a fase posterior.
- ✅ Comando Tauri `connect_node(node_id, credential_id?)`; el secreto **nunca** vuelve al frontend.
- ✅ Frontend: `connectNode` en `usecases/workspace.ts` (**+2 tests vitest, 8 totales**) y **botón
  "Conectar" en el panel de propiedades** del nodo (usa la credencial predeterminada), más un botón
  de conectar por credencial en la lista. Decisión de UX: se prefirió un botón explícito al doble click.
- ✅ **Terminal "hold"**: el comando SSH se envuelve en `bash -c "<cmd>; read"` con escapado seguro
  de argumentos (`shell_quote`). Doble beneficio: (1) la terminal **no se cierra** al terminar/fallar,
  así se leen los errores de `ssh`; (2) al invocar siempre `bash` (que existe), emuladores Qt como
  **konsole** ya no crashean cuando el binario destino falta.
  → Hallazgo de campo: un crash de konsole (`QLayout: Cannot add a null widget`) aparecía cuando el
  binario destino no existía; el envoltorio en `bash` lo evita.
- **19 tests cargo nuevos (33 totales)**: armado SSH llave/contraseña, prioridad llave, detección y
  envoltura de terminal, `hold_wrapper` + `shell_quote`, apertura de URL por SO, RDP no soportado, y
  `resolve` (ip>hostname, url, credencial específica vs. predeterminada, errores sin credencial/host).
  Verificado: `cargo clippy` limpio en `connections.rs`, `svelte-check` limpio, `vite build` OK.
- Pendiente Fase 3: menú contextual (click derecho) sobre el nodo; contraseña automática de VNC; RDP;
  soporte Windows/macOS (envoltura de terminal propia); plantillas de comando configurables
  (`launch_templates`, tabla ya creada en el esquema).

**Fase 2 — MVP del canvas y organización (2026-08-08):**

- ✅ Backend Rust: entidades del workspace en `domain/workspace.rs` (Folder, Map, Node, Edge,
  Credential, Graph; serializadas en `camelCase`). Casos de uso CRUD en `usecases/workspace.rs`
  sobre la conexión descifrada del vault (`VaultService::with_conn`), con IDs generados por el PRNG
  de SQLite (sin dependencia de `uuid`). **6 tests nuevos (14 totales en cargo)**: anidación y
  posición de carpetas, reparent + cascada, round-trip del grafo con propiedades y aristas, borrado
  en cascada de aristas, y credenciales (secreto nunca listado pero revelable, `is_default` exclusivo).
- ✅ Comandos Tauri (`lib.rs`): 27 comandos nuevos (folders/maps/graph/nodes/edges/credentials) que
  delegan en los casos de uso; el secreto solo sale por `credential_reveal` bajo demanda.
- ✅ Frontend: `usecases/workspace.ts` (puente Tauri inyectable, **3 tests vitest, 6 totales**),
  entidades alineadas en `domain/infra.ts`.
- ✅ Sidebar (`Views/Workspace/Sidebar.svelte`): árbol recursivo real con snippets, crear
  carpeta/diagrama, renombrar inline (doble click), eliminar, y **drag & drop para anidar/mover**
  (con prevención de ciclos y zona raíz). El arrastre distingue tres zonas por fila —tercio
  superior/inferior para **reordenar** entre hermanos (renumerando posiciones) y centro para
  **anidar** dentro de una carpeta— con línea de inserción como indicador.
- ✅ Canvas Svelte Flow (`Views/Workspace/canvas/`): `Canvas` provee `SvelteFlowProvider` y remonta
  por diagrama; `FlowEditor` carga el grafo y orquesta el **autoguardado** (posición al soltar nodo,
  alta/baja de nodos y aristas, viewport en `moveend` con debounce). Pan/zoom, minimapa, controles y
  snap a grid (16px). `NodePalette` arrastra tipos al lienzo; `InfraNode` es el nodo custom con icono,
  etiqueta y resumen (IP/hostname visible). `PropertiesPanel` edita etiqueta, propiedades sugeridas por
  tipo y credenciales (secreto oculto por defecto, mostrar/copiar, marcar por defecto). Aristas con
  etiqueta editable (click). Verificado: `svelte-check` limpio, `vite build` OK.
**Fase 6 — Health check de nodos (2026-08-17):**

- ✅ **Sonda TCP** (no ICMP → sin privilegios): `usecases/health.rs` deriva el host del contexto activo
  (endpoint) o hostname (reusa `connections::node_host`, ahora `pub(crate)`) y el puerto con `pick_port`
  (núcleo puro: credencial predeterminada → su `port` explícito → puerto por tipo ssh/rdp/vnc/web →
  22). `TcpStream::connect_timeout` (3s) distingue `reachable`/`unreachable`/`unresolved`/`noTarget` y
  reporta latencia. Comando Tauri `node_health(node_id, context_id)`. **+5 tests cargo (70 totales)**.
- ✅ **UI**: botón **"Estado"** en el `Panel` del canvas comprueba todos los nodos con dirección (las
  zonas se saltan) en paralelo; store `nodeHealth.svelte.ts` (estado por nodo en memoria de sesión, no
  viaja con el vault); `InfraNode` pinta un **punto de color** (verde/rojo/ámbar/gris + pulso al
  comprobar) con tooltip. Usecase `checkHealth`.
- **Verificación**: 70 cargo, 32 vitest, `svelte-check` limpio, `vite build` OK. No verificado en vivo.

**Fase 6 — Sondeo de datos del equipo al conectar (2026-08-17):**

- ✅ **Sin botón, al conectar, indiferente llave/contraseña** (decisión del usuario). Como la conexión
  por contraseña es interactiva (Karto no ve el stdout), se usa **multiplexado SSH (`ControlMaster`)**:
  la conexión SSH lanza primero una sesión de **sondeo** (`ssh_facts_line`) que corre un script remoto
  y vuelca `clave=valor` a un archivo local, y luego la **interactiva sin modificar** que reutiliza el
  socket → **una sola autenticación**. Si el multiplexado falla, la interactiva solo re-pide credencial
  (degradado, no roto). Solo se activa en **Linux + SSH**.
- ✅ **Backend** `usecases/facts.rs`: `remote_script()` (sin comillas simples, marcadores BEGIN/END),
  `parse_facts` (extrae el bloque, normaliza `ram_kb`→GiB, `virt=none`→bare-metal, descarta vacíos;
  `None` si aún no está el marcador de fin), `facts_file_path`/`control_path` (en `temp_dir`), y `poll`
  que lee el archivo, **fusiona (upsert) en `node_properties`** sin pisar otras claves, y lo borra.
  Comando Tauri `facts_poll(node_id)`. `connect_node` usa `build_ssh_with_facts` (SSH+Linux) y limpia
  el sondeo anterior. **+6 tests cargo (64 totales)**: script/parse/virt/incompleto/upsert y forma de
  `ssh_facts_line` (probe+redirección+reuse).
- ✅ **Frontend**: `connectFlow` reintenta `pollFacts` ~40s tras conectar por SSH y avisa vía listener
  (`onFactsCollected`) que el `FlowEditor` registra para **parchear las propiedades del nodo en vivo**
  (el panel las refleja). Usecase `pollFacts`.
- **Verificación**: 64 cargo, 32 vitest, `svelte-check` limpio, `vite build` OK. **No verificado en
  vivo** (sin display ni host SSH): revisar quoting del script remoto y el comportamiento de
  ControlMaster/`uptime -p` en la primera conexión real.

**Fase 6 — Export del diagrama (2026-08-16):**

- ✅ **PNG y SVG**: botón "Exportar" en un `Panel` (esquina superior derecha del canvas) con menú
  PNG/SVG. `FlowEditor.exportImage` calcula la caja de todos los nodos (`getNodesBounds`) y el
  transform para encuadrarlos (`getViewportForBounds`, fondo `#0b0f17`, lienzo = caja + margen, tope
  4096px). Se persiste con el comando backend `export_write(path, data)` (`std::fs::write`; no toca el
  vault, sin plugin fs). Diálogo nativo `pickExportImagePath` (nombre sugerido = nombre del diagrama).
  Nueva dependencia `html-to-image`.
- ✅ **Composición manual de aristas** (fix Linux): WebKitGTK (Tauri) **no rasteriza el SVG de las
  aristas** dentro del `<foreignObject>` que genera html-to-image (salían los nodos pero no las
  líneas). Solución: rasterizar **solo la capa de nodos** (HTML, fondo transparente) con html-to-image
  y **dibujar nosotros las aristas** leyendo el `d`/`stroke` de cada `.svelte-flow__edge-path` del DOM:
  en PNG con Canvas 2D (`Path2D` + `translate/scale`), en SVG como `<path>` vectoriales bajo la capa de
  nodos embebida como `<image>`. Así el export es fiable en Linux.
- **Verificación**: `svelte-check` limpio, `vite build` OK, `cargo build`/`clippy` sin warnings nuevos.
- Pendiente: **PDF** (requiere `jspdf`, se valorará aparte). No verificado en vivo (este entorno no
  tiene display); html-to-image inlina la fuente Devicon empaquetada al exportar.

**Fase 6 — Búsqueda global (2026-08-16):**

- ✅ **Backend**: núcleo puro `node_search_match` (case-insensitive, subcadena; prioridad
  etiqueta → hostname → dirección/endpoint → otras propiedades, con orden estable) y `search_nodes`
  que recorre **todos los diagramas** (join `nodes`+`maps`, más props y endpoints de todos los nodos),
  devuelve `SearchHit { nodeId, mapId, mapName, kind, label, matched }` ordenado por diagrama+etiqueta.
  Comando Tauri `node_search(query)`. **+2 tests cargo (58 totales)**: prioridad del matcher y
  búsqueda multi-mapa (hostname/IP/etiqueta, consulta vacía = sin resultados).
- ✅ **Frontend**: usecase `searchNodes`; componente `GlobalSearch` (input con debounce 200ms,
  descarte de respuestas fuera de orden, dropdown con etiqueta/campo casado/diagrama, Enter=primero,
  Escape/clic-fuera cierra) montado en `DiagramsSection` sobre el árbol. Al elegir un resultado:
  cambia `selectedMapId` y deja el nodo a enfocar en el store reactivo `focusNode.svelte.ts`; el
  `FlowEditor` del mapa destino lo atiende vía `$effect` (selecciona + `setCenter`), cubriendo tanto
  el remonte por cambio de diagrama como el diagrama ya abierto. **+1 vitest (32 totales)**.
- **Verificación**: 58 tests cargo, 32 vitest, `svelte-check` limpio, `vite build` OK.
- Pendiente de Fase 6: health check de nodos, export del diagrama, nodos compartidos. **Import
  known_hosts/CSV queda aplazado a propósito**.

**Fase 2 — Restaurar viewport (2026-08-16):**

- ✅ El viewport ya se guardaba **por-mapa** en `maps.viewport` (BD cifrada, portable con el `.karto`);
  faltaba restaurarlo al reabrir. `FlowEditor.onMount` ahora lee el viewport del mapa (`listMaps` →
  `parseViewport`, helper puro que descarta `'{}'`/JSON inválido) y aplica `setViewport` tras `tick`;
  si no hay uno válido se mantiene el `fitView` inicial de `<SvelteFlow>`.
- Decisión de diseño: el viewport es **contenido del diagrama** → va en `maps.*` (vault, por-mapa, no
  afecta a otros diagramas), **no** en `settings` (preferencia global) ni en `localStorage` (reservado
  a estado atado a este equipo/red, p. ej. el contexto de red activo).
- **Verificación**: `svelte-check` limpio, `vite build` OK.

Actualizado (scaffold Fase 1): 2026-08-07.

**Scaffold del monorepo completo y verificado** (Node 25 / pnpm 11.4 / Cargo 1.94):

- ✅ Raíz: pnpm workspaces + Turborepo (`turbo.json`), `tsconfig.base.json`, `.gitignore`, git init, README.
- ✅ `packages/ui` (`@karto/ui`): componentes Svelte 5 `Button` y `Badge` + `styles.css`. `svelte-check` sin errores.
- ✅ `apps/desktop` frontend (Svelte 5 + Vite + TS): estructura `Views/` (Welcome, Unlock, Workspace),
  `components/` (PasswordField compartido), `domain/` (vault, infra) y `usecases/` (vault, dialog, bridge Tauri).
  `svelte-check` limpio, `vite build` OK, **3 tests (vitest) verdes**.
- ✅ `apps/desktop/src-tauri` (Rust, Tauri 2): **arquitectura limpia por capas** —
  `domain/` (entidades + puerto `VaultStore`), `infra/` (SQLCipher implementa el puerto + migraciones),
  `usecases/` (servicios de aplicación: `vault`, `connections`, `ssh_import`), y `lib.rs` como capa de
  adaptadores (comandos Tauri que delegan). `cargo check` OK y **8 tests (cargo) verdes**: casos de uso
  probados con un store en memoria (fake) — ciclo crear/bloquear/desbloquear, contraseña incorrecta,
  acceso a conexión bloqueado/abierto — más el round-trip real de SQLCipher en la capa de infra.
  Iconos generados. Alineado con la convención de arquitectura del proyecto hermano Lumik.
- ✅ `apps/landing` (Astro + Svelte, consume `@karto/ui`): `astro build` OK.
- 🟡 `apps/storybook`: **dev server arranca OK** (`storybook dev`, flujo de trabajo real). El `storybook build`
  estático falla por incompatibilidad conocida Storybook 8.6 + Svelte 5 al compilar `.svelte` internos del
  runtime. Pendiente: subir a Storybook 9 o añadir `@storybook/addon-svelte-csf`. No bloquea el desarrollo.

**Sistema de diseño (2026-08-08):**
- Tokens en `@karto/ui/styles.css`: fondo degradado lineal con el azul dominante (`#090d15` sólido
  hasta 55% y luego funde a `#000`), accent `#11b245`, superficies/bordes/texto y tipografía.
- Fuentes empaquetadas offline vía `@fontsource`: **Titillium Web** (títulos) y **Ubuntu Sans** (texto).
- **Iconografía: HugeIcons (free)** vía `@hugeicons/svelte` + `@hugeicons/core-free-icons`. Componente
  `Icon` en `@karto/ui` (defaults: `currentColor`, stroke 1.8) + catálogo curado `icons` (folder,
  diagram, eye, lock, connect, terminal…) y `nodeTypeIcon` (server/router/database/firewall/cdn/generic).
  Emojis reemplazados por iconos en sidebar, unlock, password field, workspace y landing.
  Nota: HugeIcons inyecta el SVG en `onMount`, por eso en Astro (landing) van con `client:visible`.
- Componente **`Checkbox`** (`@karto/ui`): control **circular** con borde de acento que se rellena
  de verde al marcarse; envuelve un `<input>` nativo oculto (accesible, bindable, foco por teclado)
  con etiqueta opcional como snippet. Usado en el modal de onboarding de llave SSH.
- Componente **`Typography`** (`@karto/ui`) con variantes: display/h1/h2/h3/title (Titillium) y
  subtitle/body/body-sm/caption/label (Ubuntu Sans), con props `color` y `align`.
- Componente **`Logo`** (`@karto/ui`) reconstruido desde `/docs` (isotipo + logotipo), props `size` y
  `variant` (`full` | `iso`); todo con `currentColor` para temar iso+texto con un solo `color`.
- **Splash de arranque**: la app muestra el logo completo centrado sobre el degradado (duración mínima
  ~1.6 s) mientras consulta el estado del vault; luego pasa a bienvenida/desbloqueo/workspace.
- Landing y vista de bienvenida actualizadas a la nueva marca; `Button` usa el accent. Stories de
  `Logo` y `Typography` añadidas.

**Notas técnicas del scaffold:**
- El vault usa la **KDF nativa de SQLCipher** (PBKDF2-HMAC-SHA512, salt en cabecera). El paso a
  **Argon2id** se **aplazó** en la Fase 5 (decisión 2026-08-15): requiere rediseñar el formato del
  vault (guardar salt/params de Argon2 legibles antes de descifrar) y migrar vaults existentes; se
  retomará como endurecimiento cripto dedicado.
- `@karto/ui` expone condición de export `svelte` + `default` para que lo resuelvan tanto los bundlers
  con soporte Svelte (desktop, landing) como los que no.
- **Capabilities de Tauri 2:** los comandos propios de la app **no** necesitan entrar en
  `capabilities/default.json` (confirmado contra el proyecto Lumik, que solo lista permisos de
  core/plugins). El ACL solo aplica a comandos de plugins/core. Por eso `core:default` + `dialog:default`
  bastan. El testeo de comandos se hace a nivel de casos de uso (con fakes), no vía el harness IPC de
  Tauri (mismo enfoque que Lumik).

## Fases

### Fase 1 — Fundación
- [x] Scaffold del monorepo (Tauri 2 + Svelte 5 + TS) — hecho manualmente sobre Turborepo, no con `create-tauri-app`.
- [x] Integrar rusqlite + SQLCipher; build verificado en Linux (CI para Win/mac pendiente).
- [x] Flujo de vault: crear/abrir/desbloquear/cerrar (con KDF nativa de SQLCipher; Argon2id → Fase 5).
- [x] Migraciones de esquema versionadas (`PRAGMA user_version`, esquema inicial completo).
- [x] Pantallas: bienvenida (crear/abrir vault) y desbloqueo.
- [x] Comandos del vault (status/create/unlock/lock) + import SSH, con arquitectura limpia y probados.
- [ ] Smoke test en vivo (`tauri dev`) — pendiente: este entorno no tiene display/xvfb.
- [ ] CI de build multiplataforma (Windows/macOS).

### Fase 2 — MVP del canvas y organización
- [x] Sidebar con árbol de carpetas/diagramas: crear, renombrar, anidar, mover con drag & drop.
- [x] Canvas Svelte Flow: pan/zoom, minimapa, snap a grid.
- [x] Paleta de tipos de nodo con drag & drop: server, router, database, firewall, cdn, genérico.
- [x] Nodos custom con icono, etiqueta y resumen (p. ej. IP visible en la caja).
- [x] Panel lateral de propiedades por tipo, incluyendo credenciales (ocultas por defecto, botón copiar).
- [x] Conexiones entre nodos con etiqueta editable.
- [x] Persistencia: autoguardado de nodos/edges/viewport vía comandos Tauri.
- [x] Reordenar y anidar carpetas/diagramas por drag & drop (zonas antes/después/dentro con
  indicador de inserción y renumeración de posiciones).
- [x] Restaurar viewport guardado al reabrir un diagrama (por-mapa en `maps.viewport`, portable con
  el vault). Al cargar el grafo, `FlowEditor` lee el viewport del mapa y hace `setViewport`; si no hay
  uno válido (mapa nuevo = `'{}'`) cae al `fitView` inicial.

### Fase 3 — Conexión (botón en el panel del nodo) · **objetivo: Linux completo**

> **Estrategia (2026-08-08):** cerramos **todos** los puntos en Linux primero. Las particularidades de
> macOS/Windows (terminal, clientes RDP/VNC, apertura de URLs) se agrupan en la **Fase 7** para
> implementarlas y probarlas directamente en esos equipos.

- [x] SSH con llave en Linux (caso base) + detección de terminal (`LINUX_TERMINALS` + `PATH`).
- [x] SSH con contraseña — **interactiva**: la terminal abre `ssh` y el usuario teclea la contraseña.
  Karto no intermedia el secreto al conectar (se descartó `sshpass`/sidecar por dependencia+licencia;
  el secreto se sigue guardando en el vault solo para copiar/documentar).
- [x] SSH: opciones extra por credencial (una por línea → `-o <opción>`: keepalive, ProxyJump…).
- [x] URL de administración web (Linux: `xdg-open`).
- [x] Credencial predeterminada por nodo (botón "Conectar" usa la default; botón por credencial).
- [x] Menú contextual (click derecho) sobre el nodo: conectar (predeterminada o por credencial),
      abrir propiedades y eliminar. Componente `NodeContextMenu` (cierra con Escape / clic fuera).
- [x] VNC: abre el **cliente por defecto del equipo** vía `vnc://[user@]host:puerto` con el abridor
      del SO (`xdg-open`/`open`/`start`, mismo patrón que Web); la contraseña se teclea a mano. Sin
      dependencias ni binario empaquetado; queda listo para mac/Windows sin código extra.
- [ ] VNC con inyección automática de contraseña — **plan diferido**: empaquetar TigerVNC como
      *sidecar* (`bundle.externalBin`, un binario por target triple) e inyectar por la env var
      `VNC_PASSWORD` (texto plano, no sale en `ps` ni toca disco). Verificado contra el manual:
      TigerVNC **no** tiene `-autopass`; sí `VNC_PASSWORD` y `-PasswordFile` (archivo DES ofuscado).
      RealVNC queda descartado para empaquetar (propietario, no redistribuible). Ojo GPLv2 de TigerVNC
      (incluir licencia + oferta de fuente en el instalador). Requiere que `resolve()` traiga el
      secreto solo para VNC y lo limpie con `zeroize`.
- [ ] RDP en Linux (`xfreerdp` con `/from-stdin` para no exponer el secreto).
- [x] Plantillas de comando configurables (comando interno SSH). **Implementado (2026-08-22)**:
      - **Biblioteca a nivel de app** (máquina) en `app_store` (migración v2, tabla `templates` con 2
        ejemplos sembrados: túnel SOCKS `-D 1080`, reenvío de agente `-A`). Comandos `template_list/
        upsert/delete`. Placeholders `{host}` `{port}` `{user}` `{key}` `{userhost}`.
      - **Ligar al vault** = `template_link_to_vault` **copia** el comando al `launch_templates` del vault
        como `os='any'` (portable). `template_vault_list`/`template_vault_unlink` gestionan lo ligado. Como
        el export del vault usa `VACUUM INTO`, los overrides **viajan con el `.karto`** sin lógica extra.
      - **Resolución** (`usecases/templates.rs`): `render` puro (sustituye por token, descarta vacíos) +
        `vault_override`. En `connect_node`, si hay override SSH+Linux **gana** y se **salta el sondeo de
        facts** (una plantilla custom manda). Sin override → ruta por defecto intacta.
      - **UI**: grupo "Plantillas de conexión" en `SettingsSection` (lista biblioteca + ligadas, alta,
        ligar/reemplazar/eliminar/desligar). Nuevo `Os::as_key()`.
      - **Verificación**: 83 cargo (render/link/roundtrip/seed-CRUD), 40 vitest (forwarding), check/build OK.
      - **Descartado (2026-08-22, decisión del usuario)**: plantillas de "envoltura" (elegir terminal/cliente
        por máquina) y override de plantilla para VNC/RDP/web. Sobre-ingeniería para casos marginales: la
        detección automática de terminal (`LINUX_TERMINALS`, con `x-terminal-emulator` = terminal por
        defecto del SO) ya cubre lo común; una terminal exótica se resuelve añadiendo una línea a esa lista.

### Fase 4 — Importación SSH
- [x] Parsear `~/.ssh/config` (Host, HostName, User, Port, IdentityFile) con `ssh2-config`.
- [x] Vista previa de import: elegir qué hosts crear y diagrama destino, detectar duplicados.
- [x] Auto-layout de los nodos importados (grid).

### Fase 5 — Endurecimiento
- [x] Auto-bloqueo por inactividad (configurable).
- [x] Limpieza automática del portapapeles tras copiar secretos.
- [x] Cambio de contraseña maestra (re-cifrado con `PRAGMA rekey`).
- [x] Export/backup cifrado del vault.
- [ ] Migración a Argon2id (aplazada por decisión: se mantiene la KDF nativa de
  SQLCipher; requiere rediseñar el formato del vault y migrar vaults existentes).
- [ ] Builds y firma para Windows/macOS; AppImage/deb para Linux → **se agrupa en Fase 7**
  (regla Linux-first: requiere hardware/VM de cada SO).

### Fase 6 — Extras (post-MVP)
- [x] Búsqueda global (por IP, hostname, etiqueta) a través de todas las carpetas.
- [x] Health check opcional de nodos (TCP) con indicador visual.
- [x] Sondeo de datos del equipo al conectar por SSH (hostname, SO, kernel, arch, CPUs, RAM,
  uptime, virtualización) → rellena las propiedades del nodo.
- [x] Export del diagrama a **PNG/SVG** (PDF pendiente: requiere `jspdf`, se valorará aparte).
- [x] **Scripts remotos** sobre los equipos de un diagrama. **Implementado (2026-08-22)**:
      - **Biblioteca global de máquina** en `app_store` (migración v4, tabla `scripts` con 3 semillas:
        info del sistema, uso de disco, procesos top). Comandos `script_list/upsert/delete`. Decisión
        del usuario: **solo global de máquina** (no viaja con el `.karto`, sin "ligar al vault" a diferencia
        de las plantillas).
      - **Ejecución** (`usecases/scripts.rs`): `build_capture_argv` (puro) arma `ssh -o BatchMode=yes
        -o ConnectTimeout=10 … bash -s` — el cuerpo va por **stdin** (sin quoting). Solo auth por **llave**;
        equipo solo-contraseña → error "requiere llave". `run_target` transmite salida **línea a línea** por
        `tauri::ipc::Channel` (stdout+stderr en hilos scoped). Comando `scripts_run` resuelve todos los
        objetivos con el lock del vault una vez y luego ejecuta **secuencial o en paralelo** (hilos) sin DB.
        `script_targets` marca qué nodos son conectables (cred SSH con llave).
      - **UI**: `ScriptsSection` funcional (elige diagrama → equipos objetivo con checkbox → editor con
        biblioteca cargable → salida por equipo con badge de estado en vivo). Editor con **resaltado de
        sintaxis** (CodeMirror 6 + modo shell, `components/CodeEditor.svelte`, tema fundido con la app);
        botón guardar flotante que se pone **amarillo** cuando hay cambios sin guardar.
      - **Concurrencia/UI**: `scripts_run` ejecuta en un **hilo de fondo** y devuelve al instante (la salida
        llega por el `Channel`) para no congelar la UI; el frontend cierra el estado "ejecutando" al recibir
        todos los `done`. Mismo criterio aplicado a `node_health` (ahora `async` + `spawn_blocking`).
      - **Intérpretes (Fase 1, 2026-08-23)**: cada script tiene `interpreter` (migración v6). `bash` → `bash -s`,
        `python` → `python3 -` (host-side por SSH, cuerpo por stdin). Selector en el editor. Filtrado de
        objetivos por `kind` (solo máquinas con shell). Icono Devicon por lenguaje en la lista.
      - **Intérpretes de BD (Fase 2, 2026-08-23)**: motores específicos `postgresql`/`mysql`/`mariadb`/
        `mongodb`/`redis` por **Modelo B** = cliente local (`psql`/`mysql`/`mongosh`/`redis-cli`) conectando
        por red al puerto de la BD (funciona con BD gestionadas sin shell). Secreto por env (`PGPASSWORD`/
        `MYSQL_PWD`/`REDISCLI_AUTH`); en mongo va en la URI (asumido). `build_db_command` puro (testeado),
        `resolve_db_target` (host=endpoint→prop `host`→hostname, cred `kind='db'`, dbname=`instancia`).
        Ejecución unificada vía `run_streamed`; detecta el cliente en el PATH (error claro si falta).
        `ScriptTarget` pasa a **capacidades crudas** (`kind`, `gestor`, `sshKey`, `dbCred`) y el frontend
        calcula compatibilidad (motor BD → `kind=database` + `gestor` exacto) y selectabilidad. **101 cargo**
        (build_db_command por motor), check/build OK. Pendiente: túnel SSH a BD privada, sqlite/otros motores.
      - **Carpetas de scripts (2026-08-23)**: migración v5 (`script_folders` + `scripts.folder_id` con
        `ON DELETE SET NULL`), CRUD `script_folder_*` + `script_set_folder`. Carpetas **planas** (un nivel),
        colapsables; asignación por **menú contextual** ("Mover a…"); renombrar inline; borrar carpeta con
        modal (los scripts se conservan sueltos). Sin drag&drop/anidamiento (eso es del Sidebar de diagramas).
      - **Verificación**: 92 cargo (argv/precondición/CRUD seed + carpetas), 44 vitest (forwarding + canal +
        carpetas), check/build OK.
- [ ] Import adicional: `known_hosts`, CSV.
- [ ] Nodos compartidos entre diagramas (mismo equipo en varios mapas).

### Fase 7 — Particularidades por SO (macOS / Windows)

> Se deja al **final a propósito**: requiere hardware/VM de cada plataforma para implementar y probar
> de verdad. En Linux queda todo cerrado antes de tocar esto. La lógica pura de armado de comandos ya
> está parametrizada por `Os` (`domain::Os`), así que aquí es sobre todo añadir ramas y probarlas.

- [ ] **Terminal**: macOS (Terminal.app / iTerm2 vía `osascript`) y Windows (Windows Terminal `wt` / `cmd`).
- [ ] **SSH**: envoltura "hold" equivalente por SO (la contraseña la teclea el usuario en la terminal).
- [ ] **Apertura de URL web**: macOS `open`, Windows `cmd /C start` (ya armado, falta probar).
- [ ] **VNC / RDP**: clientes por defecto de cada SO (Windows: `.rdp` temporal + `cmdkey`; macOS: cliente configurado).
- [ ] **Builds y firma** para Windows/macOS (se solapa con Fase 5); AppImage/deb para Linux.

## Riesgos conocidos

1. **Build de SQLCipher multiplataforma** — mitigado con el feature vendored de rusqlite; validar temprano en las 3 plataformas (Fase 1).
2. ~~**SSH con contraseña sin interacción**~~ — resuelto por decisión de diseño: la contraseña la teclea el usuario en la terminal (sin `sshpass` ni PTY propio). Se favorece la autenticación por llave. Un flujo no interactivo podría revisarse post-MVP con un PTY propio en Rust.
3. **Diversidad de terminales/clientes VNC-RDP** — mitigado con plantillas de comando configurables y defaults por SO.
4. **UX de contraseña olvidada** — comunicar claramente al crear el vault; fomentar backups.
5. **Rendimiento con mapas grandes** — Svelte Flow maneja cientos de nodos sin problema; no es riesgo real para este caso de uso.
