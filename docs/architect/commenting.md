# Comments for humans and AI contributors

Comments explain purpose, important rules and surprising choices close to the code.
They must describe the current implementation, especially when a name could make a
scaffold look production-ready. Keep them concise enough to read while changing code.

## What to explain

- At a non-obvious module boundary: what it owns, who calls it, and whether it is an
  implemented feature, a fake, generated output, or reserved scaffolding.
- Near a tricky branch: the rule being protected, what would break if removed, and
  any important caller obligation (for example, explicit Rust semantic validation).
- At an external boundary: inputs/outputs, validation, side effects, process/resource
  ownership and what success does or does not establish.
- For unfinished work: link the owning spec in this directory. Do not present planned
  leases, authentication, phone execution or persisted reports as implemented behavior.

Example from the fixture contract:

```rust
// Required key: {"nullable_note":null} is valid; omitting nullable_note is not.
```

Example from the fake executor:

```python
# Exit status reports CLI success, not whether the simulated test passed.
# Consumers read outcome from JSON; setup/invalid input exits 2.
```

## What to avoid

Do not narrate obvious assignments, repeat every type name, add decorative banners,
store change history in code, or attach vague TODOs without a spec. Comments are not
an alternative to clear names, small functions, validation or tests. Add a comment only
where it helps a reader make a correct change; no minimum comment count is required.

Use Rust module docs/doc comments, Python docstrings and TypeScript JSDoc when callers
benefit from documentation. Ordinary inline comments suit local constraints. On types
read by Schemars/Utoipa, Rust doc comments can change generated schema descriptions;
use `just types` when changing those exported descriptions. Implementation-only notes
can remain ordinary comments without changing the wire document.

Never hand-edit generated SDK/Pydantic files, schema output, lockfiles or license text
to improve comments. Update the source/generator when generated documentation needs a
change. Preserve Loco scaffold markers, license attribution, lint directives and
`@ts-expect-error` assertions exactly unless their actual purpose changes.

Review comments with their code in the same PR. After the complete edit batch, use
checks appropriate to the change: token/AST/config comparisons, links and formatting
can establish a comment-only change without repeating unrelated app suites. A changed
Python module docstring may also improve CLI help where argparse reads **doc**; it
must not quietly change command behavior. If executable code or generated schema changes,
validate the affected implementation and consumers as usual.
