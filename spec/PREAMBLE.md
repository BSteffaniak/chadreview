# ChadReview - Preamble

GitHub's PR review interface suffers from critical usability issues: comments only auto-update at the top level (not for file-level or inline comments), poor performance when reviewing large PRs, and a cluttered UI that makes it difficult to focus on the actual code changes and discussions. These limitations make the review process frustrating and inefficient, especially for complex PRs with extensive discussions.

This specification outlines the implementation of ChadReview, a high-performance PR review tool built on the HyperChad framework. The solution leverages HyperChad's SSE-based real-time updates and efficient HTML rendering to provide instant comment synchronization across all comment types, exceptional performance on large PRs, and a clean, focused interface that eliminates noise and distractions.

ChadReview will be implemented as a HyperChad application supporting both desktop (via Egui/FLTK) and web deployment from a single codebase. This approach allows for flexibility in deployment scenarios while maintaining consistent functionality and user experience across platforms. The real-time synchronization is handled automatically by HyperChad's built-in SSE infrastructure.

The implementation will follow an MVP-first approach, starting with a single PR view and essential comment interactions. The system is designed to be a complete alternative to GitHub's web UI for PR review workflows, with extensibility built in for future enhancements like CI/CD integration and advanced review workflows.

## Prerequisites

- Follow MoosicBox coding conventions (BTreeMap/BTreeSet, workspace dependencies)
- HyperChad framework from git: `https://github.com/MoosicBox/MoosicBox`
- GitHub Personal Access Token for API access (MVP provider)
- Trait-based design for git provider abstraction (GitHub-only for MVP, extensible to GitLab/Bitbucket/etc.)

## Context

- Specs use checkboxes (`- [ ]`) to track progress
- Four-phase workflow: preliminary check → deep analysis → execution → verification
- NO COMPROMISES - halt on any deviation from spec
  - Includes comprehensive test coverage for all business logic
  - Tests must be written alongside implementation, not deferred
  - Both success and failure paths must be tested
- Living documents that evolve during implementation
- After completing a checkbox, 'check' it and add details under it regarding the file/location updated as PROOF

See `spec/plan.md` for the current status and what's next to be done.
