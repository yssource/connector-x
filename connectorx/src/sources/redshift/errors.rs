use thiserror::Error;

#[derive(Error, Debug)]
pub enum RedshiftSourceError {
    #[error(transparent)]
    ConnectorXError(#[from] crate::errors::ConnectorXError),

    #[error("Get records failed")]
    GetRecordsFailed,

    #[error("Cluster ID is required")]
    ClusterIDNotFound,

    #[error("Metadata fetch failed")]
    MeatadataFetchFailed,

    #[error("Query status error: {0:?}")]
    QueryStatusError(aws_sdk_redshiftdata::model::StatusString),

    #[error(transparent)]
    RedshiftUrlError(#[from] url::ParseError),

    // #[error(transparent)]
    // SdkError(#[from] aws_sdk_redshiftdata::types::SdkError<E>),
    /// Any other errors that are too trivial to be put here explicitly.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
