use std::sync::Arc;
use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

use http::header::CONTENT_TYPE;
use mime::APPLICATION_JSON;
use apollo_router::graphql::Error;
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
use apollo_router::graphql;
use http::StatusCode;
use std::net::SocketAddr;
use hyper::Body;
use std::convert::Infallible;
use hyper::service::make_service_fn;
use axum::Server;
use http::Uri;
use http::header::HOST;
use tower::service_fn;
use std::str::FromStr;
use std::time;
use hyper::body::Buf;

const PERSISTED_QUERY_KEY: &str = "persistedQuery";
const PERSISTED_QUERY_NOT_FOUND_EXTENSION_CODE: &str = "PERSISTED_QUERY_NOT_FOUND";

async fn make_call(service: &apollo_router::services::SubgraphService) {
    // println!("sending reqst: {:?}", time::SystemTime::now());
    let resp = service
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
                .uri(Uri::from_str(
                    &format!("http://{}", SocketAddr::from_str("127.0.0.1:3333").unwrap())).unwrap()
                )
                .body(Request::builder().query("query").build())
                .expect("expecting valid request"),
            operation_kind: OperationKind::Query,
            context: Context::new(),
        })
        .await
        .unwrap();
    // println!("got response.: {:?}", time::SystemTime::now());
}
async fn emulate_expected_apq_enabled_configuration(socket_addr: SocketAddr) {
    async fn handle(request: http::Request<Body>) -> Result<http::Response<Body>, Infallible> {
        // println!("servr got req: {:?}", time::SystemTime::now());
        let (_, body) = request.into_parts();
        let graphql_request: Result<graphql::Request, &str> = hyper::body::to_bytes(body)
            .await
            .map_err(|_| ())
            .and_then(|bytes| serde_json::from_reader(bytes.reader()).map_err(|_| ()))
            .map_err(|_| "failed to parse the request body as JSON");

        match graphql_request {
            Ok(request) => {
                if !request.extensions.contains_key(PERSISTED_QUERY_KEY) {
                    panic!("persistedQuery expected when configuration has apq_enabled=true")
                }

                // println!("servr sen rsp: {:?}", time::SystemTime::now());
                return Ok(http::Response::builder()
                    .header(CONTENT_TYPE, APPLICATION_JSON.essence_str())
                    .status(StatusCode::OK)
                    .body(
                        serde_json::to_string(&Response {
                            data: Some(Value::String(ByteString::from("test"))),
                            errors: vec![Error::builder()
                                .message("Random message")
                                .extension_code(PERSISTED_QUERY_NOT_FOUND_EXTENSION_CODE)
                                .build()],
                            ..Response::default()
                        })
                            .expect("always valid")
                            .into(),
                    )
                    .unwrap());
            }
            Err(_) => {
                panic!("invalid graphql request recieved")
            }
        }
    }

    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });
    let server = Server::bind(&socket_addr).serve(make_svc);
    server.await.unwrap();
}

fn criterion_benchmark(c: &mut Criterion) {
    let subgraph_service = &SubgraphService::new("test", Some(true));
    c.bench_with_input(
        BenchmarkId::new( "hello", "subgraph service"),
        subgraph_service,
        |b, subgraph_service| {
            b.to_async(FuturesExecutor).iter(|| async {
                // println!("before starts: {:?}", time::SystemTime::now());
                make_call(subgraph_service).await;
            }) }
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);