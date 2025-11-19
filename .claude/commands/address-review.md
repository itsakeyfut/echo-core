---
description: Address code review suggestions (no commit, verify build/clippy/fmt)
allowed-tools: ["read", "edit", "write", "bash", "grep", "glob"]
---

Please address the following code review suggestions that you deem appropriate.

**Important constraints:**
- **Do NOT commit changes**
- After modifying source code, ONLY run build, clippy, and format to verify
- Focus on suggestions that improve code quality, correctness, or maintainability

**Verification steps after changes:**

1. **Format code:**
   ```bash
   cargo x fmt
   ```

2. **Check with Clippy:**
   ```bash
   cargo x clippy
   ```

3. **Build project:**
   ```bash
   cargo x build
   ```

4. **Run tests (if applicable):**
   ```bash
   cargo x test
   ```

**Review suggestions to address:**

$ARGUMENTS

---

Please analyze these suggestions, implement appropriate changes, and verify they pass all checks listed above.
