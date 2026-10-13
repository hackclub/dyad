use sqlx::PgPool;

use crate::db::requests::*;

async fn hacker(pool: &PgPool, name: &str) -> i32 {
    sqlx::query_scalar("INSERT INTO hacker (hca_id, first_name) VALUES ($1, $1) RETURNING user_id")
        .bind(name)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn fields(name: &str) -> ProjectFields {
    ProjectFields {
        name: name.into(),
        description: String::new(),
        screenshot_url: String::new(),
        code_url: String::new(),
        playable_url: String::new(),
        games: vec![],
    }
}

async fn create(pool: &PgPool, creator: i32, friend: Option<i32>, names: &[&str]) -> i32 {
    let names: Vec<String> = names.iter().map(|n| n.to_string()).collect();
    match create_project(&fields("p"), creator, friend, false, &names, pool)
        .await
        .unwrap()
    {
        CreateOutcome::Created(id) => id,
        _ => panic!("project wasn't created"),
    }
}

async fn pair(pool: &PgPool) -> (i32, i32, i32) {
    let a = hacker(pool, "a").await;
    let b = hacker(pool, "b").await;
    let p = create(pool, a, Some(b), &[]).await;
    assert!(confirm_user_project(p, b, false, pool).await.unwrap());
    (a, b, p)
}

async fn ready(pool: &PgPool, p: i32, user: i32, record: &str) -> Recorded {
    let ClaimOutcome::Claimed(claim) = claim_submission(p, user, pool).await.unwrap() else {
        panic!("claim refused");
    };
    finish_submission(p, user, claim, record, 5.0, pool)
        .await
        .unwrap()
        .expect("claim went stale")
}

async fn record_of(pool: &PgPool, p: i32, user: i32) -> Option<String> {
    get_user_project(p, user, pool)
        .await
        .unwrap()
        .unwrap()
        .airtable_record_id
}

#[sqlx::test]
async fn one_partnerless_project_per_user(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    let b = hacker(&pool, "b").await;
    create(&pool, a, None, &[]).await;
    assert!(matches!(
        create_project(&fields("p"), a, None, false, &[], &pool)
            .await
            .unwrap(),
        CreateOutcome::PartnerlessTaken
    ));
    create(&pool, a, Some(b), &[]).await;
}

#[sqlx::test]
async fn hackatime_names_cannot_be_shared(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    let b = hacker(&pool, "b").await;
    create(&pool, a, None, &["x"]).await;
    let names = ["x".to_string()];
    assert!(matches!(
        create_project(&fields("p"), a, Some(b), false, &names, &pool)
            .await
            .unwrap(),
        CreateOutcome::NamesInUse
    ));
    let p2 = create(&pool, a, Some(b), &["y"]).await;
    assert!(matches!(
        save_project(p2, a, None, Some((false, &names)), &pool)
            .await
            .unwrap(),
        SaveOutcome::NamesInUse
    ));
    let own = ["y".to_string(), "z".to_string()];
    assert!(matches!(
        save_project(p2, a, None, Some((false, &own)), &pool)
            .await
            .unwrap(),
        SaveOutcome::Saved
    ));
}

#[sqlx::test]
async fn pending_invites_are_capped(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    for name in ["b", "c", "d"] {
        let friend = hacker(&pool, name).await;
        assert!(!has_too_many_invites(a, &pool).await.unwrap());
        create(&pool, a, Some(friend), &[]).await;
    }
    assert!(has_too_many_invites(a, &pool).await.unwrap());
    let e = hacker(&pool, "e").await;
    assert!(matches!(
        create_project(&fields("p"), a, Some(e), false, &[], &pool)
            .await
            .unwrap(),
        CreateOutcome::TooManyInvites
    ));
}

#[sqlx::test]
async fn approving_a_join_request_closes_the_others(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    let b = hacker(&pool, "b").await;
    let c = hacker(&pool, "c").await;
    let p = create(&pool, a, None, &[]).await;

    assert!(matches!(
        request_join(p, b, true, &pool).await.unwrap(),
        JoinOutcome::Requested
    ));
    assert!(matches!(
        request_join(p, c, false, &pool).await.unwrap(),
        JoinOutcome::Requested
    ));
    assert!(matches!(
        request_join(p, b, true, &pool).await.unwrap(),
        JoinOutcome::AlreadyRequested
    ));
    assert!(matches!(
        request_join(p, a, false, &pool).await.unwrap(),
        JoinOutcome::Unavailable
    ));
    assert_eq!(get_incoming_join_requests(a, &pool).await.unwrap().len(), 2);

    let others = approve_join(p, a, b, &pool)
        .await
        .unwrap()
        .expect("approve failed");
    assert_eq!(others, vec![c]);
    assert_eq!(get_project(p, &pool).await.unwrap().user2, Some(b));
    let up = get_user_project(p, b, &pool).await.unwrap().unwrap();
    assert!(up.confirmed_at.is_some());
    assert!(up.first_time);
    assert_eq!(up.hours_needed, 3.0);
    assert!(
        get_incoming_join_requests(a, &pool)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(get_partnerless_project(a, &pool).await.unwrap().is_none());

    assert!(matches!(
        request_join(p, c, false, &pool).await.unwrap(),
        JoinOutcome::Unavailable
    ));
    assert!(approve_join(p, a, c, &pool).await.unwrap().is_none());
}

#[sqlx::test]
async fn declining_an_invite_reopens_or_removes_the_project(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    let b = hacker(&pool, "b").await;
    let c = hacker(&pool, "c").await;

    let p1 = create(&pool, a, Some(b), &[]).await;
    assert_eq!(decline_invite(p1, b, &pool).await.unwrap(), Some(true));
    assert_eq!(get_project(p1, &pool).await.unwrap().user2, None);
    assert!(get_user_project(p1, b, &pool).await.unwrap().is_none());
    assert_eq!(decline_invite(p1, b, &pool).await.unwrap(), None);

    let p2 = create(&pool, a, Some(c), &[]).await;
    assert_eq!(decline_invite(p2, c, &pool).await.unwrap(), Some(false));
    assert!(get_project(p2, &pool).await.is_err());
    assert_eq!(
        get_partnerless_project(a, &pool)
            .await
            .unwrap()
            .map(|p| p.project_id),
        Some(p1)
    );
}

#[sqlx::test]
async fn delete_only_by_creator_before_a_partner_accepts(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    let b = hacker(&pool, "b").await;
    let c = hacker(&pool, "c").await;

    let p1 = create(&pool, a, None, &[]).await;
    assert!(matches!(
        request_join(p1, c, false, &pool).await.unwrap(),
        JoinOutcome::Requested
    ));
    assert!(delete_project(p1, c, &pool).await.unwrap().is_none());
    let deleted = delete_project(p1, a, &pool)
        .await
        .unwrap()
        .expect("delete refused");
    assert_eq!(deleted.invitee, None);
    assert_eq!(deleted.requesters, vec![c]);
    assert!(get_project(p1, &pool).await.is_err());

    let p2 = create(&pool, a, Some(b), &[]).await;
    assert!(delete_project(p2, b, &pool).await.unwrap().is_none());
    assert!(confirm_user_project(p2, b, false, &pool).await.unwrap());
    assert!(!confirm_user_project(p2, b, false, &pool).await.unwrap());
    assert!(delete_project(p2, a, &pool).await.unwrap().is_none());
    assert!(get_project(p2, &pool).await.is_ok());
}

#[sqlx::test]
async fn submission_lifecycle(pool: PgPool) {
    let (a, b, p) = pair(&pool).await;

    let ClaimOutcome::Claimed(claim) = claim_submission(p, a, &pool).await.unwrap() else {
        panic!("claim refused");
    };
    assert!(matches!(
        claim_submission(p, a, &pool).await.unwrap(),
        ClaimOutcome::Busy
    ));
    let recorded = finish_submission(p, a, claim, "recA", 5.0, &pool)
        .await
        .unwrap()
        .unwrap();
    assert!(!recorded.shipped);
    assert_eq!(recorded.record_ids, vec!["recA"]);
    assert!(
        finish_submission(p, a, claim, "recA", 5.0, &pool)
            .await
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        claim_submission(p, a, &pool).await.unwrap(),
        ClaimOutcome::AlreadyReady
    ));
    assert_eq!(get_review_rows(&pool).await.unwrap().len(), 1);

    assert!(matches!(
        withdraw_submission(p, b, &pool).await.unwrap(),
        WithdrawOutcome::NotReady
    ));
    let WithdrawOutcome::Withdrawn(Some((record, claim))) =
        withdraw_submission(p, a, &pool).await.unwrap()
    else {
        panic!("withdraw refused");
    };
    assert_eq!(record, "recA");
    assert!(matches!(
        withdraw_submission(p, a, &pool).await.unwrap(),
        WithdrawOutcome::Busy
    ));
    release_submission(p, a, claim, &pool).await.unwrap();
    let up = get_user_project(p, a, &pool).await.unwrap().unwrap();
    assert!(
        up.airtable_record_id.is_none() && up.ship_ok_at.is_none() && up.submitting_at.is_none()
    );

    assert!(!ready(&pool, p, a, "recA2").await.shipped);
    let recorded = ready(&pool, p, b, "recB").await;
    assert!(recorded.shipped);
    assert_eq!(recorded.record_ids.len(), 2);
    assert!(get_project(p, &pool).await.unwrap().shipped_at.is_some());
    assert!(matches!(
        withdraw_submission(p, a, &pool).await.unwrap(),
        WithdrawOutcome::Shipped
    ));
    assert!(matches!(
        claim_submission(p, a, &pool).await.unwrap(),
        ClaimOutcome::Shipped
    ));
    assert!(matches!(
        save_project(p, a, Some(&fields("q")), None, &pool)
            .await
            .unwrap(),
        SaveOutcome::Locked
    ));
}

#[sqlx::test]
async fn first_time_counts_once(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    let b = hacker(&pool, "b").await;
    let c = hacker(&pool, "c").await;
    let CreateOutcome::Created(p1) = create_project(&fields("p"), a, Some(b), true, &[], &pool)
        .await
        .unwrap()
    else {
        panic!("project wasn't created");
    };
    assert!(confirm_user_project(p1, b, false, &pool).await.unwrap());
    assert_eq!(
        get_user_project(p1, a, &pool)
            .await
            .unwrap()
            .unwrap()
            .hours_needed,
        3.0
    );
    ready(&pool, p1, a, "recA").await;

    let CreateOutcome::Created(p2) = create_project(&fields("p"), a, Some(c), true, &[], &pool)
        .await
        .unwrap()
    else {
        panic!("project wasn't created");
    };
    assert!(confirm_user_project(p2, c, false, &pool).await.unwrap());
    assert!(matches!(
        claim_submission(p2, a, &pool).await.unwrap(),
        ClaimOutcome::FirstTimeUsed
    ));
    assert!(matches!(
        save_project(p2, a, None, Some((false, &[])), &pool)
            .await
            .unwrap(),
        SaveOutcome::Saved
    ));
    assert!(matches!(
        claim_submission(p2, a, &pool).await.unwrap(),
        ClaimOutcome::Claimed(_)
    ));
}

#[sqlx::test]
async fn details_lock_once_someone_is_ready(pool: PgPool) {
    let (a, b, p) = pair(&pool).await;
    ready(&pool, p, a, "recA").await;

    assert!(matches!(
        save_project(p, b, Some(&fields("q")), None, &pool)
            .await
            .unwrap(),
        SaveOutcome::Locked
    ));
    assert!(matches!(
        save_project(p, b, None, Some((true, &[])), &pool)
            .await
            .unwrap(),
        SaveOutcome::Saved
    ));
    assert!(matches!(
        save_project(p, a, None, Some((true, &[])), &pool)
            .await
            .unwrap(),
        SaveOutcome::Locked
    ));
    assert_eq!(get_project(p, &pool).await.unwrap().name, "p");
}

#[sqlx::test]
async fn a_deleted_record_sends_the_project_back(pool: PgPool) {
    let (a, b, p) = pair(&pool).await;
    ready(&pool, p, a, "recA").await;
    assert!(ready(&pool, p, b, "recB").await.shipped);

    assert!(
        send_back(p, &[(a, "stale".into())], &pool)
            .await
            .unwrap()
            .is_none()
    );
    let result = send_back(p, &[(a, "recA".into())], &pool)
        .await
        .unwrap()
        .expect("nothing sent back");
    assert!(result.shipped);
    assert_eq!(result.sent_back, vec![a]);
    assert_eq!(result.unreadied, vec![b]);
    assert_eq!(result.withdrawn_records, vec!["recB"]);
    let project = get_project(p, &pool).await.unwrap();
    assert!(project.shipped_at.is_none() && project.review_status.is_none());
    assert_eq!(record_of(&pool, p, a).await, None);
    assert_eq!(record_of(&pool, p, b).await, None);
    assert!(get_review_rows(&pool).await.unwrap().is_empty());
    assert!(matches!(
        claim_submission(p, a, &pool).await.unwrap(),
        ClaimOutcome::Claimed(_)
    ));
}

#[sqlx::test]
async fn approval_stops_review_polling(pool: PgPool) {
    let (a, b, p) = pair(&pool).await;
    ready(&pool, p, a, "recA").await;
    ready(&pool, p, b, "recB").await;
    assert_eq!(get_review_rows(&pool).await.unwrap().len(), 2);

    set_review_status(p, "pending", false, &pool).await.unwrap();
    assert!(get_project(p, &pool).await.unwrap().approved_at.is_none());
    set_review_status(p, "approved", true, &pool).await.unwrap();
    let project = get_project(p, &pool).await.unwrap();
    assert!(project.approved_at.is_some());
    assert_eq!(project.review_status.as_deref(), Some("approved"));
    assert!(get_review_rows(&pool).await.unwrap().is_empty());
    assert!(
        send_back(p, &[(a, "recA".into())], &pool)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test]
async fn wrong_friend_codes_are_rate_limited(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    let b = hacker(&pool, "b").await;
    sqlx::query("UPDATE hacker SET friend_code = 123456 WHERE user_id = $1")
        .bind(b)
        .execute(&pool)
        .await
        .unwrap();
    for _ in 0..WRONG_CODE_LIMIT {
        assert!(matches!(
            find_friend(a, 1, &pool).await.unwrap(),
            FriendLookup::NotFound
        ));
    }
    assert!(matches!(
        find_friend(a, 1, &pool).await.unwrap(),
        FriendLookup::TooManyAttempts
    ));
    assert!(matches!(
        find_friend(a, 123456, &pool).await.unwrap(),
        FriendLookup::TooManyAttempts
    ));
    assert!(matches!(
        find_friend(b, 123456, &pool).await.unwrap(),
        FriendLookup::Found(_)
    ));
}

#[sqlx::test]
async fn dms_between_a_pair_are_rate_limited(pool: PgPool) {
    let a = hacker(&pool, "a").await;
    let b = hacker(&pool, "b").await;
    for _ in 0..3 {
        assert!(allow_dm(a, b, &pool).await.unwrap());
    }
    assert!(!allow_dm(a, b, &pool).await.unwrap());
    assert!(allow_dm(b, a, &pool).await.unwrap());
}
