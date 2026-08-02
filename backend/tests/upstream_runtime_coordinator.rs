#[path = "support/runtime_database.rs"]
mod runtime_database;
#[path = "support/upstream_runtime.rs"]
mod upstream_runtime;

use std::{future::Future, task::Poll, time::Duration};

use mcpmate::core::{
    events::{Event, EventBus, EventHandlers},
    pool::UpstreamConnectionPool,
};
use upstream_runtime::SlowUpstreamFixture;

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
