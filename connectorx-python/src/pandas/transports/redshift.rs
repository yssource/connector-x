use crate::errors::ConnectorXPythonError;
use crate::pandas::destination::PandasDestination;
use crate::pandas::typesystem::PandasTypeSystem;
use chrono::{DateTime, NaiveDate, Utc};
use connectorx::{
    impl_transport,
    sources::redshift::{RedshiftSource, RedshiftTypeSystem},
    typesystem::TypeConversion,
};

pub struct RedshiftPandasTransport<'py>(&'py ());

impl_transport!(
    name = RedshiftPandasTransport<'tp>,
    error = ConnectorXPythonError,
    systems = RedshiftTypeSystem => PandasTypeSystem,
    route = RedshiftSource => PandasDestination<'tp>,
    mappings = {
        { Integer[i64]                  => I64[i64]                     | conversion auto }
        { Float[f64]                    => I64[i64]                     | conversion auto }
        { String[&'r str]               => Str[&'r str]                 | conversion auto }
        { Date[NaiveDate]               => DateTime[DateTime<Utc>]      | conversion option }
    }
);

impl<'py> TypeConversion<NaiveDate, DateTime<Utc>> for RedshiftPandasTransport<'py> {
    fn convert(val: NaiveDate) -> DateTime<Utc> {
        DateTime::from_utc(val.and_hms(0, 0, 0), Utc)
    }
}
