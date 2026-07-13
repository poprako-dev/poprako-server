use poprako_orchestra::Oper;

use poprako_util::page::Page;

use crate::model::unit::{
    UnitContent, UnitCounters, UnitIndex, UnitIndexUpdate, UnitInfo,
};

pub enum ListUnitInfos<'a> {
    Page { page_id: &'a str, page: Page },
    AllPage { page_id: &'a str },
}

impl<'a> Oper for ListUnitInfos<'a> {
    type Output = Vec<UnitInfo>;
}

pub struct CreateUnit<'a> {
    pub page_id: &'a str,
    pub id: &'a str,
    pub payload: &'a UnitContent,
}

impl<'a> Oper for CreateUnit<'a> {
    type Output = ();
}

pub struct SaveUnit<'a> {
    pub page_id: &'a str,
    pub id: &'a str,
    pub payload: &'a UnitContent,
}

impl<'a> Oper for SaveUnit<'a> {
    type Output = ();
}

pub struct DeleteUnit<'a> {
    pub page_id: &'a str,
    pub id: &'a str,
}

impl<'a> Oper for DeleteUnit<'a> {
    type Output = ();
}

pub struct ListUnitIndexes<'a> {
    pub page_id: &'a str,
}

impl<'a> Oper for ListUnitIndexes<'a> {
    type Output = Vec<UnitIndex>;
}

pub struct UpdateUnitIndexes<'a> {
    pub page_id: &'a str,
    pub updates: &'a [UnitIndexUpdate],
}

impl<'a> Oper for UpdateUnitIndexes<'a> {
    type Output = ();
}

pub struct CountUnits<'a> {
    pub page_id: &'a str,
}

impl<'a> Oper for CountUnits<'a> {
    type Output = UnitCounters;
}
