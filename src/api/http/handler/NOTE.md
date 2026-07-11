# NOTE

## Do not see `use HttpBody` as a violation

As `HttpBody` is only imported when swagger feature is enabled, it cannot be merged into normal `use`s.
