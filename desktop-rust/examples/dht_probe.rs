fn main() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let _logging = librqbit::tracing_subscriber_config_utils::init_logging(
            librqbit::tracing_subscriber_config_utils::InitLoggingOptions {
                default_rust_log_value: Some("debug"),
                log_file: None,
                log_file_rust_log: None,
                log_file_json: false,
            },
        )
        .unwrap();
        let root = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
        let session = librqbit::Session::new_with_opts(
            root,
            librqbit::SessionOptions {
                dht: Some(librqbit::DhtSessionConfig {
                    persistence: None,
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let api = librqbit::Api::new(session.clone(), None, None);
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
        let stats = serde_json::to_value(api.api_dht_stats().unwrap()).unwrap();
        println!("DHT stats: {stats}");
        session.stop().await;
        assert!(
            stats["routing_table_size"].as_u64().unwrap_or(0) > 0,
            "DHT routing table is empty"
        );
    });
}
