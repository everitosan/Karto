// Catálogo de nodos de infraestructura: categorías, tipos, propiedades
// sugeridas por tipo y resolución del icono (por propiedad vía Devicon, con
// respaldo a HugeIcons). El backend guarda `kind` como string libre y
// `properties` como mapa libre, así que este catálogo vive solo en el front.
import type { IconSvgElement } from "@hugeicons/svelte";
import {
  // Red
  InternetIcon,
  Router01Icon,
  DistributionIcon,
  Wifi01Icon,
  SatelliteIcon,
  ExchangeIcon,
  ConnectIcon,
  RepeatIcon,
  CloudServerIcon,
  // Seguridad
  FirewallIcon,
  ShieldIcon,
  SecurityCheckIcon,
  SquareLock02Icon,
  KeyIcon,
  // Identidad
  FingerPrintIcon,
  UserGroupIcon,
  // Cómputo
  ServerStack01Icon,
  CpuIcon,
  Layers01Icon,
  PackageIcon,
  DashboardSquare01Icon,
  FunctionIcon,
  ComputerIcon,
  // Aplicación
  ApiIcon,
  GlobalIcon,
  WorkflowSquare01Icon,
  // Datos / almacenamiento
  Database01Icon,
  CloudUploadIcon,
  HardDriveIcon,
  Archive01Icon,
  // Mensajería
  QueueIcon,
  MessageMultiple01Icon,
  // Observabilidad
  Analytics01Icon,
  ChartLineData01Icon,
  AnalyticsUpIcon,
  // Externo / cliente
  CloudIcon,
  UserIcon,
  LaptopIcon,
  SmartPhone01Icon,
  // Agrupadores
  FrameIcon,
} from "@hugeicons/core-free-icons";
import type { NodeCategory, NodeKind } from "./types";

/** Opción de una propiedad de tipo `select`. */
export interface PropertyOption {
  value: string;
  label: string;
  /** Clase base de Devicon (sin el prefijo `devicon-`), p. ej. "postgresql-plain". */
  icon?: string;
}

/** Definición de una propiedad sugerida de un tipo de nodo. */
export interface PropertySpec {
  key: string;
  label: string;
  type: "text" | "select";
  options?: PropertyOption[];
  placeholder?: string;
}

/** Metadatos de un tipo de nodo. */
export interface NodeSpec {
  kind: NodeKind;
  category: NodeCategory;
  label: string;
  /** Icono de respaldo (HugeIcons) cuando no hay icono por propiedad. */
  icon: IconSvgElement;
  /** ¿Es un nodo al que uno se conecta (plano 1) o solo contexto? */
  connectable: boolean;
  /** Propiedad cuyo valor decide el icono de marca (Devicon). */
  iconProperty?: string;
  properties: PropertySpec[];
}

export const CATEGORY_LABELS: Record<NodeCategory, string> = {
  network: "Red",
  security: "Seguridad",
  identity: "Identidad",
  compute: "Cómputo",
  application: "Aplicación",
  data: "Datos",
  storage: "Almacenamiento",
  messaging: "Mensajería",
  observability: "Observabilidad",
  external: "Externo",
  client: "Cliente / actor",
  grouping: "Agrupadores",
};

/** Orden de las categorías en la paleta. */
export const NODE_CATEGORIES: NodeCategory[] = [
  "network",
  "security",
  "identity",
  "compute",
  "application",
  "data",
  "storage",
  "messaging",
  "observability",
  "external",
  "client",
  "grouping",
];

// Helpers para declarar propiedades de forma compacta.
const t = (key: string, label: string, placeholder?: string): PropertySpec => ({
  key,
  label,
  type: "text",
  placeholder,
});
const sel = (
  key: string,
  label: string,
  options: PropertyOption[],
): PropertySpec => ({ key, label, type: "select", options });
const notas = t("notas", "Notas");

// Opciones reutilizables (valor → icono Devicon verificado en el paquete).
const DB_ENGINES: PropertyOption[] = [
  { value: "postgresql", label: "PostgreSQL", icon: "postgresql-plain" },
  { value: "mysql", label: "MySQL", icon: "mysql-original" },
  { value: "mariadb", label: "MariaDB", icon: "mariadb-original" },
  { value: "sqlite", label: "SQLite", icon: "sqlite-plain" },
  { value: "mongodb", label: "MongoDB", icon: "mongodb-plain" },
  { value: "redis", label: "Redis", icon: "redis-plain" },
  { value: "memcached", label: "Memcached", icon: "memcached-plain" },
  { value: "neo4j", label: "Neo4j", icon: "neo4j-plain" },
  { value: "elasticsearch", label: "Elasticsearch", icon: "elasticsearch-plain" },
  { value: "cassandra", label: "Cassandra", icon: "cassandra-plain" },
  { value: "influxdb", label: "InfluxDB", icon: "influxdb-plain" },
];
const APP_FRAMEWORKS: PropertyOption[] = [
  { value: "react", label: "React", icon: "react-original" },
  { value: "svelte", label: "Svelte", icon: "svelte-plain" },
  { value: "vue", label: "Vue", icon: "vuejs-plain" },
  { value: "angular", label: "Angular", icon: "angular-plain" },
  { value: "node", label: "Node.js", icon: "nodejs-plain" },
  { value: "python", label: "Python", icon: "python-plain" },
  { value: "go", label: "Go", icon: "go-original-wordmark" },
  { value: "rust", label: "Rust", icon: "rust-original" },
  { value: "java", label: "Java", icon: "java-plain" },
  { value: "spring", label: "Spring", icon: "spring-plain" },
  { value: "dotnet", label: ".NET", icon: "dot-net-plain" },
  { value: "php", label: "PHP", icon: "php-plain" },
];
const CLOUDS: PropertyOption[] = [
  { value: "aws", label: "AWS", icon: "amazonwebservices-plain-wordmark" },
  { value: "gcp", label: "Google Cloud", icon: "googlecloud-plain" },
  { value: "azure", label: "Azure", icon: "azure-plain" },
  { value: "cloudflare", label: "Cloudflare", icon: "cloudflare-plain" },
];

export const NODE_CATALOG: Record<NodeKind, NodeSpec> = {
  // ── Red ────────────────────────────────────────────────────────────────
  internet: {
    kind: "internet",
    category: "network",
    label: "Internet",
    icon: InternetIcon,
    connectable: false,
    properties: [t("asn", "ASN / rango"), notas],
  },
  router: {
    kind: "router",
    category: "network",
    label: "Router",
    icon: Router01Icon,
    connectable: true,
    properties: [
      t("modelo", "Marca / modelo"),
      t("firmware", "Firmware"),
      t("nat", "NAT (activo/reglas)"),
      t("vlans", "VLANs"),
      t("url_admin", "URL admin"),
      notas,
    ],
  },
  switch: {
    kind: "switch",
    category: "network",
    label: "Switch",
    icon: DistributionIcon,
    connectable: true,
    properties: [
      t("puertos", "Puertos (usados/total)"),
      t("vlans", "VLANs"),
      t("modelo", "Marca / modelo"),
      notas,
    ],
  },
  access_point: {
    kind: "access_point",
    category: "network",
    label: "Access Point",
    icon: Wifi01Icon,
    connectable: true,
    properties: [
      t("ssid", "SSID"),
      t("banda", "Banda / estándar"),
      t("controlador", "Controlador"),
      notas,
    ],
  },
  dns: {
    kind: "dns",
    category: "network",
    label: "DNS",
    icon: SatelliteIcon,
    connectable: true,
    properties: [
      t("rol", "Rol (autoritativo/recursivo)"),
      t("zonas", "Zonas / dominios"),
      t("upstreams", "Forwarders"),
      notas,
    ],
  },
  dhcp: {
    kind: "dhcp",
    category: "network",
    label: "DHCP",
    icon: ExchangeIcon,
    connectable: true,
    properties: [
      t("rango", "Scope / rango"),
      t("gateway", "Gateway entregado"),
      notas,
    ],
  },
  vpn: {
    kind: "vpn",
    category: "network",
    label: "VPN",
    icon: ConnectIcon,
    connectable: true,
    properties: [
      sel("protocolo", "Protocolo", [
        { value: "wireguard", label: "WireGuard" },
        { value: "openvpn", label: "OpenVPN" },
        { value: "ipsec", label: "IPsec" },
      ]),
      t("endpoint", "Endpoint público"),
      t("subredes", "Subredes que expone"),
      notas,
    ],
  },
  load_balancer: {
    kind: "load_balancer",
    category: "network",
    label: "Load Balancer",
    icon: RepeatIcon,
    connectable: true,
    iconProperty: "software",
    properties: [
      sel("software", "Software", [
        { value: "nginx", label: "nginx", icon: "nginx-original" },
        { value: "haproxy", label: "HAProxy" },
        { value: "traefik", label: "Traefik", icon: "traefikproxy-plain" },
      ]),
      sel("tipo", "Tipo", [
        { value: "l4", label: "L4" },
        { value: "l7", label: "L7" },
      ]),
      t("vip", "VIP + puertos"),
      t("backends", "Backends / pool"),
      notas,
    ],
  },
  cdn: {
    kind: "cdn",
    category: "network",
    label: "CDN",
    icon: CloudServerIcon,
    connectable: false,
    iconProperty: "proveedor",
    properties: [
      sel("proveedor", "Proveedor", [
        { value: "cloudflare", label: "Cloudflare", icon: "cloudflare-plain" },
        { value: "fastly", label: "Fastly" },
        { value: "akamai", label: "Akamai" },
      ]),
      t("dominios", "Dominios servidos"),
      t("origen", "Origen(es)"),
      notas,
    ],
  },

  // ── Seguridad ──────────────────────────────────────────────────────────
  firewall: {
    kind: "firewall",
    category: "security",
    label: "Firewall",
    icon: FirewallIcon,
    connectable: true,
    properties: [
      t("modelo", "Marca / modelo"),
      t("firmware", "Firmware"),
      t("zonas", "Zonas / interfaces"),
      t("url_admin", "URL admin"),
      notas,
    ],
  },
  waf: {
    kind: "waf",
    category: "security",
    label: "WAF",
    icon: ShieldIcon,
    connectable: false,
    properties: [
      sel("modo", "Modo", [
        { value: "block", label: "Bloqueo" },
        { value: "monitor", label: "Monitor" },
      ]),
      t("dominios", "Dominios / apps"),
      t("ruleset", "Ruleset"),
      t("origen", "Origen protegido"),
      notas,
    ],
  },
  ids_ips: {
    kind: "ids_ips",
    category: "security",
    label: "IDS / IPS",
    icon: SecurityCheckIcon,
    connectable: true,
    properties: [
      sel("modo", "Modo", [
        { value: "ids", label: "Detección (IDS)" },
        { value: "ips", label: "Prevención (IPS)" },
      ]),
      t("motor", "Motor (Suricata/Snort/Zeek)"),
      t("segmentos", "Segmentos monitorizados"),
      notas,
    ],
  },
  bastion: {
    kind: "bastion",
    category: "security",
    label: "Bastion / Jump host",
    icon: SquareLock02Icon,
    connectable: true,
    properties: [
      t("os", "SO + versión"),
      t("redes", "Redes/hosts destino"),
      t("metodo", "Método (SSH/MFA)"),
      notas,
    ],
  },
  secrets_manager: {
    kind: "secrets_manager",
    category: "security",
    label: "Gestor de secretos",
    icon: KeyIcon,
    connectable: true,
    properties: [
      t("producto", "Producto (Vault/1Password/…)"),
      t("endpoint", "Endpoint / URL"),
      t("auth", "Método de auth"),
      notas,
    ],
  },

  // ── Identidad ──────────────────────────────────────────────────────────
  idp: {
    kind: "idp",
    category: "identity",
    label: "Identity Provider",
    icon: FingerPrintIcon,
    connectable: false,
    iconProperty: "producto",
    properties: [
      sel("producto", "Producto", [
        { value: "okta", label: "Okta", icon: "okta-plain" },
        { value: "keycloak", label: "Keycloak" },
        { value: "auth0", label: "Auth0" },
        { value: "entra", label: "Entra ID" },
        { value: "oauth", label: "Genérico OAuth/OIDC", icon: "oauth-plain" },
      ]),
      t("protocolos", "Protocolos (OIDC/SAML)"),
      t("issuer", "URL / issuer"),
      notas,
    ],
  },
  directory: {
    kind: "directory",
    category: "identity",
    label: "Directorio (LDAP/AD)",
    icon: UserGroupIcon,
    connectable: true,
    properties: [
      sel("tipo", "Tipo", [
        { value: "ad", label: "Active Directory" },
        { value: "openldap", label: "OpenLDAP" },
        { value: "freeipa", label: "FreeIPA" },
      ]),
      t("dominio", "Dominio / base DN"),
      notas,
    ],
  },

  // ── Cómputo ────────────────────────────────────────────────────────────
  server: {
    kind: "server",
    category: "compute",
    label: "Servidor físico",
    icon: ServerStack01Icon,
    connectable: true,
    properties: [
      t("hostname", "Hostname / FQDN"),
      t("os", "SO + versión"),
      t("recursos", "CPU / RAM / disco"),
      t("ubicacion", "Ubicación (rack/DC)"),
      t("oob", "Gestión OOB (iLO/IPMI)"),
      notas,
    ],
  },
  vm: {
    kind: "vm",
    category: "compute",
    label: "Máquina virtual",
    icon: CpuIcon,
    connectable: true,
    properties: [
      t("hostname", "Hostname / FQDN"),
      t("host_padre", "Host / hypervisor padre"),
      t("os", "SO + versión"),
      t("recursos", "vCPU / RAM / disco"),
      notas,
    ],
  },
  hypervisor: {
    kind: "hypervisor",
    category: "compute",
    label: "Hypervisor host",
    icon: Layers01Icon,
    connectable: true,
    iconProperty: "plataforma",
    properties: [
      sel("plataforma", "Plataforma", [
        { value: "proxmox", label: "Proxmox", icon: "proxmox-plain" },
        { value: "esxi", label: "VMware ESXi" },
        { value: "hyperv", label: "Hyper-V" },
      ]),
      t("url_admin", "URL de gestión"),
      t("capacidad", "Capacidad"),
      notas,
    ],
  },
  container: {
    kind: "container",
    category: "compute",
    label: "Contenedor",
    icon: PackageIcon,
    connectable: false,
    iconProperty: "runtime",
    properties: [
      sel("runtime", "Runtime", [
        { value: "docker", label: "Docker", icon: "docker-plain" },
        { value: "podman", label: "Podman", icon: "podman-plain" },
      ]),
      t("imagen", "Imagen + tag"),
      t("host", "Host que lo ejecuta"),
      t("puertos", "Puertos publicados"),
      notas,
    ],
  },
  k8s_cluster: {
    kind: "k8s_cluster",
    category: "compute",
    label: "Cluster Kubernetes",
    icon: DashboardSquare01Icon,
    connectable: true,
    iconProperty: "distribucion",
    properties: [
      sel("distribucion", "Distribución", [
        { value: "kubernetes", label: "Kubernetes", icon: "kubernetes-plain" },
        { value: "k3s", label: "k3s", icon: "kubernetes-plain" },
        { value: "rancher", label: "Rancher", icon: "rancher-plain" },
        { value: "eks", label: "EKS", icon: "amazonwebservices-plain-wordmark" },
        { value: "gke", label: "GKE", icon: "googlecloud-plain" },
        { value: "aks", label: "AKS", icon: "azure-plain" },
      ]),
      t("endpoint", "API endpoint"),
      t("nodos", "Nº nodos / pools"),
      t("namespaces", "Namespaces clave"),
      notas,
    ],
  },
  serverless: {
    kind: "serverless",
    category: "compute",
    label: "Función serverless",
    icon: FunctionIcon,
    connectable: false,
    iconProperty: "plataforma",
    properties: [
      sel("plataforma", "Plataforma", CLOUDS),
      t("runtime", "Runtime"),
      t("trigger", "Trigger (HTTP/evento/cron)"),
      t("url", "URL"),
      notas,
    ],
  },
  generic: {
    kind: "generic",
    category: "compute",
    label: "Genérico",
    icon: ComputerIcon,
    connectable: true,
    properties: [t("hostname", "Hostname"), notas],
  },

  // ── Aplicación ─────────────────────────────────────────────────────────
  api_gateway: {
    kind: "api_gateway",
    category: "application",
    label: "API Gateway",
    icon: ApiIcon,
    connectable: false,
    properties: [
      t("producto", "Producto (Kong/APIM/…)"),
      t("rutas", "Rutas / endpoints"),
      t("auth", "Auth (keys/OAuth/JWT)"),
      t("upstreams", "Upstreams"),
      notas,
    ],
  },
  reverse_proxy: {
    kind: "reverse_proxy",
    category: "application",
    label: "Reverse Proxy",
    icon: ExchangeIcon,
    connectable: false,
    iconProperty: "software",
    properties: [
      sel("software", "Software", [
        { value: "nginx", label: "nginx", icon: "nginx-original" },
        { value: "traefik", label: "Traefik", icon: "traefikproxy-plain" },
        { value: "apache", label: "Apache", icon: "apache-plain" },
        { value: "haproxy", label: "HAProxy" },
      ]),
      t("escucha", "Host:puerto de escucha"),
      t("upstreams", "Backends"),
      t("dominios", "Vhosts / dominios"),
      notas,
    ],
  },
  web_server: {
    kind: "web_server",
    category: "application",
    label: "Web Server",
    icon: GlobalIcon,
    connectable: false,
    iconProperty: "software",
    properties: [
      sel("software", "Software", [
        { value: "nginx", label: "nginx", icon: "nginx-original" },
        { value: "apache", label: "Apache", icon: "apache-plain" },
      ]),
      t("url", "URL", "https://sitio.ejemplo.com"),
      t("sitios", "Sitios / vhosts"),
      t("puertos", "Puertos (80/443)"),
      notas,
    ],
  },
  application: {
    kind: "application",
    category: "application",
    label: "Aplicación",
    icon: WorkflowSquare01Icon,
    connectable: false,
    iconProperty: "framework",
    properties: [
      sel("framework", "Runtime / framework", APP_FRAMEWORKS),
      t("url", "URL", "https://app.ejemplo.com"),
      t("version", "Versión"),
      t("repo", "Repositorio"),
      t("puerto", "Puerto"),
      t("dependencias", "Dependencias (BD/cache/colas)"),
      notas,
    ],
  },
  worker: {
    kind: "worker",
    category: "application",
    label: "Worker / Job",
    icon: RepeatIcon,
    connectable: false,
    iconProperty: "runtime",
    properties: [
      sel("runtime", "Runtime", APP_FRAMEWORKS),
      sel("tipo", "Tipo", [
        { value: "cron", label: "Cron" },
        { value: "queue", label: "Cola" },
        { value: "stream", label: "Stream" },
      ]),
      t("origen", "Programación / cola origen"),
      notas,
    ],
  },

  // ── Datos ──────────────────────────────────────────────────────────────
  database: {
    kind: "database",
    category: "data",
    label: "Base de datos",
    icon: Database01Icon,
    connectable: true,
    iconProperty: "gestor",
    properties: [
      sel("gestor", "Gestor", DB_ENGINES),
      sel("modelo", "Modelo", [
        { value: "relacional", label: "Relacional" },
        { value: "documental", label: "Documental" },
        { value: "grafo", label: "Grafo" },
        { value: "keyvalue", label: "Clave-valor / caché" },
        { value: "busqueda", label: "Búsqueda" },
        { value: "timeseries", label: "Time-series" },
      ]),
      t("version", "Versión"),
      t("hostname", "Hostname / FQDN"),
      t("instancia", "Nombre BD / instancia"),
      t("replicacion", "Replicación / HA"),
      t("backup", "Backup (destino/frecuencia)"),
      notas,
    ],
  },

  // ── Almacenamiento ─────────────────────────────────────────────────────
  object_storage: {
    kind: "object_storage",
    category: "storage",
    label: "Object storage (S3)",
    icon: CloudUploadIcon,
    connectable: false,
    iconProperty: "proveedor",
    properties: [
      sel("proveedor", "Proveedor", CLOUDS),
      t("endpoint", "Endpoint"),
      t("bucket", "Bucket(s)"),
      notas,
    ],
  },
  nas: {
    kind: "nas",
    category: "storage",
    label: "NAS",
    icon: HardDriveIcon,
    connectable: true,
    properties: [
      t("protocolo", "Protocolo (NFS/SMB)"),
      t("capacidad", "Capacidad"),
      notas,
    ],
  },
  backup: {
    kind: "backup",
    category: "storage",
    label: "Backup / respaldos",
    icon: Archive01Icon,
    connectable: false,
    properties: [
      t("destino", "Destino"),
      t("frecuencia", "Frecuencia"),
      t("retencion", "Retención"),
      notas,
    ],
  },

  // ── Mensajería ─────────────────────────────────────────────────────────
  message_broker: {
    kind: "message_broker",
    category: "messaging",
    label: "Message broker / Cola",
    icon: QueueIcon,
    connectable: true,
    iconProperty: "producto",
    properties: [
      sel("producto", "Producto", [
        { value: "rabbitmq", label: "RabbitMQ", icon: "rabbitmq-original" },
        { value: "sqs", label: "Amazon SQS", icon: "amazonwebservices-plain-wordmark" },
        { value: "activemq", label: "ActiveMQ" },
      ]),
      t("hostname", "Hostname / FQDN"),
      t("colas", "Colas / exchanges"),
      notas,
    ],
  },
  event_streaming: {
    kind: "event_streaming",
    category: "messaging",
    label: "Event streaming",
    icon: MessageMultiple01Icon,
    connectable: true,
    iconProperty: "producto",
    properties: [
      sel("producto", "Producto", [
        { value: "kafka", label: "Apache Kafka", icon: "apachekafka-original" },
        { value: "pulsar", label: "Apache Pulsar" },
      ]),
      t("brokers", "Brokers / bootstrap"),
      t("topics", "Topics"),
      t("retencion", "Retención"),
      notas,
    ],
  },

  // ── Observabilidad ─────────────────────────────────────────────────────
  monitoring: {
    kind: "monitoring",
    category: "observability",
    label: "Monitoring",
    icon: Analytics01Icon,
    connectable: false,
    iconProperty: "producto",
    properties: [
      sel("producto", "Producto", [
        { value: "prometheus", label: "Prometheus", icon: "prometheus-original" },
        { value: "grafana", label: "Grafana", icon: "grafana-plain" },
        { value: "zabbix", label: "Zabbix" },
      ]),
      t("url", "URL"),
      t("targets", "Targets vigilados"),
      notas,
    ],
  },
  logging: {
    kind: "logging",
    category: "observability",
    label: "Logging",
    icon: ChartLineData01Icon,
    connectable: false,
    iconProperty: "stack",
    properties: [
      sel("stack", "Stack", [
        { value: "elastic", label: "Elastic (ELK)", icon: "elasticsearch-plain" },
        { value: "loki", label: "Loki", icon: "grafana-plain" },
        { value: "graylog", label: "Graylog" },
      ]),
      t("endpoint", "Endpoint de ingesta"),
      t("retencion", "Retención"),
      notas,
    ],
  },
  apm: {
    kind: "apm",
    category: "observability",
    label: "APM",
    icon: AnalyticsUpIcon,
    connectable: false,
    properties: [
      t("producto", "Producto (Datadog/Jaeger…)"),
      t("apps", "Apps instrumentadas"),
      t("endpoint", "Endpoint"),
      notas,
    ],
  },

  // ── Externo ────────────────────────────────────────────────────────────
  saas: {
    kind: "saas",
    category: "external",
    label: "SaaS",
    icon: CloudIcon,
    connectable: false,
    iconProperty: "proveedor",
    properties: [
      sel("proveedor", "Proveedor", CLOUDS),
      t("url", "URL"),
      t("proposito", "Propósito"),
      t("criticidad", "Criticidad"),
      notas,
    ],
  },
  third_party_api: {
    kind: "third_party_api",
    category: "external",
    label: "API de terceros",
    icon: ApiIcon,
    connectable: false,
    properties: [
      t("base_url", "Base URL"),
      t("auth", "Auth (key/OAuth)"),
      t("rate_limits", "Rate limits"),
      notas,
    ],
  },

  // ── Cliente / actor ────────────────────────────────────────────────────
  user: {
    kind: "user",
    category: "client",
    label: "Usuario / actor",
    icon: UserIcon,
    connectable: false,
    properties: [
      t("rol", "Rol / persona"),
      sel("origen", "Origen", [
        { value: "interno", label: "Interno" },
        { value: "externo", label: "Externo" },
      ]),
      notas,
    ],
  },
  workstation: {
    kind: "workstation",
    category: "client",
    label: "Workstation",
    icon: LaptopIcon,
    connectable: true,
    properties: [
      t("os", "SO"),
      t("usuario", "Usuario asignado"),
      notas,
    ],
  },
  mobile: {
    kind: "mobile",
    category: "client",
    label: "Móvil",
    icon: SmartPhone01Icon,
    connectable: false,
    properties: [
      sel("plataforma", "Plataforma", [
        { value: "android", label: "Android" },
        { value: "ios", label: "iOS" },
      ]),
      t("proposito", "Propósito"),
      notas,
    ],
  },

  // ── Agrupadores visuales ────────────────────────────────────────────────
  zone: {
    kind: "zone",
    category: "grouping",
    label: "Zona / área",
    icon: FrameIcon,
    connectable: false,
    properties: [
      sel("tipo", "Tipo", [
        { value: "zona", label: "Zona / ambiente" },
        { value: "vpc", label: "VPC" },
        { value: "subred", label: "Subred" },
        { value: "region", label: "Región" },
        { value: "datacenter", label: "Datacenter" },
      ]),
      t("cidr", "CIDR / rango"),
      sel("color", "Color", [
        { value: "slate", label: "Gris" },
        { value: "green", label: "Verde" },
        { value: "blue", label: "Azul" },
        { value: "amber", label: "Ámbar" },
        { value: "rose", label: "Rosa" },
        { value: "violet", label: "Violeta" },
      ]),
      notas,
    ],
  },
};

/** Todos los tipos en el orden de declaración del catálogo. */
export const NODE_KINDS = Object.keys(NODE_CATALOG) as NodeKind[];

/** Etiquetas legibles por tipo (para paleta y paneles). */
export const NODE_KIND_LABELS = Object.fromEntries(
  NODE_KINDS.map((k) => [k, NODE_CATALOG[k].label]),
) as Record<NodeKind, string>;

/** Icono de respaldo (HugeIcons) por tipo. */
export const nodeTypeIcon = Object.fromEntries(
  NODE_KINDS.map((k) => [k, NODE_CATALOG[k].icon]),
) as Record<NodeKind, IconSvgElement>;

/** Tipos agrupados por categoría, en el orden de la paleta. */
export function nodesByCategory(): {
  category: NodeCategory;
  label: string;
  kinds: NodeKind[];
}[] {
  return NODE_CATEGORIES.map((category) => ({
    category,
    label: CATEGORY_LABELS[category],
    kinds: NODE_KINDS.filter((k) => NODE_CATALOG[k].category === category),
  })).filter((g) => g.kinds.length > 0);
}

/** Icono resuelto de un nodo: de marca (Devicon) si la propiedad lo define, o el de respaldo. */
export type ResolvedNodeIcon =
  | { type: "devicon"; name: string }
  | { type: "hugeicon"; icon: IconSvgElement };

export function resolveNodeIcon(
  kind: NodeKind,
  properties: Record<string, string> | undefined,
): ResolvedNodeIcon {
  const spec = NODE_CATALOG[kind] ?? NODE_CATALOG.generic;
  const key = spec.iconProperty;
  if (key && properties?.[key]) {
    const option = spec.properties
      .find((p) => p.key === key)
      ?.options?.find((o) => o.value === properties[key]);
    if (option?.icon) return { type: "devicon", name: option.icon };
  }
  return { type: "hugeicon", icon: spec.icon };
}
