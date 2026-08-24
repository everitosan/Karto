# Changelog

Todos los cambios notables de Karto se documentan aquí.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el versionado sigue [SemVer](https://semver.org/lang/es/).
Los pre-releases usan el sufijo `-rc.N` (ej. `0.1.0-rc.1`).

> **Cómo se usa:** anota los cambios bajo `## [Unreleased]` en las subsecciones
> `Added` / `Changed` / `Fixed` / `Removed`. `make deploy-rc-app` publica esas
> notas como prerelease sin vaciarlas; `make deploy-app` las cierra en una
> sección versionada.

## [Unreleased]

### Added

- **Mapa de infraestructura en lienzo**: catálogo de nodos con 11 categorías
  (Red, Seguridad, Identidad, Cómputo, Datastore…), iconos de marca a color y
  agrupadores visuales (zonas) para organizar el diagrama.
- **Vault cifrado**: base de datos SQLCipher protegida con contraseña maestra;
  cambio de contraseña, backup cifrado (exportación) y bloqueo/desbloqueo del mapa.
- **Auto-bloqueo por inactividad** y **limpieza automática del portapapeles**,
  ambos configurables.
- **Conexiones SSH** con llave o contraseña interactiva, opciones SSH por
  credencial y credenciales por nodo; apertura de servicios web/admin con el
  navegador del sistema.
- **Contextos de acceso** (Oficina, VPN, …): cada nodo puede tener direcciones
  distintas según el punto de vista de red, con selector de contexto activo.
- **Importación de `~/.ssh/config`**: descubrimiento recursivo bajo `~/.ssh` y
  asistente en dos etapas (origen → selección de hosts).
- **Onboarding de llave SSH**: aprovisiona una llave en el equipo remoto para
  pasar de contraseña a autenticación por llave.
- **Exportación de subconjuntos**: exporta los nodos seleccionados y sus aristas,
  con contenido opt-in (credenciales, direcciones por contexto).
- **Conexiones a bases de datos** y soporte de scripting/configuración.
- **Recientes y atajos** persistentes entre arranques; pantalla de bienvenida
  con la lista de mapas recientes.
- **Diagnóstico configurable**: registro en `karto.log` (nivel info/warning/error)
  que nunca guarda secretos ni direcciones.
- **Internacionalización es/en** con detección del idioma del sistema operativo y
  selector de idioma en Configuración.

### Security

- **CSP estricta** del webview (se elimina `csp: null`).
- **Argon2id** como derivación de clave por defecto del vault, entregando la
  clave a SQLCipher en modo raw key.
- **Bloqueo de opciones SSH peligrosas** para evitar RCE al abrir un vault
  compartido; las plantillas de un vault importado no se ejecutan en silencio.
- **Materialización de llaves privadas** con permisos restrictivos (dir `0700`,
  archivo `0600`).

### Fixed
