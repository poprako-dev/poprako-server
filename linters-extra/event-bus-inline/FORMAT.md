# Inline event bus emission

Domain events are emitted inline by constructing the `Event::Variant` on
the receiver of `.emit_on(bus)`. Do not bind an event to a variable
before emitting it, and never call `EventBus::emit` at a caller.

```rust
Event::UserSignedUp(UserSignedUpEvent {
    team_id,
    invitor_id,
    invitee_qid,
})
.emit_on(bus)
.await;
```

```bash
uv run linters-extra/event-bus-inline/check.py
uv run linters-extra/event-bus-inline/check.py --self-test
```
