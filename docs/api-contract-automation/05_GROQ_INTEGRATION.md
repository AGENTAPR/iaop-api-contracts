# GROQ Integration

Tool: `tools/groq_contract_assistant.py`

```
python tools/groq_contract_assistant.py --service agency-service     --operation listAgencies --kind example     --evidence evidence/agency-service/listAgencies.json
```

Behavior:
- Builds a prompt containing ONLY the target operation's spec excerpt and the
  supplied evidence payload. Without evidence it instructs the model not to
  invent values.
- Calls GROQ (`GROQ_API_KEY` env, model `GROQ_MODEL` default
  llama-3.3-70b-versatile, JSON mode, temperature 0.2).
- Independently validates `--kind example` output against the resolved
  response schema — recursive `$ref`, `oneOf`/`anyOf`, array-item resolution.
- Writes to `proposals/<service>-<operation>-<kind>.json` with `_meta`
  provenance and `_validation` PASS/FAIL. It can NEVER write to `specs/`.

`--dry-run` prints the prompt without calling the API. Exit code 1 when
validation fails, so CI can gate on it.
