//! Transport from Redshift Source to Arrow Destination.

use crate::{
    destinations::arrow::{typesystem::ArrowTypeSystem, ArrowDestination, ArrowDestinationError},
    impl_transport,
    sources::redshift::{RedshiftSource, RedshiftSourceError, RedshiftTypeSystem},
    typesystem::TypeConversion,
};
use chrono::NaiveDate;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RedshiftArrowTransportError {
    #[error(transparent)]
    Source(#[from] RedshiftSourceError),

    #[error(transparent)]
    Destination(#[from] ArrowDestinationError),

    #[error(transparent)]
    ConnectorX(#[from] crate::errors::ConnectorXError),
}

/// Convert Redshift data types to Arrow data types.
pub struct RedshiftArrowTransport;

impl_transport!(
    name = RedshiftArrowTransport,
    error = RedshiftArrowTransportError,
    systems = RedshiftTypeSystem => ArrowTypeSystem,
    route = RedshiftSource => ArrowDestination,
    mappings = {
        { Integer[i64]                  => Int64[i64]                   | conversion auto }
        { Float[f64]                    => Float64[f64]                 | conversion auto }
        { String[&'r str]               => LargeUtf8[String]            | conversion owned }
        { Date[NaiveDate]               => Date32[NaiveDate]            | conversion auto }
    }
);
