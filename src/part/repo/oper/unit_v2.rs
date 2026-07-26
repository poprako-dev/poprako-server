use poprako_orchestra::Oper;

use crate::model::read::proj::unit::{UnitInfo, UnitOrder};
use crate::model::write::unit::UnitEdit;

#[derive(Oper)]
#[oper(output = Vec<UnitInfo>)]
pub struct ListUnitInfos<'a> {
    pub page_id: &'a str,
}

#[derive(Oper)]
#[oper(output = Vec<UnitOrder>)]
pub struct ListUnitPositions<'a> {
    pub page_id: &'a str,
}

#[derive(Oper)]
#[oper(output = ())]
pub struct ApplyUnitEdits<'a> {
    pub orders: &'a [UnitOrder],
    pub edits: &'a [UnitEdit],
}
