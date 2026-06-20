use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use feedgen::apis::backfill_worker;
use feedgen::handlers::*;
use feedgen::metrics;
use feedgen::state::AppState;
use feedgen::{ReadReplicaConn, WriteDbConn};
use identity::types::IdentityResolverOpts;
use identity::IdResolver;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let write_database_url = env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");
    let read_database_url = env::var("READ_REPLICA_URL")
        .expect("READ_REPLICA_URL must be set");
    let write_pool_size: u32 = env::var("WRITE_POOL_SIZE")
        .unwrap_or(40.to_string())
        .parse()
        .expect("WRITE_POOL_SIZE must be a valid u32");
    let read_pool_size: u32 = env::var("READ_POOL_SIZE")
        .unwrap_or(40.to_string())
        .parse()
        .expect("READ_POOL_SIZE must be a valid u32");

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let mgr_write = deadpool_diesel::postgres::Manager::new(
        write_database_url,
        deadpool_diesel::Runtime::Tokio1,
    );
    let write_db_pool = deadpool_diesel::postgres::Pool::builder(mgr_write)
        .max_size(write_pool_size as usize)
        .build()
        .unwrap();
    let write_db = WriteDbConn(write_db_pool);

    let mgr_read = deadpool_diesel::postgres::Manager::new(
        read_database_url,
        deadpool_diesel::Runtime::Tokio1,
    );
    let read_db_pool = deadpool_diesel::postgres::Pool::builder(mgr_read)
        .max_size(read_pool_size as usize)
        .build()
        .unwrap();
    let read_db = ReadReplicaConn(read_db_pool);

    let id_resolver = IdResolver::new(IdentityResolverOpts {
        timeout: None,
        plc_url: None,
        did_cache: None,
        backup_nameservers: None,
    });

    let state = AppState {
        read_db,
        write_db,
        id_resolver,
    };

    let enable_backfill = env::var("ENABLE_BACKFILL")
        .unwrap_or("false".to_string())
        == "true";
    if enable_backfill {
        tokio::spawn(backfill_worker(state.clone()));
    } else {
        tracing::info!("Backfill worker disabled (set ENABLE_BACKFILL=true to enable)");
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/xrpc/app.bsky.feed.getFeedSkeleton", get(index))
        .route(
            "/user_feed_preference",
            get(user_config).put(update_user_config),
        )
        .route(
            "/following_preferences",
            get(following_preferences_fetch).put(following_preferences_update),
        )
        .route("/queue/{lex}/create", post(queue_creation))
        .route("/queue/{lex}/delete", post(queue_deletion))
        .route("/xrpc/app.bsky.feed.describeFeedGenerator", get(describe_feed_generator))
        .route("/.well-known/did.json", get(well_known))
        .route("/health", get(health_check))
        .route("/cursor", get(get_cursor).put(update_cursor))
        .route(
            "/janitor/config",
            get(get_janitor_config).put(update_janitor_config),
        )
        .route("/stats", get(get_usage_stats))
        .route("/visitors", get(get_visitors))
        .route("/metrics", get(metrics::metrics_handler))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(metrics::metrics_middleware))
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind TCP listener");
    axum::serve(listener, app)
        .await
        .expect("Server failed");
}
