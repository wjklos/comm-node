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
| Task Queue | Task management & dependency graph | Beads integration (read-only consumer) |
| Event Bus | Internal event routing | Tokio channels |
| Lock Manager | Advisory file locks, prevent conflicts | In-memory HashMap with periodic snapshots |
| Artifact Store | Cross-domain work products | Git-backed filesystem |
| Domain Registry | Agent discovery & peer awareness | TOML config + JSON registry |
| Boundary Checker | Enforce architectural boundaries | Path scoping + import analysis |
| Deployment Coordinator | Synchronize final deployment | Readiness checks + deploy scripts |

### Persistence Strategy

- **Artifacts** -> Git repos (durable, versioned, human-readable diffs)
- **Locks** -> Snapshot file every 30s (fast in-memory, auto-expire stale locks on recovery)
- **Events** -> Append-only log (TSV: timestamp + JSON, queryable with grep/jq)
- **Agent state** -> Ephemeral in repo (agents write own `status.json`, rebuilt on restart)
- **Tasks** -> Beads owns this (comm-node is read-only consumer, polls `bd ready --json`)

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
    budget.json               # Token budget (written by comm-node)
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

### Token Budget

Agents have a token budget for inter-agent communication:
- Reading messages: deducted from budget
- Sending messages: deducted from budget
- Actual work (coding): separate budget
- Agents can request more budget with a reason
- Agents that always request maximum get flagged for low efficiency

### Semantic Shorthand (Dialect Guide)

```
Status:  ✓(done) ⚠(blocked) ⏳(in-progress) ❌(failed)
Priority: [!](high) [~](medium) [-](low)
Direction: A->B (from A to B)
Artifacts: artifact-name.ext
Action: ->do-this
```

## Agent Lifecycle

### Phase 0: Setup (comm-node does this)
1. Create `.orchestrator/` directory structure in each domain
2. Write `registry.json` with all domain info (peer discovery)
3. Write `PROTOCOL.md` (communication rules)
4. Generate domain-specific CLAUDE.md from template

### Phase 1: Registration (kickoff meeting)
1. Each agent reads `registry.json` (discovers peers)
2. Each agent writes `status.json` (announces self)
3. Comm-node waits until all expected agents registered
4. Comm-node broadcasts START signal to all agents

### Phase 2: Work
1. Agent polls Beads for ready tasks (`bd ready --json`)
2. Comm-node validates artifact-aware readiness (Beads ready + required artifacts exist)
3. Agent acquires file locks through comm-node
4. Agent does work
5. Agent generates required artifacts
6. Agent notifies dependent agents via outbox
7. Agent updates Beads (`bd close bd-XXX`)
8. Agent asks "what's next?" -> go to step 1

### Phase 3: Integration
- Integration Coordinator agent activates when all domains report complete
- Runs cross-domain integration tests
- Verifies all contracts satisfied
- Checks boundary compliance
- Deploys if all checks pass

## Boundary Enforcement

The comm-node can impose architectural boundaries retroactively on codebases that don't practice DDD.

```toml
[domains.routes]
scope = ["src/routes/**"]
forbidden_imports = ["src/database/**"]
allowed_imports = ["src/services/**", "src/middleware/**"]

[domains.services]
scope = ["src/services/**"]
forbidden_imports = ["src/routes/**"]
allowed_imports = ["src/database/**", "src/models/**"]
```

Enforced at three levels:
1. **Lock acquisition** - agents can't acquire locks outside their domain scope
2. **Import analysis** - static checking of import statements in modified files
3. **Pre-commit hook** - blocks commits that introduce boundary violations

## Key Design Principles

1. **Agents are dumb terminals** - they don't need to understand routing, budgets, or boundary enforcement. They follow simple rules: write to outbox, read from inbox, trust the comm-node.
2. **No peer-to-peer** - all communication routes through the comm-node. Agents don't know each other's locations.
3. **Artifacts are the currency** - not just messages. The actual work products (API contracts, type definitions, specs) are what flow between domains.
4. **Abstract interfaces everywhere** - filesystem now, but every component behind a trait for future swapping:
   - `ArtifactStore` (filesystem -> S3/GCS)
   - `CommunicationTransport` (filesystem -> HTTP/gRPC)
   - `LockManager` (in-memory -> Redis/DynamoDB)
   - `RegistryBackend` (local file -> etcd/Consul)
   - `EventLog` (append-only file -> CloudWatch/DataDog)

## Tech Stack

- **Language:** Rust
- **Async runtime:** Tokio
- **Task management:** Beads (external dependency)
- **Config format:** TOML
- **Serialization:** serde + serde_json + serde_yaml
- **File watching:** notify crate (inotify on Linux, FSEvents on macOS)
- **Web dashboard:** axum + tower-http (SSE + HTML)
- **Tracing:** tracing + tracing-subscriber

### Cargo Dependencies (planned)

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
notify = "6"
glob = "0.3"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
axum = "0.7"
tower-http = { version = "0.5", features = ["fs", "trace"] }
uuid = { version = "1", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
regex = "1"
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

## Cloud Layer Considerations (noted, not designed yet)

- **Security:** agent identity verification, encryption at rest/in transit, secret management, multi-tenancy
- **Latency:** geographic distribution, protocol optimization, caching, connection pooling
- **Redundancy:** comm-node HA (leader election), state replication, message delivery guarantees
- **Observability:** distributed tracing, metrics aggregation, centralized logging
- **Cost:** bandwidth/egress, compute for always-on orchestrator, storage retention policies

## Open Design Questions

- **UX for interaction and monitoring/management** - CLI? TUI? Web dashboard? All three?
- **"Hand of God" intervention mechanisms** - emergency stop, budget override, task reassignment, manual artifact injection
- **Kickoff meeting protocol** - what exactly happens in the agent initialization ceremony?
- **Error recovery & rollback** - what happens when an agent produces wrong artifacts or integration tests fail?
- **Cost tracking** - per-agent API cost tracking, budget limits, cost/benefit analysis of parallelism
- **Non-coding use cases** - creative director, visual design, content strategy workflows (the architecture supports this naturally via sub-agents as expertise lenses)
