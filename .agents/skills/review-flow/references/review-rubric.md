# Review Rubric

Use this reference when a review needs explicit severity calibration or structured decision output.

## Core principles

- Prefer simpler data relationships, linear flow, early returns, exhaustive matches, and explicit ownership over special-case branches.
- Judge compatibility against the current freeze state. Breaking changes are acceptable before freeze with companion updates; post-freeze contract changes need migration or deprecation planning.
- Keep severity proportional to real user or maintainer impact. Do not overbuild speculative infrastructure for unproven problems.

## Review output

When there are findings:

1. Lead with the highest-severity finding first.
2. Ground each finding in the changed contract, behavior, or missing validation.
3. Call out the smallest credible improvement, not an abstract ideal rewrite.

When there are no findings:

- say that clearly
- mention any residual risk
- mention any validation gaps that still exist

## Decision framing

For design or merge-readiness decisions, report:

- core judgment
- key data relationships or removable complexity
- biggest remaining risk
- smallest credible next step

Do not convert a merge-readiness judgment into `gh pr merge`, auto-merge, branch deletion, or Project `Done` status unless Loocor explicitly asks for that action in the current session.
