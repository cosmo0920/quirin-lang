# Readable logger hypotheses

This readable layer describes the proposed `model.json`; it is not an inline
production `.qv` declaration and is not fed silently to an older loader.
`|^+` and `|^-` below describe custody transitions, not proof of C allocation
or release. All fifteen named obligations in the model remain **Open**.

```
(configuration_lifetime ∧ formatting_extent ∧ tls_thread_name) |-> log0
(configuration_lifetime ∧ allocator_fail_stop ∧ copy_extent) |-> queued_log
invalid_configuration_or_dead_callback !-> log0
log0 ||-> filtered / no_destination / immediate_callback / queued_log
queued_log |^+ operation
queued_log |^+ log_text
queued_log ||-> local_queue / forwarded_queue / disabled_no_reply / disabled_reply
local_queue ->| operation_and_text_queued
forwarded_queue ||-> forwarded_retention / forwarded_destruction
forwarded_retention ->| operation_and_text_queued
forwarded_destruction ->| operation_and_text_destroyed
disabled_no_reply |^- log_text
disabled_no_reply |^- operation
disabled_no_reply ->| operation_and_text_destroyed
disabled_reply ||-> reply_retention / reply_destruction
reply_retention ->| operation_and_text_queued
reply_destruction ->| operation_and_text_destroyed
immediate_callback ->| borrowed_stack_text_not_retained
```

All queue paths independently require `queue_synchronization`,
`forwarding_termination`, `io_event_effects`, `serve_callback_effects`,
`reply_queue_effects`, `generic_cleanup_effects` and eventual consumer cleanup.
A callback also requires its own effect, code/context lifetime and return
obligations. It can reenter logging. The model keeps that cycle and the queue
forward/reply cycle; finite effect reachability does not prove either terminates.

Creation tracks the fresh operation separately from the duplicated string.
A transfer moves both to queue custody. A future consumer must take custody
before releasing the string and then the operation. Transfer supplies no
immediate destruction or eventual-cleanup proof. The caller cannot free queued
objects. A forwarded enqueue's return value is not a reliable retained/destroyed
classifier: its source discards the recursive result before returning one.

`source_model_correspondence` remains an independent universal premise. The
model's finite path inventory is reviewed data, not extracted complete C paths;
its exact expected digest prevents silently deleting an alternative. Formatting,
TLS, callbacks, generic queue cleanup, allocator failure, concurrency and future
consumption need further source/build-specific semantic consumers before any
production source-Safe claim could use this decomposition.
