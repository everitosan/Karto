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
- 🟡 **VNC básico**: abre `vncviewer host:puerto` (sin inyección automática de contraseña todavía).
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
- Pendiente menor: restaurar el viewport guardado al reabrir (hoy `fitView`); reordenar entre hermanos.

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
- Componente **`Typography`** (`@karto/ui`) con variantes: display/h1/h2/h3/title (Titillium) y
  subtitle/body/body-sm/caption/label (Ubuntu Sans), con props `color` y `align`.
- Componente **`Logo`** (`@karto/ui`) reconstruido desde `/docs` (isotipo + logotipo), props `size` y
  `variant` (`full` | `iso`); todo con `currentColor` para temar iso+texto con un solo `color`.
- **Splash de arranque**: la app muestra el logo completo centrado sobre el degradado (duración mínima
  ~1.6 s) mientras consulta el estado del vault; luego pasa a bienvenida/desbloqueo/workspace.
- Landing y vista de bienvenida actualizadas a la nueva marca; `Button` usa el accent. Stories de
  `Logo` y `Typography` añadidas.

**Notas técnicas del scaffold:**
- El vault usa por ahora la **KDF nativa de SQLCipher** (PBKDF2-HMAC-SHA512, salt en cabecera). El paso a
  **Argon2id** se hará en Fase 5 (endurecimiento), pues requiere guardar salt/params de Argon2 de forma
  legible antes de descifrar.
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
- [ ] Restaurar viewport guardado al reabrir un diagrama (hoy se guarda; al cargar se usa `fitView`).

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
- [ ] VNC completo en Linux (inyección de contraseña con archivo `vncpasswd`/stdin del cliente).
- [ ] RDP en Linux (`xfreerdp` con `/from-stdin` para no exponer el secreto).
- [ ] Plantillas de comando configurables (`launch_templates`) — cubre terminales/clientes no
      contemplados y flags sueltos (`-D 1080`, `-L`…).

### Fase 4 — Importación SSH
- [ ] Parsear `~/.ssh/config` (Host, HostName, User, Port, IdentityFile) con `ssh2-config`.
- [ ] Vista previa de import: elegir qué hosts crear y carpeta destino, detectar duplicados.
- [ ] Auto-layout de los nodos importados (grid o dagre).

### Fase 5 — Endurecimiento
- [ ] Auto-bloqueo por inactividad (configurable).
- [ ] Limpieza automática del portapapeles tras copiar secretos.
- [ ] Cambio de contraseña maestra (re-cifrado con `PRAGMA rekey`).
- [ ] Export/backup cifrado del vault.
- [ ] Builds y firma para Windows/macOS; AppImage/deb para Linux.

### Fase 6 — Extras (post-MVP)
- [ ] Búsqueda global (por IP, hostname, etiqueta) a través de todas las carpetas.
- [ ] Health check opcional de nodos (ping/tcp) con indicador visual.
- [ ] Export del diagrama a PNG/SVG/PDF.
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
