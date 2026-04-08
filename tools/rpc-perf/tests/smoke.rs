use rpc_perf::harness::smoke_send_raw_pass;

#[tokio::test]
async fn smoke_single_send_raw_pass_returns_ok() {
    smoke_send_raw_pass()
        .await
        .expect("smoke_send_raw_pass"); // one sendRawTransaction, HTTP 2xx
}
