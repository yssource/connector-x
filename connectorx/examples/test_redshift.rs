use aws_config;
use aws_sdk_redshiftdata;
use aws_sdk_redshiftdata::model::StatusString;

#[tokio::main]
async fn main() {
    let shared_config = aws_config::load_from_env().await;
    let client = aws_sdk_redshiftdata::Client::new(&shared_config);
    println!("ok!");

    let req = client.execute_statement();
    let resp = req
        .sql("select * from lineitem limit 10")
        .cluster_identifier("redshift-cluster-1")
        .database("dev")
        .db_user("redshift")
        .send()
        .await
        .unwrap();
    println!("resp: {:?}", resp);

    let id = resp.id().unwrap();

    loop {
        let req = client.describe_statement();
        let resp = req.id(id).send().await.unwrap();
        println!("status: {:?}", resp.status());

        let status = resp.status().unwrap();
        if status == &StatusString::Finished {
            break;
        }
    }

    let req = client.get_statement_result();
    let resp = req.id(id).send().await.unwrap();

    // println!("records: {:?}", resp.records);
    println!("metadata: {:?}", resp.column_metadata);
    println!("total # rows: {:?}", resp.total_num_rows);
    println!("next token: {:?}", resp.next_token);

    // let req = client.list_tables();
    // let resp = req
    //     .cluster_identifier("redshift-cluster-1")
    //     .database("dev")
    //     .db_user("redshift")
    //     .send()
    //     .await
    //     .unwrap();
    // println!("Current redshift tables: {:?}", resp.tables);
}
