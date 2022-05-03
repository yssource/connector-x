use aws_sdk_redshiftdata::model::ColumnMetadata;

#[derive(Copy, Clone, Debug)]
pub enum RedshiftTypeSystem {
    Integer(bool),
}

impl_typesystem! {
    system = RedshiftTypeSystem,
    mappings = {
        { Integer => i64 }
    }
}

impl<'a> From<&'a ColumnMetadata> for RedshiftTypeSystem {
    fn from(ty: &'a ColumnMetadata) -> RedshiftTypeSystem {
        use RedshiftTypeSystem::*;

        match ty.type_name().unwrap() {
            "int8" => Integer(true),
            _ => unimplemented!("{}", format!("{:?}", ty)),
        }
    }
}
