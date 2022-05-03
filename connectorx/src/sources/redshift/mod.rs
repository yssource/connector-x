//! Source implementation for Redshift.

mod errors;
mod typesystem;

pub use self::errors::RedshiftSourceError;
pub use self::typesystem::RedshiftTypeSystem;

use crate::{
    data_order::DataOrder,
    errors::ConnectorXError,
    sources::{PartitionParser, Produce, Source, SourcePartition},
    sql::{limit1_query, CXQuery},
};
use anyhow::anyhow;
use aws_config;
use aws_sdk_redshiftdata;
use aws_sdk_redshiftdata::client::Client;
use aws_sdk_redshiftdata::model::StatusString;
use aws_sdk_redshiftdata::output::GetStatementResultOutput;
use fehler::{throw, throws};
use log::debug;
use sqlparser::dialect::PostgreSqlDialect;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

pub fn submit_sql(
    query: &str,
    client: &Client,
    rt: &Arc<Runtime>,
    config: &RedshiftConfig,
) -> String {
    let req = client.execute_statement();
    let resp = rt
        .block_on(
            req.sql(query)
                .cluster_identifier(config.cluster_identifier.clone())
                .database(config.database.clone())
                .db_user(config.db_user.clone())
                .send(),
        )
        .unwrap();
    resp.id().unwrap().to_string()
}

#[throws(RedshiftSourceError)]
pub fn fetch_sql_result(id: &str, client: &Client, rt: &Arc<Runtime>) -> GetStatementResultOutput {
    loop {
        let req = client.describe_statement();
        let resp = rt.block_on(req.id(id).send()).unwrap();
        debug!("status: {:?}", resp.status());

        let status = resp.status().unwrap();
        match *status {
            StatusString::Finished => break,
            StatusString::Submitted | StatusString::Picked | StatusString::Started => {}
            _ => throw!(RedshiftSourceError::QueryStatusError(status.clone())),
        }
        sleep(Duration::from_millis(1000));
    }
    let req = client.get_statement_result();
    rt.block_on(req.id(id).send()).unwrap()
}

#[throws(RedshiftSourceError)]
pub fn parse_redshift_config(conn: &str) -> RedshiftConfig {
    let url = Url::parse(conn)?;
    let cluster_id = url
        .host_str()
        .ok_or(RedshiftSourceError::ClusterIDNotFound)?;
    let database = &url.path()[1..];
    let db_user = url.username();

    RedshiftConfig {
        cluster_identifier: cluster_id.to_string(),
        database: database.to_string(),
        db_user: db_user.to_string(),
    }
}

#[derive(Clone)]
pub struct RedshiftConfig {
    cluster_identifier: String,
    database: String,
    db_user: String,
}

pub struct RedshiftSource {
    rt: Arc<Runtime>,
    config: RedshiftConfig,
    client: Client,
    origin_query: Option<String>,
    queries: Vec<CXQuery<String>>,
    names: Vec<String>,
    schema: Vec<RedshiftTypeSystem>,
}

impl RedshiftSource {
    #[throws(RedshiftSourceError)]
    pub fn new(rt: Arc<Runtime>, conn: &str) -> Self {
        let config = parse_redshift_config(conn)?;

        let shared_config = rt.block_on(aws_config::load_from_env());
        let client = aws_sdk_redshiftdata::Client::new(&shared_config);

        Self {
            rt,
            config,
            client,
            origin_query: None,
            queries: vec![],
            names: vec![],
            schema: vec![],
        }
    }
}

impl Source for RedshiftSource
where
    RedshiftSourcePartition: SourcePartition<TypeSystem = RedshiftTypeSystem>,
{
    const DATA_ORDERS: &'static [DataOrder] = &[DataOrder::RowMajor];
    type Partition = RedshiftSourcePartition;
    type TypeSystem = RedshiftTypeSystem;
    type Error = RedshiftSourceError;

    #[throws(RedshiftSourceError)]
    fn set_data_order(&mut self, data_order: DataOrder) {
        if !matches!(data_order, DataOrder::RowMajor) {
            throw!(ConnectorXError::UnsupportedDataOrder(data_order));
        }
    }

    fn set_queries<Q: ToString>(&mut self, queries: &[CXQuery<Q>]) {
        self.queries = queries.iter().map(|q| q.map(Q::to_string)).collect();
    }

    fn set_origin_query(&mut self, query: Option<String>) {
        self.origin_query = query;
    }

    #[throws(RedshiftSourceError)]
    fn fetch_metadata(&mut self) {
        assert!(!self.queries.is_empty());

        let first_query = &self.queries[0];
        let l1query = limit1_query(first_query, &PostgreSqlDialect {})?;
        let id = submit_sql(l1query.as_ref(), &self.client, &self.rt, &self.config);
        let resp = fetch_sql_result(id.as_str(), &self.client, &self.rt)?;

        match resp.column_metadata() {
            Some(meta) => meta.iter().for_each(|m| {
                self.names.push(m.name().unwrap().to_string());
                self.schema.push(RedshiftTypeSystem::from(m));
            }),
            None => throw!(RedshiftSourceError::MeatadataFetchFailed),
        }
    }

    #[throws(RedshiftSourceError)]
    fn result_rows(&mut self) -> Option<usize> {
        None
    }

    fn names(&self) -> Vec<String> {
        self.names.clone()
    }

    fn schema(&self) -> Vec<Self::TypeSystem> {
        self.schema.clone()
    }

    #[throws(RedshiftSourceError)]
    fn partition(self) -> Vec<Self::Partition> {
        let mut ret = vec![];
        for query in self.queries {
            let id = submit_sql(query.as_ref(), &self.client, &self.rt, &self.config);
            ret.push(RedshiftSourcePartition::new(
                self.rt.clone(),
                self.client.clone(),
                id,
                &self.schema,
            ));
        }
        ret
    }
}

pub struct RedshiftSourcePartition {
    rt: Arc<Runtime>,
    client: Client,
    schema: Vec<RedshiftTypeSystem>,
    id: String,
    output: Option<GetStatementResultOutput>,
    nrows: usize,
    ncols: usize,
}

impl RedshiftSourcePartition {
    pub fn new(
        rt: Arc<Runtime>,
        client: Client,
        id: String,
        schema: &[RedshiftTypeSystem],
    ) -> Self {
        Self {
            rt,
            client,
            id,
            output: None,
            schema: schema.to_vec(),
            nrows: 0,
            ncols: schema.len(),
        }
    }
}

impl SourcePartition for RedshiftSourcePartition {
    type TypeSystem = RedshiftTypeSystem;
    type Parser<'a> = RedshiftSourceParser;
    type Error = RedshiftSourceError;

    #[throws(RedshiftSourceError)]
    fn result_rows(&mut self) {
        let output = fetch_sql_result(self.id.as_str(), &self.client, &self.rt)?;
        self.nrows = output.total_num_rows() as usize;
        self.output = Some(output);
    }

    #[throws(RedshiftSourceError)]
    fn parser(&mut self) -> Self::Parser<'_> {
        let output: Option<GetStatementResultOutput> = if self.output.is_none() {
            Some(fetch_sql_result(self.id.as_str(), &self.client, &self.rt)?)
        } else {
            self.output.take()
        };

        RedshiftSourceParser::new(
            self.rt.clone(),
            self.client.clone(),
            output.unwrap(),
            self.id.clone(),
            self.schema.len(),
        )
    }

    fn nrows(&self) -> usize {
        self.nrows
    }

    fn ncols(&self) -> usize {
        self.ncols
    }
}

pub struct RedshiftSourceParser {
    rt: Arc<Runtime>,
    client: Client,
    output: GetStatementResultOutput,
    id: String,
    ncols: usize,
    current_col: usize,
    current_row: usize,
    init: bool,
}

impl RedshiftSourceParser {
    fn new(
        rt: Arc<Runtime>,
        client: Client,
        output: GetStatementResultOutput,
        id: String,
        ncols: usize,
    ) -> Self {
        Self {
            rt,
            client,
            output,
            id,
            ncols,
            current_row: 0,
            current_col: 0,
            init: true,
        }
    }

    #[throws(RedshiftSourceError)]
    fn next_loc(&mut self) -> (usize, usize) {
        let ret = (self.current_row, self.current_col);
        self.current_row += (self.current_col + 1) / self.ncols;
        self.current_col = (self.current_col + 1) % self.ncols;
        ret
    }
}

impl<'a> PartitionParser<'a> for RedshiftSourceParser {
    type TypeSystem = RedshiftTypeSystem;
    type Error = RedshiftSourceError;

    #[throws(RedshiftSourceError)]
    fn fetch_next(&mut self) -> (usize, bool) {
        // the first batch is already fetched
        if self.init {
            self.init = false;
            return match self.output.records() {
                Some(r) => (r.len(), self.output.next_token().is_none()),
                None => (0, true),
            };
        }

        let req = self.client.get_statement_result();
        match self.output.next_token() {
            Some(token) => {
                let resp = self
                    .rt
                    .block_on(req.id(&self.id).next_token(token).send())
                    .unwrap();
                self.output = resp;
                match self.output.records() {
                    Some(r) => (r.len(), self.output.next_token().is_none()),
                    None => (0, true),
                }
            }
            None => (0, true),
        }
    }
}

impl<'r> Produce<'r, i64> for RedshiftSourceParser {
    type Error = RedshiftSourceError;

    #[throws(RedshiftSourceError)]
    fn produce(&'r mut self) -> i64 {
        let (row, col) = self.next_loc()?;
        let val = &self
            .output
            .records()
            .ok_or(RedshiftSourceError::GetRecordsFailed)?[row][col];
        *val.as_long_value()
            .map_err(|e| anyhow!("cannot get long value from field: {:?}", e))?
    }
}

impl<'r> Produce<'r, Option<i64>> for RedshiftSourceParser {
    type Error = RedshiftSourceError;

    #[throws(RedshiftSourceError)]
    fn produce(&'r mut self) -> Option<i64> {
        let (row, col) = self.next_loc()?;
        let val = &self
            .output
            .records()
            .ok_or(RedshiftSourceError::GetRecordsFailed)?[row][col];

        match val.is_is_null() {
            true => None,
            false => Some(
                *val.as_long_value()
                    .map_err(|e| anyhow!("cannot get long value from field: {:?}", e))?,
            ),
        }
    }
}
