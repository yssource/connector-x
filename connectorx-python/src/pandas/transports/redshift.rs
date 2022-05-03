use crate::errors::ConnectorXPythonError;
use crate::pandas::destination::PandasDestination;
use crate::pandas::typesystem::PandasTypeSystem;
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
        { Integer[i64]                  => I64[i64]                | conversion auto }
    }
);
