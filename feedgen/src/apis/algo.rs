use crate::agent::{get_agent, get_follows};
use crate::db::*;
use crate::models::domain::post_result::PostResultReason;
use crate::models::*;
use crate::{DbObject, ReadReplicaConn};
use chrono::{DateTime, NaiveDateTime};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_query;
use std::collections::HashSet;
use std::fmt::Write;

pub const SHOW_REPLIES_FOR_FOLLOWING_ONLY: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5fyouhr7z26";
pub const DONT_SHOW_REPOSTS: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5fyptviqu2f";
pub const DONT_SHOW_QUOTEPOSTS: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5fyqh7fbr26";
pub const NUMBER_OF_LIKES: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5fyvglu472z";
pub const RESET_PREF: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l5g74kd7my26";
pub const HIDE_SEEN_POSTS: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3l7edu2ufdp2u";
pub const HIDE_NOT_ALT_TEXT_POSTS: &str =
    "at://did:plc:cimwguwdlh2i2mebdqczgcyl/app.bsky.feed.post/3lbsxswsgus2f";
pub const CURSOR_TIMESTAMP_TOLERANCE_NS: u32 = 230 * 1_000_000;
/// Number of hours after which a user's cached follows are considered stale
/// and will be reconciled against their PDS.
pub const FOLLOW_REFRESH_HOURS: i64 = 24;
pub const USER_PREF_OPTIONS: [&str; 6] = [
    RESET_PREF,
    DONT_SHOW_QUOTEPOSTS,
    DONT_SHOW_REPOSTS,
    SHOW_REPLIES_FOR_FOLLOWING_ONLY,
    HIDE_SEEN_POSTS,
    HIDE_NOT_ALT_TEXT_POSTS,
];

/// Moves fetched posts to seen posts and invalidates them from the fetched posts table for a user.
#[tracing::instrument(skip(did, conn))]
pub fn update_seen_posts(did: &str, conn: &mut PgConnection) {
    tracing::info!("Updating seen posts for {}", did);
    let fetched_posts = get_fetched_posts(did, conn);
    insert_seen_posts(fetched_posts.clone(), conn);

    let mut uri_list: Vec<String> = Vec::new();
    for fetched_post in fetched_posts {
        uri_list.push(fetched_post.uri);
    }

    invalidate_fetched_posts(did, uri_list, conn);
}

/// Generates a SQL query string for fetching posts with media from followed users.
#[tracing::instrument]
pub fn post_media_query_str(following: &str) -> String {
    format!(
        "select uri,
       \"indexedAt\",
       cid,
       \"replyParent\",
       \"replyRoot\",
       prev,
       \"sequence\",
       \"text\",
       lang,
       author,
       \"externalUri\",
       \"externalTitle\",
       \"externalDescription\",
       \"externalThumb\",
       null as \"quoteCid\",
       null as \"quoteUri\",
       \"media\",
\"alt\"
from (select p1.uri,
             p1.cid,
             p1.\"replyParent\",
             p1.\"replyRoot\",
             p1.prev,
             p1.\"sequence\",
             p1.\"text\",
             p1.lang,
             p1.author,
             p1.\"externalUri\",
             p1.\"externalTitle\",
             p1.\"externalDescription\",
             p1.\"externalThumb\",
             p1.\"quoteCid\",
             p1.\"quoteUri\",
             p1.\"indexedAt\",
p1.\"media\",
p1.\"alt\"
      from post p1
      where p1.author in ({authors})
        and (p1.media is true)
      group by p1.uri, p1.cid, p1.author) as x
where true=true",
        authors = following,
    )
}

/// Generates a SQL query string for fetching posts based on user preferences and following list.
#[tracing::instrument]
pub fn post_query_str(
    hide_seen_posts: bool,
    hide_no_alt_text: bool,
    following: &str,
    user_config: &UserFeedPreference,
    did: &str,
) -> String {
    let did = sanitize_did(did);
    if hide_seen_posts {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       \"replyParent\",
       \"replyRoot\",
       prev,
       \"sequence\",
       \"text\",
       lang,
       author,
       \"externalUri\",
       \"externalTitle\",
       \"externalDescription\",
       \"externalThumb\",
       null as \"quoteCid\",
       null as \"quoteUri\",
       \"media\",
\"alt\"
from (select p1.uri,
             p1.cid,
             p1.\"replyParent\",
             p1.\"replyRoot\",
             p1.prev,
             p1.\"sequence\",
             p1.\"text\",
             p1.lang,
             p1.author,
             p1.\"externalUri\",
             p1.\"externalTitle\",
             p1.\"externalDescription\",
             p1.\"externalThumb\",
             p1.\"quoteCid\",
             p1.\"quoteUri\",
             (select count(*) from public.like m where p1.uri = m.\"subjectUri\" and m.author in ({authors})) as likeCount,
             p1.\"indexedAt\",
p1.\"media\",
p1.\"alt\"
      from post p1
               left join post p2
                         on p1.\"replyParent\" = p2.uri
               left join \"like\" l1
                    on l1.\"subjectUri\" = p1.uri and l1.author in ({authors})
               LEFT OUTER JOIN seen_post s1 ON s1.did = '{did}' and s1.uri = p1.uri
      where p1.author in ({authors})
        and ({quotes_included} or p1.\"quoteUri\" is null)
        and ({hide_no_alt_text}=false or p1.\"media\" is false or p1.\"alt\" is not null)
        and ({replies_included} or p1.\"replyParent\" is null)
        and s1.id is null
        and ({all_replies} or p2.author is null or (p2.author in ({authors})))
      group by p1.uri, p1.cid, p1.author) as x
where (\"replyParent\" is null or likeCount >= {like_threshold})",
            authors = following,
            quotes_included = user_config.show_quote_posts,
            replies_included = user_config.show_replies,
            all_replies = !user_config.reply_filter_followed_only,
            like_threshold = user_config.reply_filter_likes,
            did = did
        )
    } else {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       \"replyParent\",
       \"replyRoot\",
       prev,
       \"sequence\",
       \"text\",
       lang,
       author,
       \"externalUri\",
       \"externalTitle\",
       \"externalDescription\",
       \"externalThumb\",
       null as \"quoteCid\",
       null as \"quoteUri\",
        \"media\",
        alt
from (select p1.uri,
             p1.cid,
             p1.\"replyParent\",
             p1.\"replyRoot\",
             p1.prev,
             p1.\"sequence\",
             p1.\"text\",
             p1.lang,
             p1.author,
             p1.\"externalUri\",
             p1.\"externalTitle\",
             p1.\"externalDescription\",
             p1.\"externalThumb\",
             p1.\"quoteCid\",
             p1.\"quoteUri\",
             (select count(*) from public.like m where p1.uri = m.\"subjectUri\" and m.author in ({authors})) as likeCount,
             p1.\"indexedAt\",
            p1.\"media\",
p1.\"alt\"
      from post p1
               left join post p2
                         on p1.\"replyParent\" = p2.uri
      where p1.author in ({authors})
        and ({quotes_included} or p1.\"quoteUri\" is null)
        and ({hide_no_alt_text}=false or p1.\"media\" is false or p1.\"alt\" is not null)
        and ({replies_included} or p1.\"replyParent\" is null)
        and ({all_replies} or p2.author is null or (p2.author in ({authors})))
      group by p1.uri, p1.cid, p1.author) as x
where (\"replyParent\" is null or likeCount >= {like_threshold})",
            authors = following,
            quotes_included = user_config.show_quote_posts,
            replies_included = user_config.show_replies,
            all_replies = !user_config.reply_filter_followed_only,
            like_threshold = user_config.reply_filter_likes
        )
    }
}

/// Generates a SQL query string for fetching reposts based on the following list and seen status.
pub fn repost_query_str(
    hide_seen_posts: bool,
    following_reposts_string: &str,
    did: &str,
) -> String {
    let did = sanitize_did(did);
    if hide_seen_posts {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       null   as \"replyParent\",
       null   as \"replyRoot\",
       prev,
       \"sequence\",
       null   as \"text\",
       null   as lang,
       author as author,
       null   as \"externalUri\",
       null   as \"externalTitle\",
       null   as \"externalDescription\",
       null   as \"externalThumb\",
       \"subjectCid\"   as \"quoteCid\",
       \"subjectUri\"   as \"quoteUri\",
       false   as \"media\",
       null as alt
from (select r1.uri as uri,
             r1.cid as cid,
             r1.\"subjectUri\" as \"subjectUri\",
             r1.\"subjectCid\" as \"subjectCid\",
             r1.author,
             r1.\"indexedAt\",
             r1.prev,
             r1.\"sequence\"
      from repost r1
          LEFT OUTER JOIN seen_post s1 ON s1.did = '{did}' and s1.uri = r1.uri
      where r1.author in ({authors}) and s1.id is null) as x",
            authors = following_reposts_string,
            did = did
        )
    } else {
        format!(
            "select uri,
       \"indexedAt\",
       cid,
       null   as \"replyParent\",
       null   as \"replyRoot\",
       prev,
       \"sequence\",
       null   as \"text\",
       null   as lang,
       author as author,
       null   as \"externalUri\",
       null   as \"externalTitle\",
       null   as \"externalDescription\",
       null   as \"externalThumb\",
       \"subjectCid\"   as \"quoteCid\",
       \"subjectUri\"   as \"quoteUri\",
       false   as \"media\",
       null as alt
from (select r1.uri as uri,
             r1.cid as cid,
             r1.\"subjectUri\" as \"subjectUri\",
             r1.\"subjectCid\" as \"subjectCid\",
             r1.author,
             r1.\"indexedAt\",
             r1.prev,
             r1.\"sequence\"
      from repost r1
      where r1.author in ({authors})
      ) as x",
            authors = following_reposts_string
        )
    }
}

/// Retrieves a paginated feed of posts for a user based on their follows and preferences.
///
/// # Errors
///
/// Returns a `ValidationErrorMessageResponse` if database interactions fail.
#[tracing::instrument(skip(connection))]
pub async fn get_posts_by_user_feed(
    did: String,
    _limit: Option<i64>,
    params_cursor: Option<&str>,
    connection: ReadReplicaConn,
) -> Result<AlgoResponse, ValidationErrorMessageResponse> {
    let limit: i64 = _limit.unwrap_or(30);
    let params_cursor = params_cursor.map(|params_cursor| params_cursor.to_string());

    let follow_dids = refresh_follows_if_needed(did.clone(), &connection).await?;

    if follow_dids.is_empty() {
        return Ok(AlgoResponse {
            cursor: None,
            feed: Vec::new(),
        });
    }

    let following = format_did_list(&follow_dids);

    let result = connection
        .0
        .get()
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Failed to get database connection".to_string()),
        })?
        .interact(move |conn: &mut PgConnection| {
            let user_config = get_or_create_user_config(did.clone(), conn);

            handle_seen_posts_invalidation(did.clone(), &user_config, limit, &params_cursor, conn);

            let following_reposts_string =
                get_following_reposts_string(did.clone(), &follow_dids, conn);

            let mut query_str = post_query_str(
                user_config.hide_seen_posts,
                user_config.hide_no_alt_text,
                following.as_str(),
                &user_config,
                did.as_str(),
            );
            let mut repost_query_str = repost_query_str(
                user_config.hide_seen_posts,
                following_reposts_string.as_str(),
                did.as_str(),
            );

            if let Some(cursor_str) = params_cursor {
                let (query_update, repost_update) =
                    apply_cursor_to_queries(&cursor_str, &query_str, &repost_query_str)?;
                query_str = query_update;
                repost_query_str = repost_update;
            }
            let order_str = format!(" ORDER BY \"indexedAt\" DESC, cid DESC LIMIT {} ", limit);
            let query_str = format!("{}{};", &query_str, &order_str);
            let repost_query_str = format!("{}{};", &repost_query_str, &order_str);

            let mut results = sql_query(query_str)
                .load::<Post>(conn)
                .unwrap_or_default();

            if user_config.show_reposts {
                let mut repost_results = sql_query(repost_query_str)
                    .load::<crate::models::Post>(conn)
                    .unwrap_or_default();
                results.append(&mut repost_results);
                results.sort_by(|a, b| {
                    let fmt = "%+";
                    let a_date = NaiveDateTime::parse_from_str(a.indexed_at.as_str(), fmt).unwrap();
                    let b_date = NaiveDateTime::parse_from_str(b.indexed_at.as_str(), fmt).unwrap();
                    b_date
                        .and_utc()
                        .timestamp()
                        .cmp(&a_date.and_utc().timestamp())
                });
            }

            let slice_size = std::cmp::min(limit as usize, results.len());
            let final_result: Vec<Post> = results.into_iter().take(slice_size).collect();

            let cursor = generate_cursor_from_last_post(final_result.last());

            let post_results: Vec<PostResult> = final_result
                .iter()
                .map(|result| {
                    if let Some(quote_uri) = &result.quote_uri {
                        PostResult {
                            post: quote_uri.clone(),
                            reason: Some(PostResultReason {
                                reason_type: "app.bsky.feed.defs#skeletonReasonRepost".to_string(),
                                repost_uri: result.uri.clone(),
                            }),
                        }
                    } else {
                        PostResult {
                            post: result.uri.clone(),
                            reason: None,
                        }
                    }
                })
                .collect();

            if user_config.hide_seen_posts && limit != 1 {
                track_fetched_posts(did.clone(), &final_result, conn);
            }

            Ok(AlgoResponse {
                cursor,
                feed: post_results,
            })
        })
        .await
        .map_err(|e| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some(format!("Database interaction failed: {}", e)),
        })?;
    result
}

/// Fetches follows from the remote service if they are not already cached in the database.
/// If follows exist but haven't been refreshed within `FOLLOW_REFRESH_HOURS`,
/// triggers a reconciliation against the user's PDS to pick up missed changes.
#[tracing::instrument(skip(connection))]
pub async fn refresh_follows_if_needed(
    did: String,
    connection: &ReadReplicaConn,
) -> Result<Vec<String>, ValidationErrorMessageResponse> {
    let mut follow_dids = get_saved_follows(did.clone(), connection).await;
    
    if follow_dids.is_empty() {
        // Bootstrap — no cached follows, fetch fresh from PDS
        tracing::info!("Creating followers and following for {}", did);
        if let Ok(agent) = get_agent().await {
            let follows = get_follows(&agent, did.as_ref()).await;
            // TODO: Implement get_followers and call it here.
            // let followers = get_followers(&agent, did.as_ref()).await;
            
            let conn = connection
                .0
                .get()
                .await
                .map_err(|_| ValidationErrorMessageResponse {
                    code: Some(ErrorCode::ValidationError),
                    message: Some("Failed to get database connection".to_string()),
                })?;
            let did_clone = did.clone();
            conn.interact(move |conn| {
                insert_follows(follows, conn);
                upsert_follow_refresh(&did_clone, conn);
                // insert_follows(followers, conn); // Need a different insert_follows that takes (follower, subject) correctly?
            })
            .await
            .map_err(|_| ValidationErrorMessageResponse {
                code: Some(ErrorCode::ValidationError),
                message: Some("Database interaction failed".to_string()),
            })?;
            follow_dids = get_saved_follows(did.clone(), connection).await;
        }
    } else {
        // Follows exist in cache — check if they need reconciliation
        let did_for_check = did.clone();
        let needs_refresh = connection
            .0
            .get()
            .await
            .map_err(|_| ValidationErrorMessageResponse {
                code: Some(ErrorCode::ValidationError),
                message: Some("Failed to get database connection".to_string()),
            })?
            .interact(move |conn: &mut PgConnection| {
                let last_refreshed = get_follow_last_refreshed(&did_for_check, conn);
                match last_refreshed {
                    None => {
                        tracing::info!("Follows for {} have never been fully refreshed from PDS, triggering reconciliation", did_for_check);
                        true
                    }
                    Some(ts) => {
                        let cutoff =
                            chrono::Utc::now().naive_utc() - chrono::Duration::hours(FOLLOW_REFRESH_HOURS);
                        if ts < cutoff {
                            tracing::info!(
                                "Follows for {} are stale (last refreshed: {}), triggering reconciliation",
                                did_for_check,
                                ts
                            );
                            true
                        } else {
                            false
                        }
                    }
                }
            })
            .await
            .map_err(|_| ValidationErrorMessageResponse {
                code: Some(ErrorCode::ValidationError),
                message: Some("Database interaction failed".to_string()),
            })?;

        if needs_refresh {
            reconcile_follows(did.clone(), connection).await?;
            follow_dids = get_saved_follows(did, connection).await;
        }
    }

    Ok(follow_dids)
}

/// Reconciles the cached follows for a user against their PDS.
///
/// Compares the local `follow` table against the user's PDS follow records
/// and repairs any inconsistencies:
/// - Deletes follows that exist locally but no longer exist on the PDS
/// - Inserts follows from the PDS that are missing locally
/// - Updates the `follow_refresh` timestamp
#[tracing::instrument(skip(connection))]
async fn reconcile_follows(
    did: String,
    connection: &ReadReplicaConn,
) -> Result<(), ValidationErrorMessageResponse> {
    tracing::info!("Reconciling follows for {} against PDS", did);

    let agent = get_agent().await.map_err(|_| ValidationErrorMessageResponse {
        code: Some(ErrorCode::ValidationError),
        message: Some("Failed to create agent for follow reconciliation".to_string()),
    })?;
    let pds_follows = get_follows(&agent, &did).await;

    let conn = connection
        .0
        .get()
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Failed to get database connection for follow reconciliation".to_string()),
        })?;

    conn.interact(move |conn: &mut PgConnection| {
        use crate::schema::follow::dsl::*;

        // Load current DB follows for this user
        let db_follows: Vec<Follow> = follow
            .filter(author.eq(&did))
            .select(Follow::as_select())
            .load(conn)
            .expect("Error loading follows for reconciliation");

        // Build a set of URIs that exist on the PDS
        let pds_uri_set: HashSet<String> = pds_follows.iter().map(|f| f.uri.clone()).collect();

        // Delete follows that are in the DB but no longer on the PDS
        let uris_to_delete: Vec<String> = db_follows
            .iter()
            .filter(|f| !pds_uri_set.contains(&f.uri))
            .map(|f| f.uri.clone())
            .collect();

        if !uris_to_delete.is_empty() {
            let deleted_count = diesel::delete(follow.filter(uri.eq_any(&uris_to_delete)))
                .execute(conn)
                .expect("Error deleting stale follows during reconciliation");
            tracing::info!("Deleted {} stale follows for {}", deleted_count, &did);
        }

        // Build a set of URIs already in the DB
        let db_uri_set: HashSet<String> = db_follows.iter().map(|f| f.uri.clone()).collect();

        // Insert follows from PDS that aren't yet in the DB
        let new_follows: Vec<Follow> = pds_follows
            .into_iter()
            .filter(|f| !db_uri_set.contains(&f.uri))
            .collect();

        if !new_follows.is_empty() {
            let new_count = new_follows.len();
            insert_follows(new_follows, conn);
            tracing::info!("Inserted {} new follows for {} during reconciliation", new_count, &did);
        }

        // Update the refresh timestamp
        upsert_follow_refresh(&did, conn);
        tracing::info!("Follow reconciliation complete for {}", &did);
    })
    .await
    .map_err(|_| ValidationErrorMessageResponse {
        code: Some(ErrorCode::ValidationError),
        message: Some("Database interaction failed during follow reconciliation".to_string()),
    })?;

    Ok(())
}

/// Sanitizes a DID string for safe SQL interpolation by escaping single quotes.
/// This prevents SQL injection via maliciously crafted DIDs.
fn sanitize_did(did: &str) -> String {
    did.replace('\'', "''")
}

/// Formats a list of DIDs into a comma-separated string of quoted DIDs for SQL `IN` clauses.
fn format_did_list(dids: &[String]) -> String {
    dids.iter()
        .map(|did| format!("'{}'", sanitize_did(did)))
        .collect::<Vec<_>>()
        .join(",")
}

/// Retrieves user feed configuration or returns default settings if none exist.
fn get_or_create_user_config(did: String, conn: &mut PgConnection) -> UserFeedPreference {
    get_user_config(did.as_str(), conn).unwrap_or(UserFeedPreference {
        did,
        show_replies: true,
        reply_filter_likes: 0,
        reply_filter_followed_only: false,
        show_reposts: true,
        show_quote_posts: true,
        hide_seen_posts: false,
        hide_no_alt_text: false,
    })
}

/// Handles invalidation or updating of seen posts based on user preferences and pagination status.
pub fn handle_seen_posts_invalidation(
    did: String,
    user_config: &UserFeedPreference,
    limit: i64,
    params_cursor: &Option<String>,
    conn: &mut PgConnection,
) {
    if user_config.hide_seen_posts && limit != 1 {
        match params_cursor {
            None => invalidate_all_fetched_posts(did.as_str(), conn),
            Some(_) => {
                if get_total_fetches(did.as_str(), conn) >= 60 {
                    update_seen_posts(did.as_str(), conn)
                }
            }
        }
    }
}

/// Filters the list of followed DIDs to only those whose reposts should be shown.
pub fn get_following_reposts_string(
    did: String,
    follow_dids: &[String],
    conn: &mut PgConnection,
) -> String {
    let following_preferences = get_following_preferences2(did, conn);
    follow_dids
        .iter()
        .filter(|&did| {
            !following_preferences
                .iter()
                .any(|p| &p.did == did && !p.show_reposts)
        })
        .map(|did| format!("'{}'", sanitize_did(did)))
        .collect::<Vec<_>>()
        .join(",")
}

/// Appends cursor conditions to the SQL query strings.
pub fn apply_cursor_to_queries(
    cursor_str: &str,
    query_str: &str,
    repost_query_str: &str,
) -> Result<(String, String), ValidationErrorMessageResponse> {
    let v = cursor_str
        .split("::")
        .take(2)
        .map(String::from)
        .collect::<Vec<_>>();
    if let [indexed_at_c, _cid_c] = &v[..] {
        if let Ok(timestamp) = indexed_at_c.parse::<i64>() {
            let nanoseconds = CURSOR_TIMESTAMP_TOLERANCE_NS;
            let datetime = DateTime::from_timestamp(timestamp / 1000, nanoseconds).unwrap();
            let mut timestr = String::new();
            if write!(timestr, "{}", datetime.format("%+")).is_ok() {
                let cursor_filter_str = format!(" AND (\"indexedAt\" < '{0}')", timestr);
                let cursor_repost_filter_str = format!(" WHERE (\"indexedAt\" < '{0}')", timestr);
                return Ok((
                    format!("{}{}", query_str, cursor_filter_str),
                    format!("{}{}", repost_query_str, cursor_repost_filter_str),
                ));
            }
        }
    }
    Err(ValidationErrorMessageResponse {
        code: Some(ErrorCode::ValidationError),
        message: Some("malformed cursor".into()),
    })
}

/// Appends a cursor condition to a single SQL query string.
fn apply_cursor_to_single_query(
    cursor_str: &str,
    query_str: &mut String,
) -> Result<(), ValidationErrorMessageResponse> {
    let v = cursor_str
        .split("::")
        .take(2)
        .map(String::from)
        .collect::<Vec<_>>();
    if let [indexed_at_c, _cid_c] = &v[..] {
        if let Ok(timestamp) = indexed_at_c.parse::<i64>() {
            let nanoseconds = CURSOR_TIMESTAMP_TOLERANCE_NS;
            let datetime = DateTime::from_timestamp(timestamp / 1000, nanoseconds).unwrap();
            let mut timestr = String::new();
            match write!(timestr, "{}", datetime.format("%+")) {
                Ok(_) => {
                    let cursor_filter_str =
                        format!(" AND (\"indexedAt\" < '{0}')", timestr);
                    query_str.push_str(&cursor_filter_str);
                }
                Err(error) => tracing::error!("Error formatting: {error:?}"),
            }
            return Ok(());
        }
    }
    Err(ValidationErrorMessageResponse {
        code: Some(ErrorCode::ValidationError),
        message: Some("malformed cursor".into()),
    })
}

/// Generates a pagination cursor from the last post in a feed.
pub fn generate_cursor_from_last_post(last_post: Option<&Post>) -> Option<String> {
    last_post.and_then(|lp| {
        NaiveDateTime::parse_from_str(&lp.indexed_at, "%+")
            .ok()
            .map(|parsed_time| format!("{}::{}", parsed_time.and_utc().timestamp_millis(), lp.cid))
    })
}

/// Records a list of posts as having been fetched for a user.
pub fn track_fetched_posts(did: String, posts: &[Post], conn: &mut PgConnection) {
    let fetched_posts = posts
        .iter()
        .map(|p| FetchedPost {
            did: did.clone(),
            uri: p.uri.clone(),
        })
        .collect();
    insert_fetched_posts(fetched_posts, conn);
}

/// Retrieves a paginated feed of posts containing media from followed users.
///
/// # Errors
///
/// Returns a `ValidationErrorMessageResponse` if database interactions fail.
#[tracing::instrument(skip(connection))]
pub async fn get_posts_by_following_media(
    did: String,
    _limit: Option<i64>,
    params_cursor: Option<&str>,
    connection: ReadReplicaConn,
) -> Result<AlgoResponse, ValidationErrorMessageResponse> {
    let limit: i64 = _limit.unwrap_or(30);
    let params_cursor = params_cursor.map(|params_cursor| params_cursor.to_string());
    let mut following = String::from("");

    let mut follow_dids = get_saved_follows(did.clone(), &connection).await;
    if follow_dids.is_empty() {
        tracing::info!("Creating followers for {}", did);
        let agent = get_agent().await.unwrap();
        let follows = get_follows(&agent, did.clone().as_ref()).await;
        let conn: DbObject =
            connection
                .0
                .get()
                .await
                .map_err(|_| ValidationErrorMessageResponse {
                    code: Some(ErrorCode::ValidationError),
                    message: Some("Failed to get database connection".to_string()),
                })?;
        conn.interact(move |conn: &mut PgConnection| {
            insert_follows(follows, conn);
        })
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Database interaction failed".to_string()),
        })?;
        follow_dids = get_saved_follows(did.clone(), &connection).await;
    }

    if follow_dids.is_empty() {
        return Ok(AlgoResponse {
            cursor: None,
            feed: Vec::new(),
        });
    }

    for follow_did in follow_dids.iter() {
        following += ("\'".to_string() + follow_did.as_str() + "\',").as_str();
    }
    following.pop();

    let result = connection
        .0
        .get()
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Failed to get database connection".to_string()),
        })?
        .interact(move |conn: &mut PgConnection| {
            let mut query_str: String = post_media_query_str(following.as_str());

            if let Some(cursor_str) = params_cursor {
                apply_cursor_to_single_query(&cursor_str, &mut query_str)?;
            }
            let order_str = format!(" ORDER BY \"indexedAt\" DESC, cid DESC LIMIT {} ", limit);
            let query_str = format!("{}{};", &query_str, &order_str);

            let results = sql_query(query_str)
                .load::<Post>(conn)
                .map_err(|e| ValidationErrorMessageResponse {
                    code: Some(ErrorCode::ValidationError),
                    message: Some(format!("Database query failed: {}", e)),
                })?;

            let mut post_results = Vec::new();
            let mut cursor: Option<String> = None;

            if let Some(last_post) = results.last() {
                if let Ok(parsed_time) = NaiveDateTime::parse_from_str(&last_post.indexed_at, "%+")
                {
                    cursor = Some(format!(
                        "{}::{}",
                        parsed_time.and_utc().timestamp_millis(),
                        last_post.cid
                    ));
                }
            }

            for result in &results {
                let post_result = if let Some(quote_uri) = &result.quote_uri {
                    let reason = PostResultReason {
                        reason_type: "app.bsky.feed.defs#skeletonReasonRepost".to_string(),
                        repost_uri: result.uri.clone(),
                    };
                    PostResult {
                        post: quote_uri.clone(),
                        reason: Some(reason),
                    }
                } else {
                    PostResult {
                        post: result.uri.clone(),
                        reason: None,
                    }
                };
                post_results.push(post_result);
            }

            let new_response = AlgoResponse {
                cursor,
                feed: post_results,
            };
            Ok(new_response)
        })
        .await
        .map_err(|e| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some(format!("Database interaction failed: {}", e)),
        })?;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_did_list() {
        let dids = vec!["did:plc:123".to_string(), "did:plc:456".to_string()];
        assert_eq!(format_did_list(&dids), "'did:plc:123','did:plc:456'");

        let empty: Vec<String> = vec![];
        assert_eq!(format_did_list(&empty), "");

        let single = vec!["did:plc:abc".to_string()];
        assert_eq!(format_did_list(&single), "'did:plc:abc'");
    }

    #[test]
    fn test_format_did_list_special_chars() {
        let dids = vec!["did:key:zQ3sh...abc".to_string()];
        assert_eq!(format_did_list(&dids), "'did:key:zQ3sh...abc'");
    }

    #[test]
    fn test_generate_cursor_from_last_post() {
        let post = Post {
            uri: "at://did:plc:abc/app.bsky.feed.post/123".to_string(),
            cid: "bafyreih".to_string(),
            indexed_at: "2023-10-20T10:00:00.000000+00:00".to_string(),
            ..Default::default()
        };
        let cursor = generate_cursor_from_last_post(Some(&post)).unwrap();
        // 2023-10-20T10:00:00Z is 1697796000000 ms
        assert_eq!(cursor, "1697796000000::bafyreih");

        assert!(generate_cursor_from_last_post(None).is_none());
    }

    #[test]
    fn test_apply_cursor_to_queries() {
        let query = "SELECT * FROM posts";
        let repost_query = "SELECT * FROM reposts";
        let cursor = "1697796000000::bafyreih";
        let (q, rq) = apply_cursor_to_queries(cursor, query, repost_query).unwrap();

        println!("Generated query: {}", q);
        println!("Generated repost query: {}", rq);

        // 1697796000000 is 2023-10-20T10:00:00Z.
        // The function adds 230ms: 2023-10-20T10:00:00.230Z
        // We check if it contains the expected parts.
        assert!(q.contains("SELECT * FROM posts"));
        assert!(q.contains("AND (\"indexedAt\" < '2023-10-20T10:00:00.230"));
        assert!(rq.contains("SELECT * FROM reposts"));
        assert!(rq.contains("WHERE (\"indexedAt\" < '2023-10-20T10:00:00.230"));
    }

    #[test]
    fn test_apply_cursor_to_queries_malformed() {
        let query = "SELECT * FROM posts";
        let repost_query = "SELECT * FROM reposts";
        let result = apply_cursor_to_queries("invalid", query, repost_query);
        assert!(result.is_err());
    }

    #[test]
    fn test_mutuals_query_str() {
        let did = "did:plc:123";
        let limit = 30;
        let query = mutuals_query_str(did, limit);
        assert!(query.contains("did:plc:123"));
        assert!(query.contains("LIMIT 30"));
        assert!(query.contains("JOIN follow f2 ON f1.subject = f2.author AND f1.author = f2.subject"));
        // Verify the query returns post data columns
        assert!(query.contains("SELECT uri"));
        assert!(query.contains("FROM post"));
        assert!(query.contains("ORDER BY \"indexedAt\" DESC, cid DESC"));
    }

    #[test]
    fn test_mutuals_query_str_edge_cases() {
        // Empty DID (still generates valid SQL, just no matches)
        let query = mutuals_query_str("", 30);
        assert!(query.contains("WHERE f1.author = ''"));

        // Zero limit
        let query = mutuals_query_str("did:plc:abc", 0);
        assert!(query.contains("LIMIT 0"));

        // Large limit
        let query = mutuals_query_str("did:plc:abc", 9999);
        assert!(query.contains("LIMIT 9999"));
    }

    #[test]
    fn test_post_media_query_str() {
        let following = "'did:plc:123','did:plc:456'";
        let query = post_media_query_str(following);
        assert!(query.contains("p1.media is true"));
        assert!(query.contains("did:plc:123"));
        assert!(query.contains("did:plc:456"));
        assert!(query.contains("select uri"));
        assert!(query.contains("p1.author in ('did:plc:123','did:plc:456')"));
    }

    #[test]
    fn test_post_media_query_str_empty_following() {
        let query = post_media_query_str("");
        assert!(query.contains("p1.author in ()"));
    }

    #[test]
    fn test_post_query_str_with_seen_posts() {
        let following = "'did:plc:abc'";
        let config = UserFeedPreference {
            show_quote_posts: true,
            show_replies: true,
            reply_filter_followed_only: false,
            reply_filter_likes: 0,
            ..Default::default()
        };
        let query = post_query_str(true, false, following, &config, "did:plc:requester");
        // Should contain LEFT OUTER JOIN seen_post
        assert!(query.contains("seen_post"));
        assert!(query.contains("s1.uri = p1.uri"));
    }

    #[test]
    fn test_post_query_str_without_seen_posts() {
        let following = "'did:plc:abc'";
        let config = UserFeedPreference::default();
        let query = post_query_str(false, false, following, &config, "did:plc:requester");
        // Should NOT contain seen_post
        assert!(!query.contains("seen_post"));
    }

    #[test]
    fn test_post_query_str_with_hide_no_alt_text() {
        let following = "'did:plc:abc'";
        let config = UserFeedPreference::default();
        let query = post_query_str(false, true, following, &config, "did:plc:requester");
        // Should contain alt text filter
        assert!(query.contains("alt"));
        assert!(query.contains("media"));
    }

    #[test]
    fn test_mutuals_query_str_sql_injection_attempt() {
        // Verify that a DID with special characters doesn't break SQL structure
        let did = "did:plc:abc'; DROP TABLE post; --";
        let query = mutuals_query_str(did, 30);
        // The DID should be interpolated as-is (string formatting), but the SQL
        // structure (SELECT, FROM, WHERE, JOIN, ORDER BY, LIMIT) should remain intact
        assert!(query.contains("SELECT uri"));
        assert!(query.contains("FROM post"));
        assert!(query.contains("ORDER BY"));
        assert!(query.contains("LIMIT 30"));
        assert!(query.contains("DROP TABLE post"));
    }
}

pub fn mutuals_query_str(did: &str, limit: i64) -> String {
    let sanitized_did = sanitize_did(did);
    format!(
        "SELECT uri,
       \"indexedAt\",
       cid,
       \"replyParent\",
       \"replyRoot\",
       prev,
       \"sequence\",
       \"text\",
       lang,
       author,
       \"externalUri\",
       \"externalTitle\",
       \"externalDescription\",
       \"externalThumb\",
       null as \"quoteCid\",
       null as \"quoteUri\",
       \"media\",
       alt
         FROM post
         WHERE author IN (
             SELECT f1.subject
             FROM follow f1
             JOIN follow f2 ON f1.subject = f2.author AND f1.author = f2.subject
             WHERE f1.author = '{did}'
         )
         ORDER BY \"indexedAt\" DESC, cid DESC
         LIMIT {limit}",
        did = sanitized_did,
        limit = limit
    )
}

#[tracing::instrument(skip(connection))]
pub async fn get_posts_by_mutuals(
    did: String,
    _limit: Option<i64>,
    params_cursor: Option<&str>,
    connection: ReadReplicaConn,
) -> Result<AlgoResponse, ValidationErrorMessageResponse> {
    let limit: i64 = _limit.unwrap_or(30);
    let params_cursor = params_cursor.map(|params_cursor| params_cursor.to_string());

    let result = connection
        .0
        .get()
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Failed to get database connection".to_string()),
        })?
        .interact(move |conn: &mut PgConnection| {
            let mut query_str: String = mutuals_query_str(did.as_str(), limit);

            if let Some(cursor_str) = params_cursor {
                apply_cursor_to_single_query(&cursor_str, &mut query_str)?;
            }
            let _order_str = format!(" ORDER BY \"indexedAt\" DESC, cid DESC LIMIT {} ", limit);
            // Re-adding LIMIT in case of cursor issue, though it's already in the string.
            // The previous queries did it this way.

            let results = sql_query(query_str)
                .load::<Post>(conn)
                .map_err(|e| ValidationErrorMessageResponse {
                    code: Some(ErrorCode::ValidationError),
                    message: Some(format!("Database query failed: {}", e)),
                })?;

            let mut post_results = Vec::new();
            let mut cursor: Option<String> = None;

            if let Some(last_post) = results.last() {
                if let Ok(parsed_time) = NaiveDateTime::parse_from_str(&last_post.indexed_at, "%+")
                {
                    cursor = Some(format!(
                        "{}::{}",
                        parsed_time.and_utc().timestamp_millis(),
                        last_post.cid
                    ));
                }
            }

            for post in &results {
                post_results.push(PostResult {
                    post: post.uri.clone(),
                    reason: None,
                });
            }

            Ok(AlgoResponse {
                cursor,
                feed: post_results,
            })
        })
        .await
        .map_err(|_| ValidationErrorMessageResponse {
            code: Some(ErrorCode::ValidationError),
            message: Some("Database interaction failed".to_string()),
        })??;

    Ok(result)
}
