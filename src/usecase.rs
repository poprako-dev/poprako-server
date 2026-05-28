pub mod hook;
pub mod value_object;

pub mod user;

pub mod result {
    use crate::domain::result::DomainErr;
    use crate::util::rename::StdResl;

    #[derive(Debug)]
    pub struct UseCaseErr(DomainErr);

    impl std::fmt::Display for UseCaseErr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "UseCaseErr({})", self.0)
        }
    }

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
