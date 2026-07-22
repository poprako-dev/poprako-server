//! Incl path planning shared by incl opt enums.

#[cfg(test)]
mod tests;

/// Incl opt that can expand itself into the full dependency path.
pub trait InclOpt: Eq {
    /// Returns the dependency-ordered path required to populate this option.
    fn path(self) -> &'static [Self]
    where
        Self: Sized + 'static;
}

/// Expands requested incl opts into a dependency-ordered, de-duplicated plan.
pub fn expand_incl_opts<I>(incl_opts: &[I]) -> Vec<I>
where
    I: InclOpt + Copy + 'static,
{
    let mut expanded_incl_opts = Vec::new();

    for incl_opt in incl_opts {
        push_path(&mut expanded_incl_opts, incl_opt.path());
    }

    expanded_incl_opts
}

/// Append the full dependency path for a single incl opt, expanding
/// dependencies in order and deduplicating as we go.
fn push_path<I>(expanded_incl_opts: &mut Vec<I>, path: &[I])
where
    I: InclOpt + Copy,
{
    for incl_opt in path {
        push_unique(expanded_incl_opts, *incl_opt);
    }
}

/// Append a single incl opt to the expanded list if it is not already present,
/// preserving insertion order.
fn push_unique<I>(expanded_incl_opts: &mut Vec<I>, incl_opt: I)
where
    I: InclOpt + Copy,
{
    if expanded_incl_opts.contains(&incl_opt) {
        return;
    }

    expanded_incl_opts.push(incl_opt);
}
