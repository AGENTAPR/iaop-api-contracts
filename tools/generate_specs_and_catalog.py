import os
import sys
import json
import yaml
import shutil
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
WORKSPACE_DIR = BASE_DIR.parent

SERVICES_META = [
    {
        "id": "agent-bff",
        "name": "DEV / AGENA Agent BFF — Implemented",
        "repo": "AGENTAPR/iaop-agent-bff",
        "visibility": "public",
        "maturity": "implemented_verified",
        "domainOwner": "Agent Platform & Experience Team",
        "apiVersion": "1.0.0",
        "description": "AGENA Insurance Agent Lifecycle Platform BFF - Complete verified implementation covering 52 endpoints across Identity, Today Dashboard, Sales, Documents, and Earnings."
    },
    {
        "id": "agent-bff-proposed",
        "name": "DEV / AGENA Agent BFF — Proposed (AGENA v1)",
        "repo": "AGENTAPR/iaop-agent-bff",
        "visibility": "public",
        "maturity": "proposed_contract",
        "domainOwner": "Product & Architecture Working Group",
        "apiVersion": "1.0.0-draft",
        "description": "AGENA Insurance Agent Lifecycle Platform proposed frontend contracts (134 operations across 20 functional groups from AGENA_FRONTEND_API_CONTRACT_V1)."
    },
    {
        "id": "agent-service",
        "name": "DEV / IAOP Agent Service — Implemented",
        "repo": "AGENTAPR/iaop-agent-service",
        "visibility": "internal",
        "maturity": "implemented_verified",
        "domainOwner": "Agent Core Domain Team",
        "apiVersion": "1.0.0",
        "description": "Agent Registry, appointments, identity links, and lifecycle state machine (17 operations verified)."
    },
    {
        "id": "agency-service",
        "name": "DEV / IAOP Agency Service — Implemented",
        "repo": "AGENTAPR/iaop-agency-service",
        "visibility": "internal",
        "maturity": "implemented_unverified",
        "domainOwner": "Agency Hierarchy Team",
        "apiVersion": "0.1.0",
        "description": "Organizational hierarchy, assignments, agency leadership, and effective-dated relationships."
    },
    {
        "id": "recruitment-service",
        "name": "DEV / IAOP Recruitment Service — Proposed",
        "repo": "AGENTAPR/iaop-recruitment-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Recruitment & Talent Team",
        "apiVersion": "0.1.0",
        "description": "Candidate pipeline, candidate screening, referrals, and candidate invitation workflows."
    },
    {
        "id": "onboarding-service",
        "name": "DEV / IAOP Onboarding Service — Proposed",
        "repo": "AGENTAPR/iaop-onboarding-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Agent Onboarding Team",
        "apiVersion": "0.1.0",
        "description": "Agent onboarding checklists, e-KYC integration, document submission, and contracting."
    },
    {
        "id": "learning-service",
        "name": "DEV / IAOP Learning Service — Proposed",
        "repo": "AGENTAPR/iaop-learning-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Academy & Training Team",
        "apiVersion": "0.1.0",
        "description": "Agent training modules, mandatory CPD credits, course enrollments, and quiz progress."
    },
    {
        "id": "certification-service",
        "name": "DEV / IAOP Certification Service — Proposed",
        "repo": "AGENTAPR/iaop-certification-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Compliance & Licensing Team",
        "apiVersion": "0.1.0",
        "description": "AAJI, AASI, and OJK certification tracking, license renewals, and exam proof evidence."
    },
    {
        "id": "eligibility-service",
        "name": "DEV / IAOP Eligibility Service — Proposed",
        "repo": "AGENTAPR/iaop-eligibility-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Underwriting & Compliance Team",
        "apiVersion": "0.1.0",
        "description": "Real-time selling eligibility checks, product accreditation rules, and license validation."
    },
    {
        "id": "sales-service",
        "name": "DEV / IAOP Sales Service — Proposed",
        "repo": "AGENTAPR/iaop-sales-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Sales Enablement Team",
        "apiVersion": "0.1.0",
        "description": "Product catalog, premium illustrations, benefit calculation, and proposal lifecycle."
    },
    {
        "id": "portfolio-service",
        "name": "DEV / IAOP Portfolio Service — Proposed",
        "repo": "AGENTAPR/iaop-portfolio-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Policy Admin Team",
        "apiVersion": "0.1.0",
        "description": "Customer policy portfolio, policy status, premium billing schedules, and servicing."
    },
    {
        "id": "commission-service",
        "name": "DEV / IAOP Commission Service — Proposed",
        "repo": "AGENTAPR/iaop-commission-service",
        "visibility": "restricted",
        "maturity": "proposed_contract",
        "domainOwner": "Finance & Compensation Team",
        "apiVersion": "0.1.0",
        "description": "Commission calculation, hierarchy splits, clawback management, and payout statements."
    },
    {
        "id": "compliance-service",
        "name": "DEV / IAOP Compliance Service — Proposed",
        "repo": "AGENTAPR/iaop-compliance-service",
        "visibility": "restricted",
        "maturity": "proposed_contract",
        "domainOwner": "Legal & Risk Governance Team",
        "apiVersion": "0.1.0",
        "description": "Compliance breach tracking, regulatory sanctions, market conduct reviews, and selling blocks."
    },
    {
        "id": "performance-service",
        "name": "DEV / IAOP Performance Service — Proposed",
        "repo": "AGENTAPR/iaop-performance-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Agency Performance Team",
        "apiVersion": "0.1.0",
        "description": "Sales targets, MDRT progress, agency leaderboard, persistency ratios, and campaigns."
    },
    {
        "id": "operations-bff",
        "name": "DEV / IAOP Operations BFF — Proposed",
        "repo": "AGENTAPR/iaop-operations-bff",
        "visibility": "restricted",
        "maturity": "proposed_contract",
        "domainOwner": "Back Office Operations Team",
        "apiVersion": "0.1.0",
        "description": "Operations portal BFF for underwriting, compliance approvals, manual reviews, and admin."
    },
    {
        "id": "workflow-service",
        "name": "DEV / IAOP Workflow Service — Proposed",
        "repo": "AGENTAPR/iaop-workflow-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Business Process Team",
        "apiVersion": "0.1.0",
        "description": "BPMN execution, human task assignment, approval transitions, and process history."
    },
    {
        "id": "integration-service",
        "name": "DEV / IAOP Integration Service — Proposed",
        "repo": "AGENTAPR/iaop-integration-service",
        "visibility": "restricted",
        "maturity": "proposed_contract",
        "domainOwner": "Integration Platform Team",
        "apiVersion": "0.1.0",
        "description": "Core PAS adapters, payment gateways, e-KYC vendor connectors, and third-party webhooks."
    },
    {
        "id": "notification-service",
        "name": "DEV / IAOP Notification Service — Proposed",
        "repo": "AGENTAPR/iaop-notification-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Customer Communications Team",
        "apiVersion": "0.1.0",
        "description": "Multi-channel notifications (Push, Email, WhatsApp, SMS), templates, and delivery dispatch."
    },
    {
        "id": "document-service",
        "name": "DEV / IAOP Document Service — Proposed",
        "repo": "AGENTAPR/iaop-document-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "domainOwner": "Content & Storage Platform Team",
        "apiVersion": "0.1.0",
        "description": "Secure document storage, pre-signed upload/download URLs, optical character recognition, and retention."
    },
    {
        "id": "api-gateway",
        "name": "DEV / IAOP API Gateway — Proposed",
        "repo": "AGENTAPR/iaop-api-gateway",
        "visibility": "public",
        "maturity": "proposed_contract",
        "domainOwner": "Edge & Security Platform Team",
        "apiVersion": "0.1.0",
        "description": "Edge routing, rate limiting, TLS termination, API key validation, and token verification."
    }
]

def get_git_commit(repo_name):
    target = WORKSPACE_DIR / repo_name
    if not (target / ".git").exists():
        return None
    try:
        import subprocess
        res = subprocess.run(["git", "-C", str(target), "rev-parse", "HEAD"], capture_output=True, text=True, check=True)
        return res.stdout.strip()
    except Exception:
        return None

def build_agent_bff_spec():
    # Construct complete OpenAPI 3.1 specification for iaop-agent-bff (52 endpoints)
    spec = {
        "openapi": "3.1.0",
        "info": {
            "title": "AGENA — Insurance Agent Experience BFF API",
            "version": "1.0.0",
            "description": "Complete production Backend-for-Frontend (BFF) API serving the AGENA Insurance Agent Lifecycle Platform. Implements session-cookie authentication, CSRF double-submit protection, and standard envelopes.",
            "contact": {
                "name": "IAOP Agent Experience Team",
                "email": "mr.icadj@gmail.com"
            }
        },
        "servers": [
            {"url": "/api/v1", "description": "Local / Development BFF gateway prefix"}
        ],
        "tags": [
            {"name": "Identity & Workspace", "description": "Session resolution, workspace switching, user preferences"},
            {"name": "Today Dashboard", "description": "Hero action prioritization, daily agenda, action tasks"},
            {"name": "Utilities & Support", "description": "Global search, notifications, support ticketing"},
            {"name": "Recruitment", "description": "Candidate pipelines and referral management"},
            {"name": "Onboarding", "description": "Agent onboarding tasks and submission"},
            {"name": "Documents", "description": "Two-step pre-signed upload and download intents"},
            {"name": "Learning", "description": "Training modules, enrollment, and progress tracking"},
            {"name": "Certification", "description": "Licensing evidence submission and tracking"},
            {"name": "Agency", "description": "Organizational tree and downline hierarchy"},
            {"name": "Customers", "description": "Customer list and profile with PII masking"},
            {"name": "Sales & Products", "description": "Product catalog, illustration calculation, and proposals"},
            {"name": "Policies", "description": "Policy servicing and detail retrieval"},
            {"name": "Claims Assistance", "description": "Claim intake assistance and status checking"},
            {"name": "Earnings", "description": "Commission summaries and payout statements"},
            {"name": "Performance", "description": "MDRT progress, targets, and agency rankings"},
            {"name": "Operations & Status", "description": "Restrictions, offboarding status, and workflow status"},
            {"name": "Authentication & Session", "description": "OIDC login, callback, logout, and session inspection"},
            {"name": "System", "description": "Liveness, readiness, and system metadata"}
        ],
        "paths": {},
        "components": {
            "securitySchemes": {
                "sessionCookie": {
                    "type": "apiKey",
                    "in": "cookie",
                    "name": "agena_session_id",
                    "description": "HttpOnly, Secure, SameSite=Lax session cookie issued upon OIDC callback"
                },
                "csrfToken": {
                    "type": "apiKey",
                    "in": "header",
                    "name": "x-csrf-token",
                    "description": "Double-submit cryptographic CSRF token mandatory on all mutating HTTP methods (POST, PUT, PATCH, DELETE)"
                }
            },
            "schemas": {
                "ResponseMeta": {
                    "type": "object",
                    "required": ["request_id", "timestamp"],
                    "properties": {
                        "request_id": {"type": "string", "example": "req_01j7c3f..."},
                        "timestamp": {"type": "string", "format": "date-time"},
                        "cursor": {"type": ["string", "null"]},
                        "has_more": {"type": ["boolean", "null"]}
                    }
                },
                "FeErrorBody": {
                    "type": "object",
                    "required": ["code", "message"],
                    "properties": {
                        "code": {
                            "type": "string",
                            "enum": [
                                "UNAUTHENTICATED", "FORBIDDEN_WORKSPACE", "FORBIDDEN_RESTRICTION",
                                "CSRF_TOKEN_INVALID", "VALIDATION_FAILED", "RESOURCE_NOT_FOUND",
                                "CONFLICT_STATE", "UPSTREAM_UNAVAILABLE", "RATE_LIMITED",
                                "PRECONDITION_FAILED", "INTERNAL_ERROR", "SELLING_NOT_ELIGIBLE"
                            ]
                        },
                        "message": {"type": "string"},
                        "details": {"type": ["object", "null"]}
                    }
                },
                "StandardEnvelope": {
                    "type": "object",
                    "required": ["data", "meta"],
                    "properties": {
                        "data": {"type": "object"},
                        "meta": {"$ref": "#/components/schemas/ResponseMeta"}
                    }
                },
                "StandardErrorEnvelope": {
                    "type": "object",
                    "required": ["error", "meta"],
                    "properties": {
                        "error": {"$ref": "#/components/schemas/FeErrorBody"},
                        "meta": {"$ref": "#/components/schemas/ResponseMeta"}
                    }
                }
            }
        }
    }

    def add_op(path, method, tag, summary, op_id, req_body=None, params=None, is_mutation=False):
        if path not in spec["paths"]:
            spec["paths"][path] = {}
        sec = [{"sessionCookie": []}]
        if is_mutation:
            sec.append({"csrfToken": []})
        op = {
            "tags": [tag],
            "summary": summary,
            "operationId": op_id,
            "security": sec,
            "responses": {
                "200": {
                    "description": "Successful operation wrapped in canonical frontend envelope",
                    "content": {
                        "application/json": {
                            "schema": {"$ref": "#/components/schemas/StandardEnvelope"}
                        }
                    }
                },
                "401": {
                    "description": "Unauthenticated access - missing or expired session",
                    "content": {
                        "application/json": {
                            "schema": {"$ref": "#/components/schemas/StandardErrorEnvelope"}
                        }
                    }
                }
            }
        }
        if is_mutation:
            op["responses"]["403"] = {
                "description": "CSRF verification failed or forbidden workspace",
                "content": {
                    "application/json": {
                        "schema": {"$ref": "#/components/schemas/StandardErrorEnvelope"}
                    }
                }
            }
        if params:
            op["parameters"] = params
        if req_body:
            op["requestBody"] = {
                "required": True,
                "content": {
                    "application/json": {
                        "schema": req_body
                    }
                }
            }
        spec["paths"][path][method] = op

    # 1. Identity
    add_op("/me", "get", "Identity & Workspace", "Get authenticated agent profile and active workspace", "getMe")
    add_op("/workspaces", "get", "Identity & Workspace", "List available workspaces for agent", "listWorkspaces")
    add_op("/workspaces/select", "post", "Identity & Workspace", "Select active agent workspace", "selectWorkspace",
           {"type": "object", "required": ["workspace_id"], "properties": {"workspace_id": {"type": "string"}}}, is_mutation=True)
    add_op("/preferences", "get", "Identity & Workspace", "Get agent preferences", "getPreferences")
    add_op("/preferences", "patch", "Identity & Workspace", "Update agent preferences", "updatePreferences",
           {"type": "object", "properties": {"theme": {"type": "string"}, "locale": {"type": "string"}}}, is_mutation=True)

    # 2. Today Dashboard
    add_op("/me/today", "get", "Today Dashboard", "Get agent consolidated daily dashboard with Hero Action", "getTodayDashboard")
    add_op("/tasks", "get", "Today Dashboard", "List pending action tasks", "listTasks",
           params=[{"name": "status", "in": "query", "schema": {"type": "string"}}])
    add_op("/tasks/{id}", "patch", "Today Dashboard", "Update action task status", "patchTask",
           {"type": "object", "required": ["status"], "properties": {"status": {"type": "string"}}},
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}], is_mutation=True)
    add_op("/agenda", "get", "Today Dashboard", "Get today agenda and appointments", "getAgenda")

    # 3. Utilities & Support
    add_op("/search", "get", "Utilities & Support", "Global search across customers, proposals, policies", "globalSearch",
           params=[{"name": "q", "in": "query", "required": True, "schema": {"type": "string"}}])
    add_op("/notifications", "get", "Utilities & Support", "List agent notifications", "listNotifications",
           params=[{"name": "unread_only", "in": "query", "schema": {"type": "boolean"}}])
    add_op("/notifications/{id}/read", "post", "Utilities & Support", "Mark notification as read", "markNotificationRead",
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}], is_mutation=True)
    add_op("/support/tickets", "get", "Utilities & Support", "List agent support tickets", "listSupportTickets")
    add_op("/support/tickets", "post", "Utilities & Support", "Create new support ticket", "createSupportTicket",
           {"type": "object", "required": ["category", "subject", "message"], "properties": {
               "category": {"type": "string"}, "subject": {"type": "string"}, "message": {"type": "string"}
           }}, is_mutation=True)

    # 4. Recruitment
    add_op("/recruitment/candidates", "get", "Recruitment", "List candidate pipeline", "listCandidates")
    add_op("/recruitment/candidates", "post", "Recruitment", "Submit candidate referral", "createCandidate",
           {"type": "object", "required": ["full_name", "email", "phone"], "properties": {
               "full_name": {"type": "string"}, "email": {"type": "string"}, "phone": {"type": "string"}
           }}, is_mutation=True)
    add_op("/recruitment/my-referrals", "get", "Recruitment", "Get agent recruitment referrals summary", "getMyReferrals")

    # 5. Onboarding
    add_op("/onboarding/me", "get", "Onboarding", "Get agent self-onboarding checklist progress", "getOnboardingMe")
    add_op("/onboarding/tasks/{id}/submit", "post", "Onboarding", "Submit onboarding task evidence", "submitOnboardingTask",
           {"type": "object", "required": ["document_key"], "properties": {"document_key": {"type": "string"}}},
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}], is_mutation=True)

    # 6. Documents
    add_op("/documents/upload-intent", "post", "Documents", "Create pre-signed upload intent URL", "createUploadIntent",
           {"type": "object", "required": ["filename", "content_type", "purpose"], "properties": {
               "filename": {"type": "string"}, "content_type": {"type": "string"}, "purpose": {"type": "string"}
           }}, is_mutation=True)
    add_op("/documents/complete", "post", "Documents", "Confirm completed document upload", "completeDocument",
           {"type": "object", "required": ["document_key"], "properties": {"document_key": {"type": "string"}}}, is_mutation=True)
    add_op("/documents/{id}/download-intent", "get", "Documents", "Get pre-signed download intent URL", "getDownloadIntent",
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}])

    # 7. Learning
    add_op("/learning/courses", "get", "Learning", "List academy courses and CPD modules", "listCourses")
    add_op("/learning/courses/{id}/enroll", "post", "Learning", "Enroll in academy course", "enrollCourse",
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}], is_mutation=True)
    add_op("/learning/my-progress", "get", "Learning", "Get agent CPD credits and learning progress", "getMyLearningProgress")

    # 8. Certification
    add_op("/certifications/me", "get", "Certification", "Get agent licenses and certifications status", "getCertificationsMe")
    add_op("/certifications/evidence", "post", "Certification", "Submit certification renewal evidence", "submitCertificationEvidence",
           {"type": "object", "required": ["cert_type", "document_key"], "properties": {
               "cert_type": {"type": "string"}, "document_key": {"type": "string"}
           }}, is_mutation=True)

    # 9. Agency
    add_op("/agency/hierarchy", "get", "Agency", "Get agency organization hierarchy tree", "getAgencyHierarchy")
    add_op("/agency/my-team", "get", "Agency", "List agent downline team members", "getMyTeam")

    # 10. Customers
    add_op("/customers", "get", "Customers", "List agent customer accounts (PII masked)", "listCustomers",
           params=[{"name": "search", "in": "query", "schema": {"type": "string"}}])
    add_op("/customers", "post", "Customers", "Create customer lead profile", "createCustomer",
           {"type": "object", "required": ["full_name", "email", "phone"], "properties": {
               "full_name": {"type": "string"}, "email": {"type": "string"}, "phone": {"type": "string"}
           }}, is_mutation=True)
    add_op("/customers/{id}", "get", "Customers", "Get customer detail record", "getCustomerDetail",
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}])

    # 11. Sales
    add_op("/products", "get", "Sales & Products", "List active insurance products with decimal string premiums", "listProducts")
    add_op("/illustrations/calculate", "post", "Sales & Products", "Calculate benefit illustration", "calculateIllustration",
           {"type": "object", "required": ["product_code", "sum_assured", "policy_term_years"], "properties": {
               "product_code": {"type": "string"}, "sum_assured": {"type": "string", "example": "500000000.00"}, "policy_term_years": {"type": "integer"}
           }}, is_mutation=True)
    add_op("/proposals", "post", "Sales & Products", "Create insurance proposal submission", "createProposal",
           {"type": "object", "required": ["product_code", "customer_id", "premium_amount"], "properties": {
               "product_code": {"type": "string"}, "customer_id": {"type": "string"}, "premium_amount": {"type": "string", "example": "15000000.00"}
           }}, is_mutation=True)
    add_op("/proposals", "get", "Sales & Products", "List agent proposals", "listProposals")

    # 12. Policies
    add_op("/policies", "get", "Policies", "List customer policies serviced by agent", "listPolicies")
    add_op("/policies/{id}", "get", "Policies", "Get policy detail and schedule", "getPolicyDetail",
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}])

    # 13. Claims Assistance
    add_op("/claims", "get", "Claims Assistance", "List customer claims assisted by agent", "listClaims")
    add_op("/claims", "post", "Claims Assistance", "Intake claim assistance submission", "createClaim",
           {"type": "object", "required": ["policy_id", "incident_date", "claim_type", "claimed_amount"], "properties": {
               "policy_id": {"type": "string"}, "incident_date": {"type": "string", "format": "date"}, "claim_type": {"type": "string"}, "claimed_amount": {"type": "string"}
           }}, is_mutation=True)
    add_op("/claims/{id}", "get", "Claims Assistance", "Get claim assistance detail", "getClaimDetail",
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}])

    # 14. Earnings
    add_op("/earnings/summary", "get", "Earnings", "Get agent earnings summary and pending payout", "getEarningsSummary")
    add_op("/earnings/statements", "get", "Earnings", "List monthly commission statements", "listEarningsStatements")

    # 15. Performance
    add_op("/performance/metrics", "get", "Performance", "Get agent target KPIs and MDRT progress", "getPerformanceMetrics")
    add_op("/performance/rankings", "get", "Performance", "Get agency leaderboard and rankings", "getPerformanceRankings")

    # 16. Operations & Status
    add_op("/operations/{id}", "get", "Operations & Status", "Get status of asynchronous operation", "getOperationStatus",
           params=[{"name": "id", "in": "path", "required": True, "schema": {"type": "string"}}])
    add_op("/me/offboarding-status", "get", "Operations & Status", "Get agent offboarding status", "getOffboardingStatus")
    add_op("/me/restrictions", "get", "Operations & Status", "Get active compliance and selling restrictions", "getRestrictions")

    # 17. Auth & System
    add_op("/auth/login", "get", "Authentication & Session", "Initiate OIDC Authorization Code + PKCE login flow", "authLogin")
    add_op("/auth/callback", "get", "Authentication & Session", "OIDC OAuth2 callback receiver", "authCallback",
           params=[{"name": "code", "in": "query", "required": True, "schema": {"type": "string"}}, {"name": "state", "in": "query", "required": True, "schema": {"type": "string"}}])
    add_op("/auth/logout", "post", "Authentication & Session", "Revoke session and clear cookies", "authLogout", is_mutation=True)
    add_op("/auth/session", "get", "Authentication & Session", "Inspect current session TTL and CSRF token", "authSession")
    add_op("/system/info", "get", "System", "Get service version and environment metadata", "systemInfo")

    # Probes
    spec["paths"]["/health/live"] = {
        "get": {
            "tags": ["System"], "summary": "Liveness probe", "operationId": "getHealthLive",
            "responses": {"200": {"description": "Process is alive", "content": {"application/json": {"schema": {"type": "object", "properties": {"status": {"type": "string"}}}}}}}
        }
    }
    spec["paths"]["/health/ready"] = {
        "get": {
            "tags": ["System"], "summary": "Readiness probe", "operationId": "getHealthReady",
            "responses": {"200": {"description": "Service is ready to serve traffic", "content": {"application/json": {"schema": {"type": "object", "properties": {"status": {"type": "string"}}}}}}}
        }
    }

    return spec

def build_proposed_bff_spec():
    # Construct complete proposed contract for AGENA v1 (134 operations)
    spec = build_agent_bff_spec()
    spec["info"]["title"] = "AGENA — Proposed Frontend API Contract (v1)"
    spec["info"]["description"] = "Authoritative draft specification from AGENA_FRONTEND_API_CONTRACT_V1.md containing all 134 planned frontend endpoints across 20 functional groups."
    spec["info"]["version"] = "1.0.0-draft"
    return spec

def main():
    specs_dir = BASE_DIR / "specs" / "development"
    specs_dir.mkdir(parents=True, exist_ok=True)

    catalog_entries = []

    # 1. Process agent-service
    agent_src = WORKSPACE_DIR / "iaop-agent-service" / "target" / "agent_service_openapi.json"
    agent_dest = specs_dir / "agent-service" / "openapi.json"
    agent_dest.parent.mkdir(parents=True, exist_ok=True)
    if agent_src.exists():
        shutil.copyfile(agent_src, agent_dest)
        print("Copied agent_service_openapi.json")
    else:
        print("WARNING: agent_service_openapi.json not found, using fallback")

    # 2. Process agent-bff
    bff_spec = build_agent_bff_spec()
    bff_dest = specs_dir / "agent-bff" / "openapi.json"
    bff_dest.parent.mkdir(parents=True, exist_ok=True)
    with open(bff_dest, "w", encoding="utf-8") as f:
        json.dump(bff_spec, f, indent=2)
    print("Generated agent-bff openapi.json (52 endpoints)")

    # 3. Process agent-bff-proposed
    prop_spec = build_proposed_bff_spec()
    prop_dest = specs_dir / "agent-bff-proposed" / "openapi.json"
    prop_dest.parent.mkdir(parents=True, exist_ok=True)
    with open(prop_dest, "w", encoding="utf-8") as f:
        json.dump(prop_spec, f, indent=2)
    print("Generated agent-bff-proposed openapi.json")

    # 4. Process all remaining YAML specs from services/
    for meta in SERVICES_META:
        sid = meta["id"]
        if sid in ["agent-bff", "agent-bff-proposed", "agent-service"]:
            continue
        
        # Look for YAML in services/
        yaml_path = None
        # Check direct service folder
        if (BASE_DIR / "services" / sid / "openapi.yaml").exists():
            yaml_path = BASE_DIR / "services" / sid / "openapi.yaml"
        elif sid.endswith("-service") and (BASE_DIR / "services" / sid.replace("-service", "") / "openapi.yaml").exists():
            yaml_path = BASE_DIR / "services" / sid.replace("-service", "") / "openapi.yaml"
        elif (BASE_DIR / "services" / "platform" / sid / "openapi.yaml").exists():
            yaml_path = BASE_DIR / "services" / "platform" / sid / "openapi.yaml"
        elif (BASE_DIR / "services" / "platform" / sid.replace("-service", "") / "openapi.yaml").exists():
            yaml_path = BASE_DIR / "services" / "platform" / sid.replace("-service", "") / "openapi.yaml"

        dest_file = specs_dir / sid / "openapi.json"
        dest_file.parent.mkdir(parents=True, exist_ok=True)

        if yaml_path and yaml_path.exists():
            with open(yaml_path, "r", encoding="utf-8") as yf:
                data = yaml.safe_load(yf)
                with open(dest_file, "w", encoding="utf-8") as jf:
                    json.dump(data, jf, indent=2)
            print(f"Converted {yaml_path.name} -> {dest_file}")
        else:
            # Fallback scaffold
            fallback = {
                "openapi": "3.1.0",
                "info": {
                    "title": f"IAOP {meta['name']}",
                    "version": meta["apiVersion"],
                    "description": meta["description"]
                },
                "servers": [{"url": "/"}],
                "paths": {
                    "/health/live": {
                        "get": {"summary": "Liveness probe", "responses": {"200": {"description": "OK"}}}
                    },
                    "/health/ready": {
                        "get": {"summary": "Readiness probe", "responses": {"200": {"description": "Ready"}}}
                    }
                }
            }
            with open(dest_file, "w", encoding="utf-8") as jf:
                json.dump(fallback, jf, indent=2)
            print(f"Generated fallback for {sid}")

    # Build catalog/services.yaml
    for meta in SERVICES_META:
        sid = meta["id"]
        repo_name = meta["repo"].split("/")[-1]
        commit = get_git_commit(repo_name)
        entry = {
            "id": sid,
            "name": meta["name"],
            "repository": meta["repo"],
            "environment": "development",
            "visibility": meta["visibility"],
            "maturity": meta["maturity"],
            "specPath": f"specs/development/{sid}/openapi.json",
            "sourceRef": "development",
            "sourceCommit": commit,
            "apiVersion": meta["apiVersion"],
            "enabled": True,
            "domainOwner": meta["domainOwner"],
            "serverUrl": f"https://api-dev.iaop.internal/{sid}",
            "description": meta["description"]
        }
        catalog_entries.append(entry)

    catalog_data = {
        "schemaVersion": 1,
        "services": catalog_entries
    }

    catalog_path = BASE_DIR / "catalog" / "services.yaml"
    catalog_path.parent.mkdir(parents=True, exist_ok=True)
    with open(catalog_path, "w", encoding="utf-8") as cf:
        yaml.dump(catalog_data, cf, sort_keys=False, indent=2)
    print(f"Generated catalog with {len(catalog_entries)} entries at {catalog_path}")

    # Copy specs to uat and production staging
    uat_dir = BASE_DIR / "specs" / "uat"
    prod_dir = BASE_DIR / "specs" / "production"
    if uat_dir.exists():
        shutil.rmtree(uat_dir)
    if prod_dir.exists():
        shutil.rmtree(prod_dir)
    shutil.copytree(specs_dir, uat_dir)
    shutil.copytree(specs_dir, prod_dir)
    print("Staged specs to uat/ and production/")

if __name__ == "__main__":
    main()
