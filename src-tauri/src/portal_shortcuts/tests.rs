//! In-process D-Bus contract tests, not compositor or physical Wayland QA.
use super::*;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};
use zbus::{message::Header, zvariant::Str};

struct Bus {
    process: Child,
    address: String,
}
impl Bus {
    fn new() -> Self {
        let mut process = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("dbus-daemon is required");
        let mut address = String::new();
        BufReader::new(process.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        assert!(
            !address.trim().is_empty(),
            "Private D-Bus could not start; run in a sandbox that allows local sockets"
        );
        Self {
            process,
            address: address.trim().to_owned(),
        }
    }
    async fn connection(&self) -> Connection {
        zbus::connection::Builder::address(self.address.as_str())
            .unwrap()
            .build()
            .await
            .unwrap()
    }
}
impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[derive(Default)]
struct State {
    registered: Vec<String>,
    requested: Vec<String>,
    session: String,
    bindings: Vec<(String, Option<String>)>,
    response: u32,
    hold: bool,
    request_closed: usize,
    session_closed: usize,
    configured: usize,
}
#[derive(Clone)]
struct Mock(Arc<Mutex<State>>);
#[derive(Clone)]
struct Registry(Arc<Mutex<State>>);
struct Request(Arc<Mutex<State>>);
struct Session(Arc<Mutex<State>>);

#[zbus::interface(name = "org.freedesktop.host.portal.Registry")]
impl Registry {
    fn register(&self, app_id: &str, _options: Dict) {
        self.0.lock().unwrap().registered.push(app_id.into());
    }
}
#[zbus::interface(name = "org.freedesktop.portal.Request")]
impl Request {
    fn close(&self) {
        self.0.lock().unwrap().request_closed += 1;
    }
}
#[zbus::interface(name = "org.freedesktop.portal.Session")]
impl Session {
    fn close(&self) {
        self.0.lock().unwrap().session_closed += 1;
    }
}
fn value(s: &str) -> OwnedValue {
    OwnedValue::from(Str::from(s))
}
fn response_bindings(bindings: &[(String, Option<String>)]) -> Dict {
    let shortcuts: Shortcuts = bindings
        .iter()
        .map(|(id, trigger)| {
            let mut info = Dict::from([("description".into(), value(id))]);
            if let Some(trigger) = trigger {
                info.insert("trigger_description".into(), value(trigger));
            }
            (id.clone(), info)
        })
        .collect();
    Dict::from([(
        "shortcuts".into(),
        OwnedValue::try_from(Value::new(shortcuts)).unwrap(),
    )])
}
fn handle(header: &Header<'_>, options: &Dict, kind: &str, key: &str) -> OwnedObjectPath {
    let sender = header
        .sender()
        .unwrap()
        .as_str()
        .trim_start_matches(':')
        .replace('.', "_");
    let token = options.get(key).unwrap().downcast_ref::<&str>().unwrap();
    OwnedObjectPath::try_from(format!("{PATH}/{kind}/{sender}/{token}")).unwrap()
}
async fn respond(
    connection: &Connection,
    path: &OwnedObjectPath,
    code: u32,
    values: Dict,
) -> zbus::fdo::Result<()> {
    connection
        .emit_signal(
            None::<&str>,
            path,
            "org.freedesktop.portal.Request",
            "Response",
            &(code, values),
        )
        .await
        .map_err(|error| zbus::fdo::Error::Failed(error.to_string()))
}
#[zbus::interface(name = "org.freedesktop.portal.GlobalShortcuts")]
impl Mock {
    #[zbus(property, name = "version")]
    fn version(&self) -> u32 {
        2
    }
    async fn create_session(
        &self,
        options: Dict,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        assert_eq!(
            self.0.lock().unwrap().registered.last().map(String::as_str),
            Some(APP_ID)
        );
        let request = handle(&header, &options, "request", "handle_token");
        let session = handle(&header, &options, "session", "session_handle_token");
        self.0.lock().unwrap().session = session.to_string();
        connection
            .object_server()
            .at(&session, Session(self.0.clone()))
            .await?;
        connection
            .object_server()
            .at(&request, Request(self.0.clone()))
            .await?;
        respond(
            connection,
            &request,
            0,
            Dict::from([("session_handle".into(), value(session.as_str()))]),
        )
        .await?;
        Ok(request)
    }
    async fn bind_shortcuts(
        &self,
        _session: OwnedObjectPath,
        shortcuts: Shortcuts,
        _parent: String,
        options: Dict,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let request = handle(&header, &options, "request", "handle_token");
        connection
            .object_server()
            .at(&request, Request(self.0.clone()))
            .await?;
        let (code, bindings, hold) = {
            let mut state = self.0.lock().unwrap();
            state.requested = shortcuts.into_iter().map(|(id, _)| id).collect();
            (state.response, state.bindings.clone(), state.hold)
        };
        if !hold {
            respond(connection, &request, code, response_bindings(&bindings)).await?;
        }
        Ok(request)
    }
    async fn list_shortcuts(
        &self,
        _session: OwnedObjectPath,
        options: Dict,
        #[zbus(header)] header: Header<'_>,
        #[zbus(connection)] connection: &Connection,
    ) -> zbus::fdo::Result<OwnedObjectPath> {
        let request = handle(&header, &options, "request", "handle_token");
        let bindings = self.0.lock().unwrap().bindings.clone();
        respond(connection, &request, 0, response_bindings(&bindings)).await?;
        Ok(request)
    }
    fn configure_shortcuts(&self, _session: OwnedObjectPath, _parent: String, _options: Dict) {
        self.0.lock().unwrap().configured += 1;
    }
}
async fn mock_server(bus: &Bus, state: &Arc<Mutex<State>>) -> Connection {
    zbus::connection::Builder::address(bus.address.as_str())
        .unwrap()
        .name(DESTINATION)
        .unwrap()
        .serve_at(PATH, Registry(state.clone()))
        .unwrap()
        .serve_at(PATH, Mock(state.clone()))
        .unwrap()
        .build()
        .await
        .unwrap()
}
async fn new_client(bus: &Bus) -> Client {
    Client::from_connection(bus.connection().await)
        .await
        .unwrap()
}
fn descriptions() -> [String; 3] {
    ["Capture", "Pause/Resume", "Stop"].map(str::to_owned)
}
async fn significant(client: &mut Client) -> Result<Event> {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            match client.next_event().await? {
                Event::Other => {}
                value => return Ok(value),
            }
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
#[ignore = "requires private local D-Bus sockets; CI runs these explicitly"]
async fn identity_subset_remap_revoke_and_configure_use_real_dbus_wire() {
    let bus = Bus::new();
    let state = Arc::new(Mutex::new(State {
        bindings: vec![("capture".into(), Some("Ctrl+A, Super+A".into()))],
        ..Default::default()
    }));
    let server = mock_server(&bus, &state).await;
    let mut client = new_client(&bus).await;
    let bindings = client
        .bind(&descriptions(), &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(bindings, state.lock().unwrap().bindings);
    assert_eq!(
        state.lock().unwrap().requested,
        vec!["capture", "pause-resume", "stop"]
    );
    client.configure().await.unwrap();
    assert_eq!(state.lock().unwrap().configured, 1);
    state.lock().unwrap().bindings = vec![("stop".into(), Some("Alt+S".into()))];
    assert_eq!(client.list().await.unwrap()[0].0, "stop");
    state.lock().unwrap().bindings.clear();
    assert!(client.list().await.unwrap().is_empty());
    client.shutdown().await;
    assert_eq!(state.lock().unwrap().session_closed, 1);
    server.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires private local D-Bus sockets; CI runs these explicitly"]
async fn cancellation_closes_pending_request_and_session_without_waiting_for_dialog() {
    let bus = Bus::new();
    let state = Arc::new(Mutex::new(State {
        hold: true,
        ..Default::default()
    }));
    let _server = mock_server(&bus, &state).await;
    let mut client = new_client(&bus).await;
    let cancel = CancellationToken::new();
    let trigger = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        trigger.cancel();
    });
    assert!(matches!(
        client.bind(&descriptions(), &cancel).await,
        Err(Error::Declined)
    ));
    client.shutdown().await;
    assert!(state.lock().unwrap().request_closed >= 1);
    assert_eq!(state.lock().unwrap().session_closed, 1);
}

#[tokio::test]
#[ignore = "requires private local D-Bus sockets; CI runs these explicitly"]
async fn denial_is_not_a_successful_binding() {
    let bus = Bus::new();
    let state = Arc::new(Mutex::new(State {
        response: 1,
        ..Default::default()
    }));
    let _server = mock_server(&bus, &state).await;
    let mut client = new_client(&bus).await;
    assert!(matches!(
        client
            .bind(&descriptions(), &CancellationToken::new())
            .await,
        Err(Error::Declined)
    ));
    client.shutdown().await;
}

#[tokio::test]
#[ignore = "requires private local D-Bus sockets; CI runs these explicitly"]
async fn signals_are_session_scoped_and_service_restart_invalidates_peer() {
    let bus = Bus::new();
    let state = Arc::new(Mutex::new(State::default()));
    let server = mock_server(&bus, &state).await;
    let mut client = new_client(&bus).await;
    client
        .bind(&descriptions(), &CancellationToken::new())
        .await
        .unwrap();
    let wrong = OwnedObjectPath::try_from("/other/session").unwrap();
    server
        .emit_signal(
            None::<&str>,
            PATH,
            INTERFACE,
            "Activated",
            &(wrong, "capture", 1u64, empty()),
        )
        .await
        .unwrap();
    let path = OwnedObjectPath::try_from(state.lock().unwrap().session.clone()).unwrap();
    server
        .emit_signal(
            None::<&str>,
            PATH,
            INTERFACE,
            "Activated",
            &(path.clone(), "stop", 2u64, empty()),
        )
        .await
        .unwrap();
    assert!(
        matches!(significant(&mut client).await.unwrap(), Event::Activated(id) if id == "stop")
    );
    server
        .emit_signal(
            None::<&str>,
            &path,
            "org.freedesktop.portal.Session",
            "Closed",
            &(empty(),),
        )
        .await
        .unwrap();
    assert!(matches!(
        significant(&mut client).await.unwrap(),
        Event::Closed
    ));
    server.close().await.unwrap();
    assert!(matches!(
        significant(&mut client).await,
        Ok(Event::Closed) | Err(Error::Closed)
    ));
    client.shutdown().await;
    let _replacement = mock_server(&bus, &state).await;
    let new = new_client(&bus).await;
    assert_eq!(state.lock().unwrap().registered.len(), 2);
    new.shutdown().await;
}

#[tokio::test]
#[ignore = "requires private local D-Bus sockets; CI runs these explicitly"]
async fn missing_global_shortcut_interface_is_not_reported_available() {
    let bus = Bus::new();
    let state = Arc::new(Mutex::new(State::default()));
    let _server = zbus::connection::Builder::address(bus.address.as_str())
        .unwrap()
        .name(DESTINATION)
        .unwrap()
        .serve_at(PATH, Registry(state))
        .unwrap()
        .build()
        .await
        .unwrap();
    assert!(Client::from_connection(bus.connection().await)
        .await
        .is_err());
}

#[tokio::test]
#[ignore = "requires private local D-Bus sockets; CI runs these explicitly"]
async fn portal_disappearance_during_approval_finishes_without_waiting_for_timeout() {
    let bus = Bus::new();
    let state = Arc::new(Mutex::new(State {
        hold: true,
        ..Default::default()
    }));
    let server = mock_server(&bus, &state).await;
    let mut client = new_client(&bus).await;
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        server.close().await.unwrap();
    });
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        client.bind(&descriptions(), &CancellationToken::new()),
    )
    .await
    .unwrap();
    assert!(matches!(result, Err(Error::Closed) | Err(Error::Bus(_))));
    client.shutdown().await;
}
