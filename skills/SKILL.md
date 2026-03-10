---
name: mem9
version: 0.1.0
description: Provision, reconnect, search, and import mem9 cloud memory from IronClaw using the mem9 REST API.
activation:
  keywords:
    - mem9
    - install mem9
    - setup mem9
    - connect mem9
    - cloud memory
    - import memories
    - upload memories
    - memory backup
    - restore memory
  patterns:
    - "install.*mem9"
    - "setup.*mem9"
    - "connect.*mem9"
    - "import.*mem(ory|ories).*mem9"
    - "backup.*memory"
    - "restore.*memory"
  max_context_tokens: 2600
---

# mem9 for IronClaw

Use this skill when the user wants mem9 cloud memory with IronClaw.

## Scope

- IronClaw does not expose OpenClaw's `plugins.slots.memory` or `openclaw.json` memory-plugin flow.
- Do not tell the user to run `openclaw plugins install @mem9/mem9`.
- Do not claim mem9 automatically injects context into IronClaw prompts.
- In IronClaw today, mem9 is an external cloud memory companion reached through the mem9 REST API.
- Prefer IronClaw's built-in `http`, `memory_tree`, `memory_read`, `memory_search`, and `memory_write` tools.
- Use shell `curl` only when the `http` tool is unavailable or the user explicitly asks for shell commands.

## Terminology

When talking to users:

- Say "space ID" or "cloud memory space ID"
- Do not lead with `tenantID`
- If the user asks about `tenantID`, explain that it is the mem9-internal field name for the same space ID value
- Treat "space ID" and "token" as the same value unless the user clearly means something else

Plain explanation:

> This ID tells mem9 which cloud memory space to use. Reusing the same ID reconnects the same cloud memory later.

## First Question

Ask this before doing anything else:

> Do you already have a mem9 space ID from a previous install or another machine?

If the user already has one:

1. Save it as `SPACE_ID`
2. Verify it before continuing

Verification request:

```bash
curl -sf "https://api.mem9.ai/v1alpha1/mem9s/$SPACE_ID/memories?limit=1" \
  && echo "OK" || echo "UNREACHABLE"
```

If the check succeeds, say:

> Connected to your existing cloud memory space. Continuing with that space ID.

If it fails, say:

> That space ID did not respond. Double-check it, or create a new space instead.

If the user does not have one, provision a new space.

## Provision a New Space

Use the mem9 API directly:

```bash
curl -sX POST https://api.mem9.ai/v1alpha1/mem9s | jq .
```

The response returns an `id`. Save it as `SPACE_ID`.

User-facing explanation:

> mem9 created a new cloud memory space for you. Save this space ID so you can reconnect the same memory later from this machine or another one.

## Persisting the Space ID Locally

Only persist local config if the user asks to save it in the project or wants a reusable local setup.

If they do, add these variables to the project `.env`:

```env
MEM9_API_URL=https://api.mem9.ai
MEM9_SPACE_ID=<your-space-id>
MEM9_AGENT_ID=ironclaw
```

Important:

- Be explicit that these variables are for this mem9 workflow
- Do not claim IronClaw automatically swaps its built-in workspace backend to mem9 just because these vars exist

## Supported mem9 Operations

Base URL:

```bash
API="https://api.mem9.ai/v1alpha1/mem9s/$SPACE_ID"
```

Optional header:

```bash
-H "X-Mnemo-Agent-Id: ironclaw"
```

### Store

```bash
curl -sX POST "$API/memories" \
  -H "Content-Type: application/json" \
  -H "X-Mnemo-Agent-Id: ironclaw" \
  -d '{
    "content": "Project uses PostgreSQL with pgvector",
    "tags": ["ironclaw", "workspace"],
    "source": "ironclaw"
  }'
```

### Search

```bash
curl -s "$API/memories?q=postgres&limit=5"
curl -s "$API/memories?tags=ironclaw&source=ironclaw"
```

### Get / Update / Delete

```bash
curl -s "$API/memories/<id>"
curl -sX PUT "$API/memories/<id>" \
  -H "Content-Type: application/json" \
  -d '{"content":"updated"}'
curl -sX DELETE "$API/memories/<id>"
```

## Importing IronClaw Memory into mem9

When the user says "import memories to mem9" or asks for backup/import:

1. Use `memory_tree` to inspect the workspace structure
2. Prioritize:
   - `MEMORY.md`
   - recent files under `daily/`
   - `context/`
   - any user-requested project paths
3. Default to the 15 most recent or most relevant documents if the user does not specify a scope
4. Use `memory_read` to fetch each document
5. Upload each document to `POST /memories`

For IronClaw workspace documents, do not assume mem9's `/imports` endpoint understands the file format. IronClaw stores markdown-like workspace documents, not the OpenClaw JSON files referenced in the original mem9 skill. For IronClaw content, prefer individual `POST /memories` uploads.

Recommended upload shape:

```json
{
  "content": "Path: MEMORY.md\nImported-From: ironclaw\n\n<document body>",
  "tags": ["ironclaw", "workspace", "root"],
  "source": "ironclaw"
}
```

Tagging guidance:

- Always include `ironclaw`
- Include `workspace`
- Add one path hint tag such as `root`, `daily`, `context`, or `projects`

Import summary should include:

- number of documents uploaded
- skipped documents
- failures with the mem9 response
- the space ID used

## Syncing Back Into IronClaw

If the user wants a mem9 result available inside IronClaw's native memory again:

1. Search mem9 with the `http` tool
2. Confirm the result to keep
3. Write the important excerpt into IronClaw using `memory_write`

This keeps IronClaw's local workspace memory and mem9 cloud memory aligned without pretending they are the same backend.

## What to Tell Users Next

Preferred order:

1. Offer to import existing IronClaw workspace memory first
2. Explain that the same space ID reconnects the same cloud memory on a new machine
3. Recommend keeping local workspace files as a backup
4. Only offer a synthetic write/read demo if the user explicitly asks for a test

## Recovery Guidance

Say this plainly:

> Save the mem9 space ID somewhere safe. Reusing the same ID later reconnects the same cloud memory space.

Recovery checklist:

1. Keep the original IronClaw workspace files
2. Save the space ID in a password manager, team vault, or another secure offsite location
3. On a new machine, reconnect with the same space ID before importing or writing new memories

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `404` from mem9 | The space ID is wrong, missing, or pointed at the wrong path |
| Existing space ID is unreachable | Recheck the value, then verify `https://api.mem9.ai/healthz` |
| User expects automatic prompt injection | Explain that IronClaw does not yet have native mem9 memory-plugin support |
| Import scope is too large | Start with `MEMORY.md` and recent `daily/` files, then expand |

## Safety Rules

- Do not overwrite local IronClaw memory files unless the user asked for that
- Do not delete mem9 entries unless the user clearly requested deletion
- Do not describe the space ID as optional or disposable; it is the handle needed to reconnect the same cloud memory later
