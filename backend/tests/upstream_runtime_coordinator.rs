#[path = "support/runtime_database.rs"]
mod runtime_database;
#[path = "support/upstream_runtime.rs"]
mod upstream_runtime;

use std::{
    future::Future,
    sync::Arc,
    task::Poll,
    time::{Duration, Instant},
};

use mcpmate::core::{
    capability::{AffinityKey, ConnectionSelection},
    events::{Event, EventBus, EventHandlers},
    pool::UpstreamConnectionPool,
};
use upstream_runtime::{SlowUpstreamFixture, StartupBehavior};

#[tokio::test]
async fn profile_enable_startup_keeps_runtime_snapshot_available() {
    let fixture = SlowUpstreamFixture::new(
        "server-slow-runtime",
        "slow_runtime_fixture",
        Duration::from_millis(1500),
    )
    .await;
    let mut handlers = EventHandlers::new();
    handlers.set_connection_pool(fixture.pool.clone());
    handlers.init().expect("install event handlers");

    EventBus::global().publish(Event::ServerEnabledInProfileChanged {
        server_id: fixture.server_id.to_string(),
        server_name: fixture.server_name.to_string(),
        profile_id: "profile-test".to_string(),
        enabled: true,
    });
    fixture.wait_until_initializing().await;

    let snapshot = tokio::time::timeout(Duration::from_millis(200), async {
        fixture.pool.lock().await.get_snapshot()
    })
    .await
    .expect("runtime snapshot must remain available while the upstream initializes");
    assert!(
        snapshot.contains_key(fixture.server_id),
        "runtime snapshot must expose the initializing upstream"
    );

    let instance_id = fixture.wait_until_ready().await;

    fixture
        .pool
        .lock()
        .await
        .disconnect(fixture.server_id, &instance_id)
        .await
        .expect("disconnect stdio fixture");
}

#[tokio::test]
async fn disable_during_coordinated_startup_prevents_restart() {
    let fixture = SlowUpstreamFixture::new(
        "server-disable-runtime",
        "disable_runtime_fixture",
        Duration::from_secs(1),
    )
    .await;
    let startup_pool = fixture.pool.clone();
    let server_id = fixture.server_id;
    let startup =
        tokio::spawn(async move { UpstreamConnectionPool::enable_server_coordinated(&startup_pool, server_id).await });
    fixture.wait_until_initializing().await;

    fixture
        .pool
        .lock()
        .await
        .disable_server(fixture.server_id)
        .await
        .expect("disable initializing upstream");
    let startup_result = tokio::time::timeout(Duration::from_secs(5), startup)
        .await
        .expect("startup task must observe disable")
        .expect("startup task must not panic");
    assert!(startup_result.is_err(), "disabled startup must not publish or retry");

    let guard = fixture.pool.lock().await;
    assert!(!guard.config.mcp_servers.contains_key(fixture.server_id));
    assert!(!guard.connections.contains_key(fixture.server_id));
    drop(guard);
    assert_eq!(
        fixture.startup_count(),
        1,
        "disable must not allow a second process startup"
    );
}

#[tokio::test]
async fn disable_before_startup_registration_supersedes_enable() {
    let fixture = SlowUpstreamFixture::new(
        "server-disable-before-registration",
        "disable_before_registration_fixture",
        Duration::from_millis(10),
    )
    .await;
    let guard = fixture.pool.lock().await;
    let mut startup = Box::pin(UpstreamConnectionPool::enable_server_coordinated(
        &fixture.pool,
        fixture.server_id,
    ));
    std::future::poll_fn(|cx| match startup.as_mut().poll(cx) {
        Poll::Pending => Poll::Ready(()),
        Poll::Ready(_) => panic!("enable must wait for the held pool lock"),
    })
    .await;
    let mut disable = Box::pin(async { fixture.pool.lock().await.disable_server(fixture.server_id).await });
    std::future::poll_fn(|cx| match disable.as_mut().poll(cx) {
        Poll::Pending => Poll::Ready(()),
        Poll::Ready(_) => panic!("disable must wait behind enable for the held pool lock"),
    })
    .await;
    drop(guard);

    let (startup_result, disable_result) = tokio::join!(startup, disable);
    disable_result.expect("disable upstream before startup registration");
    assert!(
        startup_result.is_err(),
        "stale enable must not start a disabled upstream"
    );

    let guard = fixture.pool.lock().await;
    assert!(!guard.config.mcp_servers.contains_key(fixture.server_id));
    assert!(!guard.connections.contains_key(fixture.server_id));
    drop(guard);
    assert_eq!(fixture.startup_count(), 0, "disabled upstream must not spawn a process");
}

#[tokio::test]
async fn slow_server_startup_does_not_block_an_independent_server() {
    let fixture = SlowUpstreamFixture::new("server-slow-a", "slow_server_a", Duration::from_millis(1500)).await;
    let fast_server = fixture
        .add_server("server-fast-b", "fast_server_b", Duration::ZERO, StartupBehavior::Ready)
        .await;

    let slow_pool = fixture.pool.clone();
    let slow_start =
        tokio::spawn(
            async move { UpstreamConnectionPool::enable_server_coordinated(&slow_pool, "server-slow-a").await },
        );
    fixture.wait_until_initializing().await;

    let fast_pool = fixture.pool.clone();
    let started_at = Instant::now();
    let fast_instance = tokio::time::timeout(Duration::from_secs(2), async move {
        UpstreamConnectionPool::enable_server_coordinated(&fast_pool, fast_server.server_id).await
    })
    .await
    .expect("fast server startup must remain responsive")
    .expect("fast server must start");
    assert!(started_at.elapsed() < Duration::from_secs(2));

    let slow_instance = slow_start
        .await
        .expect("slow startup task must not panic")
        .expect("slow server must start");
    let mut pool = fixture.pool.lock().await;
    pool.disconnect(fixture.server_id, &slow_instance)
        .await
        .expect("disconnect slow server");
    pool.disconnect(fast_server.server_id, &fast_instance)
        .await
        .expect("disconnect fast server");
}

#[tokio::test]
async fn concurrent_demands_for_one_server_share_one_startup_attempt() {
    let fixture = SlowUpstreamFixture::new_with_behavior(
        "server-single-flight",
        "single_flight_fixture",
        Duration::from_millis(500),
        StartupBehavior::Fail,
    )
    .await;
    let first_pool = fixture.pool.clone();
    let second_pool = fixture.pool.clone();
    let first = tokio::spawn(async move {
        UpstreamConnectionPool::enable_server_coordinated(&first_pool, "server-single-flight").await
    });
    fixture.wait_until_initializing().await;
    let second = tokio::spawn(async move {
        UpstreamConnectionPool::enable_server_coordinated(&second_pool, "server-single-flight").await
    });

    let first_error = first
        .await
        .expect("first demand must not panic")
        .expect_err("failing fixture must reject startup");
    let second_error = second
        .await
        .expect("second demand must not panic")
        .expect_err("joiner must observe startup failure");
    assert_eq!(format!("{first_error:?}"), format!("{second_error:?}"));
    assert_eq!(fixture.startup_count(), 1);
}

#[tokio::test]
async fn stale_startup_result_is_not_published_after_config_change() {
    let fixture = SlowUpstreamFixture::new(
        "server-stale-startup",
        "stale_startup_fixture",
        Duration::from_millis(800),
    )
    .await;
    let startup_pool = fixture.pool.clone();
    let startup = tokio::spawn(async move {
        UpstreamConnectionPool::enable_server_coordinated(&startup_pool, "server-stale-startup").await
    });
    fixture.wait_until_initializing().await;

    {
        let mut pool = fixture.pool.lock().await;
        Arc::make_mut(&mut pool.config)
            .mcp_servers
            .get_mut(fixture.server_id)
            .expect("runtime fixture config must be registered")
            .source_fingerprint = Some("changed-during-startup".to_string());
    }

    let instance_id = startup
        .await
        .expect("startup task must not panic")
        .expect("startup must retry under the current configuration");
    let pool = fixture.pool.lock().await;
    let connection = pool
        .get_instance(fixture.server_id, &instance_id)
        .expect("current startup result must be published");
    assert_eq!(connection.config_fingerprint.as_deref(), Some("changed-during-startup"));
}

#[tokio::test]
async fn affinity_bound_startup_publishes_to_the_bound_instance_map() {
    let fixture = SlowUpstreamFixture::new("server-affinity", "affinity_fixture", Duration::ZERO).await;
    fixture.register_runtime_config_without_instance().await;
    let selection = ConnectionSelection {
        server_id: fixture.server_id.to_string(),
        affinity_key: AffinityKey::PerClient("client-1".to_string()),
    };
    let instance_id = UpstreamConnectionPool::ensure_connected_coordinated(&fixture.pool, &selection)
        .await
        .expect("affinity-bound startup must succeed");

    let mut pool = fixture.pool.lock().await;
    let bound = pool
        .client_bound_connections
        .get(&(fixture.server_id.to_string(), "client-1".to_string()))
        .and_then(|instances| instances.get(&instance_id))
        .expect("affinity-bound instance must be published to its bound map");
    assert!(bound.is_connected());
    pool.disconnect(fixture.server_id, &instance_id)
        .await
        .expect("disconnect affinity-bound server");
}
