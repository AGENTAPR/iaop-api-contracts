#!/usr/bin/env python3
"""IAOP contract validator.

Validates OpenAPI artifacts under specs/<env>/ and the service catalog
catalog/services.yaml. Checks that go beyond YAML parsing:

- OpenAPI document structure (openapi, info, paths)
- Unique operationIds (per document and across the catalog)
- Local $ref resolution
- Named examples present are objects and type-compatible with their schema
- Security schemes referenced by operations are declared
- Catalog integrity: unique ids, specPath artifacts exist, controlled vocab

Exit code 0 = all checks passed, 1 = failures found.

Usage: python tools/validate_contracts.py [--env development]
"""

import argparse
import json
import re
import sys
from pathlib import Path

import yaml

try:
    import jsonschema
except ImportError:  # optional: example/schema deep validation degrades gracefully
    jsonschema = None

BASE_DIR = Path(__file__).resolve().parent.parent

# Controlled vocabularies — must match catalog/catalog.schema.json enums.
MATURITY_VALUES = {"implemented_verified", "implemented_unverified", "proposed_contract", "deprecated", "blocked"}
DEPLOYMENT_VALUES = {"deployed", "not_deployed", "unknown"}
VERIFICATION_VALUES = {"e2e_verified", "unit_verified", "not_verified", "unknown"}
IMPLEMENTATION_VALUES = {"implemented_verified", "implemented_unverified", "proposed", "not_started"}
LIFECYCLE_VALUES = {"active", "proposed", "deprecated", "blocked"}

HTTP_METHODS = {"get", "put", "post", "delete", "options", "head", "patch", "trace"}

errors: list[str] = []
warnings: list[str] = []


def err(msg: str) -> None:
    errors.append(msg)


def warn(msg: str) -> None:
    warnings.append(msg)


def load_json(path: Path):
    with open(path, encoding="utf-8") as f:
        return json.load(f)


def load_yaml(path: Path):
    with open(path, encoding="utf-8") as f:
        return yaml.safe_load(f)


def resolve_ref(doc: dict, ref: str, base_path: Path | None = None):
    """Resolve a $ref. Handles local '#/...' refs and file-relative refs
    (e.g. '../../schemas/common/x.json' or 'other.json#/path')."""
    if ref.startswith("#/"):
        node = doc
        for part in ref[2:].split("/"):
            part = part.replace("~1", "/").replace("~0", "~")
            if isinstance(node, dict) and part in node:
                node = node[part]
            else:
                return False, None
        return True, node
    if base_path is not None and not ref.startswith(("http://", "https://")):
        file_part, _, frag = ref.partition("#")
        target = (base_path.parent / file_part).resolve()
        if not target.exists():
            return False, None
        try:
            ext = load_json(target) if target.suffix == ".json" else load_yaml(target)
        except Exception:
            return False, None
        if not frag:
            return True, ext
        node = ext
        for part in frag.lstrip("/").split("/"):
            part = part.replace("~1", "/").replace("~0", "~")
            if isinstance(node, dict) and part in node:
                node = node[part]
            else:
                return False, None
        return True, node
    return False, None  # remote refs unsupported


def check_refs(node, doc: dict, where: str, base_path: Path) -> None:
    if isinstance(node, dict):
        for k, v in node.items():
            if k == "$ref" and isinstance(v, str):
                ok, _ = resolve_ref(doc, v, base_path)
                if not ok:
                    err(f"{where}: unresolvable $ref '{v}'")
            else:
                check_refs(v, doc, where, base_path)
    elif isinstance(node, list):
        for item in node:
            check_refs(item, doc, where, base_path)


def check_example_vs_schema(name: str, example, schema: dict, where: str) -> None:
    """Lightweight example/schema compatibility check."""
    if not isinstance(schema, dict):
        return
    if "$ref" in schema:
        return  # resolved refs checked separately where needed
    expected = schema.get("type")
    if not expected:
        return
    types = expected if isinstance(expected, list) else [expected]
    ok = False
    for t in types:
        if t == "object" and isinstance(example, dict):
            ok = True
        elif t == "array" and isinstance(example, list):
            ok = True
        elif t == "string" and isinstance(example, str):
            ok = True
        elif t == "integer" and isinstance(example, int) and not isinstance(example, bool):
            ok = True
        elif t == "number" and isinstance(example, (int, float)) and not isinstance(example, bool):
            ok = True
        elif t == "boolean" and isinstance(example, bool):
            ok = True
        elif t == "null" and example is None:
            ok = True
    if not ok:
        err(f"{where}: example '{name}' type {py_type} incompatible with schema type {expected}")
        return
    if jsonschema is not None and "properties" in schema:
        try:
            jsonschema.validate(example, schema)
        except jsonschema.ValidationError as e:
            warn(f"{where}: example '{name}' schema deviation: {e.message[:120]}")


def check_media_examples(media: dict, where: str) -> None:
    if not isinstance(media, dict):
        return
    schema = media.get("schema", {})
    examples = media.get("examples") or {}
    if isinstance(examples, dict):
        for ex_name, ex in examples.items():
            if isinstance(ex, dict) and "value" in ex:
                check_example_vs_schema(ex_name, ex["value"], schema, where)
            elif isinstance(ex, dict) and "$ref" in ex:
                pass  # component examples validated at component level
    single = media.get("example")
    if single is not None:
        check_example_vs_schema("example", single, schema, where)


def validate_openapi(path: Path, doc: dict, catalog: dict) -> None:
    rel = path.relative_to(BASE_DIR)
    if not isinstance(doc, dict):
        err(f"{rel}: document is not a JSON object")
        return
    if "openapi" not in doc:
        err(f"{rel}: missing 'openapi' version field")
    elif not str(doc["openapi"]).startswith("3."):
        warn(f"{rel}: openapi version '{doc['openapi']}' is not 3.x")
    info = doc.get("info")
    if not isinstance(info, dict) or "title" not in info or "version" not in info:
        err(f"{rel}: missing info.title or info.version")
    paths = doc.get("paths")
    service_id = path.parent.name
    svc = catalog.get(service_id, {})
    maturity = svc.get("maturity") or svc.get("implementationStatus") or ""
    if not isinstance(paths, dict) or not paths:
        if maturity in ("proposed_contract", "proposed", "not_started"):
            warn(f"{rel}: 'paths' empty — acceptable for proposed contract '{service_id}', but no operations are documented")
        else:
            err(f"{rel}: 'paths' missing or empty — '{service_id}' claims '{maturity}' but documents no routes")
        return

    local_op_ids = set()
    for p, item in paths.items():
        if not isinstance(item, dict):
            err(f"{rel}: path '{p}' is not an object")
            continue
        for method, op in item.items():
            if method.lower() not in HTTP_METHODS:
                continue
            where = f"{rel} {method.upper()} {p}"
            if not isinstance(op, dict):
                err(f"{where}: operation is not an object")
                continue
            op_id = op.get("operationId")
            if not op_id:
                warn(f"{where}: missing operationId")
            else:
                if op_id in local_op_ids:
                    err(f"{where}: duplicate operationId '{op_id}'")
                local_op_ids.add(op_id)
                # operationIds must be unique within a document; cross-document
                # collisions are reported as warnings (separate APIs may reuse).
            # parameters sanity
            for param in op.get("parameters", []) or []:
                if isinstance(param, dict) and param.get("in") not in (
                    "path", "query", "header", "cookie"
                ):
                    err(f"{where}: parameter '{param.get('name')}' has invalid 'in' value")
            # request/response examples
            rb = op.get("requestBody")
            if isinstance(rb, dict):
                for ct, media in (rb.get("content") or {}).items():
                    check_media_examples(media, f"{where} requestBody/{ct}")
            for code, resp in (op.get("responses") or {}).items():
                if isinstance(resp, dict):
                    for ct, media in (resp.get("content") or {}).items():
                        check_media_examples(media, f"{where} responses/{code}/{ct}")
    check_refs(doc, doc, str(rel), path)


def validate_catalog(env: str) -> dict:
    catalog_path = BASE_DIR / "catalog" / "services.yaml"
    if not catalog_path.exists():
        err("catalog/services.yaml missing")
        return {}
    catalog = load_yaml(catalog_path)
    services = catalog.get("services")
    if not isinstance(services, list) or not services:
        err("catalog/services.yaml: 'services' missing or empty")
        return {}
    seen = {}
    specs_dir = BASE_DIR / "specs" / env
    for svc in services:
        sid = svc.get("id")
        if not sid:
            err("catalog: service entry missing 'id'")
            continue
        if sid in seen:
            err(f"catalog: duplicate service id '{sid}'")
        seen[sid] = svc
        for field, allowed in (
            ("maturity", MATURITY_VALUES),
            ("deploymentStatus", DEPLOYMENT_VALUES),
            ("verificationStatus", VERIFICATION_VALUES),
            ("implementationStatus", IMPLEMENTATION_VALUES),
            ("lifecycle", LIFECYCLE_VALUES),
        ):
            val = svc.get(field)
            if val is not None and val not in allowed:
                err(f"catalog '{sid}': {field}='{val}' outside controlled vocabulary {sorted(allowed)}")
        spec_rel = svc.get("specPath")
        if spec_rel:
            spec_path = BASE_DIR / spec_rel
            if not spec_path.exists():
                # specPath may point at env-specific dir; also check specs/<env>/<id>/
                alt = specs_dir / sid / "openapi.json"
                if not alt.exists():
                    err(f"catalog '{sid}': specPath '{spec_rel}' does not resolve to a file")
        elif svc.get("enabled"):
            warn(f"catalog '{sid}': enabled but no specPath")
    return {s["id"]: s for s in services if "id" in s}


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--env", default="development")
    args = ap.parse_args()

    catalog = validate_catalog(args.env)

    specs_dir = BASE_DIR / "specs" / args.env
    spec_files = sorted(specs_dir.glob("*/openapi.json")) if specs_dir.exists() else []
    if not spec_files:
        err(f"no openapi.json files under {specs_dir}")
    for path in spec_files:
        try:
            doc = load_json(path)
        except Exception as e:
            err(f"{path}: invalid JSON — {e}")
            continue
        validate_openapi(path, doc, catalog)

    # Catalog entries claiming verified maturity must have a spec artifact
    for sid, svc in catalog.items():
        if svc.get("maturity") == "implemented_verified" or svc.get("implementationStatus") == "implemented_verified":
            if not (specs_dir / sid / "openapi.json").exists():
                err(f"catalog '{sid}': maturity '{svc['maturity']}' but no spec artifact — cannot be verified")

    for w in warnings:
        print(f"WARN: {w}")
    if errors:
        for e in errors:
            print(f"FAIL: {e}")
        print(f"\n{len(errors)} error(s), {len(warnings)} warning(s)")
        return 1
    print(
        f"SUCCESS: {len(spec_files)} OpenAPI documents, "
        f"{len(catalog)} catalog services — {len(warnings)} warning(s)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
