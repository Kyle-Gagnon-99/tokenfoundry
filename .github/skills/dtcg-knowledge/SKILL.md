---
name: dtcg-knowledge
description: 'Project-first Design Tokens Community Group (DTCG) 2025.10 guidance for the tokenfoundry Rust compiler. Use for token and resolver parsing, normalization, validation, semantic checks (such as hoisting), architecture decisions, and DTCG-compliant examples that must stay current and accurate.'
argument-hint: 'Question or goal needing DTCG expertise'
---

# DTCG Knowledge

This skill is primarily for the current tokenfoundry project, a Rust compiler that consumes DTCG resolver and token files and produces outputs.

Primary purpose: inject accurate, current context about what DTCG is and how its 2025.10 rules apply to this compiler project.

Use this skill when a request needs Design Tokens Community Group knowledge, especially for:
- DTCG concept grounding for ongoing implementation work
- DTCG-compliant token authoring
- Format, Color, and Resolver interpretation
- Schema-aware validation and troubleshooting
- Design-token naming and architecture suggestions
- Rust compiler architecture brainstorming and design tradeoff analysis
- Tradeoff analysis for aliases, groups, and modifiers
- Compiler-stage guidance for conversion, validation, and semantic checks

## Context Injection Behavior
When relevant, begin with a compact DTCG primer before implementation details:
1. What DTCG is: a shared technical specification for interoperable design token data.
2. What modules matter for this project: Format, Color, and Resolver.
3. Why it matters here: compiler outputs must remain semantically faithful to DTCG input and rules.

Keep the primer short and immediately connect it to the current compiler task.

## Architecture Brainstorm Mode
When the user is deciding how to build compiler internals, structure responses around:
1. Pipeline boundaries: parse, normalize, validate, semantic analysis, output generation.
2. Data model shape: AST vs normalized IR vs resolved IR and why.
3. Validation layering: schema-style structural checks vs semantic checks vs output-target checks.
4. Extensibility: adding new output targets without rewriting core passes.
5. Safety and diagnostics: deterministic behavior, error surfacing, and actionable diagnostics.

Prefer explicit tradeoffs and recommend incremental, testable designs.

## Project Context
Assume a compiler pipeline with these stages unless the user overrides it:
1. Parse and normalize input files (token format files and resolver files).
2. Validate against DTCG 2025.10 constraints.
3. Resolve semantics and run additional checks (for example token hoisting and related integrity checks).
4. Produce stable outputs while preserving DTCG correctness.

When answering, map advice to one or more stages above.

## Outcome
Produce answers that are:
- Normative-first: clearly separate MUST/SHOULD from recommendations
- Schema-grounded: aligned to DTCG 2025.10 schemas and module docs
- Actionable: include concrete token snippets, validation checks, and migration guidance
- Compiler-safe: examples and claims should be suitable for implementation in Rust compiler logic

## Authoritative Sources
1. Local schemas in this repository (implementation source of truth):
   - [Local Schema Map](./references/local-schema-map.md)
2. DTCG technical reports 2025.10:
   - https://www.designtokens.org/tr/2025.10/
   - https://www.designtokens.org/tr/2025.10/format/
   - https://www.designtokens.org/tr/2025.10/color/
   - https://www.designtokens.org/tr/2025.10/resolver/
3. Quality checklist for final answers:
   - [Validation and Answer Checklist](./references/validation-and-answer-checklist.md)

## Procedure
1. Classify the request.
   - Format question: token/group structure, aliases, $type, $root, $extends.
   - Color question: colorSpace, components, alpha, hex fallback, gamut implications.
   - Resolver question: sets, modifiers, contexts, inputs, resolutionOrder.
   - Design suggestion question: naming, layering, token taxonomy, semantic strategy.

2. Gather constraints from the request.
   - Determine whether the user needs strict conformance, practical guidance, or both.
   - Note target output: explanation, example JSON, review, migration plan, or fix.
   - Identify where this fits in compiler flow: parse/normalize, validate, semantic checks, or output generation.
   - Determine whether explicit DTCG context injection is needed (for example in brainstorming, design discussions, or examples).

3. Apply DTCG rules before proposing solutions.
   - Validate token or group identity using the $value rule.
   - Enforce type behavior: explicit, inherited, or alias-resolved type.
   - Validate reference semantics:
     - Curly braces for token-to-token values.
     - $ref JSON Pointer for property-level or document-level references.
   - For resolver content, validate contexts, defaults, and ordering semantics.

4. Generate answer or design recommendation.
   - Give a compliant baseline pattern first.
   - Then provide 1-2 alternatives with tradeoffs when useful.
   - Include realistic snippets that respect the declared type semantics.
   - If implementation guidance is requested, provide explicit checks that could map to Rust validation passes.

5. Run completion checks.
   - Confirm no contradictory statements against 2025.10 schemas/spec text.
   - Flag uncertainty explicitly instead of guessing.
   - Call out runtime checks not expressible in JSON Schema where relevant.
   - Reject stale or speculative examples; prefer exact, schema-valid structures.

## Decision Points
- If the user asks for strict compliance:
  - Prioritize normative requirements and schema constraints.
  - Explicitly mark invalid patterns and why.
- If the user asks for design suggestions:
  - Provide architecture patterns (base, alias, component tokens) and explain tradeoffs.
  - Keep suggestions DTCG-valid even when opinionated.
- If references are involved:
  - Use curly braces only for full token values.
  - Use JSON Pointer $ref for sub-value targeting.
- If resolver is involved:
  - Validate modifier inputs and context values.
  - Emphasize last-write-wins behavior from source and resolution ordering.
- If a compiler implementation choice is involved:
   - Separate syntax validation from semantic validation.
   - State which checks are schema-level versus runtime semantic checks.

## Quality Criteria
The response is complete when it:
- Uses correct DTCG terminology (token, group, alias/reference, composite token, set, modifier, resolution order).
- Distinguishes normative constraints from implementation advice.
- Includes at least one DTCG-valid example when proposing structure.
- Mentions key edge cases when relevant (circular refs, ambiguous token/group shape, unresolved type, invalid pointers).

## Common Pitfalls To Catch
- Treating a group as a token or vice versa.
- Allowing both $value and $ref in the same token object.
- Inferring type from value when no explicit/inherited/resolved type exists.
- Using curly references for property-level access.
- Referencing resolver paths that are forbidden (such as resolutionOrder items).
- Ignoring runtime checks such as modifier default not in contexts, or duplicate inline names in resolutionOrder.
- Providing examples that look plausible but violate 2025.10 constraints.
