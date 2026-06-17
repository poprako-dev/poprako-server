use crate::domain::query_legacy::QueryTransactional;
use crate::domain::query_legacy::workset::WorksetQueryTransactional;
use crate::domain::result::DomainResult;

pub struct WorksetComplex;

impl WorksetComplex {
    /// Deletes the workset.  When children (comics) are modelled they will be
    /// cascade-deleted here as well.
    pub async fn delete_cascade<Q>(query: &mut Q, id: &str) -> DomainResult<()>
    where
        Q: QueryTransactional,
    {
        WorksetQueryTransactional::delete(query, id).await?;

        Ok(())
    }
}

pub struct WorksetPermissionComplex;
