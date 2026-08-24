// Vocabulario de tipos del catálogo de nodos, compartido entre app, storybook y
// landing. Framework-agnóstico (sin dependencias de UI).

/** Categorías del catálogo de nodos (agrupan los tipos en la paleta). */
export type NodeCategory =
  | "network"
  | "security"
  | "identity"
  | "compute"
  | "application"
  | "data"
  | "storage"
  | "messaging"
  | "observability"
  | "external"
  | "client"
  | "grouping";

/** Tipo concreto de nodo de infraestructura. */
export type NodeKind =
  // Red
  | "internet"
  | "router"
  | "switch"
  | "access_point"
  | "dns"
  | "dhcp"
  | "vpn"
  | "load_balancer"
  | "cdn"
  // Seguridad
  | "firewall"
  | "waf"
  | "ids_ips"
  | "bastion"
  | "secrets_manager"
  // Identidad
  | "idp"
  | "directory"
  // Cómputo
  | "server"
  | "vm"
  | "hypervisor"
  | "container"
  | "k8s_cluster"
  | "serverless"
  | "generic"
  // Aplicación
  | "api_gateway"
  | "reverse_proxy"
  | "web_server"
  | "application"
  | "worker"
  // Datos
  | "database"
  | "baas"
  // Almacenamiento
  | "object_storage"
  | "nas"
  | "backup"
  // Mensajería
  | "message_broker"
  | "event_streaming"
  // Observabilidad
  | "monitoring"
  | "logging"
  | "apm"
  // Externo
  | "saas"
  | "third_party_api"
  // Cliente / actor
  | "user"
  | "workstation"
  | "mobile"
  // Agrupadores visuales (contenedores de fondo, no equipos)
  | "zone";
