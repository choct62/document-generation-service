# Document Generation Service

A Rust-based microservice for generating standards-compliant technical documentation from structured data using Google Cloud Pub/Sub messaging.

## Overview

The Document Generation Service is an event-driven microservice that consumes document generation requests from a Pub/Sub subscription, processes them using templated generators, and publishes the generated documents (PDF, HTML, Markdown) back to a response topic. It supports multiple industry-standard specification formats including ISO/IEC/IEEE 29148:2018, IEEE 830, and various report types.

### Key Features

- **Event-Driven Architecture**: Asynchronous processing via Google Cloud Pub/Sub
- **Multiple Output Formats**: PDF (via Pandoc/XeLaTeX), HTML, and Markdown
- **Standards-Compliant**: Support for ISO/IEC/IEEE 29148:2018, IEEE 830, MIL-STD-498
- **Template-Based Generation**: Handlebars templates for flexible document structure
- **Production-Ready**: Deployed on GKE with horizontal scaling, health checks, and graceful shutdown

## Architecture

```
┌─────────────────┐         ┌──────────────────────────────┐
│   Publisher     │         │  Google Cloud Pub/Sub        │
│   (Any Service) │────────▶│  document-generation-        │
└─────────────────┘         │  requests-sub                │
                            └──────────────┬───────────────┘
                                           │
                                           ▼
                            ┌──────────────────────────────┐
                            │  Document Generation Service │
                            │  - Message Handler           │
                            │  - Template Renderer         │
                            │  - Format Converters         │
                            │  - Pandoc/XeLaTeX Pipeline   │
                            └──────────────┬───────────────┘
                                           │
                                           ▼
                            ┌──────────────────────────────┐
                            │  Google Cloud Pub/Sub        │
                            │  document-generation-results │
                            └──────────────────────────────┘
                                           │
                                           ▼
                            ┌──────────────────────────────┐
                            │  Subscriber                  │
                            │  (Any Service)               │
                            └──────────────────────────────┘
```

## Pub/Sub Schema

### Request Message Schema

Messages published to `document-generation-requests-sub` must conform to the following JSON schema:

```json
{
  "specification_type": "iso29148_software_requirements",
  "output_formats": ["PDF", "HTML", "Markdown"],
  "data": {
    // Specification-specific structured data
    // See "Data Schema by Specification Type" section below
  },
  "metadata": {
    "title": "Software Requirements Specification",
    "project_name": "QXProveIt Platform",
    "version": "1.0.0",
    "author": "Engineering Team",
    "organization": "mcx Services, LLC",
    "classification": "Confidential",
    "distribution_statement": "Internal Use Only",
    "generated_date": "2026-02-05T18:00:00Z"
  }
}
```

#### Specification Types

| Type | Description |
|------|-------------|
| `ieee830_drd` | IEEE 830 Data Requirements Document |
| `ieee830_srs` | IEEE 830 Software Requirements Specification |
| `milstd498_srs` | MIL-STD-498 Software Requirements Specification |
| `iso29148_stakeholder_requirements` | ISO/IEC/IEEE 29148:2018 Stakeholder Requirements (StakRS) |
| `iso29148_system_requirements` | ISO/IEC/IEEE 29148:2018 System Requirements (SyRS) |
| `iso29148_software_requirements` | ISO/IEC/IEEE 29148:2018 Software Requirements (SRS) |
| `iso29148_concept_of_operations` | ISO/IEC/IEEE 29148:2018 Concept of Operations (ConOps) |
| `security_scan_report` | Security Vulnerability Scan Report |
| `compliance_audit_report` | Compliance Audit Report |
| `test_execution_report` | Test Execution Report |

#### Output Formats

| Format | Description | MIME Type |
|--------|-------------|-----------|
| `PDF` | Portable Document Format (via Pandoc/XeLaTeX) | `application/pdf` |
| `HTML` | HyperText Markup Language | `text/html` |
| `Markdown` | Markdown text format | `text/markdown` |

### Response Message Schema

Messages published to `document-generation-results` topic:

```json
{
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "success",
  "documents": [
    {
      "format": "PDF",
      "content_base64": "JVBERi0xLjQKJeLjz9MKMSAwIG9iago8PAov...",
      "filename": "software_requirements_specification_v1.0.0.pdf",
      "mime_type": "application/pdf",
      "size_bytes": 245678
    },
    {
      "format": "HTML",
      "content_base64": "PCFET0NUWVBFIGh0bWw+CjxodG1sIGxhbmc9ImVu...",
      "filename": "software_requirements_specification_v1.0.0.html",
      "mime_type": "text/html",
      "size_bytes": 89234
    }
  ],
  "error": null,
  "generated_at": "2026-02-05T18:01:23.456Z"
}
```

#### Error Response

```json
{
  "request_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "error",
  "documents": [],
  "error": "Template rendering failed: Missing required field 'introduction'",
  "generated_at": "2026-02-05T18:01:23.456Z"
}
```

## Data Schema by Specification Type

### ISO/IEC/IEEE 29148:2018 Software Requirements (SRS)

```json
{
  "introduction": {
    "purpose": "String describing the purpose",
    "scope": "String describing the scope",
    "definitions": [
      {"term": "API", "definition": "Application Programming Interface"}
    ],
    "references": [
      {"title": "ISO/IEC/IEEE 29148:2018", "url": "https://..."}
    ]
  },
  "overall_description": {
    "product_perspective": "String",
    "product_functions": ["Function 1", "Function 2"],
    "user_characteristics": "String",
    "constraints": ["Constraint 1", "Constraint 2"]
  },
  "requirements": [
    {
      "id": "REQ-001",
      "category": "Functional",
      "priority": "High",
      "description": "The system shall...",
      "rationale": "This is needed because...",
      "verification_method": "Test"
    }
  ]
}
```

### ISO/IEC/IEEE 29148:2018 Stakeholder Requirements (StakRS)

```json
{
  "stakeholders": [
    {
      "name": "End Users",
      "role": "Primary Users",
      "interests": ["Usability", "Performance"],
      "requirements": ["STAK-001", "STAK-002"]
    }
  ],
  "business_requirements": [
    {
      "id": "BIZ-001",
      "description": "Increase customer satisfaction by 20%",
      "success_criteria": "Net Promoter Score > 8.0"
    }
  ],
  "stakeholder_requirements": [
    {
      "id": "STAK-001",
      "stakeholder": "End Users",
      "requirement": "The system shall respond within 200ms",
      "priority": "High"
    }
  ]
}
```

### ISO/IEC/IEEE 29148:2018 Concept of Operations (ConOps)

```json
{
  "current_situation": {
    "background": "String describing current state",
    "problems": ["Problem 1", "Problem 2"],
    "opportunities": ["Opportunity 1"]
  },
  "proposed_system": {
    "vision": "String describing vision",
    "objectives": ["Objective 1", "Objective 2"],
    "capabilities": [
      {
        "name": "Real-time Analysis",
        "description": "Analyze data in real-time"
      }
    ]
  },
  "operational_scenarios": [
    {
      "scenario_id": "OPS-001",
      "title": "User Login",
      "actors": ["User", "System"],
      "steps": [
        "User navigates to login page",
        "System presents login form"
      ]
    }
  ]
}
```

### Security Scan Report

```json
{
  "scan_metadata": {
    "scan_date": "2026-02-05T18:00:00Z",
    "scanner": "Trivy v0.48.0",
    "target": "us-docker.pkg.dev/mcxtest/qxproveit/app:latest"
  },
  "summary": {
    "total_vulnerabilities": 42,
    "critical": 2,
    "high": 8,
    "medium": 15,
    "low": 17
  },
  "vulnerabilities": [
    {
      "id": "CVE-2024-1234",
      "severity": "Critical",
      "package": "openssl",
      "version": "1.1.1",
      "fixed_version": "1.1.1w",
      "description": "Buffer overflow in OpenSSL"
    }
  ]
}
```

## Configuration

The service can be configured via environment variables or a `config.toml` file.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `SERVICE__NAME` | `document-generation-service` | Service name for logging |
| `SERVICE__LOG_LEVEL` | `info` | Log level (trace, debug, info, warn, error) |
| `SERVICE__PUBSUB__PROJECT_ID` | `mcxtest` | GCP Project ID |
| `SERVICE__PUBSUB__REQUEST_SUBSCRIPTION` | `document-generation-requests-sub` | Input subscription name |
| `SERVICE__PUBSUB__RESPONSE_TOPIC` | `document-generation-results` | Output topic name |
| `SERVICE__PUBSUB__MAX_CONCURRENT_MESSAGES` | `10` | Max concurrent message processing |
| `SERVICE__TEMPLATES__PATH` | `./templates` | Path to Handlebars templates |

### Example config.toml

```toml
[service]
name = "document-generation-service"
log_level = "info"

[pubsub]
project_id = "mcxtest"
request_subscription = "document-generation-requests-sub"
response_topic = "document-generation-results"
max_concurrent_messages = 10

[templates]
path = "./templates"
```

## Building and Deployment

### Prerequisites

- Rust stable toolchain
- Docker (with buildx for cross-platform builds)
- Google Cloud SDK (gcloud)
- kubectl (for Kubernetes deployment)

### Local Development

```bash
# Install dependencies
cargo build

# Run tests
cargo test

# Run locally (requires GCP credentials)
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/service-account.json
cargo run
```

### Docker Build

```bash
# Build for linux/amd64 platform
docker buildx build --platform linux/amd64 \
  -t us-docker.pkg.dev/mcxtest/qxproveit/document-generation-service:latest \
  --load .

# Push to Artifact Registry
docker push us-docker.pkg.dev/mcxtest/qxproveit/document-generation-service:latest
```

### Kubernetes Deployment

```bash
# Apply deployment
kubectl apply -f kubernetes/deployment.yaml

# Check pod status
kubectl get pods -n qxproveit -l app=document-generation-service

# View logs
kubectl logs -n qxproveit -l app=document-generation-service -f
```

## Project Structure

```
document-generation-service/
├── src/
│   ├── main.rs                    # Service entry point and message loop
│   ├── config.rs                  # Configuration management
│   ├── models.rs                  # Pub/Sub schema definitions
│   ├── error.rs                   # Error types
│   ├── generators/                # Document generators by type
│   │   ├── mod.rs
│   │   ├── ieee830.rs
│   │   ├── iso29148_srs.rs
│   │   ├── iso29148_stakrs.rs
│   │   ├── iso29148_syrs.rs
│   │   ├── iso29148_conops.rs
│   │   └── security_report.rs
│   ├── renderers/                 # Format converters
│   │   ├── mod.rs
│   │   ├── markdown.rs
│   │   ├── html.rs
│   │   └── pdf.rs
│   └── pubsub/                    # Pub/Sub integration
│       ├── mod.rs
│       ├── handler.rs
│       └── publisher.rs
├── templates/                     # Handlebars templates
│   ├── ieee830_srs.md.hbs
│   ├── iso29148_srs.md.hbs
│   ├── iso29148_stakrs.md.hbs
│   ├── iso29148_syrs.md.hbs
│   ├── iso29148_conops.md.hbs
│   └── security_report.md.hbs
├── kubernetes/
│   └── deployment.yaml            # Kubernetes manifests
├── Dockerfile                     # Multi-stage Docker build
├── Cargo.toml                     # Rust dependencies
└── deploy.sh                      # Deployment script
```

## Dependencies

### Runtime

- **Pandoc**: Document conversion engine
- **XeLaTeX**: PDF generation via TeX Live
- **Google Cloud Pub/Sub**: Message queue

### Rust Crates

- `tokio`: Async runtime
- `google-cloud-pubsub`: GCP Pub/Sub client
- `handlebars`: Template engine
- `serde`/`serde_json`: Serialization
- `tracing`: Structured logging
- `anyhow`/`thiserror`: Error handling

## Examples

### Generating an ISO/IEC/IEEE 29148 SRS Document

```json
{
  "specification_type": "iso29148_software_requirements",
  "output_formats": ["PDF", "Markdown"],
  "data": {
    "introduction": {
      "purpose": "Define software requirements for the QXProveIt platform",
      "scope": "This document covers all software components of the platform",
      "definitions": [
        {"term": "AI/ML", "definition": "Artificial Intelligence / Machine Learning"}
      ],
      "references": [
        {
          "title": "ISO/IEC/IEEE 29148:2018",
          "url": "https://www.iso.org/standard/72089.html"
        }
      ]
    },
    "overall_description": {
      "product_perspective": "A cloud-native compliance management platform",
      "product_functions": [
        "Requirements management",
        "Test case generation",
        "Compliance verification"
      ],
      "user_characteristics": "Technical users with compliance expertise",
      "constraints": [
        "Must comply with NIST 800-53",
        "Must support air-gapped deployments"
      ]
    },
    "requirements": [
      {
        "id": "REQ-001",
        "category": "Functional",
        "priority": "High",
        "description": "The system shall generate test cases from requirements using AI/ML",
        "rationale": "Automated test generation improves coverage and reduces manual effort",
        "verification_method": "Functional Test"
      },
      {
        "id": "REQ-002",
        "category": "Performance",
        "priority": "Medium",
        "description": "The system shall respond to user queries within 500ms for 95th percentile",
        "rationale": "User experience requires responsive interaction",
        "verification_method": "Performance Test"
      }
    ]
  },
  "metadata": {
    "title": "QXProveIt Software Requirements Specification",
    "project_name": "QXProveIt Platform",
    "version": "2.1.0",
    "author": "Engineering Team",
    "organization": "mcx Services, LLC",
    "classification": "Confidential",
    "distribution_statement": "Internal Use Only"
  }
}
```

## Troubleshooting

### Pod CrashLoopBackOff

Check pod logs for startup errors:
```bash
kubectl logs -n qxproveit <pod-name>
```

Common issues:
- Missing GCP credentials (ensure service account has proper IAM roles)
- Invalid Pub/Sub subscription/topic names
- Pandoc/XeLaTeX not installed in container

### PDF Generation Failures

Ensure the Docker image includes all TeX Live packages:
- `texlive-xetex`
- `texlive-fonts-recommended`
- `texlive-fonts-extra`

### Message Processing Delays

Check HPA scaling and pod resource limits:
```bash
kubectl get hpa -n qxproveit
kubectl top pods -n qxproveit
```

## License

Copyright (c) 2026 mcx Services, LLC. All rights reserved.

## Support

For issues or questions, contact the Platform Engineering team.
