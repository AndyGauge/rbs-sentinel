# CI and Pre-commit Checks

`sentinel check` verifies that every generated `.rbs` file is up to date **without modifying anything**, and exits `1` if any signature is missing or stale — the same check your editor's `sentinel lsp`/`watch` keeps satisfied automatically, run as a gate.

```bash
bundle exec sentinel check
```

## GitHub Actions

```yaml
- name: Check RBS signatures are up to date
  run: bundle exec sentinel check

- name: Type-check with Steep
  run: bundle exec steep check
```

Run `sentinel check` before `steep check` — a stale or missing `.rbs` file means Steep is checking against the wrong signature (or none at all), so catch that first.

## Git pre-commit hook

```bash
#!/usr/bin/env bash
set -e

bundle exec sentinel check
```

```bash
chmod +x .git/hooks/pre-commit
```

Or via a hook manager like [Lefthook](https://github.com/evilmartians/lefthook) or [Husky](https://github.com/typicode/husky), if you're already using one for other checks.

If the check fails locally, run `bundle exec sentinel init` to regenerate, review the diff, and commit again.
