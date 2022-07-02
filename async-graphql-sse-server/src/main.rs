mod graphql_query;
mod schema;

use anyhow::Error;
use async_graphql::{
    async_stream::try_stream,
    extensions::Tracing,
    futures_util::{
        future::{AbortHandle, Abortable},
        StreamExt,
    },
    http::{playground_source, GraphQLPlaygroundConfig},
};

use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use axum::{
    extract::Query,
    http::Method,
    response::{self, sse::Event, IntoResponse, Sse},
    routing::get,
    Extension, Router, Server,
};
use shutdown::Shutdown;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{prelude::*, EnvFilter};

async fn graphql_handler(
    schema: Extension<schema::ExampleSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

async fn graphql_handler_query(
    schema: Extension<schema::ExampleSchema>,
    shutdown: Extension<Shutdown>,
    Query(req): Query<graphql_query::GraphQLQuery>,
) -> axum::response::Response {
    if req.in_query() {
        return GraphQLResponse::from(schema.execute(req).await).into_response();
    }

    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let handle = tokio::spawn(async move {
        shutdown._notified().await;
        abort_handle.abort()
    });

    let mut subscription = schema.execute_stream(req);

    let stream: async_graphql::async_stream::AsyncStream<Result<Event, serde_json::Error>, _> = try_stream! {
        while let Some(response) = subscription.next().await {
            yield Event::default().json_data(response)?;
        }
        handle.abort()
    };

    let stream = Abortable::new(stream, abort_registration);

    Sse::new(stream).into_response()
}

async fn graphql_playground() -> impl IntoResponse {
    response::Html(playground_source(
        GraphQLPlaygroundConfig::new("/").subscription_endpoint("/ws"),
    ))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_init()?;

    let schema = schema::build().extension(Tracing).finish();

    // schema.execute_stream(request)

    let tcp_listener =
        std::net::TcpListener::bind(std::net::SocketAddr::from(([127, 0, 0, 1], 8080)))?;

    {
        let local_addr = tcp_listener.local_addr()?;
        println!(
            "Playground: http://{}:{}/playground",
            local_addr.ip(),
            local_addr.port()
        );
        println!("Open in browser: http://localhost:8080/?query={{firstName}}")
    }

    let (shutdown, fut) = shutdown::new();

    let cors = CorsLayer::new()
        // allow `GET` and `POST` when accessing the resource
        .allow_methods([Method::GET, Method::POST])
        // allow requests from any origin
        .allow_origin(Any);

    let app = Router::new()
        .route("/", get(graphql_handler_query).post(graphql_handler))
        .route("/playground", get(graphql_playground))
        .route("/ws", GraphQLSubscription::new(schema.clone()))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(Extension(schema))
        .layer(Extension(shutdown));

    Server::from_tcp(tcp_listener)?
        .http2_enable_connect_protocol()
        .serve(app.into_make_service())
        .with_graceful_shutdown(fut)
        .await?;

    Ok(())
}

fn tracing_init() -> Result<(), tracing_subscriber::util::TryInitError> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .try_init()
}

mod shutdown {
    use std::sync::Arc;

    use async_graphql::futures_util::Future;
    use tokio::sync::{futures::Notified, Notify};

    pub fn new() -> (Shutdown, impl Future<Output = ()>) {
        use tokio::signal;

        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        let notify = Arc::new(Notify::new());
        let notify_clone = notify.clone();

        let shutdown_future = async move {
            tokio::select! {
                _ = ctrl_c => {},
                _ = terminate => {},
            }
            notify_clone.notify_waiters();
        };

        (Shutdown(notify), shutdown_future)
    }

    #[derive(Clone)]
    pub struct Shutdown(Arc<Notify>);

    impl Shutdown {
        pub fn _notified(&self) -> Notified {
            self.0.notified()
        }
    }
}
