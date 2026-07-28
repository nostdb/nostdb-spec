# Viewer exchange fixtures

Every `.bin` here was written by `generate.mjs`, which is the readable form of what the bytes are.
A fixture nobody can see the construction of is one people copy without understanding, and a
hand-edited byte array is one nobody can extend.

```bash
node fixtures/view-exchange/generate.mjs
```

The generator is idempotent: running it reproduces every fixture byte for byte, which is also the
check that the format has no unstated dependency on when or where it was written. `scripts/verify-repository.sh`
runs it and fails if any fixture changes.

Each `.bin` pairs with an `.expected` file of `key = value` lines. An accepted container declares its
counts, so a reader that parsed the header and got different ones fails rather than passing on the
strength of not having crashed.

The generator is not normative. [`../../docs/VIEW_EXCHANGE.md`](../../docs/VIEW_EXCHANGE.md) is, and
the fixtures are the gate.
