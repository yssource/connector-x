use connectorx::{
    destinations::arrow::ArrowDestination, prelude::*, sources::redshift::RedshiftSource,
    sql::CXQuery, transports::RedshiftArrowTransport,
};
use std::env;
use std::sync::Arc;
use tokio::runtime::Runtime;

fn main() {
    let dburl = env::var("REDSHIFT_URL").unwrap();
    let rt = Arc::new(Runtime::new().unwrap());
    let source = RedshiftSource::new(rt, &dburl).unwrap();
    let queries = [
        CXQuery::naked("SELECT l_orderkey, l_partkey FROM lineitem WHERE l_orderkey < 100000"),
        CXQuery::naked("SELECT l_orderkey, l_partkey FROM lineitem WHERE l_orderkey >= 100000 and l_orderkey < 200000"),
        CXQuery::naked("SELECT l_orderkey, l_partkey FROM lineitem WHERE l_orderkey >= 200000 and l_orderkey < 300000")
    ];
    let mut destination = ArrowDestination::new();
    let dispatcher =
        Dispatcher::<_, _, RedshiftArrowTransport>::new(source, &mut destination, &queries, None);
    dispatcher.run().unwrap();
    let result = destination.arrow().unwrap();
    // arrow::util::pretty::print_batches(&result).unwrap();

    let num_rows = result.into_iter().map(|rb| rb.num_rows()).sum::<usize>();
    println!("# rows: {}", num_rows);
}
