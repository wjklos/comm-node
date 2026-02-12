# Elder Comm-Node

FTL coordination for parallel AI agents. Named after the Elder communication nodes in Craig Alanson's Expeditionary Force series - ancient technology that enables faster-than-light communication by operating in a higher dimension of spacetime.

Agents work in normal space (local repos). The comm-node operates in the coordination layer (higher dimensional space), routing messages, enforcing boundaries, and managing dependencies.

## Project Vision

Rust-based orchestration middleware that coordinates N parallel Claude Code instances working across different domains/repos. Not a replacement for Claude Code or Beads - a coordination plane that sits between them.

**You (the conductor) -> comm-node (the orchestrator) -> N Claude Code instances (the workers)**

## Architecture

### Core Components

| Component | Purpose | Implementation |
|---|---|---|
| Task Queue | Task management & dependency graph | Beads integration (read-mostly; comm-node mediates state transitions) |
| Event Bus | Internal event routing | Tokio channels |
| Lock Manager | Advisory file locks, prevent conflicts | In-memory HashMap with periodic snapshots |
| Artifact Store | Cross-domain work products | Git-backed filesystem |
| Domain Registry | Agent discovery & peer awareness | TOML config + JSON registry |
| Boundary Checker | Enforce architectural boundaries | Path scoping (MVP), import analysis (Phase 2) |

### Persistence Strategy

- **Artifacts** -> Git repos (durable, versioned, human-readable diffs)
- **Locks** -> Snapshot file every 30s (fast in-memory, auto-expire stale locks on recovery)
- **Events** -> Append-only log (TSV: timestamp + JSON, queryable with grep/jq)
- **Agent state** -> Ephemeral in repo (agents write own `status.json`, rebuilt on restart)
- **Tasks** -> Beads owns this (comm-node mediates task state transitions; agents signal completion via outbox, comm-node calls `bd close`)

No database. Filesystem is the persistence layer. Add SQLite only if query limitations emerge.

### Directory Layout

```
~/.comm-node/
  state/
    locks.snapshot.json       # Updated every 30s
    event.log                 # Append-only
  config.toml                 # Orchestrator config

/project/backend/
  .orchestrator/
    artifacts/                # Git-tracked, durable
    inbox/                    # Ephemeral, cleared on read
    outbox/                   # Ephemeral, cleared on send
    status.json               # Ephemeral, agent-owned
    registry.json             # Peer discovery (written by comm-node)
    PROTOCOL.md               # Communication rules (written by comm-node)

/project/frontend/
  .orchestrator/
    [same structure]
```

## Communication Protocol

### LLM-to-LLM: Three-Tier System

All agent communication routes through the comm-node (hub-and-spoke, no peer-to-peer).

**Tier 1: Status Signals** (~10 tokens)
```
✓bd-123 backend->frontend
```
For: completions, acks, heartbeats.

**Tier 2: Action Messages** (~60-80 tokens) - DEFAULT
```yaml
---
from: backend
to: frontend
task: bd-123
type: artifact_ready
priority: high
artifacts: [api-contract.yaml]
---

Auth API done: register/login/me endpoints. Spec in artifacts/. Integrate JWT next.
```
For: 80% of coordination (work complete, blockers, questions).

**Tier 3: Context Documents** (~200-500 tokens)
Full YAML frontmatter + markdown with sections, examples, rationale.
For: complex discussions, design decisions, debugging help.

### Communication Observability

The comm-node tracks inter-agent communication volume as observability metrics, not enforcement mechanisms. Agents cannot be prevented from reading files on disk, so "budgets" are informational.

- **Message volume**: count and total size of messages sent/received per agent per session
- **Tier distribution**: percentage of Tier 1/2/3 messages (agents overusing Tier 3 are flagged)
- **Surfaced to operator**: `comm-node status` shows per-agent communication metrics
- **No enforcement**: agents are not blocked from sending or reading messages

### Semantic Shorthand (Dialect Guide)

```
Status:  ✓(done) ⚠(blocked) ⏳(in-progress) ❌(failed)
Priority: [!](high) [~](medium) [-](low)
Direction: A->B (from A to B)
Artifacts: artifact-name.ext
Action: ->do-this
```

## Agent Lifecycle

### Phase -1: Spawning

The comm-node does not currently launch Claude Code instances. Spawning is manual.

**MVP (manual):**
1. Human opens a terminal per domain
2. Human runs `claude` in each domain's working directory
3. Each agent's working directory already contains `.orchestrator/` (from Phase 0) and a domain-specific CLAUDE.md that includes the orchestration protocol

**Future (`comm-node spawn`):**
```bash
comm-node spawn --domain backend --config project.toml
```
This would launch Claude Code instances, point them at the correct working directory, and inject the domain-specific CLAUDE.md. Requires Claude Code to support programmatic invocation.

### Phase 0: Setup (comm-node does this)
1. Create `.orchestrator/` directory structure in each domain
2. Write `registry.json` with all domain info (peer discovery)
3. Write `PROTOCOL.md` (communication rules)
4. Generate domain-specific CLAUDE.md from template (includes inbox-checking rules)

### Phase 1: Registration (kickoff meeting)
1. Each agent reads `registry.json` (discovers peers)
2. Each agent writes `status.json` (announces self)
3. Comm-node waits until all expected agents registered
4. Comm-node broadcasts START signal to all agents

### Phase 2: Work
1. Agent checks inbox for new messages (hard rule: **before starting any new task**)
2. Agent picks next task (reads task assignment from inbox or uses own judgment)
3. Comm-node validates artifact-aware readiness (required artifacts exist)
4. Agent acquires file locks through comm-node
5. Agent does work
6. Agent generates required artifacts
7. Agent writes completion signal to outbox (`completion-bd-XXX.md`)
8. Comm-node routes completion to dependents and calls `bd close`
9. Agent asks "what's next?" -> go to step 1

**Agent polling latency:** Messages are files routed through the comm-node. Agents only check their inbox between tasks, not during active work. Expect 1-5 minute message hop latency depending on task duration. This is a design reality of the filesystem transport, not a bug. Real-time coordination is not a goal for MVP.

### Phase 3: Integration
- Integration Coordinator agent activates when all domains report complete
- Runs cross-domain integration tests
- Verifies all contracts satisfied
- Checks boundary compliance

## Boundary Enforcement

The comm-node can impose architectural boundaries retroactively on codebases that don't practice DDD.

```toml
[domains.routes]
scope = ["src/routes/**"]

[domains.services]
scope = ["src/services/**"]
```

### Phased Implementation

**MVP (Phase 1): Path scoping only**
- Agents can only acquire locks on files within their domain's `scope` glob patterns
- Simple, reliable, no language-specific logic required
- Provides ~80% of the value of full boundary enforcement

**Phase 2: Import analysis**
- Static checking of import/require/use statements in modified files
- Language-specific parsers (start with TypeScript/JavaScript, add others as needed)
- Config extends to include `forbidden_imports` and `allowed_imports` per domain

**Phase 3: Pre-commit hooks**
- Git pre-commit hooks that block commits introducing boundary violations
- Combines path scoping + import analysis
- Installed by `comm-node init`

## Key Design Principles

1. **Agents are protocol followers** - they follow a simple contract: read inbox, do work, write to outbox, update status.json. They don't need to understand routing, budgets, or boundary enforcement. The comm-node handles everything else.
2. **No peer-to-peer** - all communication routes through the comm-node. Agents don't know each other's locations.
3. **Artifacts are the currency** - not just messages. The actual work products (API contracts, type definitions, specs) are what flow between domains.
4. **Extract traits when needed** - build concrete implementations first, extract traits when a second implementation is needed. MVP trait boundaries:
   - `ArtifactStore` -- yes (filesystem now, cloud later is a known requirement)
   - `EventLog` -- yes (simple interface, clear future needs)
   - `LockManager`, `CommunicationTransport`, `RegistryBackend` -- build concrete, extract later

## Tech Stack

- **Language:** Rust
- **Async runtime:** Tokio
- **Task management:** Beads (external dependency; comm-node calls `bd close` on completion signals, full graph integration in Phase 2)
- **Config format:** TOML
- **Serialization:** serde + serde_json + serde_yaml
- **File watching:** notify crate (inotify on Linux, FSEvents on macOS)
- **CLI:** clap
- **Tracing:** tracing + tracing-subscriber

### Cargo Dependencies (MVP)

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
notify = "6"
glob = "0.3"
anyhow = "1"
clap = { version = "4", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
toml = "0.8"
```

## Evolution Roadmap

| Phase | Scope | Transport | Storage |
|---|---|---|---|
| 1 - MVP | Local, single machine | Filesystem + inotify | Local filesystem |
| 2 - Hybrid | Local orchestrator, cloud storage | Filesystem + optional HTTP | Local + S3/GCS |
| 3 - Distributed | Agents across machines | HTTP/gRPC | Centralized store |
| 4 - Federated | Multiple comm-nodes | Network protocol | Distributed |
| 5 - SaaS | Multi-tenant managed service | Full network stack | Cloud-native |

## Non-Goals for MVP

These are explicitly deferred. Do not implement or promise these in Phase 1:

- **Cloud distribution** -- agents and comm-node run on a single machine
- **Multi-language import analysis** -- boundary enforcement is path-scoping only
- **Artifact adapters** (e.g., OpenAPI -> TS types) -- agents generate the right format directly
- **Web dashboard** -- `comm-node status` CLI is sufficient
- **Deployment coordinator** -- a shell script, not a comm-node component
- **Token budget enforcement** -- observability only, see Communication Observability
- **Federated comm-nodes** -- single orchestrator only
- **MCP tool server** -- Phase 2; MVP agents use filesystem protocol
- **Deep Beads integration** -- comm-node does not poll `bd ready --json` or read the Beads dependency graph for MVP. It does call `bd close` on agent completion signals (simple shell command). Full task graph integration in Phase 2
- **Agent spawning automation** -- human opens terminals manually for MVP

## Agent Interface Contract

Definitive reference for what agents read and write. Domain-specific CLAUDE.md files are generated from this contract.

### What Agents Read

| File | Written By | Content |
|---|---|---|
| `.orchestrator/inbox/*.md` | comm-node (routed from other agents) | Messages in Tier 1/2/3 format |
| `.orchestrator/registry.json` | comm-node | Peer domain names and descriptions (no locations) |
| `.orchestrator/PROTOCOL.md` | comm-node | Communication rules and format reference |
| `.orchestrator/artifacts/*.{yaml,json,md}` | comm-node (copied from source agent) | Cross-domain work products |

### What Agents Write

| File | Read By | Content |
|---|---|---|
| `.orchestrator/outbox/*.md` | comm-node | Messages to other agents (YAML frontmatter required) |
| `.orchestrator/outbox/completion-bd-XXX.md` | comm-node | Task completion signal (comm-node calls `bd close`) |
| `.orchestrator/status.json` | comm-node | Agent status (see schema below) |
| `.orchestrator/artifacts/*` | comm-node (for routing to dependents) | Work products for other domains |

### status.json Schema

```json
{
  "domain": "backend",
  "status": "working",
  "current_task": "bd-123",
  "last_heartbeat": "2025-01-15T10:30:00Z",
  "artifacts_produced": ["api-contract.yaml"],
  "blocked_on": null
}
```

Valid `status` values: `idle`, `working`, `blocked`, `complete`.

### Agent Hard Rules (included in every domain CLAUDE.md)

1. **Check inbox before starting any new task.** Not during a task -- between tasks.
2. Write all inter-agent messages to `.orchestrator/outbox/`. Never write to another domain's directories.
3. Signal task completion by writing `completion-bd-XXX.md` to outbox. Do not call `bd close` directly.
4. Update `status.json` when starting/finishing work or becoming blocked.
5. Only acquire file locks through the comm-node's lock protocol (write lock request to outbox, wait for grant in inbox).
6. Trust the comm-node. Do not attempt to discover or communicate with other agents directly.

### Message Format (Outbox Files)

All outbox messages must include YAML frontmatter:

```yaml
---
from: backend
to: frontend
type: artifact_ready | blocked | question | completion | status
task: bd-123
priority: high | medium | low
artifacts: []  # optional, list of artifact filenames
---

Human-readable message body. Keep concise.
```

The `from` field must match the agent's domain. The comm-node validates and routes by `to` field. Messages with invalid frontmatter are rejected and logged.

## Cloud Layer Considerations (noted, not designed yet)

- **Security:** agent identity verification, encryption at rest/in transit, secret management, multi-tenancy
- **Latency:** geographic distribution, protocol optimization, caching, connection pooling
- **Redundancy:** comm-node HA (leader election), state replication, message delivery guarantees
- **Observability:** distributed tracing, metrics aggregation, centralized logging
- **Cost:** bandwidth/egress, compute for always-on orchestrator, storage retention policies

## Open Design Questions

- **"Hand of God" intervention mechanisms** -- emergency stop, task reassignment, manual artifact injection
- **Error recovery & rollback** -- what happens when an agent produces wrong artifacts or integration tests fail?
- **Cost tracking** -- per-agent API cost tracking, cost/benefit analysis of parallelism
- **Non-coding use cases** -- creative director, visual design, content strategy workflows (the architecture supports this naturally via sub-agents as expertise lenses)
- **MCP tools for Phase 2** -- `check_inbox()`, `send_message()`, `acquire_lock()` as MCP tools would be more reliable than filesystem polling; design when Claude Code MCP integration stabilizes
