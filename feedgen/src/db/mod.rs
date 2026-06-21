use crate::models::{FetchedPost, Follow, FollowingPreference, UserFeedPreference};
use crate::{ReadReplicaConn, WriteDbConn};
use chrono::NaiveDateTime;
use diesel::dsl::count;
use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::schema::following_preference::dsl::following_preference;
use crate::schema::user_feed_preference::dsl::user_feed_preference;

/// Retrieves the feed preference for a specific user.
pub fn get_user_config(_did: &str, conn: &mut PgConnection) -> Option<UserFeedPreference> {
    use crate::schema::user_feed_preference::dsl::*;

    let result = user_feed_preference
        .filter(did.eq(_did))
        .limit(1)
        .select(UserFeedPreference::as_select())
        .load(conn)
        .expect("Error querying user feed");

    if !result.is_empty() {
        Some(result[0].clone())
    } else {
        None
    }
}

/// Retrieves the most recent posts fetched for a specific user.
pub fn get_fetched_posts(_did: &str, conn: &mut PgConnection) -> Vec<FetchedPost> {
    use crate::schema::fetched_post::dsl::*;

    fetched_post
        .filter(did.eq(_did))
        .select(FetchedPost::as_select())
        .limit(30)
        .load(conn)
        .expect("Error querying user feed")
}

/// Gets the total count of posts fetched for a specific user.
pub fn get_total_fetches(_did: &str, conn: &mut PgConnection) -> i64 {
    use crate::schema::fetched_post::dsl::*;

    let result: i64 = fetched_post
        .filter(did.eq(_did))
        .select(count(uri))
        .first(conn)
        .unwrap();

    result
}

/// Deletes all records of fetched posts for a specific user.
pub fn invalidate_all_fetched_posts(_did: &str, conn: &mut PgConnection) {
    use crate::schema::fetched_post::did;
    use crate::schema::fetched_post::dsl::fetched_post;

    match diesel::delete(fetched_post.filter(did.eq(_did))).execute(conn) {
        Ok(_count) => {}
        Err(e) => {
            tracing::error!("{}", e.to_string())
        }
    }
}

/// Deletes specific fetched post records for a user.
pub fn invalidate_fetched_posts(_did: &str, uri_list: Vec<String>, conn: &mut PgConnection) {
    use crate::schema::fetched_post::did;
    use crate::schema::fetched_post::dsl::fetched_post;
    use crate::schema::fetched_post::uri;

    match diesel::delete(
        fetched_post
            .filter(did.eq(_did))
            .filter(uri.eq_any(uri_list)),
    )
    .execute(conn)
    {
        Ok(_count) => {}
        Err(e) => {
            tracing::error!("{}", e.to_string())
        }
    }
}

/// Inserts new records of posts fetched for a user.
pub fn insert_fetched_posts(fetched_posts: Vec<FetchedPost>, conn: &mut PgConnection) {
    use crate::schema::fetched_post::dsl as FetchedPostSchema;
    let mut fetched_posts_to_insert = Vec::new();
    for fetched_post in fetched_posts.iter() {
        let new_seen_post = (
            FetchedPostSchema::did.eq(fetched_post.did.clone()),
            FetchedPostSchema::uri.eq(fetched_post.uri.clone()),
        );
        fetched_posts_to_insert.push(new_seen_post);
    }

    diesel::insert_into(crate::schema::fetched_post::dsl::fetched_post)
        .values(&fetched_posts_to_insert)
        .execute(conn)
        .expect("Error inserting fetched_post records");
}

/// Records posts as "seen" by a user.
pub fn insert_seen_posts(fetched_posts: Vec<FetchedPost>, conn: &mut PgConnection) {
    use crate::schema::seen_post::dsl as SeenPostSchema;
    let mut seen_posts_to_insert = Vec::new();
    for fetched_post in fetched_posts.iter() {
        let new_seen_post = (
            SeenPostSchema::did.eq(fetched_post.did.clone()),
            SeenPostSchema::uri.eq(fetched_post.uri.clone()),
        );
        seen_posts_to_insert.push(new_seen_post);
    }

    diesel::insert_into(crate::schema::seen_post::dsl::seen_post)
        .values(&seen_posts_to_insert)
        .execute(conn)
        .expect("Error inserting seen_post records");
}

/// Retrieves a list of DIDs that the given user follows.
pub async fn get_saved_follows(did: String, connection: &ReadReplicaConn) -> Vec<String> {
    use crate::schema::follow::dsl::*;
    let mut follows = Vec::new();

    let result: Vec<Follow> = connection
        .0
        .get()
        .await
        .expect("Failed to get database connection")
        .interact(move |conn: &mut PgConnection| {
            follow
                .filter(author.eq(did))
                .select(Follow::as_select())
                .load(conn)
                .expect("Error querying follows")
        })
        .await
        .expect("Database interaction failed");

    for follow2 in result.iter() {
        follows.push(follow2.subject.clone());
    }
    follows
}

/// Retrieves the following preferences (DIDs of followed users) for a given user.
pub async fn get_following_preferences(did: String, connection: &ReadReplicaConn) -> Vec<String> {
    use crate::schema::follow::dsl::*;
    let mut follows = Vec::new();

    let result: Vec<Follow> = connection
        .0
        .get()
        .await
        .expect("Failed to get database connection")
        .interact(move |conn: &mut PgConnection| {
            follow
                .filter(author.eq(did))
                .select(Follow::as_select())
                .load(conn)
                .expect("Error querying follows")
        })
        .await
        .expect("Database interaction failed");

    for follow2 in result.iter() {
        follows.push(follow2.subject.clone());
    }
    follows
}

/// Checks if a user's follows have already been indexed.
pub fn user_follows_indexed(did: &str, conn: &mut PgConnection) -> bool {
    use crate::schema::follow::dsl::*;

    let follows = follow
        .filter(author.eq(did))
        .limit(1)
        .select(Follow::as_select())
        .load(conn)
        .expect("Error querying follows");

    !follows.is_empty()
}

/// Checks if a user is a known user of the feed.
pub fn is_known_user(did: &str, conn: &mut PgConnection) -> bool {
    use crate::schema::user_feed_preference::dsl::{did as user_did, user_feed_preference};
    use crate::schema::visitor::dsl::{did as visitor_did, visitor};

    let pref_exists = user_feed_preference
        .filter(user_did.eq(did))
        .limit(1)
        .load::<crate::models::UserFeedPreference>(conn)
        .map(|r| !r.is_empty())
        .unwrap_or(false);

    if pref_exists {
        return true;
    }

    let visitor_exists = visitor
        .filter(visitor_did.eq(did))
        .limit(1)
        .load::<crate::models::Visitor>(conn)
        .map(|r| !r.is_empty())
        .unwrap_or(false);

    visitor_exists
}

/// Creates a new user feed preference record in the database.
///
/// # Errors
///
/// Returns an error message if the database connection or interaction fails.
pub async fn user_config_creation(
    config: UserFeedPreference,
    connection: WriteDbConn,
) -> anyhow::Result<()> {
    use crate::schema::user_feed_preference::dsl as UserFeedSchema;

    let new_config = (
        UserFeedSchema::did.eq(config.did),
        UserFeedSchema::reply_filter_likes.eq(config.reply_filter_likes),
        UserFeedSchema::reply_filter_followed_only.eq(config.reply_filter_followed_only),
        UserFeedSchema::show_quote_posts.eq(config.show_quote_posts),
        UserFeedSchema::show_replies.eq(config.show_replies),
        UserFeedSchema::show_reposts.eq(config.show_reposts),
    );
    connection
        .0
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?
        .interact(move |conn: &mut PgConnection| {
            diesel::insert_into(UserFeedSchema::user_feed_preference)
                .values(&new_config)
                .execute(conn)
                .expect("Error inserting member records");
        })
        .await
        .map_err(|e| anyhow::anyhow!("Database interaction failed: {}", e))?;
    Ok(())
}

/// Retrieves following preferences for a given user DID.
pub fn get_following_preferences2(
    _did: String,
    conn: &mut PgConnection,
) -> Vec<FollowingPreference> {
    use crate::schema::following_preference::dsl::author;
    use crate::schema::following_preference::dsl::following_preference as FollowingPrefSchema;
    FollowingPrefSchema
        .filter(author.eq(_did))
        .select(FollowingPreference::as_select())
        .load(conn)
        .unwrap()
}

/// Fetches following preferences for a user asynchronously from the database.
pub async fn following_pref_fetch(
    _did: String,
    connection: WriteDbConn,
) -> Vec<FollowingPreference> {
    use crate::schema::following_preference::dsl::author;
    use crate::schema::following_preference::dsl::following_preference as FollowingPrefSchema;

    connection
        .0
        .get()
        .await
        .expect("Failed to get database connection")
        .interact(move |conn: &mut PgConnection| {
            FollowingPrefSchema
                .filter(author.eq(_did))
                .select(FollowingPreference::as_select())
                .load(conn)
                .unwrap()
        })
        .await
        .expect("Database interaction failed")
}

/// Updates or inserts a following preference record.
///
/// # Errors
///
/// Returns an error message if the database connection or interaction fails.
pub async fn following_pref_update(
    _following_preference: FollowingPreference,
    connection: WriteDbConn,
) -> anyhow::Result<()> {
    connection
        .0
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?
        .interact(move |conn: &mut PgConnection| {
            use crate::schema::following_preference::author;
            use crate::schema::following_preference::did;
            diesel::insert_into(following_preference)
                .values(&_following_preference)
                .on_conflict((author, did))
                .do_update()
                .set(&_following_preference)
                .execute(conn)
                .expect("Error update config records");
        })
        .await
        .map_err(|e| anyhow::anyhow!("Database interaction failed: {}", e))
}

/// Fetches user feed preferences for a given user DID asynchronously.
pub async fn user_config_fetch(_did: String, connection: WriteDbConn) -> Vec<UserFeedPreference> {
    use crate::schema::user_feed_preference::dsl::did;
    use crate::schema::user_feed_preference::dsl::user_feed_preference as UserFeedSchema;

    connection
        .0
        .get()
        .await
        .expect("Failed to get database connection")
        .interact(move |conn: &mut PgConnection| {
            UserFeedSchema
                .filter(did.eq(_did))
                .select(UserFeedPreference::as_select())
                .load(conn)
                .unwrap()
        })
        .await
        .expect("Database interaction failed")
}

/// Upserts the user feed preference in the database (inserts or updates by DID).
///
/// # Errors
///
/// Returns an error message if the database connection or interaction fails.
pub async fn user_config_update(
    config: UserFeedPreference,
    connection: WriteDbConn,
) -> anyhow::Result<()> {
    use crate::schema::user_feed_preference::dsl::did;

    connection
        .0
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?
        .interact(move |conn: &mut PgConnection| {
            diesel::insert_into(user_feed_preference)
                .values(&config)
                .on_conflict(did)
                .do_update()
                .set(&config)
                .execute(conn)
                .expect("Error upserting user feed preference");
        })
        .await
        .map_err(|e| anyhow::anyhow!("Database interaction failed: {}", e))
}

/**
 * Inserts a list of follows into the database.
 *
 * @param follows - The list of Follow records to insert.
 * @param conn - The database connection.
 */
pub fn insert_follows(follows: Vec<Follow>, conn: &mut PgConnection) {
    use crate::schema::follow::dsl as FollowSchema;
    let mut follows_to_insert = Vec::new();
    for follow in follows.iter() {
        let new_follow = (
            FollowSchema::uri.eq(follow.uri.clone()),
            FollowSchema::cid.eq(follow.cid.clone()),
            FollowSchema::author.eq(follow.author.clone()),
            FollowSchema::subject.eq(follow.subject.clone()),
            FollowSchema::createdAt.eq(follow.created_at.clone()),
            FollowSchema::indexedAt.eq(follow.indexed_at.clone()),
            FollowSchema::prev.eq(follow.prev.clone()),
            FollowSchema::sequence.eq(follow.sequence),
        );
        follows_to_insert.push(new_follow);
    }

    diesel::insert_into(crate::schema::follow::dsl::follow)
        .values(&follows_to_insert)
        .on_conflict(FollowSchema::uri)
        .do_nothing()
        .execute(conn)
        .expect("Error inserting follow records");
}

/// Gets the timestamp of the last full PDS follow refresh for a user.
pub fn get_follow_last_refreshed(did_param: &str, conn: &mut PgConnection) -> Option<NaiveDateTime> {
    use crate::schema::follow_refresh::dsl::*;

    follow_refresh
        .filter(did.eq(did_param))
        .select(refreshed_at)
        .first(conn)
        .ok()
}

/// Upserts a follow refresh timestamp for a user, marking when their follows
/// were last fully synchronized from their PDS.
pub fn upsert_follow_refresh(did_param: &str, conn: &mut PgConnection) {
    use crate::schema::follow_refresh::dsl::*;

    let now = chrono::Utc::now().naive_utc();
    diesel::insert_into(crate::schema::follow_refresh::dsl::follow_refresh)
        .values((did.eq(did_param), refreshed_at.eq(now)))
        .on_conflict(did)
        .do_update()
        .set(refreshed_at.eq(now))
        .execute(conn)
        .expect("Error upserting follow refresh timestamp");
}

/// Deletes posts from the database identified by their URIs.
pub fn delete_posts_by_uri(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(
        crate::schema::post::dsl::post.filter(crate::schema::post::dsl::uri.eq_any(delete_rows)),
    )
    .execute(conn)
    .expect("Error deleting post records");
}

/// Deletes posts from the database identified by their URIs (aliased from delete_posts_by_uri logic).
pub fn delete_posts_by_rkey(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(
        crate::schema::post::dsl::post.filter(crate::schema::post::dsl::uri.eq_any(delete_rows)),
    )
    .execute(conn)
    .expect("Error deleting post records");
}

/// Deletes reposts from the database identified by their URIs.
pub fn delete_reposts_by_uri(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(
        crate::schema::repost::dsl::repost
            .filter(crate::schema::repost::dsl::uri.eq_any(delete_rows)),
    )
    .execute(conn)
    .expect("Error deleting repost records");
}

/// Deletes follow records from the database identified by their URIs.
pub fn delete_follows_by_uri(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(
        crate::schema::follow::dsl::follow
            .filter(crate::schema::follow::dsl::uri.eq_any(delete_rows)),
    )
    .execute(conn)
    .expect("Error deleting follow records");
}

/// Deletes like records from the database identified by their URIs.
pub fn delete_likes_by_uri(delete_rows: Vec<String>, conn: &mut PgConnection) {
    diesel::delete(
        crate::schema::like::dsl::like.filter(crate::schema::like::dsl::uri.eq_any(delete_rows)),
    )
    .execute(conn)
    .expect("Error deleting like records");
}

/// Represents the state of a cursor update for a service.
pub struct CursorUpdateState {
    pub service: String,
    pub cursor: i64,
}

/// Updates or inserts the cursor state for a given service in the database.
pub fn update_cursor_db(update_state: CursorUpdateState, conn: &mut PgConnection) {
    use crate::schema::sub_state::dsl::*;

    let new_update_state = (
        service.eq(update_state.service),
        cursor.eq(update_state.cursor),
    );

    diesel::insert_into(sub_state)
        .values(&new_update_state)
        .on_conflict(service)
        .do_update()
        .set(cursor.eq(update_state.cursor))
        .execute(conn)
        .expect("Error updating cursor records");
}
