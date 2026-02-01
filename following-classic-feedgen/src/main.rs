use axum::{
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use rsky_feedgen::handlers::*;
use rsky_feedgen::state::AppState;
use rsky_feedgen::{ReadReplicaConn, WriteDbConn};
use rsky_identity::types::IdentityResolverOpts;
use rsky_identity::IdResolver;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let write_database_url = env::var("DATABASE_URL").unwrap_or_default();
    let read_database_url = env::var("READ_REPLICA_URL").unwrap_or_default();
    let write_pool_size: u32 = env::var("WRITE_POOL_SIZE")
        .unwrap_or(40.to_string())
        .parse()
        .unwrap();
    let read_pool_size: u32 = env::var("READ_POOL_SIZE")
        .unwrap_or(40.to_string())
        .parse()
        .unwrap();

    let subscriber = tracing_subscriber::FmtSubscriber::new();
    tracing::subscriber::set_global_default(subscriber).unwrap();

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
        .route("/.well-known/did.json", get(well_known))
        .route("/cursor", get(get_cursor).put(update_cursor))
        .route("/stats", get(get_usage_stats))
        .route("/visitors", get(get_visitors))
        .with_state(state)
        .layer(cors);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));
    tracing::info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
