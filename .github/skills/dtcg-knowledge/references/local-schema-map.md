# Local Schema Map

This repository includes DTCG 2025.10-aligned JSON schemas under:
- `resources/dtcg_shemas/`

Top-level schemas:
- `resources/dtcg_shemas/format.json`
- `resources/dtcg_shemas/resolver.json`

Format module schema files:
- `resources/dtcg_shemas/format/group.json`
- `resources/dtcg_shemas/format/groupOrToken.json`
- `resources/dtcg_shemas/format/token.json`
- `resources/dtcg_shemas/format/tokenType.json`
- `resources/dtcg_shemas/format/values/*.json`

Resolver module schema files:
- `resources/dtcg_shemas/resolver/set.json`
- `resources/dtcg_shemas/resolver/modifier.json`
- `resources/dtcg_shemas/resolver/resolutionOrder.json`

## High-Signal Constraints From Local Schemas

1. Naming restrictions for token/group names
- Must not start with `$`.
- Must not contain `{`, `}`, or `.`.

2. Token object rules
- A token cannot define both `$value` and `$ref`.
- If `$value` is absent, `$ref` is required.

3. Supported token types in `tokenType.json`
- color
- dimension
- fontFamily
- fontWeight
- duration
- cubicBezier
- number
- strokeStyle
- border
- transition
- shadow
- gradient
- typography

4. Group features
- Supports `$type`, `$description`, `$extensions`, `$extends`, `$deprecated`, `$root`.

5. Resolver root requirements
- `version` is required and fixed to `2025.10`.
- `resolutionOrder` is required.

6. Resolver set/modifier constraints
- Sets require `sources`.
- Modifiers require `contexts` and at least two contexts.
- Modifier `default` must be runtime-validated against context keys.

7. Resolver reference restrictions
- Sets must not reference `#/modifiers/...` or `#/resolutionOrder/...`.
- Modifiers must not reference `#/modifiers/...` or `#/resolutionOrder/...`.
- Resolution order items must not reference `#/resolutionOrder/...`.

8. Resolution ordering behavior
- Merging is order-sensitive.
- Later entries win on conflict (last write wins).

## Practical Use

When answering DTCG questions for this codebase:
1. Prefer these local schema constraints for implementation-level guidance.
2. Use technical report text to explain rationale and semantics.
3. If schema and prose appear to differ, surface the difference and state which source is being applied.
