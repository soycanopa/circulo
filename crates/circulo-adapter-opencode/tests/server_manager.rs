//! `ServerManager` lifecycle behavior against real sockets.

use std::net::TcpListener;
use std::path::PathBuf;
use std::time::Duration;

use circulo_adapter::ErrorReason;
use circulo_adapter_opencode::testing::FakeOpenCodeServer;
use circulo_adapter_opencode::{ServerConfig, ServerManager};

fn config_for(port: u16, command: Option<PathBuf>, timeout: Duration) -> ServerConfig {
    ServerConfig {
        port,
        command,
        cwd: PathBuf::from("."),
        startup_timeout: timeout,
    }
}

fn vacant_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral");
    let port = listener.local_addr().expect("addr").port();
    drop(listener);
    port
}

#[test]
fn healthy_server_is_reused_without_spawning() {
    let server = FakeOpenCodeServer::spawn();
    let manager = ServerManager::new(config_for(server.port, None, Duration::from_secs(1)));
    manager.ensure_running().expect("reuse healthy server");
    manager
        .ensure_running()
        .expect("still healthy on second check");
}

#[test]
fn missing_binary_is_reported_as_missing() {
    let port = vacant_port();
    let config = config_for(port, None, Duration::from_millis(300));
    let manager = ServerManager::with_binary_resolver(config, || None);
    let err = manager.ensure_running().expect_err("no server, no binary");
    assert_eq!(err.reason(), ErrorReason::BinaryMissing);
    assert_eq!(err.kind(), "unavailable");
}

#[test]
fn non_opencode_port_occupant_is_rejected() {
    let squatter = TcpListener::bind(("127.0.0.1", 0)).expect("squatter bind");
    let port = squatter.local_addr().expect("addr").port();
    let manager = ServerManager::new(config_for(port, None, Duration::from_millis(300)));
    let err = manager.ensure_running().expect_err("port is taken");
    assert_eq!(err.reason(), ErrorReason::PortOccupied);
}

#[test]
fn command_that_never_serves_reports_start_failure() {
    let port = vacant_port();
    let manager = ServerManager::new(config_for(
        port,
        Some(PathBuf::from("/usr/bin/yes")),
        Duration::from_millis(400),
    ));
    let err = manager.ensure_running().expect_err("never healthy");
    assert_eq!(err.reason(), ErrorReason::StartFailed);
}

#[test]
fn explicit_command_is_honored_even_when_missing() {
    let port = vacant_port();
    let manager = ServerManager::new(config_for(
        port,
        Some(PathBuf::from("/nonexistent/opencode-for-tests")),
        Duration::from_millis(200),
    ));
    let err = manager.ensure_running().expect_err("spawn fails");
    assert_eq!(err.reason(), ErrorReason::StartFailed);
}
