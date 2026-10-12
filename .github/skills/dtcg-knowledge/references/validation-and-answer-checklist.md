# Validation and Answer Checklist

Use this checklist before finalizing any DTCG-heavy answer.

## 1. Scope Check
- Did I identify whether this is Format, Color, Resolver, or mixed?
- Did I align examples and terminology to that scope?
- Did I identify the compiler stage this guidance belongs to: parse/normalize, validate, semantic checks, or output generation?

## 2. Normative Correctness Check
- Did I avoid inventing unsupported token types?
- Did I avoid mixing token and group semantics?
- If references are used, did I choose the correct syntax?
  - Curly braces for full token values.
  - JSON Pointer in `$ref` for property-level/document-level paths.
- Did I call out circular reference invalidity where relevant?

## 3. Type and Value Check
- Did each token have a valid type path?
  - Explicit token `$type`, or
  - inherited group `$type`, or
  - resolved referenced token type.
- For typed examples, does `$value` shape match the declared type?

## 4. Resolver Check (if applicable)
- Is `version` set to `2025.10`?
- Are `resolutionOrder` semantics explained as order-dependent?
- Are modifier inputs validated against known contexts?
- Did I mention runtime-only checks where JSON Schema cannot enforce them?

## 5. Design Suggestion Check
- Did I provide a compliant baseline pattern first?
- If alternatives were suggested, did I include tradeoffs?
- Did I keep the suggestion interoperable across tools?

## 6. Completion Check
- Did I include at least one DTCG-valid JSON example when architecture or implementation was requested?
- Did I avoid unsupported claims and clearly mark uncertainty?
- Is the final answer concise enough for the request while still actionable?
- Did I avoid stale or speculative examples and keep advice current with 2025.10 behavior?
