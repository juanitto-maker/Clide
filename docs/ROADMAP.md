# Clide Roadmap

> Living document tracking planned features, priorities, and future direction.

---

## Completed (v0.4.x)

- [x] Core bot functionality (Matrix + Telegram)
- [x] Gemini AI integration with function-calling (2.5 Flash + Pro fallback)
- [x] YAML skills system (18+ shipped skills)
- [x] Credential manager (`clide secret` CLI)
- [x] SSH host registry (`clide host` CLI)
- [x] GNU pass / GPG encryption layer
- [x] Age-encrypted vault backup & restore
- [x] Secret scrubber (auto-redact in AI prompts and chat)
- [x] Linux VPS support with systemd service
- [x] `/stop` command to abort running tasks
- [x] Vision / image interpretation via Telegram
- [x] Tiered memory system (cold/warm/hot)
- [x] Automatic fact extraction from conversations
- [x] LLM fallback (2.5 Flash → 2.5 Pro after failures)
- [x] `/stats` command for usage statistics
- [x] Self-update mechanism (`clide update`)

---

## In Progress

### Scheduled Tasks / Cron Integration
**Priority:** High | **Status:** In development

Run skills, commands, or AI tasks on a schedule directly from config.yaml.

```yaml
scheduled_tasks:
  - name: "daily-backup"
    schedule: "0 2 * * *"
    task: "run backup of all databases"
    enabled: true
  - name: "health-check"
    schedule: "*/30 * * * *"
    skill: "system_overview"
    enabled: true
```

- Built-in cron expression parser (no external dependency)
- Three task types: natural language (AI), skill, raw command
- Background execution alongside bot polling
- Chat notifications on completion
- Execution history in database
- `/schedule` command to view status

### Streaming Output
**Priority:** High | **Status:** In development

Live command output in chat during long-running operations instead of waiting for completion.

- Line-by-line streaming from executor to chat
- Batched updates (~500ms) to avoid API flooding
- Works on Telegram (message edits) and Matrix
- Especially valuable for: package updates, builds, large file transfers, backups

---

## Planned

### Web UI Dashboard
**Priority:** Medium | **Target:** v0.6.x

A lightweight embedded web interface for monitoring and light interaction. Served directly by the Clide binary — no external build tools or Node.js required.

**Planned pages:**
- **Overview** — Bot status, uptime, current task, system stats (CPU/RAM/disk)
- **Task history** — Log of all executed commands and skills with output, timestamps, success/fail
- **Skills browser** — View installed skills, trigger execution, inspect parameters
- **Memory inspector** — Browse stored facts, conversation summaries, session variables
- **Hosts** — View registered SSH hosts (secrets hidden)
- **Scheduled tasks** — View schedules, last run status, trigger manual runs
- **Live logs** — Streaming log viewer
- **Stats** — Token usage, commands run, uptime charts

**Technical approach:**
- Embedded HTTP server (axum or warp) compiled into the binary
- Static SPA (HTML/JS/CSS) bundled as assets
- Read-mostly with a few action buttons (trigger skill, stop task, clear memory)
- Served on a configurable port (default `:8080`)
- Chat interaction stays in Telegram/Matrix — dashboard is for visibility

### Native SSH Library (Compile-Time Feature)
**Priority:** Low | **Target:** v0.7.x

Replace shell-based SSH (`ssh -i ...` via executor) with a native Rust SSH library for non-Android builds.

**Benefits:**
- Connection pooling (open once, run many commands)
- Programmatic error handling and key negotiation
- Better timeout control and port forwarding
- No dependency on system `ssh` binary

**Approach:**
- Compile-time feature flag: `--features native-ssh`
- Default remains shell-based SSH (Android/Termux compatible)
- Library candidate: `russh` (previously failed on Android ARM64)
- Only enabled for x86_64 Linux and macOS builds

### Database Hardening
**Priority:** Medium | **Target:** v0.6.x

Secure the SQLite database at rest and enforce strict access controls. Currently the `~/.clide/` database is an unencrypted file with no authentication — security relies solely on Unix file permissions.

**Planned measures:**
- **Encryption at rest** — Integrate SQLCipher (or libsql-encryption) so the `.db` file is AES-256 encrypted; key derived from a user-supplied passphrase or device secret
- **Strict file permissions** — Automatically set `chmod 700` on `~/.clide/` and `chmod 600` on database files during init / migration
- **Optional container isolation** — Provide a Docker / Podman Compose file so Clide and its database run in an isolated filesystem namespace on shared / VPS hosts
- **WAL integrity checks** — Periodic `PRAGMA integrity_check` via a scheduled task to detect corruption early
- **Backup encryption** — Ensure `clide backup` produces encrypted archives (age / GPG) that never write plaintext database snapshots to disk

### Multi-User RBAC
**Priority:** Low | **Target:** v0.7.x

Role-based access control for team deployments.

**Planned roles:**
| Role | Capabilities |
|------|-------------|
| **Admin** | Full access: all commands, config changes, user management |
| **Operator** | Run commands and skills, view stats, manage hosts |
| **Viewer** | View-only: stats, task history, system status |

**Implementation:**
- Per-user role assignment in config.yaml
- Role check before command execution in bot loops
- Audit log enriched with role information
- Current single-user behavior preserved as default (all authorized users = admin)

### Skill Marketplace
**Priority:** Low | **Target:** v0.8.x

Community-contributed skills with discovery, versioning, and trust levels.

**Planned features:**
- `clide skill search <query>` — search community skills
- `clide skill install <name>` — download and install a skill
- `clide skill update` — update installed community skills
- Skill metadata: author, version, trust level, download count
- Trust tiers: official (shipped with Clide), verified (reviewed), community (unreviewed)
- Skills hosted as a GitHub repository or registry
- Automatic compatibility checks (Clide version, platform)

**Safety:**
- Community skills sandboxed by default (confirmation required before execution)
- Skill review process for verified tier
- Dependency declarations (required tools, packages)

---

## Ideas / Under Consideration

These are not committed to but worth exploring:

- **Docker support** — Run Clide in a container for isolated deployments
- **Multi-room support** — Listen and respond in multiple Matrix rooms simultaneously
- **Plugin system** — Beyond YAML skills, allow Rust/WASM plugins for deeper integration
- **Notification channels** — Push alerts to email, Discord webhooks, or Pushover
- **Task queuing** — Queue multiple tasks instead of rejecting while one is running
- **Conversation branching** — Allow the AI to ask clarifying questions mid-task
- **Metrics export** — Prometheus-compatible metrics endpoint for monitoring stacks
- **Local LLM support** — Ollama integration for fully offline operation

---

## Contributing

Want to help? Check [docs/CONTRIBUTING.md](CONTRIBUTING.md) for guidelines. The best way to contribute is:

1. Pick an item from the **Planned** section
2. Open a GitHub Discussion to discuss the approach
3. Submit a PR with tests

Feature requests and ideas welcome in [GitHub Discussions](https://github.com/juanitto-maker/Clide/discussions).
