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

Karto dibuja tu infraestructura en un lienzo: routers, servidores, bases de datos y
sus relaciones, cada uno con su metadata (IPs, hostnames, puertos, apps) y sus
credenciales. Todo vive cifrado en local tras una contraseña maestra, y desde el
mismo diagrama te conectas al equipo o le ejecutas un script.

## Características

- Información encriptada por medio de una llave maestra
- Diagramas con nodos desde routers hasta bases de datos con su metadata
- Administración de contraseñas y llaves para cada nodo (web, vnc, ssh, etc ...)
- Conexión en base a credenciales
- Ejecución de scripts en los nodos de forma secuencial y paralela
- Autobloqueo por inactividad
- Limpieza de portapapeles para evitar filtraciones

## Instalación

> **Estado:** `0.1.0-rc.1`. CI publica binarios de **Linux**; en Windows y macOS se
> compila desde fuente mientras se cierra la Fase 7 (ver [PLAN.md](docs/specs/PLAN.md)).

**Linux** — descarga el `.deb` o el `.AppImage` de [Releases](https://github.com/everitosan/Karto/releases):

```bash
sudo apt install ./Karto_*_amd64.deb        # Debian / Ubuntu
# o, sin instalar:
chmod +x Karto_*_amd64.AppImage && ./Karto_*_amd64.AppImage
```

**Windows y macOS** — todavía sin binario publicado: se compila desde fuente, ver
[Desarrollo](#desarrollo).

## Clientes externos

Karto no empaqueta clientes: invoca las herramientas del sistema. Todo esto se
instala en **la máquina donde corre Karto**; el equipo destino solo necesita lo
del último apartado.

### Instalación

`utils/scripts/install_clients.*` revisa qué falta en el `PATH` y lo instala con el
gestor nativo del SO. Es idempotente (lo que ya tengas no se toca), muestra el plan
y pide confirmación antes de instalar nada.

**Windows** — PowerShell **como administrador** (lo exige el cliente OpenSSH):

```powershell
powershell -ExecutionPolicy Bypass -File .\utils\scripts\install_clients.ps1
```

**Linux** — apt · dnf · pacman · zypper:

```bash
bash utils/scripts/install_clients.sh
```

**macOS** — requiere [Homebrew](https://brew.sh):

```bash
bash utils/scripts/install_clients.sh
```

Opciones (en Linux/macOS también `make check-clients` y `make install-clients`):

| Para | Linux / macOS | Windows |
| --- | --- | --- |
| Ver qué falta, sin instalar | `--check` | `-Check` |
| Listar los grupos | `--list` | `-List` |
| Instalar solo unos grupos | `--only pg,redis` | `-Only pg,redis` |
| Desatendido, sin preguntar | `--yes` | `-Yes` |

### Qué instala

| Grupo | Para qué | Linux (Debian/Ubuntu) | macOS (brew) | Windows |
| --- | --- | --- | --- | --- |
| `ssh` | Conectar por SSH, ejecutar scripts y aprovisionar llaves | `openssh-client` | ya viene | capacidad *OpenSSH Client* |
| `web` | Abrir URLs de administración web | `xdg-utils` | ya viene | ya viene |
| `terminal` | Terminal donde se abre la sesión SSH | `xterm` (o `gnome-terminal`, `konsole`, `kitty`, `alacritty`) | ya viene | Windows Terminal |
| `vnc` | Visor registrado para `vnc://` | `remmina` + `remmina-plugin-vnc` | ya viene | RealVNC Viewer |
| `pg` | Cliente `psql` | `postgresql-client` | `libpq` | PostgreSQL 17 |
| `mysql` | Cliente `mysql` (MySQL y MariaDB) | `default-mysql-client` | `mysql-client` | MariaDB |
| `mongo` | Cliente `mongosh` | repo oficial de MongoDB | `mongosh` | zip oficial de MongoDB |
| `redis` | Cliente `redis-cli` | `redis-tools` | `redis` | zip de [redis-windows](https://github.com/redis-windows/redis-windows) |

Detalles de Windows que conviene saber:

- `psql` y `mysql` llegan **con su servidor**: winget no tiene paquete cliente-solo.
  El script añade su `bin` al `PATH` de usuario, que los instaladores no hacen.
- `mongosh` y `redis-cli` no tienen paquete en winget: se bajan del zip oficial a
  `%LOCALAPPDATA%\Karto\tools`.
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
