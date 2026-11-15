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
# Canonical service registry
# catalog/services.yaml is the single source of truth for service metadata.
# This generator derives its inputs from it — no duplicated inventory.
# ---------------------------------------------------------------------------

import yaml

CATALOG_PATH = BASE_DIR / "catalog" / "services.yaml"


def load_service_ids() -> list:
    """Return the ordered list of service ids from the canonical catalog."""
    with open(CATALOG_PATH, encoding="utf-8") as f:
        catalog = yaml.safe_load(f)
    return [svc["id"] for svc in catalog.get("services", [])]


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
    for sid in load_service_ids():
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
