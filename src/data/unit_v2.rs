use crate::{
    model::write::unit::UnitEdit,
    result::{BaseError, BaseResult},
};

pub enum UnitEditVal {
    Create {},
    Patch {},
    Delete {},
}

impl TryInto<UnitEdit> for UnitEditVal {
    type Error = BaseError;

    fn try_into(self) -> BaseResult<UnitEdit> {
        // gen_id for Create to transform it into a model-layer Save

        todo!()
    }
}
