# Windows — Plan de adaptación

Estado de partida (auditoría 2026-08-29): el backend **compila y pasa los 136 tests
en Windows** (`cargo test --lib`), y el frontend es agnóstico del SO. Lo que no
funciona es el **runtime**: todo el lanzamiento de procesos está escrito Linux-first
(terminal, `bash`, `PATH`, permisos POSIX, multiplexado SSH). Este documento
concreta la "Fase 7" de [PLAN.md](PLAN.md) para Windows.

Regla de trabajo: **nada de `#[cfg(windows)]` disperso**. El armado de comandos ya
está parametrizado por `Os` (funciones puras, testeables desde Linux); las ramas
por SO se añaden ahí, no en los bordes con efecto.

---

## Fase 0 — Higiene del repo (desbloquea trabajar desde Windows) ✅

- [x] **`.gitattributes`** con `* text=auto eol=lf`. Sin esto, `core.autocrlf=true`
      marca `Cargo.toml` y `desktop-schema.json` como modificados sin cambio real
      y ensucia cada commit hecho desde Windows.
- [x] **`gen/schemas/windows-schema.json`**: decidir si se commitea junto a los
      otros schemas generados o entra al `.gitignore`. → **commiteado**, como los demás.
- [x] **README**: prerequisitos de build en Windows — MSVC Build Tools, WebView2 y
      **Perl + NASM** (los exige `bundled-sqlcipher-vendored-openssl`). Sin ellos un
      contribuidor nuevo se estrella en el primer `cargo build`.

## Fase 1 — `PATH` y detección de binarios ✅

- [x] **`program_in_path` respeta `PATHEXT`** (`src-tauri/src/usecases/connections.rs:61`).
      Hoy hace `dir.join("ssh")`; en Windows el binario es `ssh.exe`, así que
      **siempre devuelve `false`**. Rompe en cascada: detección de clientes de BD,
      `detect_tools()` del log de arranque y la futura detección de terminal.
      Resuelto extrayendo dos funciones puras — `executable_exts()` (lee `PATHEXT`,
      con la cadena vacía siempre primero para no alterar Unix ni romper nombres que
      ya traen extensión, tipo `wt.exe`) y `find_program()` (con el predicado de
      existencia inyectado, así el comportamiento de Windows se prueba compilando en
      Linux). 6 tests nuevos, uno de ellos contra el `PATH` real del SO.

## Fase 2 — Terminal y envoltura de comandos

- [ ] **`WINDOWS_TERMINALS`** análogo a `LINUX_TERMINALS`, en orden de preferencia:
      `wt.exe` (Windows Terminal) → `powershell.exe` → `cmd.exe`. Cada uno con su
      flag de ejecución (`wt` acepta el comando suelto; `powershell -Command`;
      `cmd /C`).
- [ ] **`hold_wrapper` / `hold_line` por SO** (`connections.rs:114`). Hoy hardcodean
      `bash -c "…; read -n1 -s -r"` para que la ventana no se cierre y se vea el
      error de `ssh`. En Windows no hay `bash`: hace falta el equivalente
      (`cmd /k`, o `powershell -NoExit` / `Read-Host`).
- [ ] **Quoting por SO**: `to_shell_line`/`shell_quote` son POSIX (comillas simples).
      Reutilizarlos para armar una línea de `cmd`/PowerShell escapa mal — y no es
      cosmético, es superficie de **inyección de argumentos**. Necesita su propio
      quoting, con tests de casos hostiles.
- [ ] **`plan()` deja de devolver error en Windows** (`connections.rs:316`): la rama
      `_ => None` es la que hoy hace fallar *toda* conexión SSH con "no se encontró
      una terminal soportada".

Al cerrar esta fase, **SSH conecta en Windows** — el 80% del valor de la app.

## Fase 3 — Llaves SSH y permisos

- [ ] **ACL de la llave privada en `materialize_key`** (`connections.rs:446`). Todos
      los `set_permissions(0o600)` están bajo `#[cfg(unix)]`; en Windows la llave
      hereda las ACL del directorio y **`ssh.exe` la rechaza** con *UNPROTECTED
      PRIVATE KEY FILE*. Equivalente con `icacls`: quitar herencia, conceder solo al
      usuario actual. Bug funcional **y** de seguridad.
- [ ] **Verificación al reusar una llave existente**: hoy el chequeo de permisos
      inseguros tampoco corre en Windows (mismo `#[cfg(unix)]`), así que se conecta
      con una llave potencialmente legible por otros sin avisar.
- [ ] **Aprovisionamiento** (`ssh_provision.rs:94`): `launch_in_terminal` aborta con
      `_ =>`. Además `ssh-copy-id` **no viene** con el OpenSSH de Windows → usar
      `type key.pub | ssh host "cat >> ~/.ssh/authorized_keys"`.

## Fase 4 — Sondeo de facts

- [ ] **El multiplexado SSH no existe en Windows.** `ssh_facts_line`
      (`connections.rs:171`) y `control_path` (`facts.rs:110`) usan un socket Unix y
      redirección `> file 2>/dev/null`. El OpenSSH de Windows no soporta
      `ControlMaster`/`ControlPath`. Decidir entre:
      - **(a)** sondeo aparte en `BatchMode=yes` — solo funciona con llave, pero no
        pide doble autenticación; o
      - **(b)** omitir los facts en Windows y documentarlo.

      Preferencia: (a), degradando a (b) cuando la credencial es por contraseña.

## Fase 5 — Conexiones restantes

- [ ] **BD**: quitar el gate `if Os::current() != Os::Linux` (`connections.rs:621`).
      Se destraba solo cuando estén las Fases 1 y 2.
- [ ] **RDP** (`connections.rs:334` devuelve error hoy): es *el* tipo nativo de
      Windows. `mstsc` + `.rdp` temporal + `cmdkey` para la credencial, como ya
      prevé PLAN.md:96.
- [ ] **VNC**: `build_vnc` delega en el esquema `vnc://`, que en Windows casi nadie
      tiene registrado → sale el diálogo "no hay app asociada". Necesita ruta de
      cliente configurable.

## Fase 6 — Diagnóstico y presentación

- [ ] **`record_startup`** (`diagnostics.rs:149`) loguea `XDG_CURRENT_DESKTOP` y
      `LANG`, vacíos en Windows: el encabezado de soporte pierde justo el contexto
      útil. Sustituir por el equivalente por SO.
- [ ] **Directorio de datos**: `data_dir()` (`app_store.rs:95`) da
      `%USERPROFILE%\.local\share\app.karto.desktop`. Funciona, pero no es
      idiomático. **Decidir antes del primer release de Windows**: cambiar a
      `%APPDATA%` después obliga a escribir una migración de `app.db` + `karto.log`.
- [ ] **Ventana**: `transparent: true` + `decorations: false` en `tauri.conf.json`
      quita en Windows la sombra nativa, las esquinas redondeadas de Win11 y los
      Snap Layouts. Funcional (ya hay `WindowResizeZones`), pero se ve no-nativo.
      Revisar con la app abierta antes de decidir.

## Fase 7 — Distribución

- [ ] **CI con matriz**: `release.yml` es `ubuntu-22.04` con `--bundles deb,appimage`.
      Añadir `windows-latest` con `--bundles nsis,msi`.
- [ ] **Firma de código**: sin firmar, SmartScreen bloquea el instalador a todo el
      que lo descargue. Es lo que más fricción va a generar en usuarios reales.

---

## Orden de ejecución

Fase 0 → Fase 1 → Fase 2 → Fase 3 → Fase 5 (BD) → Fase 7 (CI) → Fase 6 (decisión
`%APPDATA%`) → Fase 4 → Fase 5 (RDP/VNC).

Las tres primeras desbloquean SSH; el resto se puede repartir sin bloquear a nadie.
