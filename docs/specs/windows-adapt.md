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

## Fase 2 — Terminal y envoltura de comandos ✅

- [x] **`WINDOWS_TERMINALS`**: acabó siendo **solo `powershell.exe`**, contra lo
      previsto. Medido en Windows 11 + Windows Terminal 1.24: `wt` trocea la línea
      en cada `;` y el script del hold **nunca llega a ejecutarse** (escapando
      `\;` sí funciona, pero es apilar un segundo parser sobre datos del vault:
      más superficie de inyección a cambio de nada). La alternativa resultó mejor:
      lanzar PowerShell con `CREATE_NEW_CONSOLE` — Karto es GUI y no tiene consola
      propia — y **Windows enruta esa consola nueva a `wt` solo** si el usuario lo
      tiene como terminal predeterminada. Se obtiene su UX sin tocar su parser.
- [x] **`hold_wrapper` / `hold_line` por SO** (`connections.rs:114`). Hoy hardcodean
      `bash -c "…; read -n1 -s -r"` para que la ventana no se cierre y se vea el
      error de `ssh`. En Windows no hay `bash`: hace falta el equivalente
      (`cmd /k`, o `powershell -NoExit` / `Read-Host`).
- [x] **Quoting por SO**: `to_pwsh_line`/`pwsh_quote` — literales de PowerShell
      (comilla simple duplicada) más el operador de llamada `&`, necesario porque
      un programa entrecomillado se interpretaría como cadena y se imprimiría.
      Dentro de comillas simples PowerShell no expande `$`, backtick, `;` ni `|`,
      así que un host/usuario/opción del vault no puede volverse código; hay test
      con payload hostil y verificación end-to-end contra una consola real.
- [x] **`plan()` deja de devolver error en Windows** (`connections.rs:316`): la rama
      `_ => None` es la que hoy hace fallar *toda* conexión SSH con "no se encontró
      una terminal soportada".

También se separó `terminal_command` (con `CREATE_NEW_CONSOLE`) de
`external_command` (con `CREATE_NO_WINDOW`, que evita el parpadeo de consola al
abrir una URL con `cmd /C start`), y `detect_tools()` del log de arranque pasó a
listar las terminales del SO real en vez de las de Linux siempre.

**SSH ya conecta en Windows** — el 80% del valor de la app. Queda pendiente
verificarlo con la app GUI empaquetada (`tauri:dev`), no solo desde los tests.

## Fase 3 — Llaves SSH y permisos (ACLs ✅ · aprovisionamiento pendiente)

- [x] **ACL de la llave privada en `materialize_key`**. Confirmado contra
      OpenSSH_for_Windows_9.5p2: una llave con `Everyone` en su DACL —aunque sea
      por **herencia** del directorio— se rechaza con *UNPROTECTED PRIVATE KEY
      FILE!*, así que era un bloqueante real de la conexión por llave.

      La receta que circula, `icacls <f> /inheritance:r /grant:r usuario:F`, **no
      sirve**: `/grant:r` sólo reemplaza las ACEs *de ese usuario*, y una ACE
      explícita de `Everyone` sobrevive (medido: la llave sigue rechazada). Hace
      falta `/reset` **antes**, y los grupos por **SID** y no por nombre, porque
      los nombres están localizados (`BUILTIN\Administradores` en español).

      Vive en `infra/file_perms.rs`, con una API por SO (`restrict_to_owner`,
      `audit`) en vez de `#[cfg]` dispersos. `icacls.exe` se invoca por ruta
      absoluta bajo `System32`, no vía `PATH`: uno interpuesto se ejecutaría con
      nuestros privilegios sobre la ruta de una llave privada.
- [~] **Verificación al reusar una llave existente**: en Unix sigue el chequeo de
      modo (ahora vía `file_perms::audit`). En Windows leer la DACL exigiría una
      dependencia nueva de la API del SO, así que `audit` devuelve `Unknown` y
      Karto **no toca** el archivo —es del usuario, y apretarle los permisos en
      silencio no desharía una exposición previa, sólo la ocultaría—; queda
      registro en el log y `ssh` lo rechaza él mismo si de verdad está abierta.
      Cerrarlo del todo pide leer la DACL (`windows-sys`) o parsear `icacls /save`,
      que es SDDL y por tanto independiente del idioma.
- [ ] **Ojo con el `PATH`**: si hay Git para Windows instalado, su `ssh`/`ssh-keygen`
      son un build **MSYS** que *no* hace la comprobación de ACL y se comporta
      distinto del OpenSSH del sistema. La app GUI hereda el `PATH` del sistema y
      usa el de `System32\OpenSSH`, pero conviene tenerlo presente al diagnosticar
      un "a mí me funciona" desde una terminal de Git Bash.
- [ ] **Aprovisionamiento** (`ssh_provision.rs:94`): `launch_in_terminal` aborta con
      `_ =>`. Además `ssh-copy-id` **no viene** con el OpenSSH de Windows → usar
      `type key.pub | ssh host "cat >> ~/.ssh/authorized_keys"`.

## Fase 3b — Llaves gestionadas por Karto (portabilidad del vault)

Sale de una observación al revisar el export: `copy_subset` incluye `private_key`
en el `INSERT`, así que **el material de la llave viaja dentro del `.karto`** si
se marca "credenciales". Pero `private_key` sólo se puebla al aprovisionar con
"guardar en el vault"; una credencial dada de alta a mano lleva sólo `key_path`,
que en otra máquina no apunta a nada.

La idea: que Karto **no se lleve nunca llaves que no creó**, y aun así garantice
que el vault es portable. Una llave del usuario sirve de *arranque*, no de carga.

- [x] **Reformular el disparador**. `keyOnboardingReason` sustituye a la pregunta
      "¿le falta llave?" por **"¿puede el vault llevarse esta credencial?"**, y
      devuelve el motivo: `password` (SSH pelada) o `local-key` (tiene llave, pero
      sólo existe en este equipo). `has_vault_key` se expone en el DTO como
      booleano derivado en SQL (`private_key IS NOT NULL AND <> ''`) — nunca el
      material; `credential_upsert` lo relee para que una edición no lo pierda.

      `needsKeyOnboarding` ya es `!== null`: se ofrecen los dos casos, y el modal
      cambia título, explicación y botón según el motivo. En `local-key` viene
      marcado "guardar en el vault", porque ahí la portabilidad **es** el objetivo.
- [ ] **Reconocer las llaves propias**. `provision()` ya genera con
      `-C karto:<node_id>`, y ese comentario queda **embebido en la llave privada**:
      se recupera con `ssh-keygen -y -f <llave>` (tercer campo). Funciona
      retroactivamente sobre las llaves ya generadas, sin migración.

      **Obligatorio pasar `-P ""`**: comprobado en Windows, sobre una llave con
      passphrase `ssh-keygen -y` se queda esperando la passphrase y **cuelga la
      app**. Con `-P ""` sale en el acto con código 255. De paso resuelve la
      clasificación: las llaves de Karto se generan con `-N ""`, así que una llave
      con passphrase no es nuestra por construcción.
- [ ] **Árbol de decisión** al registrar una credencial con llave:

      | Llave que aporta el usuario | Acción |
      | --- | --- |
      | Comentario `karto:*` | leer el material y registrarlo en el vault; **no se crea nada** |
      | Con passphrase | no es de Karto → rama de generar |
      | Cualquier otra | ofrecer generar una de Karto, usando la existente como *bootstrap* |

- [x] **Fuera `ssh-copy-id`, instalación directa** (`usecases/key_install.rs`).
      La duda sobre los flags de `ssh-copy-id` se resuelve no usándolo: un `ssh` a
      secas con el script remoto como último elemento del argv (mismo patrón que
      `facts::remote_script`). Hace lo que hacía él —`umask 077`, crear `~/.ssh`
      con 700, y `grep -qxF` para no duplicar la entrada al reaprovisionar— y de
      paso **desaparece la dependencia que no existe en Windows**: un solo camino
      para los tres SO.

      La pública se incrusta entre comillas simples en el script remoto, así que
      se rechaza una que traiga comilla o salto de línea. Las de Karto nunca las
      traen; es red de seguridad ante una pública de origen inesperado.

      **Sustituirlo obliga a replicar sus casos límite**, y la primera versión se
      dejó varios (encontrados al revisar si no habría sido mejor dejar
      `ssh-copy-id` en Linux con compilación condicional):

      - *Salto de línea final.* Si `authorized_keys` no termina en salto, un
        `printf >>` pega la llave nueva al final de la anterior y **corrompe las
        dos**. Reproducido y corregido.
      - *Permisos de un archivo preexistente.* `umask 077` sólo cubre lo que se
        crea ahora; `sshd` ignora un `authorized_keys` accesible por otros.
      - *SELinux.* En RHEL/Fedora un `~/.ssh` recién creado nace con un contexto
        que `sshd` rechaza; `restorecon` (best-effort) lo corrige.
      - *Estado de salida.* Los pasos van con `&&`, no `;`: de su código depende
        que se escriba el marcador y que Karto toque la credencial.

      Se descartó la vía de mantener `ssh-copy-id` sólo en Linux: su `-i` mezcla
      "llave a instalar" con "llave para autenticar", así que el **arranque con la
      llave del usuario** —lo nuevo de la Fase 3b— quedaría sin soporte ahí o con
      una combinación de flags sin verificar. Bifurcar dejaría la funcionalidad
      nueva peor en Linux que en Windows, y macOS pediría una tercera decisión.

      Verificado ejecutando el script generado con `sh` real: sin salto final ya
      no corrompe, tres pasadas no duplican, la primera crea `~/.ssh`, y un
      `$HOME` inaccesible devuelve ≠ 0. Los `chmod` no se pudieron comprobar
      (el banco de pruebas es un sistema de archivos Windows).
- [x] **Bootstrap por llave existente**: `ssh -i <la del usuario>
      -o IdentitiesOnly=yes` para autenticarse e instalar la nueva, y el usuario
      no teclea nada. `IdentitiesOnly` es necesario: sin él `ssh` ofrece antes las
      identidades del agente y puede autenticar con otra o agotar los intentos.
      Sólo se usa si el archivo **existe** en este equipo.
- [~] **Marcador estable / reconocer llaves propias**: **descartado por ahora**.
      Sin usuarios productivos no hay nada que migrar, y el comentario `karto:` es
      una afirmación falsificable: reconocerlo abriría el único camino por el que
      una llave ajena podría acabar dentro del vault. Hoy esa garantía se cumple
      **por construcción** —`commit_if_provisioned` es el único punto que escribe
      `private_key`, y lee la ruta que él mismo deriva de `key_path_for`, o sea la
      llave que Karto acaba de generar—. Si algún día hace falta reconocerlas, que
      sea por huella registrada al generarla, no por el comentario.
- [x] **Avisar del callejón sin salida**. Hay ruta de llave, el archivo no está en
      este equipo y el vault no lleva material: no hay con qué autenticar, ni
      siquiera para instalar una nueva. Es lo que pasa al abrir un `.karto` cuya
      credencial apuntaba a una llave personal del otro equipo. `key_present` en el
      DTO + `keyIsUnreachable` lo detectan y el modal lo dice, en vez de dejar que
      el usuario se coma un `Permission denied (publickey)` sin explicación.

      Es un **aviso, no un bloqueo**: el servidor podría admitir contraseña, y eso
      Karto no lo sabe.

Dos límites que conviene no perder de vista:

- **Esto reubica la exposición, no la elimina.** La llave de Karto sigue viajando
  en claro dentro del `.karto` cifrado. Lo que se gana es que lo que viaja sea de
  ámbito Karto y revocable por su cuenta, mientras la identidad personal o
  corporativa del usuario no sale de su máquina. Es radio de daño, no anulación.
- **El comentario es una afirmación, no una prueba**: cualquiera puede ponerle
  `-C karto:loquesea` a su llave, y el falso positivo hace justo lo que queremos
  evitar. Por eso la detección debe **proponer un valor por defecto visible**, no
  actuar en silencio. La alternativa infalsificable —guardar la huella al
  generar— no cubre el caso interesante ("llave de Karto que *este* vault no
  conoce": otro vault, un backup, la credencial recreada), así que en todo caso
  sería un complemento: huella = certeza, comentario = probable.
- [x] **Aprovisionamiento en Windows** (heredado de la Fase 3): resuelto de paso
      al quitar `ssh-copy-id`. `launch_in_terminal` deja de rechazar Windows y el
      encadenado del script se hace por SO — **Windows PowerShell 5.1 no tiene
      `&&`** (llegó en PowerShell 7), así que allí la condición se escribe con
      `$LASTEXITCODE` y el marcador con `New-Item`. Falta probarlo contra un
      servidor real desde Windows.

### Fase 3b.0 — Atomicidad del aprovisionamiento ✅

- [x] `provision()` lanzaba la terminal con `spawn()` —sin esperar— y **acto seguido
      escribe en la BD** (`store_in_vault`, `set_default_key`), sin saber si
      `ssh-copy-id` funcionó. Hoy eso deja la credencial apuntando a una llave que
      el servidor no acepta. Con la Fase 3b pasa a ser **pérdida de acceso**: se
      repunta `key_path` de la llave que funcionaba a una que no.

      Resuelto con el patrón que ya existía para los facts: el script de la
      terminal deja un **marcador** en disco entre la copia y la sesión
      interactiva (`copy && touch <marca> && ssh`) —ahí a propósito, para que
      aparezca en cuanto la copia acaba y no al cerrar la terminal—, `provision()`
      ya no escribe en la BD, y lo hace `commit_if_provisioned` (idempotente:
      consume la marca) vía el comando `ssh_provision_poll`. El frontend lo sondea
      con el mismo bucle que `collectFacts`, con margen amplio porque entre medias
      el usuario teclea su contraseña.

      Si la confirmación no llega, no se hace nada: la credencial se queda como
      estaba, que es justo lo correcto cuando `ssh-copy-id` falló.

### Fase 3b.1 — Rutas de llave entre SO ✅

Salió al ir a probar un `.karto` exportado desde Linux en Windows, que es
exactamente el escenario que justifica guardar el material en el vault.

- [x] Una ruta POSIX **no es absoluta en Windows**: se resuelve contra la unidad
      actual, así que `materialize_key` escribía en `C:\\home\\eve\\...` —basura en
      la raíz del disco— y `ssh -i` no encontraba la llave. El caso simétrico
      (`C:\\Users\\...` abierto en Linux) falla igual.
- [x] Y `icacls` remataba: lee un argumento que empieza por `/` como modificador
      suyo (`/reset`, `/grant`…), así que devolvía error de sintaxis (exit 87) en
      vez de aplicar la ACL. Ahora `restrict_to_owner` exige ruta absoluta y lo
      dice claro.
- [x] `is_usable_key_path` + `local_key_path`: si la ruta guardada sirve en este
      SO se respeta; si viene de otro se remapea a `~/.ssh/karto/<cred>_ed25519`.
      `is_absolute()` responde la pregunta correcta en cada plataforma, así que
      cubre las dos direcciones sin ramas por SO.

Verificado end-to-end en Windows (`windows_foreign_vault`, `#[ignore]`):
`/home/eve/.ssh/karto/cfv1_ed25519` → `C:\\Users\\rockf\\.ssh\\karto\\cfv1_ed25519`,
con ACL correcta y aceptada por el OpenSSH del sistema.

### Fase 3b.2 — Todo hijo por los helpers de spawn ✅

Reportado probando: al aprovisionar, el prompt de huella de `ssh` salía en la
terminal donde corre `pnpm tauri dev`, no en una consola propia.

- [x] `provision()` seguía usando `Command::new` a secas: el hijo heredaba la
      consola del padre. En `tauri dev` eso mezcla los prompts con los logs de
      vite; en la **app empaquetada**, que no tiene consola, no salen en ninguna
      parte y el aprovisionamiento parece colgado. Ahora usa `terminal_command`.
- [x] Mismo problema, al revés, en `scripts::run_target`, `run_db_target`,
      `health::run_client_ping` y el `ssh-keygen` del aprovisionamiento: capturan
      o descartan la salida, pero en Windows un ejecutable de consola lanzado
      desde una app GUI **abre una consola propia** si no se le dice lo
      contrario. Pasan por `external_command` (`CREATE_NO_WINDOW`).
- [x] Efecto colateral en Linux: esos tres puntos tampoco saneaban el entorno
      del AppImage, que era lo que arregló `702b26b` para las conexiones y se
      quedó fuera aquí. `external_command` ya lo hace.

Regla, documentada en los helpers: **ningún hijo se lanza con `Command::new` a
secas**; o `terminal_command` (necesita consola) o `external_command` (no debe
mostrar ventana).

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

Fase 0 ✅ → Fase 1 ✅ → Fase 2 ✅ → Fase 3 (ACLs) ✅ → **Fase 3b.0 (atomicidad)** →
Fase 3b → Fase 5 (BD) → Fase 7 (CI) → Fase 6 (decisión `%APPDATA%`) → Fase 4 →
Fase 5 (RDP/VNC).

Las tres primeras desbloquean SSH; el resto se puede repartir sin bloquear a nadie.
