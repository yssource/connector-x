use aws_sdk_redshiftdata::model::ColumnMetadata;
use chrono::NaiveDate;

#[derive(Copy, Clone, Debug)]
pub enum RedshiftTypeSystem {
    Integer(bool),
    Float(bool),
    String(bool),
    Date(bool),
}

impl_typesystem! {
    system = RedshiftTypeSystem,
    mappings = {
        { Integer => i64 }
        { Float => f64 }
        { String => &'r str }
        { Date => NaiveDate }
    }
}

impl<'a> From<&'a ColumnMetadata> for RedshiftTypeSystem {
    fn from(ty: &'a ColumnMetadata) -> RedshiftTypeSystem {
        use RedshiftTypeSystem::*;

        let nullable = ty.nullable() != 0;
        match ty.type_name().unwrap() {
            "int8" | "int4" => Integer(nullable),
            "numeric" => Float(nullable),
            "bpchar" | "varchar" => String(nullable),
            "date" => Date(nullable),
            _ => unimplemented!("{}", format!("{:?}", ty)),
        }
    }
}
