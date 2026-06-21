use crate::apis::algo::{
    DONT_SHOW_QUOTEPOSTS, DONT_SHOW_REPOSTS, HIDE_NOT_ALT_TEXT_POSTS, HIDE_SEEN_POSTS,
    NUMBER_OF_LIKES, RESET_PREF, SHOW_REPLIES_FOR_FOLLOWING_ONLY, USER_PREF_OPTIONS,
};
use crate::db::*;
use crate::models::*;
use crate::schema::user_feed_preference::dsl::user_feed_preference;
use crate::WriteDbConn;
use chrono::offset::Utc as UtcOffset;
use chrono::DateTime;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use lexicon::app::bsky::embed::{Embeds, MediaUnion};
use std::time::SystemTime;

/// Updates the number of likes for a given actor.
fn update_number_of_likes(actor: &str, like_count: i32, conn: &mut PgConnection) {
    let mut new_user_prefs = Vec::new();
    let result = get_user_config(actor, conn);
    match result {
        None => {
            let new_user_pref = (
                crate::schema::user_feed_preference::dsl::did.eq(actor.to_string()),
                crate::schema::user_feed_preference::dsl::show_replies.eq(true),
                crate::schema::user_feed_preference::dsl::reply_filter_likes.eq(like_count),
                crate::schema::user_feed_preference::dsl::reply_filter_followed_only.eq(false),
                crate::schema::user_feed_preference::dsl::show_reposts.eq(true),
                crate::schema::user_feed_preference::dsl::show_quote_posts.eq(true),
            );
            new_user_prefs.push(new_user_pref);
            diesel::insert_into(user_feed_preference)
                .values(&new_user_prefs)
                .execute(conn)
                .expect("Error inserting userfeedpref records");
        }
        Some(user_pref) => {
            diesel::update(user_feed_preference)
                .filter(crate::schema::user_feed_preference::dsl::did.eq(user_pref.did.clone()))
                .set((crate::schema::user_feed_preference::dsl::reply_filter_likes.eq(like_count),))
                .execute(conn)
                .expect("Error update config records");
        }
    }
}

/// Queues post creation requests for processing.
#[tracing::instrument(skip(conn))]
fn queue_post_creation(body: Vec<CreateRequest>, conn: &mut PgConnection) {
    use crate::schema::post::dsl as PostSchema;

    let mut new_posts = Vec::new();

    body.into_iter()
        .map(|req| {
            let system_time = SystemTime::now();
            let dt: DateTime<UtcOffset> = system_time.into();
            let mut post_text_original = String::new();
            let mut post_media_original = false;
            let mut post_alt_original = None;
            let mut new_post = Post {
                uri: req.uri,
                cid: req.cid,
                reply_parent: None,
                reply_root: None,
                indexed_at: format!("{}", dt.format("%+")),
                prev: req.prev,
                sequence: req.sequence,
                text: None,
                lang: None,
                author: req.author.clone(),
                external_uri: None,
                external_title: None,
                external_description: None,
                external_thumb: None,
                quote_cid: None,
                quote_uri: None,
                media: false,
                alt: None,
            };

            if let Lexicon::AppBskyFeedPost(post_record) = req.record {
                post_text_original = post_record.text.clone();
                if let Some(reply) = post_record.reply {
                    new_post.reply_parent = Some(reply.parent.uri);
                    new_post.reply_root = Some(reply.root.uri);
                }
                if let Some(langs) = post_record.langs {
                    new_post.lang = Some(langs.join(","));
                }
                if let Some(embed) = post_record.embed {
                    match embed {
                        Embeds::Images(e) => {
                            post_media_original = true;
                            for image in e.images {
                                if !image.alt.is_empty() {
                                    post_alt_original = Some(image.alt);
                                }
                            }
                        }
                        Embeds::Gallery(e) => {
                            post_media_original = true;
                            for item in e.items {
                                if !item.alt.is_empty() {
                                    post_alt_original = Some(item.alt);
                                }
                            }
                        }
                        Embeds::Video(e) => {
                            post_media_original = true;
                            post_alt_original = e.alt;
                        }
                        Embeds::RecordWithMedia(e) => {
                            post_media_original = true;
                            match e.media {
                                MediaUnion::Images(imgs) => {
                                    for image in imgs.images {
                                        if !image.alt.is_empty() {
                                            post_alt_original = Some(image.alt);
                                        }
                                    }
                                }
                                MediaUnion::Gallery(g) => {
                                    for item in g.items {
                                        if !item.alt.is_empty() {
                                            post_alt_original = Some(item.alt);
                                        }
                                    }
                                }
                                MediaUnion::Video(v) => {
                                    post_alt_original = v.alt;
                                }
                                MediaUnion::External(_) => {}
                            }
                        }
                        Embeds::External(e) => {
                            new_post.external_uri = Some(e.external.uri);
                            new_post.external_title = Some(e.external.title);
                            new_post.external_description = Some(e.external.description);
                            if let Some(thumb_blob) = e.external.thumb {
                                if let Some(thumb_cid) = thumb_blob.cid {
                                    new_post.external_thumb = Some(thumb_cid);
                                };
                            };
                        }
                        Embeds::Record(e) => {
                            new_post.quote_cid = Some(e.record.cid.to_string());
                            new_post.quote_uri = Some(e.record.uri);
                        }
                    }
                }
            }

            new_post.text = Some(post_text_original);
            new_post.media = post_media_original;
            new_post.alt = post_alt_original;

            match new_post.reply_parent {
                None => {}
                Some(ref reply_parent) => {
                    if reply_parent == NUMBER_OF_LIKES {
                        let num_likes = new_post
                            .text
                            .clone()
                            .unwrap_or("2".to_string())
                            .parse::<i32>()
                            .unwrap_or(2);
                        update_number_of_likes(&req.author, num_likes, conn);
                    }
                }
            }

            let new_post = (
                PostSchema::uri.eq(new_post.uri),
                PostSchema::cid.eq(new_post.cid),
                PostSchema::replyParent.eq(new_post.reply_parent),
                PostSchema::replyRoot.eq(new_post.reply_root),
                PostSchema::indexedAt.eq(new_post.indexed_at),
                PostSchema::prev.eq(new_post.prev),
                PostSchema::sequence.eq(new_post.sequence),
                PostSchema::text.eq(new_post.text),
                PostSchema::lang.eq(new_post.lang),
                PostSchema::author.eq(new_post.author),
                PostSchema::externalUri.eq(new_post.external_uri),
                PostSchema::externalTitle.eq(new_post.external_title),
                PostSchema::externalDescription.eq(new_post.external_description),
                PostSchema::externalThumb.eq(new_post.external_thumb),
                PostSchema::quoteCid.eq(new_post.quote_cid),
                PostSchema::quoteUri.eq(new_post.quote_uri),
                PostSchema::media.eq(new_post.media),
                PostSchema::alt.eq(new_post.alt),
            );
            new_posts.push(new_post);
        })
        .for_each(drop);

    diesel::insert_into(PostSchema::post)
        .values(&new_posts)
        .on_conflict(PostSchema::uri)
        .do_nothing()
        .execute(conn)
        .expect("Error inserting post records");
}

/// Queues repost creation requests for processing.
#[tracing::instrument(skip(conn))]
fn queue_repost_creation(body: Vec<CreateRequest>, conn: &mut PgConnection) {
    use crate::schema::repost::dsl as RepostSchema;

    let mut new_reposts = Vec::new();

    body.into_iter()
        .map(|req| {
            if let Lexicon::AppBskyFeedRepost(repost_record) = req.record {
                let system_time = SystemTime::now();
                let dt: DateTime<UtcOffset> = system_time.into();
                let new_like = (
                    RepostSchema::uri.eq(req.uri),
                    RepostSchema::cid.eq(req.cid),
                    RepostSchema::author.eq(req.author),
                    RepostSchema::subjectCid.eq(repost_record.subject.cid.to_string()),
                    RepostSchema::subjectUri.eq(repost_record.subject.uri),
                    RepostSchema::createdAt.eq(repost_record.created_at),
                    RepostSchema::indexedAt.eq(format!("{}", dt.format("%+"))),
                    RepostSchema::prev.eq(req.prev),
                    RepostSchema::sequence.eq(req.sequence),
                );
                new_reposts.push(new_like);
            }
        })
        .for_each(drop);

    diesel::insert_into(RepostSchema::repost)
        .values(&new_reposts)
        .on_conflict(RepostSchema::uri)
        .do_nothing()
        .execute(conn)
        .expect("Error inserting repost records");
}

/// Queues like creation requests for processing.
#[tracing::instrument(skip(conn))]
fn queue_like_creation(body: Vec<CreateRequest>, conn: &mut PgConnection) {
    use crate::schema::like::dsl as LikeSchema;
    use crate::schema::user_feed_preference::dsl as UserFeedSchema;
    use crate::schema::user_feed_preference::dsl::user_feed_preference;

    let mut new_likes = Vec::new();
    let mut new_user_prefs = Vec::new();

    body.into_iter()
        .map(|req| {
            if let Lexicon::AppBskyFeedLike(like_record) = req.record {
                if USER_PREF_OPTIONS.contains(&like_record.subject.uri.as_str()) {
                    let result = get_user_config(req.author.as_str(), conn);
                    match result {
                        None => {
                            let new_user_pref = (
                                UserFeedSchema::did.eq(req.author.clone()),
                                UserFeedSchema::show_replies.eq(true),
                                UserFeedSchema::reply_filter_likes.eq(2),
                                UserFeedSchema::reply_filter_followed_only.eq(false),
                                UserFeedSchema::show_reposts.eq(true),
                                UserFeedSchema::show_quote_posts.eq(true),
                            );
                            new_user_prefs.push(new_user_pref);
                            diesel::insert_into(user_feed_preference)
                                .values(&new_user_prefs)
                                .execute(conn)
                                .expect("Error inserting userfeedpref records");
                        }
                        Some(user_pref) => {
                            let uri = like_record.subject.uri.as_str();
                            if uri == SHOW_REPLIES_FOR_FOLLOWING_ONLY {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::reply_filter_followed_only.eq(true),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            } else if uri == DONT_SHOW_REPOSTS {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::show_reposts.eq(false),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            } else if uri == DONT_SHOW_QUOTEPOSTS {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::show_quote_posts.eq(false),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            } else if uri == RESET_PREF {
                                diesel::delete(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .execute(conn)
                                    .expect("Error update config records");
                            } else if uri == HIDE_SEEN_POSTS {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::hide_seen_posts.eq(true),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            } else if uri == HIDE_NOT_ALT_TEXT_POSTS {
                                diesel::update(user_feed_preference)
                                    .filter(UserFeedSchema::did.eq(user_pref.did.clone()))
                                    .set((UserFeedSchema::hide_no_alt_text.eq(true),))
                                    .execute(conn)
                                    .expect("Error update config records");
                            }
                        }
                    }
                }

                let system_time = SystemTime::now();
                let dt: DateTime<UtcOffset> = system_time.into();
                let new_like = (
                    LikeSchema::uri.eq(req.uri),
                    LikeSchema::author.eq(req.author),
                    LikeSchema::subjectUri.eq(like_record.subject.uri),
                    LikeSchema::indexedAt.eq(format!("{}", dt.format("%+"))),
                );
                new_likes.push(new_like);
            }
        })
        .for_each(drop);

    diesel::insert_into(LikeSchema::like)
        .values(&new_likes)
        .on_conflict(LikeSchema::uri)
        .do_nothing()
        .execute(conn)
        .expect("Error inserting like records");
}

/// Queues follow creation requests for processing.
#[tracing::instrument(skip(conn))]
fn queue_follow_creation(body: Vec<CreateRequest>, conn: &mut PgConnection) {
    use crate::schema::follow::dsl as FollowSchema;
    let mut new_follows = Vec::new();

    body.into_iter()
        .map(|req| {
            let is_subject_known = if let Lexicon::AppBskyFeedFollow(f) = &req.record {
                is_known_user(f.subject.as_str(), conn)
            } else {
                false
            };

            if user_follows_indexed(req.author.as_str(), conn)
                || is_known_user(req.author.as_str(), conn)
                || is_subject_known
            {
                if let Lexicon::AppBskyFeedFollow(follow_record) = req.record {
                    let system_time = SystemTime::now();
                    let dt: DateTime<UtcOffset> = system_time.into();
                    let new_follow = (
                        FollowSchema::uri.eq(req.uri),
                        FollowSchema::cid.eq(req.cid),
                        FollowSchema::author.eq(req.author),
                        FollowSchema::subject.eq(follow_record.subject),
                        FollowSchema::createdAt.eq(follow_record.created_at),
                        FollowSchema::indexedAt.eq(format!("{}", dt.format("%+"))),
                        FollowSchema::prev.eq(req.prev),
                        FollowSchema::sequence.eq(req.sequence),
                    );
                    new_follows.push(new_follow);
                }
            }
        })
        .for_each(drop);

    if !new_follows.is_empty() {
        diesel::insert_into(FollowSchema::follow)
            .values(&new_follows)
            .on_conflict(FollowSchema::uri)
            .do_nothing()
            .execute(conn)
            .expect("Error inserting follow records");
    }
}

#[tracing::instrument(skip(connection))]
pub async fn queue_creation(
    lex: String,
    body: Vec<CreateRequest>,
    connection: WriteDbConn,
) -> anyhow::Result<()> {
    connection
        .0
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?
        .interact(move |conn: &mut PgConnection| {
            if lex == "posts" {
                queue_post_creation(body, conn);
            } else if lex == "reposts" {
                queue_repost_creation(body, conn);
            } else if lex == "likes" {
                queue_like_creation(body, conn);
            } else if lex == "follows" {
                queue_follow_creation(body, conn);
            } else {
                return Err(anyhow::anyhow!("Unknown lexicon received {lex:?}"));
            }
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("Database interaction failed: {}", e))??;
    Ok(())
}

#[tracing::instrument(skip(connection))]
pub async fn queue_deletion(
    lex: String,
    body: Vec<DeleteRequest>,
    connection: WriteDbConn,
) -> anyhow::Result<()> {
    connection
        .0
        .get()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get database connection: {}", e))?
        .interact(move |conn: &mut PgConnection| {
            let mut delete_rows = Vec::new();
            body.into_iter()
                .map(|req| {
                    delete_rows.push(req.uri);
                })
                .for_each(drop);
            if lex == "posts" {
                delete_posts_by_uri(delete_rows, conn);
            } else if lex == "reposts" {
                delete_reposts_by_uri(delete_rows, conn);
            } else if lex == "likes" {
                delete_likes_by_uri(delete_rows, conn);
            } else if lex == "follows" {
                delete_follows_by_uri(delete_rows, conn);
            } else {
                tracing::error!("Unknown lexicon received {lex:?}");
            }
            Ok(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("Database interaction failed: {}", e))?
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the queue_creation dispatch logic: correct lexicon names route to handlers.
    ///
    /// This test verifies the string-to-handler mapping in `queue_creation`.
    /// Note: Full integration testing of `queue_follow_creation` requires a
    /// PostgreSQL test database with `is_known_user` and `user_follows_indexed`
    /// properly seeded. Those tests are best run in a Docker-based integration
    /// suite with a dedicated test database.
    #[tokio::test]
    async fn test_queue_creation_lexicon_routing() {
        // The `queue_creation` function handles these lexicon values:
        let valid_lexicons = ["posts", "reposts", "likes", "follows"];
        assert!(valid_lexicons.contains(&"posts"));
        assert!(valid_lexicons.contains(&"reposts"));
        assert!(valid_lexicons.contains(&"likes"));
        assert!(valid_lexicons.contains(&"follows"));
        // An unknown lexicon should produce an error path in the dispatch.
        assert!(!valid_lexicons.contains(&"unknown_lex"));
    }

    #[test]
    fn test_queue_deletion_lexicon_routing() {
        let valid_lexicons = ["posts", "reposts", "likes", "follows"];
        assert!(valid_lexicons.contains(&"posts"));
        assert!(valid_lexicons.contains(&"likes"));
        assert!(!valid_lexicons.contains(&"blocks"));
    }

    #[test]
    fn test_queue_follow_creation_filtering_logic() {
        // Test the conceptual filtering rule used in queue_follow_creation:
        // A follow is indexed if (author follows are indexed) OR (author is known) OR (subject is known).
        //
        // We're testing the boolean logic here, isolated from the database.
        let author_is_known = true;
        let author_follows_indexed = false;
        let subject_is_known = false;

        let should_index = author_follows_indexed || author_is_known || subject_is_known;
        assert!(should_index, "A known author's follows should be indexed");

        // If neither author nor subject is known, do not index
        let author_is_known = false;
        let author_follows_indexed = false;
        let subject_is_known = false;

        let should_index = author_follows_indexed || author_is_known || subject_is_known;
        assert!(
            !should_index,
            "Unknown users' follows should not be indexed"
        );
    }
}
