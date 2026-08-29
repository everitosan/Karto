<div align="center">
  <img src="docs/isotipo.svg" alt="Karto" width="88">
  <h1>Karto</h1>
  <p><em>Tu mapa en la infraestructura.</em></p>
  <p>
    <a href="https://github.com/everitosan/Karto/releases"><img alt="Release" src="https://img.shields.io/github/v/release/everitosan/Karto?include_prereleases&label=release"></a>
    <a href="LICENSE"><img alt="Licencia" src="https://img.shields.io/github/license/everitosan/Karto"></a>
    <a href="https://tauri.app"><img alt="Tauri 2" src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri&logoColor=white"></a>
  </p>
</div>

<!-- TODO: captura o GIF de la app aquí; es lo que más ayuda a quien llega por primera vez. -->

## Qué es

Karto es una aplicación de escritorio que contiene el diagrama de tu infraestructura en un canvas: routers, servidores, bases de datos y
sus relaciones, cada uno con su metadata (IPs, hostnames, puertos, apps) y sus credenciales. Todo se concentra en un archivo cifrado en local tras una contraseña maestra y desde el mismo diagrama tienes acceso a ejecuciones remotas o una conexión directa por ssh o vnc.

## Características

- Información encriptada por medio de una llave maestra
- Diagramas con nodos desde routers hasta bases de datos con su metadata
- Administración de contraseñas y llaves para cada nodo (web, vnc, ssh, etc ...)
- Conexión en base a credenciales
- Ejecución de scripts en los nodos de forma secuencial y paralela
- Autobloqueo por inactividad
- Limpieza de portapapeles para evitar filtraciones

## Instalación

Descarga la última versión desde **[Releases](https://github.com/everitosan/Karto/releases)**.

### Linux

Con el `.deb` (Debian, Ubuntu, Mint):

```bash
sudo apt install ./Karto_*_amd64.deb
```

O con el `.AppImage`, que corre en cualquier distro sin instalar nada:

```bash
chmod +x Karto_*_amd64.AppImage
./Karto_*_amd64.AppImage
```

### Windows y macOS

Todavía no hay instalador listo para descargar. Mientras tanto puedes compilarlo tú
siguiendo los pasos de [Desarrollo](#desarrollo).

### Dependencias

Karto se apoya en las herramientas del sistema para conectarse (`ssh`, `psql`,
`redis-cli`, un visor VNC…). Este repo contiene un [instalador de clientes](#clientes-externos)
que puede ayudarte para tener todo listo para el funcionamiento completo de karto.

## Clientes externos

Karto no empaqueta clientes: invoca las herramientas del sistema. Todo esto se
instala en **la máquina donde corre Karto**; el equipo destino solo necesita lo
del último apartado.

### Instalación

Un script revisa qué te falta y lo instala con el gestor de paquetes de tu sistema.
No toca lo que ya tengas: te muestra qué va a hacer y pide confirmación antes.

**Windows** — abre PowerShell **como administrador** (lo exige el cliente de SSH):

```powershell
irm https://raw.githubusercontent.com/everitosan/Karto/main/utils/scripts/install_clients.ps1 -OutFile install_clients.ps1
powershell -ExecutionPolicy Bypass -File .\install_clients.ps1
```

**Linux** — apt · dnf · pacman · zypper:

```bash
curl -fsSLO https://raw.githubusercontent.com/everitosan/Karto/main/utils/scripts/install_clients.sh
bash install_clients.sh
```

**macOS** — requiere [Homebrew](https://brew.sh):

```bash
curl -fsSLO https://raw.githubusercontent.com/everitosan/Karto/main/utils/scripts/install_clients.sh
bash install_clients.sh
```

Si ya tienes el repositorio clonado, el script está en `utils/scripts/` y en
Linux/macOS hay atajos: `make check-clients` y `make install-clients`.

Opciones:

| Para | Linux / macOS | Windows |
| --- | --- | --- |
| Ver qué falta, sin instalar | `--check` | `-Check` |
| Listar los grupos | `--list` | `-List` |
| Instalar solo unos grupos | `--only pg,redis` | `-Only pg,redis` |
| Desatendido, sin preguntar | `--yes` | `-Yes` |

### ¿Qué se instala?

| Grupo | Para qué | Linux (Debian/Ubuntu) | macOS (brew) | Windows |
| --- | --- | --- | --- | --- |
| `ssh` | Conectar por SSH, ejecutar scripts y aprovisionar llaves | `openssh-client` | ya viene | capacidad *OpenSSH Client* |
| `web` | Abrir URLs de administración web | `xdg-utils` | ya viene | ya viene |
| `terminal` | Terminal donde se abre la sesión SSH | `xterm` (o `gnome-terminal`, `konsole`, `kitty`, `alacritty`) | ya viene | Windows Terminal |
| `vnc` | Visor registrado para `vnc://` | `remmina` + `remmina-plugin-vnc` | ya viene | RealVNC Viewer |
| `pg` | Cliente `psql` | `postgresql-client` | `libpq` | `psql` (choco) o PostgreSQL (winget) |
| `mysql` | Cliente `mysql` (MySQL y MariaDB) | `default-mysql-client` | `mysql-client` | `mysql-cli` (choco) o MariaDB (winget) |
| `mongo` | Cliente `mongosh` | repo oficial de MongoDB | `mongosh` | zip oficial de MongoDB |
| `redis` | Cliente `redis-cli` | `redis-tools` | `redis` | zip de [redis-windows](https://github.com/redis-windows/redis-windows) |

Detalles de Windows que conviene saber:

- Si tienes **[Chocolatey](https://chocolatey.org/install)**, `psql` y `mysql` salen
  de sus paquetes cliente-solo (`psql` y `mysql-cli`): el binario y nada más. Es la
  vía recomendada, y el script la prefiere sola si detecta `choco`.
- Sin Chocolatey se cae a winget, que de esos dos **solo publica el instalador
  completo** — instala el servidor además del cliente. En ese caso el script añade
  su `bin` al `PATH` de usuario, cosa que esos instaladores no hacen.
- `mongosh` y `redis-cli` no tienen paquete en winget. Chocolatey sí (`mongodb-shell`
  y `redis`), pero el script baja el zip oficial a `%LOCALAPPDATA%\Karto\tools`: para
  mongosh es exactamente el mismo archivo que empaqueta choco, y para redis-cli es
  una versión bastante más nueva (8.x contra la 6.2 de choco). Así queda igual con
  Chocolatey o sin él.
- `ssh-copy-id` no existe en Windows; el aprovisionamiento de llaves usa otra ruta.
- El runtime de Windows sigue en curso: ver [windows-adapt.md](docs/specs/windows-adapt.md).

### En el equipo destino

- **SSH / bash / python**: `openssh-server` corriendo y `bash` (scripts bash) o
  `python3` (scripts python) instalados en el host.
- **Bases de datos**: solo el servidor de BD escuchando en su puerto. Karto usa el
  cliente **local** por red (Modelo B), así que **no** hace falta cliente en el host
  (funciona con BD gestionadas/remotas sin shell).

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

**Linux** — dependencias de sistema de Tauri (las mismas que instala CI):

```bash
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev \
  patchelf libgtk-3-dev build-essential file
```

**Windows** — además de lo anterior, el backend no compila sin:

| Requisito | Por qué |
| --- | --- |
| Visual Studio Build Tools (workload *Desktop development with C++*) | toolchain MSVC que usa Rust en Windows |
| WebView2 Runtime | motor web de Tauri (ya viene en Windows 11) |
| Perl (Strawberry Perl) y NASM, en el `PATH` | los exige `bundled-sqlcipher-vendored-openssl`, que compila OpenSSL desde fuente |

El soporte de **runtime** en Windows está en curso: ver
[windows-adapt.md](docs/specs/windows-adapt.md).

### Arranque rápido

```bash
pnpm install                             # instala todo el workspace
pnpm --filter @karto/desktop tauri:dev   # app de escritorio nativa
```

### Comandos

```bash
pnpm desktop          # (Vite) frontend de la app en el navegador
pnpm storybook        # Storybook en :6006
pnpm landing          # landing en dev
pnpm build            # build de todo vía turbo
pnpm test             # tests de todo vía turbo
pnpm lint             # lint de todo vía turbo
```

```bash
make help             # lista los targets disponibles
make version          # versión actual (fuente de verdad: Cargo.toml)
make check-clients    # qué clientes externos faltan
make deploy-rc-app    # publica un release candidate (tag + CI)
make deploy-app       # promueve el RC actual a versión estable
```

## Documentación

| Documento | De qué trata |
| --- | --- |
| [PLAN.md](docs/specs/PLAN.md) | Diseño completo, stack y fases del proyecto |
| [STRATEGY.md](docs/specs/STRATEGY.md) | Estrategia de producto |
| [MEASUREMENT.md](docs/specs/MEASUREMENT.md) | Qué se mide y cómo |
| [windows-adapt.md](docs/specs/windows-adapt.md) | Plan de adaptación del runtime a Windows |
| [CHANGELOG.md](CHANGELOG.md) | Cambios por versión ([Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) + [SemVer](https://semver.org/lang/es/)) |

## Contribuir

Los issues y PRs son bienvenidos. Un par de convenciones del repo:

- **Commits** con [Conventional Commits](https://www.conventionalcommits.org/es/):
  `feat(connections): …`, `fix(vault): …`, `chore(windows): …`.
- **Changelog**: anota el cambio bajo `## [Unreleased]` en [CHANGELOG.md](CHANGELOG.md).
  Los scripts de release son los que cierran las secciones versionadas.
- **Antes de abrir el PR**: `pnpm lint && pnpm test`.

## Licencia

[AGPL-3.0-only](LICENSE). Si distribuyes una versión modificada, o la ofreces como
servicio en red, el código fuente de tus cambios también debe estar disponible.
