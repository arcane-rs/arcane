pub trait Aggregate {
    type Id: ?Sized;

    fn type_name(&self) -> TypeName;

    fn id(&self) -> &Self::Id;
}

pub type TypeName = &'static str;
