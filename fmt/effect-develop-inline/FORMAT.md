# Inline effect development

Concrete domain events must be constructed directly on the receiver of
`.develop_on(develop)`. Do not bind an event before dispatching it, call
`EffectDevelop::develop` at a caller, or wrap a concrete event in `Event`.

```rust
UserSignedUpEvent {
    team_id,
    invitor_id,
    invitee_qid,
}
.develop_on(develop)
.await;
```

```bash
uv run fmt/effect-develop-inline/check.py
```
