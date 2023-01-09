use std::sync::Arc;
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use http::header::CONTENT_TYPE;
use mime::APPLICATION_JSON;
use apollo_router::graphql::Request;
use apollo_router::graphql::Response;
use apollo_router::services::SubgraphService;
use apollo_router::services::subgraph::Request as SubgraphRequest;
use apollo_router::query_planner::OperationKind;
use serde_json_bytes::{Value, ByteString};
use criterion::async_executor::FuturesExecutor;
use tower::ServiceExt;
use apollo_router::Context;
use tokio::runtime::Runtime;

async fn make_call() {
    let subgraph_service = SubgraphService::new("test", Some(false));
    let resp = subgraph_service
        .clone()
        .oneshot(SubgraphRequest {
            supergraph_request: Arc::new(
                http::Request::builder()
                    .header(CONTENT_TYPE, APPLICATION_JSON.essence_str())
                    .body(Request::builder().query("query").build())
                    .expect("expecting valid request"),
            ),
            subgraph_request: http::Request::builder()
                .header(CONTENT_TYPE, APPLICATION_JSON.essence_str())
                .uri("https://www.example.org/")
                .body(Request::builder().query("query").build())
                .expect("expecting valid request"),
            operation_kind: OperationKind::Query,
            context: Context::new(),
        })
        .await
        .unwrap();

    assert_eq!(resp.response.body().data, Some(Value::String(ByteString::from("test"))));
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function(
        "benchmark a call",
        |b| {
            b.to_async(FuturesExecutor).iter(|| async {
                make_call().await;
            });
        }
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);