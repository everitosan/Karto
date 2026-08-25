// Contenido de la landing en los dos idiomas soportados.
// Añadir un idioma = añadir una clave a `content` y una ruta en src/pages/.
import { icons } from "@karto/ui";

export type Locale = "es" | "en";
export const locales: Locale[] = ["es", "en"];
export const defaultLocale: Locale = "es";

// Cada idioma comparte la misma forma; los iconos y los ids no se traducen.
export const content = {
  es: {
    lang: "es",
    // Ruta a la que apunta el conmutador de idioma (hacia el "otro" idioma)
    altLocale: "en" as Locale,
    altHref: "/en/",
    altLabel: "English",
    dir: "ltr",
    meta: {
      title: "Karto — Mapa visual y cifrado de tu infraestructura (SSH, VNC, RDP)",
      description:
        "Dibuja tu red como una carta estelar, guarda cada credencial en un vault cifrado (AES-256 + Argon2id) y conéctate por SSH, VNC o RDP con un click. Local, sin nube, un solo archivo. Alternativa a Termius sin sincronización en la nube.",
      ogTitle: "Karto — el mapa vivo y cifrado de tu infraestructura",
      ogDescription:
        "El diagrama es el inventario y es el acceso. Un solo archivo cifrado, local, sin cuenta.",
    },
    badge: "v0.x — visible desde Linux",
    hero: {
      titleA: "Tu infraestructura es un universo.",
      titleEm: "Kartografíala.",
      sub: "Diagrama tu red como una carta estelar. Karto te permite guardar cada credencial en un vault cifrado y conectarte a cualquier equipo con un click.",
      subStrong: "Local, seguro y portable.",
      ctaPrimary: "Descargar para Linux",
      ctaGhost: "Ver el mapa en acción",
    },
    facts: [
      { n: "1", label: "archivo cifrado" },
      { n: "0", label: "nube" },
      { n: "0", label: "cuentas" },
      { n: "3", label: "SO (Linux hoy)" },
    ],
    mapAct: {
      label: "Acto I · Cartografía",
      title: "Dibuja tu mapa",
      sub: "El lienzo se vuelve tu guía para cualquier entorno.",
    },
    mapFeatures: [
      {
        id: "KRT-001",
        icon: icons.diagram,
        title: "Cada nodo, una estrella",
        body: "Usa el canvas libre con ~40 tipos de nodos. Especifica tus servidores, routers, bases de datos, firewalls, cámaras, etc.; entenderás todo al primer vistazo.",
      },
      {
        id: "KRT-002",
        icon: icons.folder,
        title: "Constelaciones, no listas",
        body: "Un diagrama por proyecto o ambiente, organizados en carpetas. La búsqueda global encuentra cualquier equipo por nombre, IP o etiqueta, esté en el mapa que esté.",
      },
      {
        id: "KRT-003",
        icon: icons.address,
        title: "Zonas y contextos de red",
        body: "Agrupa nodos por VPC o segmento y define direcciones por contexto: el mismo equipo responde a su IP pública desde fuera y a la privada desde dentro.",
      },
    ],
    navAct: {
      label: "Acto II · Navegación",
      title: "El mapa te lleva",
      sub: "La diferencia entre documentar tu red y navegarla: aquí el diagrama abre sesiones.",
    },
    igniteCaption: "Los nodos encienden su señal cuando el equipo responde.",
    igniteAria: "Mapa de red cuyos nodos encienden su luz de salud",
    actionFeatures: [
      {
        id: "KRT-004",
        icon: icons.connect,
        title: "Un click y estás dentro",
        body: "SSH, VNC, RDP o web directo desde el nodo, sin teclear credenciales. El mapa no es documentación muerta: es la puerta de entrada.",
      },
      {
        id: "KRT-005",
        icon: icons.terminal,
        title: "Tus registros actuales",
        body: "Importa tu ~/.ssh/config en segundos y sube tu llave a equipos nuevos desde la propia app, así empiezas con el mapa de lo que ya tienes.",
      },
      {
        id: "KRT-006",
        icon: icons.eye,
        title: "Salud a simple vista",
        body: "Al conectarse, Karto sondea datos del equipo (SO, hostname, discos) y los guarda en su ficha; también puedes comprobar si un nodo es alcanzable desde tu red actual.",
      },
    ],
    secAct: {
      label: "Acto III · El observatorio",
      title: "Un observatorio bien protegido",
      sub: "Seguro desde el primer byte: Karto guarda tus accesos como un gestor de secretos, no como un bloc de notas.",
    },
    security: [
      {
        icon: icons.lock,
        title: "Cifrado de grado alto",
        body: "Tu vault se cifra con AES-256 (SQLCipher) y la clave se deriva con Argon2id, resistente a ataques por GPU. La contraseña maestra nunca se guarda.",
      },
      {
        icon: icons.eyeOff,
        title: "Tu secreto no sale de tu equipo",
        body: "Las credenciales nunca llegan a la interfaz, ni a la línea de comandos, ni a los logs de soporte. Solo se revelan con una acción explícita tuya.",
      },
      {
        icon: icons.key,
        title: "A prueba de vaults maliciosos",
        body: "Bloquea las opciones SSH que ejecutan comandos y te pide confirmación antes de correr plantillas de un vault de terceros. Abrir un archivo ajeno no ejecuta código.",
      },
      {
        icon: icons.settings,
        title: "Aislamiento y superficie mínima",
        body: "Interfaz con Content-Security-Policy estricta, permisos con lista blanca, enlaces limitados a http/https y llaves SSH con permisos restrictivos en disco.",
      },
    ],
    audAct: {
      label: "Tripulación",
      title: "¿Quién navega con Karto?",
    },
    audiences: [
      {
        label: "Sysadmin / SRE",
        title: "Un parque heterogéneo, un solo mapa",
        body: "Servidores, routers, BDs y VPS sueltos dejan de vivir en tu cabeza. Dibujas la topología real, y desde ella entras a cualquier equipo.",
      },
      {
        label: "Consultor / freelance",
        title: "Un archivo .karto por cliente",
        body: "Cada cliente es un vault independiente y portable. Lo llevas contigo, lo respaldas como cualquier archivo y entregas solo lo que toca.",
      },
      {
        label: "Homelab / self-hosted",
        title: "Local-first de verdad",
        body: "Sin cuenta, sin nube, sin telemetría. Tu lab entero en un archivo que es tuyo, como debe ser todo lo self-hosted.",
      },
    ],
    launch: {
      title: "Despega",
      subA: "Descarga Karto, crea tu primer archivo",
      subB: "y dibuja tu cielo.",
      cta: "Descargar para Linux",
      note: "mac y Windows en órbita próxima · sin telemetría",
    },
    download: {
      // Textos del selector de descargas (build + refresco en cliente).
      versionLabel: "Versión",
      prerelease: "Release candidate",
      yourSystem: "Tu sistema",
      comingSoon: "Próximamente",
      viewAll: "Ver todos los releases en GitHub",
      errorTitle: "No pudimos cargar las descargas.",
      errorCta: "Ábrelas en GitHub",
      heroDownload: "Descargar para",
      heroFallback: "Ver descargas",
      osNames: { linux: "Linux", mac: "macOS", windows: "Windows" },
      formatHints: {
        AppImage: "Universal · sin instalar",
        deb: "Debian · Ubuntu",
        rpm: "Fedora · RHEL",
        Flatpak: "Flatpak",
        dmg: "macOS",
        msi: "Instalador",
        exe: "Instalador",
      },
    },
    faq: [
      {
        q: "¿Y si olvido la contraseña maestra?",
        a: "No hay recuperación posible: el cifrado es real y no existe puerta trasera. La contraseña nunca se guarda en ningún lado, así que consérvala en un lugar seguro (por ejemplo, tu gestor de contraseñas personal).",
      },
      {
        q: "¿Hay telemetría?",
        a: 'Ninguna. Karto no abre conexiones de red salvo las que tú pides hacia tus propios equipos. No hay cuenta, no hay analytics, no hay "llamadas a casa".',
      },
      {
        q: "¿Cuándo para mac y Windows?",
        a: "En órbita próxima. El núcleo ya es multiplataforma; estamos cerrando la experiencia completa en Linux antes de encender los otros dos sistemas.",
      },
      {
        q: "¿Es open source?",
        a: "Sí, bajo licencia AGPL-3.0: puedes leer, auditar y modificar el código en GitHub. Si distribuyes una variación (o la ofreces como servicio), tu código también debe ser abierto. Para un uso comercial con código cerrado, hablemos de una licencia aparte.",
      },
    ],
    footer: {
      madeIn: "Hecho en la Tierra por",
    },
  },

  en: {
    lang: "en",
    altLocale: "es" as Locale,
    altHref: "/",
    altLabel: "Español",
    dir: "ltr",
    meta: {
      title: "Karto — A visual, encrypted map of your infrastructure (SSH, VNC, RDP)",
      description:
        "Draw your network as a star chart, keep every credential in an encrypted vault (AES-256 + Argon2id) and connect over SSH, VNC or RDP with one click. Local, no cloud, a single file. A Termius alternative without cloud sync.",
      ogTitle: "Karto — the living, encrypted map of your infrastructure",
      ogDescription:
        "The diagram is the inventory and it is the access. A single encrypted file, local, no account.",
    },
    badge: "v0.x — live on Linux",
    hero: {
      titleA: "Your infrastructure is a universe.",
      titleEm: "Chart it.",
      sub: "Chart your network as a star map. Karto lets you keep every credential in an encrypted vault and connect to any machine with one click.",
      subStrong: "Local, secure and portable.",
      ctaPrimary: "Download for Linux",
      ctaGhost: "See the map in action",
    },
    facts: [
      { n: "1", label: "encrypted file" },
      { n: "0", label: "cloud" },
      { n: "0", label: "accounts" },
      { n: "3", label: "OSes (Linux today)" },
    ],
    mapAct: {
      label: "Act I · Cartography",
      title: "Draw your map",
      sub: "The canvas becomes your guide for any environment.",
    },
    mapFeatures: [
      {
        id: "KRT-001",
        icon: icons.diagram,
        title: "Every node, a star",
        body: "Use the free-form canvas with ~40 node types. Lay out your servers, routers, databases, firewalls, cameras, and more; you'll take it all in at a glance.",
      },
      {
        id: "KRT-002",
        icon: icons.folder,
        title: "Constellations, not lists",
        body: "One diagram per project or environment, organized in folders. Global search finds any machine by name, IP or tag, no matter which map it lives on.",
      },
      {
        id: "KRT-003",
        icon: icons.address,
        title: "Network zones and contexts",
        body: "Group nodes by VPC or segment and define addresses per context: the same machine answers on its public IP from outside and its private one from within.",
      },
    ],
    navAct: {
      label: "Act II · Navigation",
      title: "The map takes you there",
      sub: "The difference between documenting your network and navigating it: here the diagram opens sessions.",
    },
    igniteCaption: "Nodes light up their signal when the machine responds.",
    igniteAria: "Network map whose nodes light up their health signal",
    actionFeatures: [
      {
        id: "KRT-004",
        icon: icons.connect,
        title: "One click and you're in",
        body: "SSH, VNC, RDP or web straight from the node, without typing credentials. The map isn't dead documentation: it's the front door.",
      },
      {
        id: "KRT-005",
        icon: icons.terminal,
        title: "Your existing records",
        body: "Import your ~/.ssh/config in seconds and push your key to new machines from within the app, so you start with a map of what you already have.",
      },
      {
        id: "KRT-006",
        icon: icons.eye,
        title: "Health at a glance",
        body: "On connect, Karto probes machine facts (OS, hostname, disks) and stores them on its card; you can also check whether a node is reachable from your current network.",
      },
    ],
    secAct: {
      label: "Act III · The observatory",
      title: "A well-guarded observatory",
      sub: "Secure from the first byte: Karto keeps your access like a secrets manager, not a notepad.",
    },
    security: [
      {
        icon: icons.lock,
        title: "High-grade encryption",
        body: "Your vault is encrypted with AES-256 (SQLCipher) and the key is derived with Argon2id, resistant to GPU attacks. The master password is never stored.",
      },
      {
        icon: icons.eyeOff,
        title: "Your secret never leaves your machine",
        body: "Credentials never reach the interface, the command line, or the support logs. They're revealed only by an explicit action of yours.",
      },
      {
        icon: icons.key,
        title: "Hardened against malicious vaults",
        body: "It blocks SSH options that run commands and asks for confirmation before running templates from a third-party vault. Opening someone else's file doesn't run code.",
      },
      {
        icon: icons.settings,
        title: "Isolation and minimal surface",
        body: "A strict Content-Security-Policy UI, allow-listed permissions, links limited to http/https, and SSH keys with restrictive permissions on disk.",
      },
    ],
    audAct: {
      label: "Crew",
      title: "Who navigates with Karto?",
    },
    audiences: [
      {
        label: "Sysadmin / SRE",
        title: "A heterogeneous fleet, a single map",
        body: "Servers, routers, DBs and stray VPSes stop living in your head. You draw the real topology, and from it you get into any machine.",
      },
      {
        label: "Consultant / freelancer",
        title: "One .karto file per client",
        body: "Each client is an independent, portable vault. You carry it with you, back it up like any file, and hand over only what's due.",
      },
      {
        label: "Homelab / self-hosted",
        title: "Truly local-first",
        body: "No account, no cloud, no telemetry. Your entire lab in a file that's yours, as everything self-hosted should be.",
      },
    ],
    launch: {
      title: "Lift off",
      subA: "Download Karto, create your first",
      subB: "file and draw your sky.",
      cta: "Download for Linux",
      note: "mac and Windows in near orbit · no telemetry",
    },
    download: {
      versionLabel: "Version",
      prerelease: "Release candidate",
      yourSystem: "Your system",
      comingSoon: "Coming soon",
      viewAll: "See all releases on GitHub",
      errorTitle: "We couldn't load the downloads.",
      errorCta: "Open them on GitHub",
      heroDownload: "Download for",
      heroFallback: "See downloads",
      osNames: { linux: "Linux", mac: "macOS", windows: "Windows" },
      formatHints: {
        AppImage: "Universal · no install",
        deb: "Debian · Ubuntu",
        rpm: "Fedora · RHEL",
        Flatpak: "Flatpak",
        dmg: "macOS",
        msi: "Installer",
        exe: "Installer",
      },
    },
    faq: [
      {
        q: "What if I forget the master password?",
        a: "There's no recovery: the encryption is real and there's no back door. The password is never stored anywhere, so keep it somewhere safe (your personal password manager, for example).",
      },
      {
        q: "Is there telemetry?",
        a: 'None. Karto opens no network connections other than the ones you ask for toward your own machines. No account, no analytics, no "phoning home".',
      },
      {
        q: "When do mac and Windows land?",
        a: "In near orbit. The core is already cross-platform; we're finishing the full experience on Linux before switching the other two systems on.",
      },
      {
        q: "Is it open source?",
        a: "Yes, under the AGPL-3.0 license: you can read, audit and modify the code on GitHub. If you distribute a variation (or offer it as a service), your code must be open too. For closed-source commercial use, let's talk about a separate license.",
      },
    ],
    footer: {
      madeIn: "Made on Earth by",
    },
  },
} satisfies Record<Locale, unknown>;

export type Content = (typeof content)[Locale];
