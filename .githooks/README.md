# Git Hooks

This directory contains git hooks to maintain code quality.

## Installation

### Option 1: Configure git to use this hooks directory (Recommended)

```bash
git config core.hooksPath .githooks
```

This tells git to use hooks from `.githooks/` instead of `.git/hooks/`.

### Option 2: Copy hooks manually

```bash
cp .githooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Available Hooks

### pre-commit

Automatically runs `cargo fmt --all` before each commit to ensure consistent code formatting.

**What it does:**
- Runs `cargo fmt --all` on all workspace packages
- Automatically adds formatted files to the commit
- Ensures CI formatting checks never fail

**Note:** This hook will add formatting changes to your commit automatically. Your code will always be properly formatted.
