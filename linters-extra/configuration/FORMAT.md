# Project lint configuration

`CFG001` rejects unknown checker names, disabled or missing shared checkers,
unsupported configuration sections or keys, empty forbidden-rule lists, and
missing, empty or unknown identifier contexts. All 16 shared checkers must run.
The project configuration replaces shared defaults; absent contexts never
inherit defaults and would otherwise silently disable a rule.

Restored contexts follow the checked-in shared defaults for `result`, `res`,
`closure`, `connection`, `txn`, `tx`, `extension` and `previous`. The project's
`replace` and `outcome` policies use the same six declaration/binding contexts
as the earlier FBD012/FBD013 defaults. The user's explicit ten contexts for
`err` and all previously explicit contexts remain unchanged. This activates
existing policies without prohibiting legitimate modules such as `result`.

All hand-authored repository, term and termbase modules participate in the
identifier policy. `skip_module_paths` and `ignore_files` must remain empty;
the configuration gate rejects blanket skips. Generated `schema.rs` remains
excluded through `exclude_filenames`. No identifier context is widened by
removing these historical temporary file and directory exclusions.

The runner invokes this checker once per workspace. Its isolated self-test
loads the real root configuration, exercises each shared checker against a
violating Rust fixture, and exercises all 30 configured words and five prefixes
in every context selected by that rule. Missing diagnostic codes fail the test.
Mutation tests ensure disabled checkers and inactive rules fail validation.

The shared identifier checker omits module declarations. This project checker
also walks production module declarations and applies the same configured word
and prefix rules with the `module` context, preserving the configured filename,
path and file exclusions. This closes that gap without changing the shared
submodule or silently dropping the selected context.

Identifier word diagnostics use `replacement`, when present. The shared checker
does not consume a `message` on word rules, so this key is rejected. Prefix
rules consume their configured message. Trait imports are discovered directly;
the obsolete `external_traits` and `macro_traits` lists are rejected. No macro
marker suppresses checks for an entire source file in the project configuration.

The fixtures test syntax-based diagnostics; they are deliberately not compiled
Rust programs and do not claim compiler name-resolution coverage. Existing
generated-schema exclusions remain owned by their respective shared checkers.

```sh
PYTHONPATH=linters linters-extra/.venv/bin/python linters-extra/configuration/check.py --self-test
PYTHONPATH=linters linters-extra/.venv/bin/python linters-extra/configuration/check.py --root .
```
