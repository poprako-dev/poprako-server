//! Incl path planning shared by incl opt enums.

/// Incl opt that can expand itself into the full dependency path.
pub trait InclOpt: Copy + Eq + 'static {
    /// Returns the dependency-ordered path required to populate this option.
    fn path(self) -> &'static [Self];
}

/// Expands requested incl opts into a dependency-ordered, de-duplicated plan.
pub fn expand_incl_opts<I>(incl_opts: &[I]) -> Vec<I>
where
    I: InclOpt,
{
    let mut expanded_incl_opts = Vec::new();

    for incl_opt in incl_opts {
        push_path(&mut expanded_incl_opts, incl_opt.path());
    }

    expanded_incl_opts
}

fn push_path<I>(expanded_incl_opts: &mut Vec<I>, path: &[I])
where
    I: InclOpt,
{
    for incl_opt in path {
        push_unique(expanded_incl_opts, *incl_opt);
    }
}

fn push_unique<I>(expanded_incl_opts: &mut Vec<I>, incl_opt: I)
where
    I: InclOpt,
{
    if expanded_incl_opts.contains(&incl_opt) {
        return;
    }
    expanded_incl_opts.push(incl_opt);
}
