# Inline effect development

Domain events are dispatched inline by constructing the `Event::Variant` on
the receiver of `.develop_on(develop)`. Do not bind an event to a variable
before dispatching it, and never call `Develop::develop` at a caller.

```rust
Event::UserSignedUp(UserSignedUpEvent {
    team_id,
    invitor_id,
    invitee_qid,
})
.develop_on(develop)
.await;
```

```bash
uv run fmt/effect-develop-inline/check.py
```
