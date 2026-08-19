# Qualified call-site paths

Production call sites must use the project's path policy:

- `std::...` paths must be imported and used through their local name.
- Local `crate::...` paths may appear only in `use` declarations. All
  expressions, types, trait bounds, and function calls must use imported local
  names.
- `poprako_*::...` paths must be imported and used through their local name.
- The `[qualified-call-site].enforced_third_party_paths` configuration lists
  exact third-party imports whose names must remain qualified at call sites.
- Other third-party crates are exempt from the bare-name rule.
- The `[qualified-call-site].enforced_std_paths` configuration lists exact
  standard-library paths that must be imported before use.
- The `[qualified-call-site].exempt_poprako_paths` configuration lists exact
  PopRaKo paths exempt from the local-crate qualification rule.
- Macro contexts are exempt from the external and standard-library call-site
  rules. Local `crate::...` paths remain forbidden there.

The checker scans all expressions, types, and macro paths outside `use`
declarations, including code under `#[cfg(test)]`, for local `crate::...`
paths. The other call-site rules scan production code.

```bash
uv run linters-extra/qualified-call-site/check.py
```
