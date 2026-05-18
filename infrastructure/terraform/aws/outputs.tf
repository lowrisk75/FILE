# VPC Outputs
output "vpc_id" {
  description = "VPC ID"
  value       = module.vpc.vpc_id
}

output "vpc_cidr" {
  description = "VPC CIDR block"
  value       = module.vpc.vpc_cidr_block
}

output "private_subnets" {
  description = "Private subnet IDs"
  value       = module.vpc.private_subnets
}

output "public_subnets" {
  description = "Public subnet IDs"
  value       = module.vpc.public_subnets
}

# EKS Outputs
output "cluster_name" {
  description = "EKS cluster name"
  value       = module.eks.cluster_name
}

output "cluster_endpoint" {
  description = "EKS cluster endpoint"
  value       = module.eks.cluster_endpoint
}

output "cluster_version" {
  description = "EKS cluster Kubernetes version"
  value       = module.eks.cluster_version
}

output "cluster_security_group_id" {
  description = "EKS cluster security group ID"
  value       = module.eks.cluster_security_group_id
}

output "cluster_oidc_issuer_url" {
  description = "EKS cluster OIDC issuer URL"
  value       = module.eks.cluster_oidc_issuer_url
}

# Kubeconfig
output "configure_kubectl" {
  description = "Command to configure kubectl"
  value       = "aws eks update-kubeconfig --region ${var.aws_region} --name ${module.eks.cluster_name}"
}

# FILE Relay Server Outputs
output "relay_namespace" {
  description = "Kubernetes namespace for FILE Relay Server"
  value       = kubernetes_namespace.file_relay.metadata[0].name
}

output "relay_service_name" {
  description = "FILE Relay Server service name (UDP)"
  value       = kubernetes_service.relay_udp.metadata[0].name
}

output "relay_loadbalancer_hostname" {
  description = "FILE Relay Server LoadBalancer hostname"
  value       = try(kubernetes_service.relay_udp.status[0].load_balancer[0].ingress[0].hostname, "pending")
}

output "relay_loadbalancer_ip" {
  description = "FILE Relay Server LoadBalancer IP (if available)"
  value       = try(kubernetes_service.relay_udp.status[0].load_balancer[0].ingress[0].ip, "N/A")
}

output "relay_port" {
  description = "FILE Relay Server UDP port"
  value       = var.relay_port
}

output "relay_metrics_port" {
  description = "FILE Relay Server metrics port"
  value       = var.relay_metrics_port
}

output "relay_image" {
  description = "FILE Relay Server Docker image"
  value       = "${var.relay_image_repository}:${var.relay_image_tag}"
}

output "relay_replicas" {
  description = "FILE Relay Server current replica count"
  value       = kubernetes_deployment.relay_server.spec[0].replicas
}

output "relay_max_peers" {
  description = "Maximum peers per relay instance"
  value       = var.relay_max_peers
}

# HPA Outputs
output "hpa_enabled" {
  description = "Whether HPA is enabled"
  value       = var.hpa_enabled
}

output "hpa_min_replicas" {
  description = "HPA minimum replicas"
  value       = var.hpa_enabled ? var.hpa_min_replicas : "N/A"
}

output "hpa_max_replicas" {
  description = "HPA maximum replicas"
  value       = var.hpa_enabled ? var.hpa_max_replicas : "N/A"
}

# Monitoring Outputs
output "monitoring_enabled" {
  description = "Whether monitoring is enabled"
  value       = var.monitoring_enabled
}

output "prometheus_url" {
  description = "Prometheus URL (internal)"
  value       = var.monitoring_enabled ? "http://kube-prometheus-stack-prometheus.monitoring:9090" : "N/A"
}

output "grafana_url" {
  description = "Grafana URL (internal)"
  value       = var.monitoring_enabled ? "http://kube-prometheus-stack-grafana.monitoring:80" : "N/A"
}

output "alertmanager_url" {
  description = "Alertmanager URL (internal)"
  value       = var.monitoring_enabled && var.alertmanager_enabled ? "http://kube-prometheus-stack-alertmanager.monitoring:9093" : "N/A"
}

output "grafana_admin_user" {
  description = "Grafana admin username"
  value       = var.monitoring_enabled ? "admin" : "N/A"
}

# Port-forwarding commands
output "port_forward_grafana" {
  description = "Command to port-forward Grafana"
  value       = var.monitoring_enabled ? "kubectl port-forward -n monitoring svc/kube-prometheus-stack-grafana 3000:80" : "N/A"
}

output "port_forward_prometheus" {
  description = "Command to port-forward Prometheus"
  value       = var.monitoring_enabled ? "kubectl port-forward -n monitoring svc/kube-prometheus-stack-prometheus 9090:9090" : "N/A"
}

output "port_forward_alertmanager" {
  description = "Command to port-forward Alertmanager"
  value       = var.monitoring_enabled && var.alertmanager_enabled ? "kubectl port-forward -n monitoring svc/kube-prometheus-stack-alertmanager 9093:9093" : "N/A"
}

# Quick Start Commands
output "quick_start" {
  description = "Quick start commands"
  value = <<-EOT
    # Configure kubectl
    ${self.configure_kubectl}

    # Check relay pods
    kubectl get pods -n ${kubernetes_namespace.file_relay.metadata[0].name}

    # Check relay service
    kubectl get svc -n ${kubernetes_namespace.file_relay.metadata[0].name}

    # View relay logs
    kubectl logs -n ${kubernetes_namespace.file_relay.metadata[0].name} -l app=file-relay-server --tail=100

    # Check metrics
    kubectl port-forward -n ${kubernetes_namespace.file_relay.metadata[0].name} svc/${kubernetes_service.relay_metrics.metadata[0].name} ${var.relay_metrics_port}:${var.relay_metrics_port}
    curl http://localhost:${var.relay_metrics_port}/metrics

    ${var.monitoring_enabled ? "# Access Grafana\n    ${self.port_forward_grafana}\n    # Then open http://localhost:3000 (admin / ${var.grafana_admin_password != "" ? "[your-password]" : "changeme"})" : ""}

    # Health check
    curl http://${try(kubernetes_service.relay_udp.status[0].load_balancer[0].ingress[0].hostname, "pending")}:${var.relay_metrics_port}/health
  EOT
}

# DNS Configuration Guidance
output "dns_configuration" {
  description = "DNS configuration guidance"
  value = <<-EOT
    Configure your DNS to point to the LoadBalancer:

    ${try(kubernetes_service.relay_udp.status[0].load_balancer[0].ingress[0].hostname, "LoadBalancer provisioning... (run 'terraform output' again after a few minutes)")}

    Example Route 53 configuration:
    - Record Type: A or CNAME
    - Name: relay.example.com
    - Value: ${try(kubernetes_service.relay_udp.status[0].load_balancer[0].ingress[0].hostname, "[LoadBalancer hostname]")}
    - TTL: 60 seconds (for fast failover)

    For multi-region:
    - Use Route 53 Geolocation Routing Policy
    - Create one record per region
    - See docs/deployment/MULTI-REGION-GUIDE.md
  EOT
}

# Cost Estimation
output "estimated_monthly_cost" {
  description = "Estimated monthly AWS cost (USD)"
  value = <<-EOT
    Estimated monthly cost breakdown (us-east-1):

    EKS Control Plane:        $73/month
    Worker Nodes (${var.node_desired_size}x ${var.node_instance_types[0]}): ~$${var.node_desired_size * 30}/month
    NAT Gateway (${var.environment == "production" ? 3 : 1}x):         ~$${(var.environment == "production" ? 3 : 1) * 32}/month
    Network Load Balancer:    ~$18/month
    EBS Volumes (gp3):        ~$8/month
    ${var.monitoring_enabled ? "Prometheus Storage (${var.prometheus_storage_size}): ~$4/month" : ""}
    Data Transfer:            ~$10-20/month (varies by traffic)

    Total estimated:          ~$${73 + (var.node_desired_size * 30) + ((var.environment == "production" ? 3 : 1) * 32) + 18 + 8 + (var.monitoring_enabled ? 4 : 0) + 15}/month

    Note: Actual costs vary by usage, region, and traffic patterns.
    Use AWS Cost Explorer for accurate cost tracking.

    Cost optimization tips:
    - Use Spot instances (50-70% discount): set capacity_type = "SPOT" in eks.tf
    - Use Reserved Instances (40% discount, 1-year commit)
    - Reduce node_desired_size during low-traffic periods
    - Use single NAT Gateway for non-production (set single_nat_gateway = true)
    - Enable Cluster Autoscaler (already enabled)
  EOT
}
