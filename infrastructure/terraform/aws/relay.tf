# Kubernetes Namespace
resource "kubernetes_namespace" "file_relay" {
  metadata {
    name = "file-relay"

    labels = {
      name        = "file-relay"
      environment = var.environment
    }
  }

  depends_on = [module.eks]
}

# ConfigMap for relay configuration
resource "kubernetes_config_map" "relay_config" {
  metadata {
    name      = "relay-config"
    namespace = kubernetes_namespace.file_relay.metadata[0].name
  }

  data = {
    "MAX_PEERS"  = tostring(var.relay_max_peers)
    "RELAY_PORT" = tostring(var.relay_port)
    "METRICS_PORT" = tostring(var.relay_metrics_port)
    "RUST_LOG"   = var.environment == "production" ? "info" : "debug"
  }
}

# Deployment
resource "kubernetes_deployment" "relay_server" {
  metadata {
    name      = "file-relay-server"
    namespace = kubernetes_namespace.file_relay.metadata[0].name

    labels = {
      app         = "file-relay-server"
      version     = var.relay_image_tag
      environment = var.environment
    }
  }

  spec {
    replicas = var.relay_replicas

    strategy {
      type = "RollingUpdate"
      rolling_update {
        max_surge       = "1"
        max_unavailable = "0"
      }
    }

    selector {
      match_labels = {
        app = "file-relay-server"
      }
    }

    template {
      metadata {
        labels = {
          app         = "file-relay-server"
          version     = var.relay_image_tag
          environment = var.environment
        }

        annotations = {
          "prometheus.io/scrape" = "true"
          "prometheus.io/port"   = tostring(var.relay_metrics_port)
          "prometheus.io/path"   = "/metrics"
        }
      }

      spec {
        # Security context
        security_context {
          run_as_non_root = true
          run_as_user     = 1000
          fs_group        = 1000
          seccomp_profile {
            type = "RuntimeDefault"
          }
        }

        # Affinity rules
        affinity {
          pod_anti_affinity {
            required_during_scheduling_ignored_during_execution {
              label_selector {
                match_expressions {
                  key      = "app"
                  operator = "In"
                  values   = ["file-relay-server"]
                }
              }
              topology_key = "kubernetes.io/hostname"
            }
          }

          # Prefer spreading across AZs
          pod_anti_affinity {
            preferred_during_scheduling_ignored_during_execution {
              weight = 100
              pod_affinity_term {
                label_selector {
                  match_expressions {
                    key      = "app"
                    operator = "In"
                    values   = ["file-relay-server"]
                  }
                }
                topology_key = "topology.kubernetes.io/zone"
              }
            }
          }
        }

        container {
          name  = "relay"
          image = "${var.relay_image_repository}:${var.relay_image_tag}"

          image_pull_policy = "Always"

          port {
            name           = "relay"
            container_port = var.relay_port
            protocol       = "UDP"
          }

          port {
            name           = "metrics"
            container_port = var.relay_metrics_port
            protocol       = "TCP"
          }

          env_from {
            config_map_ref {
              name = kubernetes_config_map.relay_config.metadata[0].name
            }
          }

          resources {
            requests = {
              cpu    = var.relay_cpu_request
              memory = var.relay_memory_request
            }
            limits = {
              cpu    = var.relay_cpu_limit
              memory = var.relay_memory_limit
            }
          }

          liveness_probe {
            http_get {
              path = "/health"
              port = var.relay_metrics_port
            }
            initial_delay_seconds = 10
            period_seconds        = 10
            timeout_seconds       = 5
            failure_threshold     = 3
          }

          readiness_probe {
            http_get {
              path = "/ready"
              port = var.relay_metrics_port
            }
            initial_delay_seconds = 5
            period_seconds        = 5
            timeout_seconds       = 3
            failure_threshold     = 2
          }

          security_context {
            allow_privilege_escalation = false
            read_only_root_filesystem  = true
            run_as_non_root            = true
            run_as_user                = 1000
            capabilities {
              drop = ["ALL"]
            }
          }
        }

        # Topology spread constraints
        topology_spread_constraint {
          max_skew           = 1
          topology_key       = "topology.kubernetes.io/zone"
          when_unsatisfiable = "DoNotSchedule"
          label_selector {
            match_labels = {
              app = "file-relay-server"
            }
          }
        }
      }
    }
  }

  depends_on = [kubernetes_config_map.relay_config]
}

# Service (UDP LoadBalancer)
resource "kubernetes_service" "relay_udp" {
  metadata {
    name      = "file-relay-udp"
    namespace = kubernetes_namespace.file_relay.metadata[0].name

    annotations = {
      "service.beta.kubernetes.io/aws-load-balancer-type"            = "nlb"
      "service.beta.kubernetes.io/aws-load-balancer-scheme"          = "internet-facing"
      "service.beta.kubernetes.io/aws-load-balancer-cross-zone-load-balancing-enabled" = "true"
      "service.beta.kubernetes.io/aws-load-balancer-nlb-target-type" = "ip"
    }
  }

  spec {
    type = "LoadBalancer"

    selector = {
      app = "file-relay-server"
    }

    port {
      name        = "relay"
      protocol    = "UDP"
      port        = var.relay_port
      target_port = var.relay_port
    }

    session_affinity = "ClientIP"
  }

  depends_on = [kubernetes_deployment.relay_server]
}

# Service (Metrics - ClusterIP)
resource "kubernetes_service" "relay_metrics" {
  metadata {
    name      = "file-relay-metrics"
    namespace = kubernetes_namespace.file_relay.metadata[0].name

    labels = {
      app = "file-relay-server"
    }
  }

  spec {
    type = "ClusterIP"

    selector = {
      app = "file-relay-server"
    }

    port {
      name        = "metrics"
      protocol    = "TCP"
      port        = var.relay_metrics_port
      target_port = var.relay_metrics_port
    }
  }

  depends_on = [kubernetes_deployment.relay_server]
}

# HorizontalPodAutoscaler
resource "kubernetes_horizontal_pod_autoscaler_v2" "relay_hpa" {
  count = var.hpa_enabled ? 1 : 0

  metadata {
    name      = "file-relay-hpa"
    namespace = kubernetes_namespace.file_relay.metadata[0].name
  }

  spec {
    scale_target_ref {
      api_version = "apps/v1"
      kind        = "Deployment"
      name        = kubernetes_deployment.relay_server.metadata[0].name
    }

    min_replicas = var.hpa_min_replicas
    max_replicas = var.hpa_max_replicas

    metric {
      type = "Resource"
      resource {
        name = "cpu"
        target {
          type                = "Utilization"
          average_utilization = var.hpa_target_cpu
        }
      }
    }

    metric {
      type = "Resource"
      resource {
        name = "memory"
        target {
          type                = "Utilization"
          average_utilization = var.hpa_target_memory
        }
      }
    }

    behavior {
      scale_down {
        stabilization_window_seconds = 300
        policy {
          type          = "Percent"
          value         = 50
          period_seconds = 60
        }
      }

      scale_up {
        stabilization_window_seconds = 60
        policy {
          type          = "Percent"
          value         = 100
          period_seconds = 60
        }
        policy {
          type          = "Pods"
          value         = 2
          period_seconds = 60
        }
        select_policy = "Max"
      }
    }
  }

  depends_on = [kubernetes_deployment.relay_server]
}

# PodDisruptionBudget
resource "kubernetes_pod_disruption_budget_v1" "relay_pdb" {
  metadata {
    name      = "file-relay-pdb"
    namespace = kubernetes_namespace.file_relay.metadata[0].name
  }

  spec {
    min_available = 2

    selector {
      match_labels = {
        app = "file-relay-server"
      }
    }
  }

  depends_on = [kubernetes_deployment.relay_server]
}

# ServiceMonitor for Prometheus (CRD)
resource "kubernetes_manifest" "relay_service_monitor" {
  count = var.monitoring_enabled ? 1 : 0

  manifest = {
    apiVersion = "monitoring.coreos.com/v1"
    kind       = "ServiceMonitor"
    metadata = {
      name      = "file-relay-metrics"
      namespace = kubernetes_namespace.file_relay.metadata[0].name
      labels = {
        app = "file-relay-server"
      }
    }
    spec = {
      selector = {
        matchLabels = {
          app = "file-relay-server"
        }
      }
      endpoints = [
        {
          port     = "metrics"
          interval = "30s"
          path     = "/metrics"
        }
      ]
    }
  }

  depends_on = [
    kubernetes_service.relay_metrics,
    helm_release.kube_prometheus_stack
  ]
}
