# Karto — Estrategia de landing

Concepto rector: **la carta astronómica de tu infraestructura**.
Tu red no es una lista de IPs: es un cielo con estrellas (equipos), constelaciones
(diagramas), y un observatorio cifrado desde el que las miras y las alcanzas.

## 1. Posicionamiento

**Una frase:** Karto es el mapa vivo y cifrado de tu infraestructura — dibujas tu red,
guardas sus credenciales bajo cifrado real, y te conectas a cualquier equipo con un click.

**Categoría que ocupamos:** no competimos como "otro gestor SSH" (Termius, Royal TS,
mRemoteNG) ni como "otra herramienta de diagramas" (draw.io + notas). Somos la fusión:
*el diagrama ES el inventario ES el acceso*. Ese triángulo es el mensaje; nadie más lo tiene
en una sola app local.

**Pilares (en orden de venta):**
1. **Visual** — mapeas la infraestructura en un canvas; la entiendes de un vistazo.
2. **Acción** — el mapa no es documentación muerta: click y estás dentro (SSH/VNC/RDP/web).
3. **Confianza** — todo vive en UN archivo `.karto` cifrado (SQLCipher + Argon2id), local,
   sin nube, sin cuenta, portable. Aquí está la diferencia real frente a Termius (cloud sync).

**Audiencias (por prioridad):**
- Sysadmin / DevOps / SRE con parque heterogéneo (servidores, routers, BDs, VPS sueltos).
- Consultor / freelance que gestiona infra de varios clientes → export selectivo y vault
  por cliente son oro para él.
- Homelab / self-hosted (r/homelab, r/selfhosted): early adopters ruidosos, Linux-first
  encaja perfecto con ellos. Probablemente la primera comunidad que lo adopte.

## 2. Narrativa (el arco de la página)

La página cuenta una historia en 5 actos, de emoción → función → confianza → prueba → acción:

1. **El cielo** (hero): asombro. Tu infraestructura como constelación viva.
2. **Cartografía** : dibujas el mapa (canvas, categorías, zonas, iconos de marca).
3. **Navegación**: el mapa te lleva (conectar con un click, salud de nodos, sondeo
   automático de datos del equipo, búsqueda global, direcciones por contexto de red).
4. **El observatorio** (seguridad): un solo archivo cifrado, local, portable; los cuatro
   puntos de seguridad ya escritos funcionan bien aquí.
5. **Despega** (CTA final): descarga para Linux (mac/Windows "en órbita próxima").

## 3. Copy (ES primero; EN cuando haya release)

- **Hero H1:** `Tu infraestructura es un universo. Kartografíala.`
  - Alternativa más literal/SEO: `El mapa vivo de tu infraestructura.`
- **Subhead:** `Dibuja tu red como una carta estelar, guarda cada credencial en un vault
  cifrado y conéctate a cualquier equipo con un click. Local, sin nube, un solo archivo.`
- **CTA primario:** `Descargar para Linux` · **CTA secundario:** `Ver el mapa en acción`
  (scroll a demo/screenshot).
- Titulares de sección con vocabulario astronómico pero cuerpo técnico y concreto
  (la metáfora vive en títulos y visual; el body habla claro a un sysadmin):
  - "Cada equipo, una estrella" → catálogo de ~40 tipos, iconos de marca a color, zonas VPC.
  - "Constelaciones, no listas" → diagramas por proyecto/ambiente, carpetas, búsqueda global.
  - "Del mapa al equipo en un click" → SSH/VNC/RDP/web, onboarding de llave SSH, salud TCP.
  - "Un observatorio, no una nube" → sección de seguridad actual (ya está bien escrita).
- **Microcopy con guiño:** footer `Hecho en la Tierra por evesan` · badge del hero
  `v0.x — visible desde Linux` · 404 futura: `Este sector del mapa aún no está cartografiado`.
- **Regla de tono:** la metáfora nunca reemplaza al dato. Cada claim visual va acompañado
  del hecho técnico (AES-256, Argon2id, `.karto` portable, sin telemetría). El público
  es escéptico profesional: la poesía abre, la precisión cierra.

## 4. Dirección visual — jugar la carta astronómica

La base ya es cielo nocturno: degradado `#090d15 → #000` y acento verde `#11b245`
(el verde funciona como "señal viva": terminal, aurora, nodo saludable). No hay que
cambiar tokens, hay que amueblar el cielo:

1. **Campo de estrellas** de fondo: puntos diminutos con 2–3 opacidades y un `twinkle`
   sutil en CSS (sin JS, `prefers-reduced-motion` lo apaga). Densidad baja: elegancia,
   no screensaver.
2. **Hero = constelación de infra**: un SVG inline con 6–8 nodos (server, db, router,
   firewall con sus iconos) unidos por líneas finas tipo carta estelar, con etiquetas
   pequeñas estilo catálogo astronómico (`web-01 · 10.0.0.4`). Animación: las líneas se
   "trazan" (`stroke-dashoffset`) al cargar y un pulso verde recorre una arista cada pocos
   segundos (el health check como latido). Esto ES el producto y ES la metáfora a la vez.
   - Reuso real: los iconos/`catalog` de `@karto/ui` ya están en la landing.
3. **Retícula de carta náutica/astronómica**: círculos concéntricos y marcas de grados muy
   tenues (~4% opacidad) detrás del hero. Es el "instrumento" que separa esto de un
   genérico "espacio con estrellas".
4. **Cards de features como "fichas de catálogo estelar"**: borde fino, un glifo-nodo con
   halo (`box-shadow` verde suave), numeración tipo `KRT-001`.
5. **Scroll storytelling ligero**: entre acto 2 y 3, la misma constelación pasa de
   "dibujada" a "activa" (nodos encienden su punto de salud verde). Con IntersectionObserver
   básico; nada de librerías de scroll pesadas.
6. **Screenshot real de la app** en el acto 3, enmarcado como visor de telescopio (borde
   redondeado, viñeta sutil). Producto real > ilustración; genera confianza.

**Anti-patrones a evitar:** planetas/cohetes cartoon (infantiliza), morados sci-fi genéricos
(rompe la marca verde/azul), parallax agresivo, y saturar de metáfora el body copy.

## 5. Arquitectura de la página (orden de secciones)

1. Hero (constelación animada + H1 + CTA + badge Linux).
2. Barra de "hechos duros": `1 archivo cifrado · 0 nube · 0 cuenta · 3 SO (Linux hoy)`.
3. Features visuales (mapear): canvas, catálogo, zonas, iconos de marca.
4. Features de acción (conectar): 1-click SSH/VNC/RDP/web, salud, sondeo, contextos de red,
   import de `~/.ssh/config` ("tu cielo actual, importado en segundos").
5. Seguridad ("Seguro desde el primer byte" — mantener, ya es fuerte).
6. Para quién: 3 mini-perfiles (sysadmin / consultor / homelab) con un caso de uso cada uno.
7. CTA final + FAQ corta (¿y si olvido la contraseña? ¿hay telemetría? ¿mac/Windows cuándo?
   ¿es open source?).
8. Footer con `by evesan` (GitHub/LinkedIn/evesan.rocks — consistente con el About de la app).

## 6. SEO / distribución (mínimo viable)

- Title: `Karto — Mapa visual y cifrado de tu infraestructura (SSH, VNC, RDP)`.
- Meta description con las keywords reales de búsqueda: *ssh manager linux*, *alternativa a
  Termius sin nube*, *inventario de servidores cifrado*, *mapa de red con credenciales*.
- OpenGraph image = la constelación del hero (se comparte sola en HN/Reddit).
- Lanzamiento natural: r/selfhosted, r/homelab, HN "Show HN", con el ángulo
  "local-first, un solo archivo cifrado, sin cuenta" (ese ángulo gana en esas comunidades).

## 7. Qué cambia respecto a la landing actual

La landing actual ya tiene el esqueleto correcto (hero + features + seguridad) y los tokens
correctos. Lo que falta es: (a) el hero emocional con la constelación (hoy es logo + frase),
(b) corregir "Doble click y dentro" — el doble click se descartó por decisión de UX; debe
decir "Un click y dentro" o "Conectar sin teclear", (c) los actos 3–7 (acción, perfiles,
FAQ, footer), y (d) el fondo estelar/retícula. El copy de seguridad se conserva casi íntegro.
