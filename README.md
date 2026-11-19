# Messenger Nexus tests

This repository owns the Messenger research prototype's tests, separately
from its implementation. `tests/operations.rs` specifies pure operation
behavior using recording Emission and Memory implementations.

The test source has been parsed/formatted but not compiled or run. A
manifest and pinned dependency follow publication of the core checkpoint;
no local path dependency or unpublished git pin is used as gate evidence.

The live integration acceptance contract is:

1. An isolated homeserver, two throwaway users and a private encrypted room.
2. Open posts a root and remembers its explicit prototype opener and subject.
3. Say posts an encrypted reply with an `m.thread` relation to that root.
4. A decrypted reply from the configured living-fixture user reaches the
   owner as Living, preserving the subject and the complete text.
5. A decrypted room-level message reaches the configured Communicator.
6. Other-room, other-sender, echo and duplicate event identities are ignored
   by the translator before Heard is formed.
7. After restart, the same device/SQLite crypto store can decrypt and reply;
   persisted thread records continue to select the same owner.
8. Transport loss, unknown roots, store failure and unsuccessful Flow CLI
   outcomes remain distinct typed failures.
9. Account credentials and crypto material remain in private scratch files.
10. All owned processes exit at fixture cleanup; no living account or room
    receives any message.

Fixture setup alone is not evidence of SDK encryption, thread delivery,
process restart correctness, or any end-to-end acceptance condition.
