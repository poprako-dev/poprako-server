use poprako_orchestra::Step;
use poprako_orchestra_extra::prom::oper::{Defer, DeferBatch};

use crate::part::prom_new::payload::Payload;
use crate::result::RegularError;

pub mod payload;

pub trait Prom<C>:
    for<'a> Step<Defer<'a, String, Payload, ()>, C, Error = RegularError>
    + for<'t, 'a> Step<
        DeferBatch<'t, 'a, String, Payload, ()>,
        C,
        Error = RegularError,
    >
{
}
