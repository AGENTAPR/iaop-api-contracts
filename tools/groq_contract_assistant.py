#!/usr/bin/env python3
"""Evidence-bounded GROQ contract assistant for iaop-api-contracts.

Drafts OpenAPI descriptions/examples using GROQ, but GROQ output is NEVER
authoritative: every proposal is validated against the target schema and
written to proposals/ for human review. It cannot modify specs/ directly.

Usage:
  python tools/groq_contract_assistant.py --service agency-service \
      --operation listAgencies --kind description \
      --evidence evidence/agency-service/listAgencies.json

Requires GROQ_API_KEY in the environment. Without it, --dry-run prints the
prompt that would be sent.
"""
from __future__ import annotations

import argparse
import json
import os
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPECS = ROOT / "specs" / "development"
PROPOSALS = ROOT / "proposals"
GROQ_URL = "https://api.groq.com/openai/v1/chat/completions"
GROQ_MODEL = os.environ.get("GROQ_MODEL", "llama-3.3-70b-versatile")

SYSTEM = (
    "You are a contract documentation assistant. You may ONLY use facts "
    "present in the provided OpenAPI operation and evidence payload. "
    "Never invent fields, endpoints, statuses, or business rules. If the "
    "evidence is insufficient, say so. Respond with strict JSON only."
)


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def find_operation(spec: dict, operation_id: str) -> tuple[str, dict] | None:
    for path, item in spec.get("paths", {}).items():
        for method, op in item.items():
            if isinstance(op, dict) and op.get("operationId") == operation_id:
                return path, op
    return None


def resolve_schema(spec: dict, ref: str) -> dict | None:
    """Resolve a local '#/components/schemas/X' ref."""
    if not ref.startswith("#/"):
        return None
    node = spec
    for part in ref[2:].split("/"):
        node = node.get(part, {}) if isinstance(node, dict) else {}
    return node or None


def response_schema_for(op: dict, spec: dict, status: str = "200") -> dict | None:
    content = (
        op.get("responses", {})
        .get(status, {})
        .get("content", {})
        .get("application/json", {})
    )
    schema = content.get("schema", {})
    ref = schema.get("$ref")
    if ref:
        return resolve_schema(spec, ref)
    return schema or None


def type_compatible(value, schema: dict) -> bool:
    """Lightweight type check: string/integer/number/boolean/array/object."""
    t = schema.get("type")
    if not t:
        return True
    if t == "string":
        return isinstance(value, str)
    if t == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if t == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if t == "boolean":
        return isinstance(value, bool)
    if t == "array":
        return isinstance(value, list) and all(
            type_compatible(v, schema.get("items", {})) for v in value
        )
    if t == "object":
        return isinstance(value, dict)
    return True


def validate_example(example: dict, schema: dict, spec: dict, path: str = "") -> list[str]:
    """Validate example keys/types against schema properties recursively."""
    errors: list[str] = []
    ref = schema.get("$ref")
    if ref:
        resolved = resolve_schema(spec, ref)
        if resolved is None:
            return [f"{path}: unresolvable $ref {ref}"]
        return validate_example(example, resolved, spec, path)
    if "oneOf" in schema or "anyOf" in schema:
        variants = schema.get("oneOf") or schema.get("anyOf") or []
        for v in variants:
            errs = validate_example(example, v, spec, path)
            if not errs:
                return []
        return [f"{path}: does not match any allowed variant"]
    if schema.get("type") == "array" and isinstance(example, list):
        item_schema = schema.get("items", {})
        if "$ref" in item_schema:
            resolved = resolve_schema(spec, item_schema["$ref"])
            if resolved is None:
                return [f"{path}[]: unresolvable item $ref {item_schema['$ref']}"]
            item_schema = resolved
        for i, v in enumerate(example):
            errors.extend(validate_example(v, item_schema, spec, f"{path}[{i}]"))
        return errors
    if not type_compatible(example, schema):
        return [f"{path}: type mismatch (expected {schema.get('type')})"]
    if schema.get("type") == "object" and isinstance(example, dict):
        props = schema.get("properties", {})
        required = set(schema.get("required", []))
        for key in required:
            if key not in example:
                errors.append(f"{path}.{key}: missing required property")
        for key, val in example.items():
            if key in props:
                errors.extend(validate_example(val, props[key], spec, f"{path}.{key}"))
            elif schema.get("additionalProperties") is False:
                errors.append(f"{path}.{key}: property not in schema")
    return errors


def build_prompt(service: str, operation_id: str, kind: str, op: dict,
                 spec: dict, evidence: dict | None) -> str:
    parts = [
        f"Service: {service}",
        f"Operation: {operation_id}",
        f"Method+Path: {op.get('_method','?').upper()} {op.get('_path','?')}",
        f"Summary: {op.get('summary','')}",
        f"Description: {op.get('description','') or '(none)'}",
        "Response schema (200):",
        json.dumps(response_schema_for(op, spec) or {}, indent=2)[:6000],
    ]
    if evidence is not None:
        parts.append("VERIFIED EVIDENCE (captured live/fixture response):")
        parts.append(json.dumps(evidence, indent=2)[:8000])
    else:
        parts.append("No evidence payload supplied — do not invent example values.")
    if kind == "description":
        parts.append('Task: write a 1-3 sentence description. Output JSON: {"description": "..."}')
    else:
        parts.append(
            'Task: produce a realistic EXAMPLE response that conforms exactly '
            'to the schema above and is consistent with the evidence. '
            'Output JSON: {"example": {...}}'
        )
    return "\n".join(parts)


def call_groq(prompt: str, api_key: str) -> dict:
    body = json.dumps({
        "model": GROQ_MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM},
            {"role": "user", "content": prompt},
        ],
        "temperature": 0.2,
        "response_format": {"type": "json_object"},
    }).encode()
    req = urllib.request.Request(
        GROQ_URL, data=body,
        headers={"Authorization": f"Bearer {api_key}", "Content-Type": "application/json"},
    )
    with urllib.request.urlopen(req, timeout=60) as resp:
        data = json.loads(resp.read())
    content = data["choices"][0]["message"]["content"]
    return json.loads(content)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--service", required=True)
    ap.add_argument("--operation", required=True)
    ap.add_argument("--kind", choices=["description", "example"], default="description")
    ap.add_argument("--evidence", type=Path, default=None)
    ap.add_argument("--status", default="200")
    ap.add_argument("--dry-run", action="store_true")
    args = ap.parse_args()

    spec_path = SPECS / args.service / "openapi.json"
    if not spec_path.exists():
        print(f"ERROR: spec not found: {spec_path}", file=sys.stderr)
        return 2
    spec = load_json(spec_path)

    found = find_operation(spec, args.operation)
    if not found:
        print(f"ERROR: operationId '{args.operation}' not found in {args.service}", file=sys.stderr)
        return 2
    path, op = found
    op = dict(op)
    op["_path"], op["_method"] = path, next(
        m for m in ("get", "post", "put", "patch", "delete") if m in spec["paths"][path]
    )

    evidence = load_json(args.evidence) if args.evidence else None
    prompt = build_prompt(args.service, args.operation, args.kind, op, spec, evidence)

    api_key = os.environ.get("GROQ_API_KEY")
    if args.dry_run or not api_key:
        print("=== PROMPT ===")
        print(prompt)
        if not api_key and not args.dry_run:
            print("\n(GROQ_API_KEY not set — dry run only)")
        return 0

    try:
        result = call_groq(prompt, api_key)
    except Exception as e:  # noqa: BLE001
        print(f"ERROR: GROQ call failed: {e}", file=sys.stderr)
        return 3

    # Independent validation — AI output is never trusted.
    if args.kind == "example":
        schema = response_schema_for(
            {**op, "responses": op.get("responses", {})}, spec, args.status
        )
        if schema:
            errors = validate_example(result.get("example", {}), schema, spec)
            result["_validation"] = "PASS" if not errors else "FAIL"
            result["_validation_errors"] = errors
        else:
            result["_validation"] = "SKIPPED (no schema)"

    result["_meta"] = {
        "service": args.service,
        "operationId": args.operation,
        "kind": args.kind,
        "model": GROQ_MODEL,
        "evidence": str(args.evidence) if args.evidence else None,
        "note": "AI-assisted proposal — requires human review before merging into specs/",
    }

    PROPOSALS.mkdir(exist_ok=True)
    out = PROPOSALS / f"{args.service}-{args.operation}-{args.kind}.json"
    out.write_text(json.dumps(result, indent=2), encoding="utf-8")
    print(f"Proposal written: {out}")
    print(f"Validation: {result.get('_validation', 'n/a')}")
    if result.get("_validation_errors"):
        for e in result["_validation_errors"]:
            print(f"  - {e}")
    return 0 if result.get("_validation") in ("PASS", "n/a", "SKIPPED (no schema)") else 1


if __name__ == "__main__":
    sys.exit(main())
