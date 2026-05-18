variable "aws_region" {
  description = "AWS region for deployment"
  type        = string
  default     = "us-east-1"
}

variable "environment" {
  description = "Environment name (production, staging, development)"
  type        = string
  default     = "production"

  validation {
    condition     = contains(["production", "staging", "development"], var.environment)
    error_message = "Environment must be production, staging, or development."
  }
}

variable "project_name" {
  description = "Project name for resource naming"
  type        = string
  default     = "file-relay"
}

# Network Configuration

variable "vpc_cidr" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
}

variable "availability_zones" {
  description = "Availability zones for multi-AZ deployment"
  type        = list(string)
  default     = ["us-east-1a", "us-east-1b", "us-east-1c"]
}

# EKS Configuration

variable "cluster_version" {
  description = "Kubernetes version"
  type        = string
  default     = "1.28"
}

variable "node_instance_types" {
  description = "EC2 instance types for worker nodes"
  type        = list(string)
  default     = ["t3.medium"]
}

variable "node_desired_size" {
  description = "Desired number of worker nodes"
  type        = number
  default     = 3
}

variable "node_min_size" {
  description = "Minimum number of worker nodes"
  type        = number
  default     = 3
}

variable "node_max_size" {
  description = "Maximum number of worker nodes"
  type        = number
  default     = 10
}

variable "node_disk_size" {
  description = "Disk size for worker nodes (GB)"
  type        = number
  default     = 20
}

# FILE Relay Server Configuration

variable "relay_image_repository" {
  description = "Docker image repository for relay server"
  type        = string
  default     = "ghcr.io/file-network/relay-server"
}

variable "relay_image_tag" {
  description = "Docker image tag for relay server"
  type        = string
  default     = "latest"
}

variable "relay_replicas" {
  description = "Number of relay server replicas"
  type        = number
  default     = 3
}

variable "relay_max_peers" {
  description = "Maximum peers per relay instance"
  type        = number
  default     = 1000
}

variable "relay_cpu_request" {
  description = "CPU request for relay pods"
  type        = string
  default     = "100m"
}

variable "relay_cpu_limit" {
  description = "CPU limit for relay pods"
  type        = string
  default     = "1000m"
}

variable "relay_memory_request" {
  description = "Memory request for relay pods"
  type        = string
  default     = "128Mi"
}

variable "relay_memory_limit" {
  description = "Memory limit for relay pods"
  type        = string
  default     = "512Mi"
}

variable "relay_port" {
  description = "UDP port for relay server"
  type        = number
  default     = 8080
}

variable "relay_metrics_port" {
  description = "HTTP port for metrics endpoint"
  type        = number
  default     = 8081
}

# HPA Configuration

variable "hpa_enabled" {
  description = "Enable Horizontal Pod Autoscaler"
  type        = bool
  default     = true
}

variable "hpa_min_replicas" {
  description = "Minimum replicas for HPA"
  type        = number
  default     = 3
}

variable "hpa_max_replicas" {
  description = "Maximum replicas for HPA"
  type        = number
  default     = 10
}

variable "hpa_target_cpu" {
  description = "Target CPU utilization for HPA (%)"
  type        = number
  default     = 70
}

variable "hpa_target_memory" {
  description = "Target memory utilization for HPA (%)"
  type        = number
  default     = 80
}

# Monitoring Configuration

variable "monitoring_enabled" {
  description = "Deploy Prometheus and Grafana"
  type        = bool
  default     = true
}

variable "prometheus_retention" {
  description = "Prometheus data retention period"
  type        = string
  default     = "30d"
}

variable "prometheus_storage_size" {
  description = "Prometheus storage size"
  type        = string
  default     = "50Gi"
}

variable "grafana_admin_password" {
  description = "Grafana admin password"
  type        = string
  sensitive   = true
  default     = ""
}

# Alerting Configuration

variable "alertmanager_enabled" {
  description = "Deploy Alertmanager"
  type        = bool
  default     = true
}

variable "alert_slack_webhook_url" {
  description = "Slack webhook URL for alerts"
  type        = string
  sensitive   = true
  default     = ""
}

variable "alert_pagerduty_service_key" {
  description = "PagerDuty integration key for critical alerts"
  type        = string
  sensitive   = true
  default     = ""
}

# Tags

variable "tags" {
  description = "Additional tags for all resources"
  type        = map(string)
  default     = {}
}
