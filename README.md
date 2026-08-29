# Karto
*Tu mapa en la infraestructura.*

## Funcionalidades

- Información encriptada por medio de una llave maestra
- Diagramas con nodos desde routers hasta bases de datos con su metadata
- Administración de contraseñas y llaves para cada nodo (web, vnc, ssh, etc ...)
- Conexión en base a credenciales
- Ejecución de scripts en los nodos de forma secuencial y paralela
- Autobloqueo por inactividad
- Limpieza de portapapeles para evitar filtraciones


Ver [PLAN.md](PLAN.md) para el diseño completo y las fases.


## Desarrollo

### Arquitectura

Turborepo + pnpm workspaces.

```
apps/
  desktop/    App Tauri 2 + Svelte 5 (producto principal, backend Rust en src-tauri/)
  storybook/  Storybook que documenta @karto/ui
  landing/    Sitio de presentación (Astro)
packages/
  ui/         @karto/ui — componentes Svelte compartidos
```

### Requisitos

- Node ≥ 22.13 y pnpm
- Rust (cargo) para la app de escritorio
- Dependencias de sistema de Tauri (Linux: webkit2gtk-4.1, gtk3, libsoup3)

**Windows** — además de lo anterior, el backend no compila sin:

| Requisito | Por qué |
| --- | --- |
| Visual Studio Build Tools (workload *Desktop development with C++*) | toolchain MSVC que usa Rust en Windows |
| WebView2 Runtime | motor web de Tauri (ya viene en Windows 11) |
| Perl (Strawberry Perl) y NASM, en el `PATH` | los exige `bundled-sqlcipher-vendored-openssl`, que compila OpenSSL desde fuente |

El soporte de **runtime** en Windows está en curso: ver
[windows-adapt.md](docs/specs/windows-adapt.md).

### Clientes externos (en runtime)

Karto no empaqueta clientes: invoca las herramientas del sistema. Instala solo las
de las funciones que uses. Enfoque **Linux (Debian/Ubuntu)**; macOS/Windows llegan
en la Fase 7 (ver [PLAN.md](PLAN.md)).

**Local — la máquina donde corre Karto**

| Función | Herramienta | Paquete (Debian/Ubuntu) |
| --- | --- | --- |
| Conectar por SSH y ejecutar scripts bash/python | cliente SSH + una terminal | `openssh-client` + una de: `gnome-terminal` · `konsole` · `kitty` · `alacritty` · `xterm` |
| Aprovisionar llave SSH (generar + copiar) | `ssh-keygen`, `ssh-copy-id` | `openssh-client` |
| Abrir URL de administración web y VNC | `xdg-open` | `xdg-utils` |
| Visor VNC registrado para `vnc://` | `remmina` (u otro visor) | `remmina` + `remmina-plugin-vnc` |
| PostgreSQL (script/conexión) | `psql` | `postgresql-client` |
| MySQL / MariaDB (script/conexión) | `mysql` | `default-mysql-client` (o `mariadb-client`) |
| MongoDB (script/conexión) | `mongosh` | `mongodb-mongosh` (repo oficial de MongoDB) |
| Redis (script/conexión) | `redis-cli` | `redis-tools` |

No hace falta instalarlos a mano: `utils/scripts/install_clients.sh` detecta lo que
falta en el `PATH` y lo instala con el gestor nativo (apt · dnf · pacman · zypper ·
brew). Es idempotente y no toca lo que ya esté.

```bash
make check-clients     # solo reporta qué falta
make install-clients   # instala lo que falte (pregunta antes)

# Directo, con más control:
./utils/scripts/install_clients.sh --list          # grupos disponibles
./utils/scripts/install_clients.sh --only pg,redis # solo esos
./utils/scripts/install_clients.sh --yes           # desatendido
```

En Windows el equivalente es `utils/scripts/install_clients.ps1` (winget, más los
zips oficiales de `mongosh` y `redis-cli`, que no tienen paquete decente). El grupo
`ssh` necesita PowerShell como administrador. Recuerda que el runtime de Windows
sigue en curso: ver [windows-adapt.md](docs/specs/windows-adapt.md).

```powershell
.\utils\scripts\install_clients.ps1 -Check
.\utils\scripts\install_clients.ps1
```

**Remoto — el equipo destino**

- **SSH / bash / python**: `openssh-server` corriendo y `bash` (scripts bash) o
  `python3` (scripts python) instalados en el host.
- **Bases de datos**: solo el servidor de BD escuchando en su puerto. Karto usa el
  cliente **local** por red (Modelo B), así que **no** hace falta cliente en el host
  (funciona con BD gestionadas/remotas sin shell).

### Comandos

```bash
pnpm install          # instala todo el workspace
pnpm desktop          # (Vite) frontend de la app en el navegador
pnpm --filter @karto/desktop tauri:dev   # app de escritorio nativa
pnpm storybook        # Storybook en :6006
pnpm landing          # landing en dev
pnpm build            # build de todo vía turbo
pnpm test             # tests de todo vía turbo
```
