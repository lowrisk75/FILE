# Prometheus Operator Stack (kube-prometheus-stack)
resource "helm_release" "kube_prometheus_stack" {
  count = var.monitoring_enabled ? 1 : 0

  name       = "kube-prometheus-stack"
  repository = "https://prometheus-community.github.io/helm-charts"
  chart      = "kube-prometheus-stack"
  namespace  = "monitoring"
  version    = "51.0.0"

  create_namespace = true

  values = [
    yamlencode({
      # Prometheus Configuration
      prometheus = {
        prometheusSpec = {
          retention         = var.prometheus_retention
          retentionSize     = "45GB"
          walCompression    = true
          replicas          = var.environment == "production" ? 2 : 1

          storageSpec = {
            volumeClaimTemplate = {
              spec = {
                storageClassName = "gp3"
                accessModes      = ["ReadWriteOnce"]
                resources = {
                  requests = {
                    storage = var.prometheus_storage_size
                  }
                }
              }
            }
          }

          resources = {
            requests = {
              cpu    = "500m"
              memory = "2Gi"
            }
            limits = {
              cpu    = "2000m"
              memory = "8Gi"
            }
          }

          # Service Monitor selectors
          serviceMonitorSelectorNilUsesHelmValues = false
          podMonitorSelectorNilUsesHelmValues     = false
          ruleSelectorNilUsesHelmValues           = false

          # Additional scrape configs for FILE Relay
          additionalScrapeConfigs = [
            {
              job_name = "file-relay"
              kubernetes_sd_configs = [
                {
                  role = "endpoints"
                  namespaces = {
                    names = ["file-relay"]
                  }
                }
              ]
              relabel_configs = [
                {
                  source_labels = ["__meta_kubernetes_service_label_app"]
                  action        = "keep"
                  regex         = "file-relay-server"
                },
                {
                  source_labels = ["__meta_kubernetes_endpoint_port_name"]
                  action        = "keep"
                  regex         = "metrics"
                }
              ]
            }
          ]
        }

        # Enable persistence
        persistentVolume = {
          enabled = true
          size    = var.prometheus_storage_size
        }
      }

      # Grafana Configuration
      grafana = {
        enabled = true

        adminPassword = var.grafana_admin_password != "" ? var.grafana_admin_password : "changeme"

        replicas = var.environment == "production" ? 2 : 1

        persistence = {
          enabled      = true
          size         = "10Gi"
          storageClassName = "gp3"
        }

        resources = {
          requests = {
            cpu    = "100m"
            memory = "256Mi"
          }
          limits = {
            cpu    = "500m"
            memory = "1Gi"
          }
        }

        # Ingress (optional - requires cert-manager)
        ingress = {
          enabled = false
          ingressClassName = "alb"
          annotations = {
            "alb.ingress.kubernetes.io/scheme"      = "internet-facing"
            "alb.ingress.kubernetes.io/target-type" = "ip"
          }
          hosts = []
        }

        # Grafana dashboards
        dashboardProviders = {
          "dashboardproviders.yaml" = {
            apiVersion = 1
            providers = [
              {
                name      = "default"
                orgId     = 1
                folder    = ""
                type      = "file"
                disableDeletion = false
                editable  = true
                options = {
                  path = "/var/lib/grafana/dashboards/default"
                }
              }
            ]
          }
        }

        dashboardsConfigMaps = {
          default = "grafana-dashboards"
        }

        # Data sources
        datasources = {
          "datasources.yaml" = {
            apiVersion = 1
            datasources = [
              {
                name      = "Prometheus"
                type      = "prometheus"
                url       = "http://kube-prometheus-stack-prometheus.monitoring:9090"
                access    = "proxy"
                isDefault = true
                jsonData = {
                  timeInterval = "30s"
                }
              }
            ]
          }
        }

        sidecar = {
          dashboards = {
            enabled = true
            label   = "grafana_dashboard"
          }
          datasources = {
            enabled = true
            label   = "grafana_datasource"
          }
        }
      }

      # Alertmanager Configuration
      alertmanager = {
        enabled = var.alertmanager_enabled

        alertmanagerSpec = {
          replicas = var.environment == "production" ? 2 : 1

          storage = {
            volumeClaimTemplate = {
              spec = {
                storageClassName = "gp3"
                accessModes      = ["ReadWriteOnce"]
                resources = {
                  requests = {
                    storage = "10Gi"
                  }
                }
              }
            }
          }

          resources = {
            requests = {
              cpu    = "100m"
              memory = "256Mi"
            }
            limits = {
              cpu    = "500m"
              memory = "1Gi"
            }
          }
        }

        config = {
          global = {
            resolve_timeout = "5m"
          }

          route = {
            group_by        = ["alertname", "cluster", "service"]
            group_wait      = "30s"
            group_interval  = "5m"
            repeat_interval = "4h"
            receiver        = "default"

            routes = [
              {
                match = {
                  severity = "critical"
                }
                receiver        = "pagerduty"
                continue        = true
                repeat_interval = "1h"
              },
              {
                match = {
                  severity = "warning"
                }
                receiver        = "slack"
                repeat_interval = "4h"
              }
            ]
          }

          receivers = [
            {
              name = "default"
            },
            {
              name = "slack"
              slack_configs = var.alert_slack_webhook_url != "" ? [
                {
                  api_url = var.alert_slack_webhook_url
                  channel = "#file-relay-alerts"
                  title   = "{{ .GroupLabels.alertname }}"
                  text    = "{{ range .Alerts }}{{ .Annotations.description }}\n{{ end }}"
                  send_resolved = true
                }
              ] : []
            },
            {
              name = "pagerduty"
              pagerduty_configs = var.alert_pagerduty_service_key != "" ? [
                {
                  service_key = var.alert_pagerduty_service_key
                  description = "{{ .GroupLabels.alertname }} - {{ .GroupLabels.cluster }}"
                }
              ] : []
            }
          ]
        }
      }

      # Node Exporter
      nodeExporter = {
        enabled = true
      }

      # Kube State Metrics
      kubeStateMetrics = {
        enabled = true
      }
    })
  ]

  depends_on = [module.eks]
}

# ConfigMap for FILE Relay dashboard
resource "kubernetes_config_map" "grafana_dashboard_relay" {
  count = var.monitoring_enabled ? 1 : 0

  metadata {
    name      = "grafana-dashboards"
    namespace = "monitoring"

    labels = {
      grafana_dashboard = "1"
    }
  }

  data = {
    "file-relay-overview.json" = jsonencode({
      title   = "FILE Relay Server - Overview"
      uid     = "file-relay-overview"
      version = 1

      panels = [
        {
          id    = 1
          title = "Active Peers"
          type  = "graph"
          targets = [
            {
              expr = "file_relay_active_peers"
            }
          ]
          gridPos = {
            h = 8
            w = 12
            x = 0
            y = 0
          }
        },
        {
          id    = 2
          title = "Packets Forwarded (rate)"
          type  = "graph"
          targets = [
            {
              expr = "rate(file_relay_packets_forwarded_total[5m])"
            }
          ]
          gridPos = {
            h = 8
            w = 12
            x = 12
            y = 0
          }
        },
        {
          id    = 3
          title = "Peer Registration Success Rate"
          type  = "stat"
          targets = [
            {
              expr = <<-EOT
                (
                  rate(file_relay_peers_registered_total[5m])
                  /
                  (rate(file_relay_peers_registered_total[5m]) + rate(file_relay_capow_rejected_total[5m]) + rate(file_relay_rate_limited_total[5m]))
                ) * 100
              EOT
            }
          ]
          gridPos = {
            h = 4
            w = 6
            x = 0
            y = 8
          }
        },
        {
          id    = 4
          title = "CAPoW Rejection Rate"
          type  = "graph"
          targets = [
            {
              expr = "rate(file_relay_capow_rejected_total[5m])"
            }
          ]
          gridPos = {
            h = 8
            w = 12
            x = 0
            y = 12
          }
        },
        {
          id    = 5
          title = "Rate Limiting"
          type  = "graph"
          targets = [
            {
              expr = "rate(file_relay_rate_limited_total[5m])"
            }
          ]
          gridPos = {
            h = 8
            w = 12
            x = 12
            y = 12
          }
        },
        {
          id    = 6
          title = "Peer Timeouts"
          type  = "graph"
          targets = [
            {
              expr = "rate(file_relay_peers_timeout_total[5m])"
            }
          ]
          gridPos = {
            h = 8
            w = 12
            x = 0
            y = 20
          }
        },
        {
          id    = 7
          title = "Memory Usage"
          type  = "graph"
          targets = [
            {
              expr = "process_resident_memory_bytes{job=\"file-relay\"}"
            }
          ]
          gridPos = {
            h = 8
            w = 12
            x = 12
            y = 20
          }
        }
      ]
    })
  }

  depends_on = [helm_release.kube_prometheus_stack]
}

# PrometheusRule for FILE Relay alerts
resource "kubernetes_manifest" "relay_prometheus_rules" {
  count = var.monitoring_enabled ? 1 : 0

  manifest = {
    apiVersion = "monitoring.coreos.com/v1"
    kind       = "PrometheusRule"
    metadata = {
      name      = "file-relay-alerts"
      namespace = "monitoring"
      labels = {
        prometheus = "kube-prometheus-stack"
      }
    }
    spec = {
      groups = [
        {
          name = "file-relay"
          interval = "30s"
          rules = [
            {
              alert = "FileRelayServerDown"
              expr  = "up{job=\"file-relay\"} == 0"
              for   = "2m"
              labels = {
                severity = "critical"
              }
              annotations = {
                summary     = "FILE Relay Server is down"
                description = "FILE Relay Server has been down for more than 2 minutes."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#1-filerelayserverdown"
              }
            },
            {
              alert = "FileRelayCapacityReached"
              expr  = "file_relay_active_peers >= 1000"
              for   = "5m"
              labels = {
                severity = "critical"
              }
              annotations = {
                summary     = "FILE Relay Server at capacity"
                description = "FILE Relay Server has reached maximum peer count ({{ $value }} peers)."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#2-filerelaycapacityreached"
              }
            },
            {
              alert = "FileRelayCAPoWAttack"
              expr  = "rate(file_relay_capow_rejected_total[1m]) > 50"
              for   = "2m"
              labels = {
                severity = "critical"
              }
              annotations = {
                summary     = "FILE Relay Server under CAPoW attack"
                description = "CAPoW rejection rate is {{ $value }}/second, indicating a DoS attack."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#3-filerelaycapowattack"
              }
            },
            {
              alert = "FileRelayRateLimitAttack"
              expr  = "rate(file_relay_rate_limited_total[1m]) > 100"
              for   = "2m"
              labels = {
                severity = "critical"
              }
              annotations = {
                summary     = "FILE Relay Server under rate limit attack"
                description = "Rate limiting triggering at {{ $value }}/second."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#4-filerelayRatelimitAttack"
              }
            },
            {
              alert = "FileRelayHighPeerCount"
              expr  = "file_relay_active_peers > 900"
              for   = "15m"
              labels = {
                severity = "warning"
              }
              annotations = {
                summary     = "FILE Relay Server peer count high"
                description = "FILE Relay Server has {{ $value }} active peers (90% capacity)."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#5-filerelayhighpeercount"
              }
            },
            {
              alert = "FileRelayHighCAPoWRejectionRate"
              expr  = "rate(file_relay_capow_rejected_total[5m]) > 10"
              for   = "10m"
              labels = {
                severity = "warning"
              }
              annotations = {
                summary     = "FILE Relay Server high CAPoW rejection rate"
                description = "CAPoW rejection rate is {{ $value }}/second."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#6-filerelayhighcapowrejectionrate"
              }
            },
            {
              alert = "FileRelayHighPeerTimeouts"
              expr  = "rate(file_relay_peers_timeout_total[5m]) > 5"
              for   = "10m"
              labels = {
                severity = "warning"
              }
              annotations = {
                summary     = "FILE Relay Server high peer timeout rate"
                description = "Peer timeout rate is {{ $value }}/second."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#7-filerelayhighpeertimeouts"
              }
            },
            {
              alert = "FileRelayPodRestarting"
              expr  = "rate(kube_pod_container_status_restarts_total{pod=~\"file-relay.*\"}[15m]) > 0"
              labels = {
                severity = "warning"
              }
              annotations = {
                summary     = "FILE Relay pod restarting"
                description = "Pod {{ $labels.pod }} has restarted {{ $value }} times in the last 15 minutes."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#8-filerelayPodRestarting"
              }
            },
            {
              alert = "FileRelayHighMemoryUsage"
              expr  = "(container_memory_usage_bytes{pod=~\"file-relay.*\"} / container_spec_memory_limit_bytes{pod=~\"file-relay.*\"}) > 0.8"
              for   = "10m"
              labels = {
                severity = "warning"
              }
              annotations = {
                summary     = "FILE Relay pod high memory usage"
                description = "Pod {{ $labels.pod }} memory usage is {{ $value | humanizePercentage }}."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#9-filerelayhighmemoryusage"
              }
            },
            {
              alert = "FileRelayHighCPUUsage"
              expr  = "(rate(container_cpu_usage_seconds_total{pod=~\"file-relay.*\"}[5m]) / container_spec_cpu_quota{pod=~\"file-relay.*\"} * container_spec_cpu_period{pod=~\"file-relay.*\"}) > 0.8"
              for   = "10m"
              labels = {
                severity = "warning"
              }
              annotations = {
                summary     = "FILE Relay pod high CPU usage"
                description = "Pod {{ $labels.pod }} CPU usage is {{ $value | humanizePercentage }}."
                runbook_url = "https://github.com/file-network/relay-server/blob/main/docs/deployment/ALERT-RUNBOOK.md#10-filerelayhighcpuusage"
              }
            }
          ]
        }
      ]
    }
  }

  depends_on = [helm_release.kube_prometheus_stack]
}
