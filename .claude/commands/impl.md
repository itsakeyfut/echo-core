---
description: Start implementing a GitHub issue
allowed-tools: ["bash", "read", "write", "edit", "glob", "grep", "task"]
argument-hint: "<issue-number>"
---

First, fetch the issue details:

```bash
gh issue view $1
```

Now proceed with implementing this issue.

**Development Guidelines:**
- Refer to documents in `specs/` for development and design guidelines
- Follow the architecture in `CLAUDE.md`
- All comments and documentation must be written in English
- Check relevant spec documents in `specs/01-design/`, `specs/02-implementation/`, `specs/03-hardware-specs/`

**Before starting:**
1. Review the issue requirements carefully
2. Check acceptance criteria
3. Identify affected components (CPU, GPU, Memory, System, etc.)
4. Plan the implementation approach

**Implementation checklist:**
- [ ] Update code following conventions in `CLAUDE.md`
- [ ] Add unit tests in `#[cfg(test)]` sections
- [ ] Document public APIs with rustdoc comments
- [ ] Follow error handling patterns from `specs/02-implementation/error-handling.md`
- [ ] Use `#[inline(always)]` for hot paths (CPU/GPU critical methods)
- [ ] Run `cargo x fmt` before committing
- [ ] Run `cargo x clippy` to check warnings
- [ ] Run `cargo x test` to verify all tests pass

Please proceed with the implementation.
