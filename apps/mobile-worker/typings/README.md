# Minitap 4.0.0 adapter types

The installed SDK has inline annotations but no `py.typed` marker; some overloads
also contain untyped dictionaries. These stubs describe only the API surface used
by our qualification adapter, inspected from the pinned installed wheel. They do
not implement an agent or duplicate our Rust-owned transport contracts.

Use direct imports inside `execute` after environment isolation. Keep Pyright
strict; do not replace these declarations with Any, unchecked agent casts, or
suppressed diagnostics. SDK task output is deliberately opaque `object` because
our independent verifier owns the verdict. No type stub changes runtime behavior.

When updating Minitap, inspect the upstream methods and model fields, update these
stubs, and run the SDK adapter tests. The compatibility test checks these signatures
and fields against the installed package without constructing an agent or making
network calls. Stubs intentionally omit unused SDK APIs and overloads.
