---
description: Commit changes and create PR (keep under 100 lines)
allowed-tools: ["bash", "read", "grep"]
argument-hint: "[file1] [file2] ..."
---

Complete the implementation workflow:

**Steps:**

1. **Check current status:**
   ```bash
   git status
   git diff --stat
   ```

2. **Verify changes are under 100 lines:**
   ```bash
   git diff | wc -l
   ```
   Check the diff size. If over 100 lines, consider splitting into multiple PRs.

3. **Move to appropriate working branch** (if not already):
   Determine the correct branch based on the changes.
   If needed, create a feature branch:
   ```bash
   git checkout -b feature/issue-XXX
   ```

4. **Run quality checks:**
   ```bash
   cargo x fmt
   cargo x clippy
   cargo x test
   ```

5. **Stage and commit changes:**

   **File selection:**
   - If specific files were provided as arguments: `$ARGUMENTS`
     → Use: `git add $ARGUMENTS` (commit only specified files)
   - If no arguments were provided:
     → Use: `git add .` (commit all changed files)

   **Commit guidelines:**
   - Create logical, atomic commits
   - Follow conventional commits format (feat/fix/docs/refactor/test/chore)
   - Reference issue numbers with "Closes #XXX"
   - Example: `feat(gpu): implement VRAM transfer commands\n\nCloses #29`

6. **Push changes:**
   ```bash
   git push -u origin <branch-name>
   ```

7. **Create PR using gh command:**
   ```bash
   gh pr create --title "..." --body "..."
   ```

**PR Guidelines:**
- Keep PR concise (target: under 100 lines)
- Clear title following conventional commits
- Summarize changes in 2-3 bullet points
- Reference related issue with "Closes #X"
- Include test plan if applicable

Please proceed with these steps.
