use poprako_orchestra::Oper;

use crate::model::unit::{
    UnitContent, UnitCounters, UnitIndex, UnitIndexUpdate, UnitInfo,
};

pub struct ListUnitInfos<'a> {
    pub page_id: &'a str,
}

impl Oper for ListUnitInfos<'_> {
    type Output = Vec<UnitInfo>;
}

pub struct CreateUnit<'a> {
    //
    pub page_id: &'a str,
    pub id: &'a str,
    pub payload: &'a UnitContent,
}

impl Oper for CreateUnit<'_> {
    type Output = ();
}

pub struct SaveUnit<'a> {
    //
    pub page_id: &'a str,
    pub id: &'a str,
    pub payload: &'a UnitContent,
}

impl Oper for SaveUnit<'_> {
    type Output = ();
}

pub struct DeleteUnit<'a> {
    //
    pub page_id: &'a str,
    pub id: &'a str,
}

impl Oper for DeleteUnit<'_> {
    type Output = ();
}

pub struct ListUnitIndexes<'a> {
    pub page_id: &'a str,
}

impl Oper for ListUnitIndexes<'_> {
    type Output = Vec<UnitIndex>;
}

pub struct UpdateUnitIndexes<'a> {
    //
    pub page_id: &'a str,
    pub updates: &'a [UnitIndexUpdate],
}

impl Oper for UpdateUnitIndexes<'_> {
    type Output = ();
}

pub struct CountUnits<'a> {
    pub page_id: &'a str,
}

impl Oper for CountUnits<'_> {
    type Output = UnitCounters;
}
