#[async_trait::async_trait]
pub trait MemberQueryMut {
    async fn create(
        &mut self,
        form: crate::domain::model::aggr::member::MemberForm,
    ) -> crate::domain::query::QueryRetVal<crate::domain::model::aggr::member::Member>;
}
