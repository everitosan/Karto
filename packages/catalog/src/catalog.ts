// Catálogo de nodos de infraestructura: categorías, tipos, propiedades
// sugeridas por tipo y resolución del icono (por propiedad vía Devicon, con
// respaldo a HugeIcons). El backend guarda `kind` como string libre y
// `properties` como mapa libre, así que este catálogo vive solo en el front.
//
// i18n: los textos visibles (`label`/`placeholder` de propiedad y las `label` de
// opción que sí se traducen) NO son español, son **slugs** (`cp_`/`cph_`/`co_`)
// que la app resuelve vía Paraglide en `$i18n/catalog` (`catalogText`). Los
// nombres propios (PostgreSQL, nginx, AWS…) quedan literales y caen al fallback.
// Las `label` de nodo y de categoría siguen en español como respaldo, pero la app
// las traduce por su id (`nodeKind_*`/`category_*`).
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
const notas = t("notas", "cp_notas");

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
// Backends as a Service: DB + auth + storage + functions en un solo producto.
// Viven en el nodo `baas`, no como motor de base de datos. PocketBase no tiene
// icono en Devicon: cae al icono genérico del nodo.
const BAAS_PROVIDERS: PropertyOption[] = [
  { value: "firebase", label: "Firebase", icon: "firebase-plain" },
  { value: "supabase", label: "Supabase", icon: "supabase-plain" },
  { value: "pocketbase", label: "PocketBase" },
  { value: "appwrite", label: "Appwrite", icon: "appwrite-original" },
];
const APP_FRAMEWORKS: PropertyOption[] = [
  { value: "react", label: "React", icon: "react-original" },
  { value: "nextjs", label: "Next.js", icon: "nextjs-plain" },
  { value: "svelte", label: "Svelte", icon: "svelte-plain" },
  { value: "astro", label: "Astro", icon: "astro-plain" },
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
    properties: [t("asn", "cp_asn_rango"), notas],
  },
  router: {
    kind: "router",
    category: "network",
    label: "Router",
    icon: Router01Icon,
    connectable: true,
    properties: [
      t("modelo", "cp_marca_modelo"),
      t("firmware", "cp_firmware"),
      t("nat", "cp_nat_activo_reglas"),
      t("vlans", "cp_vlans"),
      t("url_admin", "cp_url_admin"),
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
      t("puertos", "cp_puertos_usados_total"),
      t("vlans", "cp_vlans"),
      t("modelo", "cp_marca_modelo"),
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
      t("ssid", "cp_ssid"),
      t("banda", "cp_banda_estandar"),
      t("controlador", "cp_controlador"),
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
      t("rol", "cp_rol_autoritativo_recursivo"),
      t("zonas", "cp_zonas_dominios"),
      t("upstreams", "cp_forwarders"),
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
      t("rango", "cp_scope_rango"),
      t("gateway", "cp_gateway_entregado"),
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
      sel("protocolo", "cp_protocolo", [
        { value: "wireguard", label: "WireGuard" },
        { value: "openvpn", label: "OpenVPN" },
        { value: "ipsec", label: "IPsec" },
      ]),
      t("endpoint", "cp_endpoint_publico"),
      t("subredes", "cp_subredes_que_expone"),
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
      sel("software", "cp_software", [
        { value: "nginx", label: "nginx", icon: "nginx-original" },
        { value: "haproxy", label: "HAProxy" },
        { value: "traefik", label: "Traefik", icon: "traefikproxy-plain" },
      ]),
      sel("tipo", "cp_tipo", [
        { value: "l4", label: "L4" },
        { value: "l7", label: "L7" },
      ]),
      t("vip", "cp_vip_puertos"),
      t("backends", "cp_backends_pool"),
      notas,
    ],
  },
  cdn: {
    kind: "cdn",
    category: "network",
    label: "CDN / Edge",
    icon: CloudServerIcon,
    connectable: false,
    iconProperty: "proveedor",
    properties: [
      sel("proveedor", "cp_proveedor", [
        { value: "cloudflare", label: "Cloudflare", icon: "cloudflare-plain" },
        { value: "fastly", label: "Fastly" },
        { value: "akamai", label: "Akamai" },
      ]),
      t("dominios", "cp_dominios_servidos"),
      t("origen", "cp_origen_es"),
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
      t("modelo", "cp_marca_modelo"),
      t("firmware", "cp_firmware"),
      t("zonas", "cp_zonas_interfaces"),
      t("url_admin", "cp_url_admin"),
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
      sel("modo", "cp_modo", [
        { value: "block", label: "co_bloqueo" },
        { value: "monitor", label: "Monitor" },
      ]),
      t("dominios", "cp_dominios_apps"),
      t("ruleset", "cp_ruleset"),
      t("origen", "cp_origen_protegido"),
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
      sel("modo", "cp_modo", [
        { value: "ids", label: "co_deteccion_ids" },
        { value: "ips", label: "co_prevencion_ips" },
      ]),
      t("motor", "cp_motor_suricata_snort_zeek"),
      t("segmentos", "cp_segmentos_monitorizados"),
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
      t("os", "cp_so_version"),
      t("redes", "cp_redes_hosts_destino"),
      t("metodo", "cp_metodo_ssh_mfa"),
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
      t("producto", "cp_producto_vault_1password"),
      t("endpoint", "cp_endpoint_url"),
      t("auth", "cp_metodo_de_auth"),
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
      sel("producto", "cp_producto", [
        { value: "okta", label: "Okta", icon: "okta-plain" },
        { value: "keycloak", label: "Keycloak" },
        { value: "auth0", label: "Auth0" },
        { value: "entra", label: "Entra ID" },
        { value: "oauth", label: "co_generico_oauth_oidc", icon: "oauth-plain" },
      ]),
      t("protocolos", "cp_protocolos_oidc_saml"),
      t("issuer", "cp_url_issuer"),
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
      sel("tipo", "cp_tipo", [
        { value: "ad", label: "Active Directory" },
        { value: "openldap", label: "OpenLDAP" },
        { value: "freeipa", label: "FreeIPA" },
      ]),
      t("dominio", "cp_dominio_base_dn"),
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
      t("hostname", "cp_hostname_fqdn"),
      t("os", "cp_so_version"),
      t("recursos", "cp_cpu_ram_disco"),
      t("ubicacion", "cp_ubicacion_rack_dc"),
      t("oob", "cp_gestion_oob_ilo_ipmi"),
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
      t("hostname", "cp_hostname_fqdn"),
      t("host_padre", "cp_host_hypervisor_padre"),
      t("os", "cp_so_version"),
      t("recursos", "cp_vcpu_ram_disco"),
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
      sel("plataforma", "cp_plataforma", [
        { value: "proxmox", label: "Proxmox", icon: "proxmox-plain" },
        { value: "esxi", label: "VMware ESXi" },
        { value: "hyperv", label: "Hyper-V" },
      ]),
      t("url_admin", "cp_url_de_gestion"),
      t("capacidad", "cp_capacidad"),
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
      sel("runtime", "cp_runtime", [
        { value: "docker", label: "Docker", icon: "docker-plain" },
        { value: "podman", label: "Podman", icon: "podman-plain" },
      ]),
      t("imagen", "cp_imagen_tag"),
      t("host", "cp_host_que_lo_ejecuta"),
      t("puertos", "cp_puertos_publicados"),
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
      sel("distribucion", "cp_distribucion", [
        { value: "kubernetes", label: "Kubernetes", icon: "kubernetes-plain" },
        { value: "k3s", label: "k3s", icon: "kubernetes-plain" },
        { value: "rancher", label: "Rancher", icon: "rancher-plain" },
        { value: "eks", label: "EKS", icon: "amazonwebservices-plain-wordmark" },
        { value: "gke", label: "GKE", icon: "googlecloud-plain" },
        { value: "aks", label: "AKS", icon: "azure-plain" },
      ]),
      t("endpoint", "cp_api_endpoint"),
      t("nodos", "cp_n_nodos_pools"),
      t("namespaces", "cp_namespaces_clave"),
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
      sel("plataforma", "cp_plataforma", CLOUDS),
      t("runtime", "cp_runtime"),
      t("trigger", "cp_trigger_http_evento_cron"),
      t("url", "cp_url"),
      notas,
    ],
  },
  generic: {
    kind: "generic",
    category: "compute",
    label: "Genérico",
    icon: ComputerIcon,
    connectable: true,
    properties: [t("hostname", "cp_hostname"), notas],
  },

  // ── Aplicación ─────────────────────────────────────────────────────────
  api_gateway: {
    kind: "api_gateway",
    category: "application",
    label: "API Gateway",
    icon: ApiIcon,
    connectable: false,
    properties: [
      t("producto", "cp_producto_kong_apim"),
      t("rutas", "cp_rutas_endpoints"),
      t("auth", "cp_auth_keys_oauth_jwt"),
      t("upstreams", "cp_upstreams"),
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
      sel("software", "cp_software", [
        { value: "nginx", label: "nginx", icon: "nginx-original" },
        { value: "traefik", label: "Traefik", icon: "traefikproxy-plain" },
        { value: "apache", label: "Apache", icon: "apache-plain" },
        { value: "haproxy", label: "HAProxy" },
      ]),
      t("escucha", "cp_host_puerto_de_escucha"),
      t("upstreams", "cp_backends"),
      t("dominios", "cp_vhosts_dominios"),
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
      sel("software", "cp_software", [
        { value: "nginx", label: "nginx", icon: "nginx-original" },
        { value: "apache", label: "Apache", icon: "apache-plain" },
      ]),
      t("url", "cp_url", "cph_https_sitio_ejemplo_com"),
      t("sitios", "cp_sitios_vhosts"),
      t("puertos", "cp_puertos_80_443"),
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
      sel("framework", "cp_runtime_framework", APP_FRAMEWORKS),
      t("url", "cp_url", "cph_https_app_ejemplo_com"),
      t("version", "cp_version"),
      t("repo", "cp_repositorio"),
      t("puerto", "cp_puerto"),
      t("dependencias", "cp_dependencias_bd_cache_colas"),
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
      sel("runtime", "cp_runtime", APP_FRAMEWORKS),
      sel("tipo", "cp_tipo", [
        { value: "cron", label: "Cron" },
        { value: "queue", label: "co_cola" },
        { value: "stream", label: "Stream" },
      ]),
      t("origen", "cp_programacion_cola_origen"),
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
      sel("gestor", "cp_gestor", DB_ENGINES),
      sel("modelo", "cp_modelo", [
        { value: "relacional", label: "co_relacional" },
        { value: "documental", label: "co_documental" },
        { value: "grafo", label: "co_grafo" },
        { value: "keyvalue", label: "co_clave_valor_cache" },
        { value: "busqueda", label: "co_busqueda" },
        { value: "timeseries", label: "Time-series" },
      ]),
      t("version", "cp_version"),
      t("hostname", "cp_hostname_fqdn"),
      t("instancia", "cp_nombre_bd_instancia"),
      t("replicacion", "cp_replicacion_ha"),
      t("backup", "cp_backup_destino_frecuencia"),
      notas,
    ],
  },
  baas: {
    kind: "baas",
    category: "data",
    label: "BaaS",
    icon: CloudServerIcon,
    connectable: false,
    iconProperty: "proveedor",
    properties: [
      sel("proveedor", "cp_proveedor", BAAS_PROVIDERS),
      sel("hosting", "cp_hosting", [
        { value: "managed", label: "co_gestionado" },
        { value: "selfhosted", label: "co_self_hosted" },
      ]),
      t("servicios", "cp_servicios"),
      t("url", "cp_url"),
      t("proyecto", "cp_proyecto"),
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
      sel("proveedor", "cp_proveedor", CLOUDS),
      t("endpoint", "cp_endpoint"),
      t("bucket", "cp_bucket_s"),
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
      t("protocolo", "cp_protocolo_nfs_smb"),
      t("capacidad", "cp_capacidad"),
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
      t("destino", "cp_destino"),
      t("frecuencia", "cp_frecuencia"),
      t("retencion", "cp_retencion"),
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
      sel("producto", "cp_producto", [
        { value: "rabbitmq", label: "RabbitMQ", icon: "rabbitmq-original" },
        { value: "sqs", label: "Amazon SQS", icon: "amazonwebservices-plain-wordmark" },
        { value: "activemq", label: "ActiveMQ" },
      ]),
      t("hostname", "cp_hostname_fqdn"),
      t("colas", "cp_colas_exchanges"),
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
      sel("producto", "cp_producto", [
        { value: "kafka", label: "Apache Kafka", icon: "apachekafka-original" },
        { value: "pulsar", label: "Apache Pulsar" },
      ]),
      t("brokers", "cp_brokers_bootstrap"),
      t("topics", "cp_topics"),
      t("retencion", "cp_retencion"),
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
      sel("producto", "cp_producto", [
        { value: "prometheus", label: "Prometheus", icon: "prometheus-original" },
        { value: "grafana", label: "Grafana", icon: "grafana-plain" },
        { value: "zabbix", label: "Zabbix" },
      ]),
      t("url", "cp_url"),
      t("targets", "cp_targets_vigilados"),
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
      sel("stack", "cp_stack", [
        { value: "elastic", label: "Elastic (ELK)", icon: "elasticsearch-plain" },
        { value: "loki", label: "Loki", icon: "grafana-plain" },
        { value: "graylog", label: "Graylog" },
      ]),
      t("endpoint", "cp_endpoint_de_ingesta"),
      t("retencion", "cp_retencion"),
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
      t("producto", "cp_producto_datadog_jaeger"),
      t("apps", "cp_apps_instrumentadas"),
      t("endpoint", "cp_endpoint"),
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
      sel("proveedor", "cp_proveedor", CLOUDS),
      t("url", "cp_url"),
      t("proposito", "cp_proposito"),
      t("criticidad", "cp_criticidad"),
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
      t("base_url", "cp_base_url"),
      t("auth", "cp_auth_key_oauth"),
      t("rate_limits", "cp_rate_limits"),
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
      t("rol", "cp_rol_persona"),
      sel("origen", "cp_origen", [
        { value: "interno", label: "co_interno" },
        { value: "externo", label: "co_externo" },
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
      t("os", "cp_so"),
      t("usuario", "cp_usuario_asignado"),
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
      sel("plataforma", "cp_plataforma", [
        { value: "android", label: "Android" },
        { value: "ios", label: "iOS" },
      ]),
      t("proposito", "cp_proposito"),
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
      sel("tipo", "cp_tipo", [
        { value: "zona", label: "co_zona_ambiente" },
        { value: "vpc", label: "VPC" },
        { value: "subred", label: "co_subred" },
        { value: "region", label: "co_region" },
        { value: "datacenter", label: "Datacenter" },
        { value: "docker_compose", label: "Docker Compose", icon: "docker-plain" },
        { value: "k8s_namespace", label: "Kubernetes", icon: "kubernetes-plain" },
      ]),
      t("cidr", "cp_cidr_rango"),
      sel("color", "cp_color", [
        { value: "slate", label: "co_gris" },
        { value: "green", label: "co_verde" },
        { value: "blue", label: "co_azul" },
        { value: "amber", label: "co_ambar" },
        { value: "rose", label: "co_rosa" },
        { value: "violet", label: "co_violeta" },
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
