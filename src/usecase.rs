pub mod hook;
pub mod value_object;

pub mod user;

pub mod result {
    use crate::domain::result::DomainErr;
    use crate::util::rename::StdResl;

    pub struct UseCaseErr(DomainErr);

    impl AsRef<DomainErr> for UseCaseErr {
        fn as_ref(&self) -> &DomainErr {
            &self.0
        }
    }

    impl From<DomainErr> for UseCaseErr {
        fn from(value: DomainErr) -> Self {
            UseCaseErr(value)
        }
    }

    pub type UseCaseResl<T> = StdResl<T, UseCaseErr>;
}
