# Clide Skills Roadmap

Suggested skills for future development. Grouped by priority and domain.
Last updated: 2026-03-22

---

## Shipped (18 skills)

| Skill | Category | File | Description |
|---|---|---|---|
| `termux_hardening` | Security | `Security/termux_hardening.yaml` | Termux security setup, SSH hardening, environment lockdown |
| `vps_hardening` | Security | `Security/vps_hardening.yaml` | Full VPS hardening: UFW, fail2ban, sshd config, auditd |
| `vps_manager` | Security | `Security/vps_manager.yaml` | VPS status dashboard — disk, mem, load, services |
| `docker_ai_sandbox` | Security | `Security/docker_ai_sandbox.yaml` | Install Docker CE + hardened AI sandbox with Ollama, seccomp, isolated networks |
| `lynis_audit` | Security | `Security/lynis_audit.yaml` | Lynis security audit via SSH (requires explicit SSH params) |
| `port_scanner` | Security | `Security/port_scanner.yaml` | Scan ports on own infrastructure |
| `intrusion_alert` | Security | `Security/intrusion_alert.yaml` | Intrusion detection and alerting |
| `vault_auto_backup` | Security | `Security/vault_auto_backup.yaml` | Automated vault backup scheduling |
| `backup_skill` | System | `System/backup_skill.yaml` | Rclone cloud backup with rotation (S3/Backblaze/Dropbox) |
| `debugger` | System | `System/debugger.yaml` | Multi-phase Docker/service fault report — logs, processes, resources, network, AI-ready output |
| `monitoring_skill` | System | `System/monitoring_skill.yaml` | Disk, memory, CPU, process health check with configurable alert threshold |
| `system_overview` | System | `System/system_overview.yaml` | Comprehensive system status report |
| `maintenance_cron` | System | `System/maintenance_cron.yaml` | Scheduled system upkeep via cron |
| `clide_install` | System | `System/clide_install.yaml` | Clide installation automation |
| `vault_skill` | System | `System/vault_skill.yaml` | Vault backup and restore operations |
| `aiwb_manager` | AI | `AI/aiwb_manager.yaml` | Drive AIworkbench make/tweak/debug pipeline headlessly — Generator-Verifier loop, multi-provider, context files |
| `telegram_manager` | Telegram | `Telegram/telegram_manager.yaml` | Telegram bot management operations |
| `example_skill` | — | `example_skill.yaml` | Template for creating new skills |

---

## Priority Queue

### Infrastructure

| Skill | Description | Notes |
|---|---|---|
| `nginx_reverse_proxy` | Install Nginx, SSL termination, route requests to Docker containers | Pair with `ssl_cert` |
| `ssl_cert` | Certbot Let's Encrypt — install, issue cert, auto-renewal via cron | Works on bare VPS or via Docker |
| `database_manager` | PostgreSQL/MySQL health check, backup, restore, vacuum | Parametrize by engine |

### AI & Bots

> `aiwb_manager` shipped — drives AIworkbench headlessly via `aiwb headless`. Needs AIWB installed + API keys.

| Skill | Description | Notes |
|---|---|---|
| `ollama_manager` | Pull/remove models, list loaded, run inference smoke-test, show VRAM usage | Builds on docker_ai_sandbox |
| `bot_deployer` | Build image from Dockerfile, push to registry, deploy/restart named container | Tag with git SHA |
| `api_health_check` | Test REST endpoints, webhooks, bot tokens (Gemini, Telegram, Matrix) | Configurable endpoint list |
| `aiwb_pipeline` | Multi-step AIWB chain — run make→tweak→debug in sequence with context threading | Builds on aiwb_manager |

### Ops & Monitoring

| Skill | Description | Notes |
|---|---|---|
| `log_analyzer` | Parse syslog/auth.log/Docker logs for errors, failed logins, anomalies | Output summary + optionally alert via Telegram |
| `system_snapshot` | Full health report saved to timestamped file: disk, mem, processes, scores | Good for cron + diff-over-time |
| `cron_manager` | List, add, remove cron jobs. Add Termux:Boot auto-start entries | Safety: show diff before writing |
| `git_manager` | Clone, pull, push, stash, branch — for projects living on the VPS | Wrap common git flows |

### Termux-Specific

| Skill | Description | Notes |
|---|---|---|
| `termux_update` | Full update cycle + security audit + backup in one shot | Combines pkg upgrade + hardening check |
| `ssh_tunnel_manager` | Open/list/kill SSH tunnels, port forwarding to VPS services | Store active tunnels in a state file |
| `network_scout` | Scan own infrastructure with nmap, check open ports, Tailscale peer status | Scope: own IPs only |

---

## Debugger Skill — Concept Design

> **Idea:** a first-class `debugger` skill that attaches to a running container or host service,
> captures diagnostic state, and returns a structured fault report — ready to hand to the AI
> agent for root-cause analysis.

### What makes it different from a monitoring skill

`system_monitoring` is **proactive and routine** — it tells you the current load.
`debugger` is **reactive** — triggered when something is already broken, and its output
is designed to be fed back into an AI reasoning loop (Gemini/Ollama).

### Parameters

```yaml
parameters:
  target:
    description: Container name, service name, or "host"
    type: string
    required: true

  mode:
    description: "logs | network | resources | process | full"
    type: string
    required: false
    default: "full"

  tail_lines:
    description: How many log tail lines to capture
    type: number
    required: false
    default: "200"

  since:
    description: Capture logs since this duration (e.g. 10m, 1h, 24h)
    type: string
    required: false
    default: "1h"
```

### Diagnostic phases

| Phase | Mode | What it captures |
|---|---|---|
| **Logs** | `logs`, `full` | `docker logs --since --tail`, journald for host services, deduplicated error lines |
| **Process tree** | `process`, `full` | `ps aux` inside container, zombie count, thread count |
| **Resources** | `resources`, `full` | CPU%, MEM%, swap, OOM events from dmesg, cgroup limits vs usage |
| **Network** | `network`, `full` | Open connections, DNS resolution test, reachability to Ollama/API endpoints |
| **Exit codes** | `full` | Restart count, last exit code (`docker inspect .State.ExitCode`), uptime |
| **Config diff** | `full` | Compare running container config with last known-good snapshot in `/opt/clide/<project>/` |

### Output format

The skill writes a structured report to `/tmp/clide-debug-<target>-<timestamp>.txt`
and prints a concise summary. The file is formatted so it can be passed directly as
context to the AI agent:

```
=== CLIDE DEBUG REPORT ===
Target   : my-bot-container
Mode     : full
Captured : 2026-02-20T14:32:01Z

[SUMMARY]
  Exit code    : 137 (OOM kill)
  Restarts     : 8 in last 1h
  Last error   : RuntimeError: CUDA out of memory

[LOGS — last 50 error lines]
  ...

[RESOURCES]
  MEM: 511m / 512m (99.8% — AT LIMIT)
  CPU: 0.48 / 0.50
  OOM events (dmesg): 3

[NETWORK]
  ollama:11434 → reachable (200ms)
  api.telegram.org → reachable (88ms)

[RECOMMENDATION HINTS]
  - Container hit memory limit repeatedly → raise bot_mem_limit or reduce model size
  - 8 restarts → check restart policy, consider circuit breaker
```

### Integration idea

The `debugger` skill output can be piped into a follow-up workflow that sends the report
to Gemini or Ollama with a prompt like:
> "Here is a debug report for a failing Docker container. Identify the root cause and
> suggest the minimal fix."

This makes `debugger` the **diagnostic layer** of a self-healing loop.

### Suggested file location

```
skills/System/debugger.yaml
```

---

## Naming conventions for new skills

- `snake_case`, verb-object style preferred (`backup_manager`, `log_analyzer`)
- Category folders: `Security/`, `System/`, `AI/`, `Telegram/`, `Network/`
- One skill = one YAML file, self-contained
- Always include `rollback_command` for any skill that mutates system state
- `require_confirmation: true` for anything that installs, deletes, or restarts services

---

## How to contribute a skill

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full process.
Short version:
1. Copy `skills/example_skill.yaml` as your starting point
2. Add it to the appropriate category folder
3. Test locally with `clide skill run <your_skill>`
4. Open a PR with the skill file and a brief description of what it solves
