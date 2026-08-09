# Karto

Mapeo visual de infraestructura en un canvas, con inventario cifrado (IPs, hostnames,
credenciales, puertos), organización por carpetas de proyecto/ambiente y conexión directa
a los equipos con doble click (SSH/VNC/RDP/web). App de escritorio multiplataforma.

Ver [PLAN.md](PLAN.md) para el diseño completo y las fases.

## Monorepo

Turborepo + pnpm workspaces.

```
apps/
  desktop/    App Tauri 2 + Svelte 5 (producto principal, backend Rust en src-tauri/)
  storybook/  Storybook que documenta @karto/ui
  landing/    Sitio de presentación (Astro)
packages/
  ui/         @karto/ui — componentes Svelte compartidos
```

## Requisitos

- Node ≥ 20 y pnpm
- Rust (cargo) para la app de escritorio
- Dependencias de sistema de Tauri (Linux: webkit2gtk-4.1, gtk3, libsoup3)

## Comandos

```bash
pnpm install          # instala todo el workspace
pnpm desktop          # (Vite) frontend de la app en el navegador
pnpm --filter @karto/desktop tauri:dev   # app de escritorio nativa
pnpm storybook        # Storybook en :6006
pnpm landing          # landing en dev
pnpm build            # build de todo vía turbo
pnpm test             # tests de todo vía turbo
```

## Convenciones

Clean Architecture y módulos pequeños/testeables. En el frontend: `Views/` para pantallas,
`components/` para lo compartido, componentes locales dentro del directorio de su View;
lógica de negocio en `domain/` y `usecases/`, fuera de la UI.
