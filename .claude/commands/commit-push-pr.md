# Commit, Push & Create PR

Commit staged changes, push to remote, and create a pull request.

## Pre-computed Context

**Branch Info:**
```bash
git branch --show-current
```

**Remote Tracking:**
```bash
git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null || echo "No upstream set"
```

**Commits ahead/behind:**
```bash
git rev-list --left-right --count @{u}...HEAD 2>/dev/null || echo "0 0"
```

**Unstaged Changes:**
```bash
git diff --stat | tail -1
```

**Staged Changes:**
```bash
git diff --cached --stat | tail -1
```

**Untracked Files:**
```bash
git ls-files --others --exclude-standard | wc -l | tr -d ' '
```

**Recent Commits (for message style):**
```bash
git log --oneline -5 2>/dev/null
```

**Changed Files (staged):**
```bash
git diff --cached --name-status
```

## Instructions

1. **Review the pre-computed context above** to understand current state

2. **If there are unstaged changes**, ask user if they want to:
   - Stage all changes (`git add -A`)
   - Stage specific files
   - Proceed with only staged changes

3. **Generate a commit message** based on:
   - The staged diff content
   - Follow the style of recent commits in this repo
   - Use conventional commit format if the repo uses it
   - Keep it concise but descriptive

4. **Create the commit:**
   ```bash
   git commit -m "commit message here"
   ```

5. **Push to remote:**
   - If no upstream, set it: `git push -u origin <branch>`
   - Otherwise: `git push`

6. **Create PR using GitHub CLI:**
   ```bash
   gh pr create --fill
   ```
   Or with custom title/body if needed.

7. **Return the PR URL** to the user

## Quick Mode

If user says "quick" or "fast", skip confirmations and:
- Stage all changes
- Auto-generate commit message
- Push and create PR with auto-filled description
