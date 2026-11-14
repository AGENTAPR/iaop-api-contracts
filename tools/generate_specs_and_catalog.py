"""
IAOP API Contracts — Spec & Catalog Generator (V2)

Extracts or copies OpenAPI specs from built service artifacts into
specs/development/. Does NOT fabricate endpoint definitions, does NOT
copy specs to UAT/production, and does NOT generate an agent-bff-proposed entry.

For services where no fresh source exists, the pinned spec already in
specs/development/{id}/openapi.json is used as-is (with a warning).
"""

import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

BASE_DIR = Path(__file__).resolve().parent.parent
WORKSPACE_DIR = BASE_DIR.parent

# ---------------------------------------------------------------------------
# Service metadata — single source of truth for the generator.
# agent-bff-proposed has been removed permanently.
# ---------------------------------------------------------------------------
SERVICES_META = [
    {
        "id": "agent-bff",
        "catalog": "frontend",
        "channel": "agent",
        "bounded_context": "Agent & Distribution",
        "lifecycle": "active",
        "repo": "AGENTAPR/iaop-agent-bff",
        "visibility": "public",
        "maturity": "implemented_verified",
        "implementation_status": "implemented_verified",
        "deployment_status": "deployed",
        "verification_status": "e2e_verified",
        "domainOwner": "Agent Platform & Experience Team",
        "apiVersion": "1.0.0",
        "serverUrl": "https://8082-agq5lpbszgbva3ygddcbarv6ea.instabox.tech",
        "description": (
            "AGENA Insurance Agent Lifecycle Platform BFF — Complete verified implementation "
            "covering 52 endpoints across Identity, Today Dashboard, Sales, Documents, and Earnings."
        ),
    },
    {
        "id": "operations-bff",
        "catalog": "frontend",
        "channel": "office",
        "bounded_context": "Back Office",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-operations-bff",
        "visibility": "restricted",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Back Office Operations Team",
        "apiVersion": "0.1.0",
        "description": "Operations portal BFF for underwriting, compliance approvals, manual reviews, and admin.",
    },
    {
        "id": "agent-service",
        "catalog": "backend",
        "bounded_context": "Agent & Distribution",
        "lifecycle": "active",
        "repo": "AGENTAPR/iaop-agent-service",
        "visibility": "internal",
        "maturity": "implemented_verified",
        "implementation_status": "implemented_verified",
        "deployment_status": "deployed",
        "verification_status": "unit_verified",
        "domainOwner": "Agent Core Domain Team",
        "apiVersion": "1.0.0",
        "description": "Agent Registry, appointments, identity links, and lifecycle state machine (17 operations verified).",
    },
    {
        "id": "agency-service",
        "catalog": "backend",
        "bounded_context": "Agent & Distribution",
        "lifecycle": "active",
        "repo": "AGENTAPR/iaop-agency-service",
        "visibility": "internal",
        "maturity": "implemented_unverified",
        "implementation_status": "implemented_unverified",
        "deployment_status": "deployed",
        "verification_status": "not_verified",
        "domainOwner": "Agency Hierarchy Team",
        "apiVersion": "0.1.0",
        "description": "Organizational hierarchy, assignments, agency leadership, and effective-dated relationships.",
    },
    {
        "id": "recruitment-service",
        "catalog": "backend",
        "bounded_context": "Recruitment & Onboarding",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-recruitment-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Recruitment & Talent Team",
        "apiVersion": "0.1.0",
        "description": "Candidate pipeline, candidate screening, referrals, and candidate invitation workflows.",
    },
    {
        "id": "onboarding-service",
        "catalog": "backend",
        "bounded_context": "Recruitment & Onboarding",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-onboarding-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Agent Onboarding Team",
        "apiVersion": "0.1.0",
        "description": "Agent onboarding checklists, e-KYC integration, document submission, and contracting.",
    },
    {
        "id": "learning-service",
        "catalog": "backend",
        "bounded_context": "Learning & Certification",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-learning-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Academy & Training Team",
        "apiVersion": "0.1.0",
        "description": "Agent training modules, mandatory CPD credits, course enrollments, and quiz progress.",
    },
    {
        "id": "certification-service",
        "catalog": "backend",
        "bounded_context": "Learning & Certification",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-certification-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Compliance & Licensing Team",
        "apiVersion": "0.1.0",
        "description": "AAJI, AASI, and OJK certification tracking, license renewals, and exam proof evidence.",
    },
    {
        "id": "eligibility-service",
        "catalog": "backend",
        "bounded_context": "Integration & Platform",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-eligibility-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Underwriting & Compliance Team",
        "apiVersion": "0.1.0",
        "description": "Real-time selling eligibility checks, product accreditation rules, and license validation.",
    },
    {
        "id": "sales-service",
        "catalog": "backend",
        "bounded_context": "Sales & Insurance",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-sales-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Sales Enablement Team",
        "apiVersion": "0.1.0",
        "description": "Product catalog, premium illustrations, benefit calculation, and proposal lifecycle.",
    },
    {
        "id": "portfolio-service",
        "catalog": "backend",
        "bounded_context": "Sales & Insurance",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-portfolio-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Policy Admin Team",
        "apiVersion": "0.1.0",
        "description": "Customer policy portfolio, policy status, premium billing schedules, and servicing.",
    },
    {
        "id": "commission-service",
        "catalog": "backend",
        "bounded_context": "Sales & Insurance",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-commission-service",
        "visibility": "restricted",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Finance & Compensation Team",
        "apiVersion": "0.1.0",
        "description": "Commission calculation, hierarchy splits, clawback management, and payout statements.",
    },
    {
        "id": "compliance-service",
        "catalog": "backend",
        "bounded_context": "Compliance & Performance",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-compliance-service",
        "visibility": "restricted",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Legal & Risk Governance Team",
        "apiVersion": "0.1.0",
        "description": "Compliance breach tracking, regulatory sanctions, market conduct reviews, and selling blocks.",
    },
    {
        "id": "performance-service",
        "catalog": "backend",
        "bounded_context": "Compliance & Performance",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-performance-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Agency Performance Team",
        "apiVersion": "0.1.0",
        "description": "Sales targets, MDRT progress, agency leaderboard, persistency ratios, and campaigns.",
    },
    {
        "id": "workflow-service",
        "catalog": "backend",
        "bounded_context": "Workflow & Documents",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-workflow-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Business Process Team",
        "apiVersion": "0.1.0",
        "description": "BPMN execution, human task assignment, approval transitions, and process history.",
    },
    {
        "id": "integration-service",
        "catalog": "backend",
        "bounded_context": "Integration & Platform",
        "lifecycle": "active",
        "repo": "AGENTAPR/iaop-integration-service",
        "visibility": "restricted",
        "maturity": "implemented_verified",
        "implementation_status": "implemented_verified",
        "deployment_status": "deployed",
        "verification_status": "unit_verified",
        "domainOwner": "Integration Platform Team",
        "apiVersion": "1.0.0",
        "description": (
            "External operations, multi-provider connectors, HMAC-SHA256 callbacks, "
            "and state reconciliation (10 operations implemented & verified)."
        ),
    },
    {
        "id": "notification-service",
        "catalog": "backend",
        "bounded_context": "Notifications",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-notification-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Customer Communications Team",
        "apiVersion": "0.1.0",
        "description": "Multi-channel notifications (Push, Email, WhatsApp, SMS), templates, and delivery dispatch.",
    },
    {
        "id": "document-service",
        "catalog": "backend",
        "bounded_context": "Workflow & Documents",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-document-service",
        "visibility": "internal",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Content & Storage Platform Team",
        "apiVersion": "0.1.0",
        "description": "Secure document storage, pre-signed upload/download URLs, optical character recognition, and retention.",
    },
    {
        "id": "api-gateway",
        "catalog": "backend",
        "bounded_context": "Integration & Platform",
        "lifecycle": "proposed",
        "repo": "AGENTAPR/iaop-api-gateway",
        "visibility": "public",
        "maturity": "proposed_contract",
        "implementation_status": "proposed",
        "deployment_status": "not_deployed",
        "verification_status": "not_verified",
        "domainOwner": "Edge & Security Platform Team",
        "apiVersion": "0.1.0",
        "description": "Edge routing, rate limiting, TLS termination, API key validation, and token verification.",
    },
]


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def get_git_commit(repo_name: str):
    target = WORKSPACE_DIR / repo_name
    if not (target / ".git").exists():
        return None
    try:
        res = subprocess.run(
            ["git", "-C", str(target), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        )
        return res.stdout.strip()
    except Exception:
        return None


def extract_bff_spec(source_repo_dir: Path) -> dict:
    """
    Extract the actual OpenAPI spec from agent-bff source.

    Looks for the utoipa-generated artifact produced by running
    `cargo run --bin export-openapi` in the agent-bff repository.

    Raises FileNotFoundError if no built artifact is found.
    """
    candidates = [
        source_repo_dir / "target" / "agent_bff_openapi.json",
        source_repo_dir / "target" / "openapi.json",
    ]
    for candidate in candidates:
        if candidate.exists():
            with open(candidate, encoding="utf-8") as f:
                return json.load(f)
    raise FileNotFoundError(
        f"agent-bff OpenAPI source not found.\n"
        f"Run 'cargo run --bin export-openapi' in {source_repo_dir} first.\n"
        f"Checked: {candidates}"
    )


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------


def main():
    specs_dir = BASE_DIR / "specs" / "development"
    specs_dir.mkdir(parents=True, exist_ok=True)

    errors = []

    # ------------------------------------------------------------------
    # 1. agent-bff: extract from source artifact, fall back to pinned spec
    # ------------------------------------------------------------------
    bff_dest = specs_dir / "agent-bff" / "openapi.json"
    bff_dest.parent.mkdir(parents=True, exist_ok=True)
    bff_source_repo = WORKSPACE_DIR / "iaop-agent-bff"

    try:
        bff_spec = extract_bff_spec(bff_source_repo)
        with open(bff_dest, "w", encoding="utf-8") as f:
            json.dump(bff_spec, f, indent=2)
        print("INFO: agent-bff spec extracted from source artifact.")
    except FileNotFoundError as e:
        if bff_dest.exists():
            print(
                f"WARNING: {e}\n"
                f"  → Using existing pinned spec at {bff_dest} (last-known-good)."
            )
        else:
            errors.append(
                f"ERROR: agent-bff spec not available and no pinned spec exists.\n  {e}"
            )

    # ------------------------------------------------------------------
    # 2. agent-service: copy from built artifact, fall back to pinned spec
    # ------------------------------------------------------------------
    agent_src = WORKSPACE_DIR / "iaop-agent-service" / "target" / "agent_service_openapi.json"
    agent_dest = specs_dir / "agent-service" / "openapi.json"
    agent_dest.parent.mkdir(parents=True, exist_ok=True)

    if agent_src.exists():
        shutil.copyfile(agent_src, agent_dest)
        print("INFO: agent-service spec copied from build artifact.")
    elif agent_dest.exists():
        print("WARNING: agent-service source artifact not found, using existing pinned spec.")
    else:
        errors.append(
            f"ERROR: agent-service spec not found and no pinned spec exists.\n"
            f"  Checked: {agent_src}"
        )

    # ------------------------------------------------------------------
    # 3. All other services: copy YAML/JSON from services/ directory
    #    or fall back to existing pinned spec.
    # ------------------------------------------------------------------
    for meta in SERVICES_META:
        sid = meta["id"]
        if sid in ("agent-bff", "agent-service"):
            continue  # Handled above

        dest_dir = specs_dir / sid
        dest_dir.mkdir(parents=True, exist_ok=True)

        # Probe candidate source locations
        json_src = BASE_DIR / "services" / sid / "openapi.json"
        yaml_src = BASE_DIR / "services" / sid / "openapi.yaml"

        if json_src.exists():
            shutil.copyfile(json_src, dest_dir / "openapi.json")
            print(f"INFO: {sid} spec copied from services/{sid}/openapi.json")
        elif yaml_src.exists():
            shutil.copyfile(yaml_src, dest_dir / "openapi.yaml")
            print(f"INFO: {sid} spec copied from services/{sid}/openapi.yaml")
        elif (dest_dir / "openapi.json").exists() or (dest_dir / "openapi.yaml").exists():
            print(f"WARNING: {sid} source not found — using existing pinned spec (no changes).")
        else:
            print(
                f"WARNING: {sid} has no spec source and no pinned spec — service will return "
                f"404 when spec is requested from portal."
            )

    # ------------------------------------------------------------------
    # 4. Exit with error if any hard failures occurred
    # ------------------------------------------------------------------
    if errors:
        print("\n--- GENERATION ERRORS ---", file=sys.stderr)
        for err in errors:
            print(err, file=sys.stderr)
        sys.exit(1)

    print(
        "\nGeneration complete."
        "\nNote: UAT/production specs are NOT modified by this script."
        "\nNote: agent-bff-proposed has been permanently removed."
    )


if __name__ == "__main__":
    main()
