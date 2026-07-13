use poprako_orchestra::Oper;

pub enum DeleteTerms<'a, 'b> {
    Termbase { termbase_id: &'a str },
    Batch { ids: &'a [&'b str] },
}

impl<'a, 'b> Oper for DeleteTerms<'a, 'b> {
    type Output = ();
}
