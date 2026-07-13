use poprako_orchestra::Oper;

use crate::model::termbase::TermbaseEntry;

pub struct CreateTermbase<'a> {
    pub entry: &'a TermbaseEntry,
}

impl<'a> Oper for CreateTermbase<'a> {
    type Output = ();
}

pub struct LockTermbaseExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Oper for LockTermbaseExcluded<'a> {
    type Output = ();
}

pub struct DeleteTermbase<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteTermbase<'a> {
    type Output = ();
}
