# Communication Protocol

## Rules

1. **Check inbox before starting any new task.** Not during a task -- between tasks.
2. Write all inter-agent messages to `.orchestrator/outbox/`. Never write to another domain's directories.
3. Signal task completion by writing `completion-bd-XXX.md` to outbox. Do not call `bd close` directly.
4. Update `status.json` when starting/finishing work or becoming blocked.
5. Only acquire file locks through the comm-node's lock protocol (write lock request to outbox, wait for grant in inbox).
6. Trust the comm-node. Do not attempt to discover or communicate with other agents directly.

## Message Format

All outbox messages must include YAML frontmatter:

```yaml
---
from: <your-domain>
to: <target-domain>
type: artifact_ready | blocked | question | completion | status
task: bd-XXX
priority: high | medium | low
artifacts: []
---

Human-readable message body.
```

## Semantic Shorthand

```
Status:    done=✓  blocked=⚠  in-progress=⏳  failed=❌
Priority:  high=[!]  medium=[~]  low=[-]
Direction: A->B
```
