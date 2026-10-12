# Tokenfoundry Copilot Instructions

This repository is building a Rust compiler that consumes DTCG token files and resolver files, then compiles them into one or more output targets.

When assisting in this repository:

1. Prioritize architecture guidance for a compiler pipeline:
- parse and normalize inputs
- validate structural and schema-aligned constraints
- run semantic checks (such as hoisting and resolution integrity)
- generate target outputs with deterministic behavior

2. Default to explicit tradeoff analysis for design decisions.
- Explain why a design is chosen.
- Call out alternatives and what they optimize for.

3. Keep DTCG correctness strict.
- Avoid speculative examples that are not compliant.
- Use DTCG 2025.10 terminology and constraints.
- Distinguish schema-level checks from runtime semantic checks.

4. For deep DTCG details, load and use the `dtcg-knowledge` skill in `.github/skills/dtcg-knowledge/`.

5. Keep examples implementation-oriented for Rust compiler work.
- Favor pass-oriented suggestions and clear data-flow boundaries.
- Include validation and diagnostics considerations when relevant.