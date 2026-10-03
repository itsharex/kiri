//! Dedicated D-Bus peer: identity, requests and sessions cannot leak into
//! Screenshot/ScreenCast's ashpd connection. No desktop name allow-list.
use super::model::{ACTIONS, APP_ID};
use futures_util::StreamExt;
use serde::Serialize;
use std::{collections::HashMap, time::Duration};
use tokio_util::sync::CancellationToken;
use zbus::{
    proxy::{CacheProperties, SignalStream},
    zvariant::{OwnedObjectPath, OwnedValue, Type, Value},
    Connection, Proxy,
};

const DESTINATION: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const INTERFACE: &str = "org.freedesktop.portal.GlobalShortcuts";
const CALL_TIMEOUT: Duration = Duration::from_secs(5);
const APPROVAL_TIMEOUT: Duration = Duration::from_secs(120);
type Dict = HashMap<String, OwnedValue>;
type Shortcuts = Vec<(String, Dict)>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Portal call failed: {0}")]
    Bus(#[from] zbus::Error),
    #[error("Portal response was invalid")]
    Invalid,
    #[error("Portal request was cancelled or declined")]
    Declined,
    #[error("Portal request failed")]
    Failed,
    #[error("Portal request timed out")]
    Timeout,
    #[error("Portal service or session closed")]
    Closed,
}

type Result<T> = std::result::Result<T, Error>;

pub enum Event {
    Activated(String),
    Deactivated(String),
    Changed,
    Closed,
    Other,
}

pub struct Client {
    connection: Connection,
    proxy: Proxy<'static>,
    signals: SignalStream<'static>,
    owners: SignalStream<'static>,
    owner: String,
    pub version: u32,
    session: Option<(OwnedObjectPath, SignalStream<'static>)>,
}

fn empty() -> HashMap<&'static str, Value<'static>> {
    HashMap::new()
}
fn token() -> String {
    format!("kiri_shortcuts_{}", uuid::Uuid::new_v4().simple())
}

async fn proxy(
    connection: &Connection,
    owner: &str,
    path: &str,
    interface: &str,
) -> Result<Proxy<'static>> {
    Ok(zbus::proxy::Builder::new(connection)
        .destination(owner.to_owned())?
        .path(path.to_owned())?
        .interface(interface.to_owned())?
        .cache_properties(CacheProperties::No)
        .build()
        .await?)
}

async fn bounded<T>(future: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    tokio::time::timeout(CALL_TIMEOUT, future)
        .await
        .map_err(|_| Error::Timeout)?
}

impl Client {
    pub async fn connect() -> Result<Self> {
        bounded(async { Self::from_connection(Connection::session().await?).await }).await
    }

    async fn from_connection(connection: Connection) -> Result<Self> {
        let bus = proxy(
            &connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await?;
        let owners = bus
            .receive_signal_with_args("NameOwnerChanged", &[(0, DESTINATION)])
            .await?;
        // StartServiceByName activates the installed service but grants nothing.
        let _: u32 = bus.call("StartServiceByName", &(DESTINATION, 0u32)).await?;
        let owner: String = bus.call("GetNameOwner", &(DESTINATION,)).await?;
        // Register before the very first portal method on this connection.
        // Sandboxed builds must use their sandbox identity instead. Kiri ships
        // a host .deb; Flatpak packaging is not a supported distribution path.
        let registry = proxy(
            &connection,
            &owner,
            PATH,
            "org.freedesktop.host.portal.Registry",
        )
        .await?;
        let _: () = registry.call("Register", &(APP_ID, empty())).await?;
        let portal = proxy(&connection, &owner, PATH, INTERFACE).await?;
        // No cached/default version: frontend protocol presence alone is not a
        // usable backend. Missing interface/property must fail this probe.
        let version: u32 = portal.get_property("version").await?;
        if version == 0 {
            return Err(Error::Invalid);
        }
        let signals = portal.receive_all_signals().await?;
        Ok(Self {
            connection,
            proxy: portal,
            signals,
            owners,
            owner,
            version,
            session: None,
        })
    }

    fn path(&self, kind: &str, token: &str) -> Result<String> {
        let sender = self
            .connection
            .unique_name()
            .ok_or(Error::Invalid)?
            .as_str()
            .trim_start_matches(':')
            .replace('.', "_");
        Ok(format!("{PATH}/{kind}/{sender}/{token}"))
    }

    async fn request<B: Serialize + Type>(
        &self,
        method: &str,
        token: &str,
        body: &B,
        cancel: &CancellationToken,
        approval: bool,
    ) -> Result<Dict> {
        let path = self.path("request", token)?;
        let request = proxy(
            &self.connection,
            &self.owner,
            &path,
            "org.freedesktop.portal.Request",
        )
        .await?;
        let mut responses = request.receive_signal("Response").await?;
        let bus = proxy(
            &self.connection,
            "org.freedesktop.DBus",
            "/org/freedesktop/DBus",
            "org.freedesktop.DBus",
        )
        .await?;
        let mut owners = bus
            .receive_signal_with_args("NameOwnerChanged", &[(0, DESTINATION)])
            .await?;
        let mut closed = if let Some((path, _)) = &self.session {
            Some(
                proxy(
                    &self.connection,
                    &self.owner,
                    path.as_str(),
                    "org.freedesktop.portal.Session",
                )
                .await?
                .receive_signal("Closed")
                .await?,
            )
        } else {
            None
        };
        let closure = async {
            match &mut closed {
                Some(stream) => {
                    stream.next().await;
                }
                None => std::future::pending::<()>().await,
            }
        };
        // Subscribe before sending: a prompt-free restored binding can reply
        // immediately. Verify the returned handle belongs to this request.
        let operation = async {
            let handle: OwnedObjectPath = self.proxy.call(method, body).await?;
            if handle.as_str() != path {
                return Err(Error::Invalid);
            }
            let message = responses.next().await.ok_or(Error::Closed)?;
            let (response, values): (u32, Dict) = message.body().deserialize()?;
            match response {
                0 => Ok(values),
                1 => Err(Error::Declined),
                _ => Err(Error::Failed),
            }
        };
        let duration = if approval {
            APPROVAL_TIMEOUT
        } else {
            CALL_TIMEOUT
        };
        let result = tokio::select! {
            _ = cancel.cancelled() => Err(Error::Declined),
            _ = owners.next() => Err(Error::Closed),
            _ = closure => Err(Error::Closed),
            value = tokio::time::timeout(duration, operation) => value.unwrap_or(Err(Error::Timeout)),
        };
        if result.is_err() {
            let _ =
                tokio::time::timeout(CALL_TIMEOUT, request.call::<_, _, ()>("Close", &())).await;
        }
        result
    }

    pub async fn bind(
        &mut self,
        descriptions: &[String; 3],
        cancel: &CancellationToken,
    ) -> Result<Vec<(String, Option<String>)>> {
        let session_token = token();
        let path = self.path("session", &session_token)?;
        let session_proxy = proxy(
            &self.connection,
            &self.owner,
            &path,
            "org.freedesktop.portal.Session",
        )
        .await?;
        let closed = session_proxy.receive_signal("Closed").await?;
        // Keep the predicted path even if CreateSession times out: cleanup can
        // close a late-created session instead of orphaning approved bindings.
        self.session = Some((
            OwnedObjectPath::try_from(path.clone()).map_err(|_| Error::Invalid)?,
            closed,
        ));
        let handle = token();
        let options = HashMap::from([
            ("handle_token", Value::from(handle.as_str())),
            ("session_handle_token", Value::from(session_token.as_str())),
        ]);
        let response = self
            .request("CreateSession", &handle, &(options,), cancel, false)
            .await?;
        let returned = response.get("session_handle").ok_or(Error::Invalid)?;
        // The protocol deliberately retains string type for session_handle.
        if returned
            .downcast_ref::<&str>()
            .map_err(|_| Error::Invalid)?
            != path
        {
            return Err(Error::Invalid);
        }
        let shortcuts: Vec<_> = ACTIONS
            .iter()
            .zip(descriptions)
            .map(|((id, _, _), description)| {
                (
                    *id,
                    HashMap::from([("description", Value::from(description.as_str()))]),
                )
            })
            .collect();
        let handle = token();
        let options = HashMap::from([("handle_token", Value::from(handle.as_str()))]);
        let session = self.session.as_ref().ok_or(Error::Closed)?.0.clone();
        // Empty parent is permitted by the Portal protocol. Do not invent a
        // Wayland handle or pass an XWayland XID as a native Wayland parent.
        let response = self
            .request(
                "BindShortcuts",
                &handle,
                &(session, shortcuts, "", options),
                cancel,
                true,
            )
            .await?;
        parse_shortcuts(response)
    }

    pub async fn list(&self) -> Result<Vec<(String, Option<String>)>> {
        let session = &self.session.as_ref().ok_or(Error::Closed)?.0;
        let handle = token();
        let options = HashMap::from([("handle_token", Value::from(handle.as_str()))]);
        parse_shortcuts(
            self.request(
                "ListShortcuts",
                &handle,
                &(session, options),
                &CancellationToken::new(),
                false,
            )
            .await?,
        )
    }

    pub async fn configure(&self) -> Result<()> {
        if self.version < 2 {
            return Err(Error::Invalid);
        }
        let session = &self.session.as_ref().ok_or(Error::Closed)?.0;
        bounded(async {
            Ok(self
                .proxy
                .call("ConfigureShortcuts", &(session, "", empty()))
                .await?)
        })
        .await
    }

    pub async fn close_session(&mut self) {
        if let Some((path, _)) = self.session.take() {
            let _ = bounded(async {
                let session = proxy(
                    &self.connection,
                    &self.owner,
                    path.as_str(),
                    "org.freedesktop.portal.Session",
                )
                .await?;
                Ok(session.call::<_, _, ()>("Close", &()).await?)
            })
            .await;
        }
    }

    pub async fn shutdown(mut self) {
        self.close_session().await;
        let _ = tokio::time::timeout(CALL_TIMEOUT, self.connection.close()).await;
    }

    pub async fn next_event(&mut self) -> Result<Event> {
        let close = async {
            match self.session.as_mut() {
                Some((_, closed)) => {
                    closed.next().await;
                }
                None => std::future::pending::<()>().await,
            }
        };
        tokio::select! {
            _ = close => Ok(Event::Closed),
            changed = self.owners.next() => {
                let message = changed.ok_or(Error::Closed)?;
                let (_, old, new): (String, String, String) = message.body().deserialize()?;
                Ok(if old == self.owner && new != self.owner { Event::Closed } else { Event::Other })
            }
            message = self.signals.next() => {
                let message = message.ok_or(Error::Closed)?;
                let header = message.header();
                let member = header.member().map(|member| member.as_str()).unwrap_or("");
                let path = self.session.as_ref().map(|(path, _)| path.as_str());
                match member {
                    "Activated" | "Deactivated" => {
                        let (session, id, _, _): (OwnedObjectPath, String, u64, Dict) = message.body().deserialize()?;
                        if Some(session.as_str()) != path { return Ok(Event::Other); }
                        Ok(if member == "Activated" { Event::Activated(id) } else { Event::Deactivated(id) })
                    }
                    "ShortcutsChanged" => {
                        let (session, _): (OwnedObjectPath, Shortcuts) = message.body().deserialize()?;
                        Ok(if Some(session.as_str()) == path { Event::Changed } else { Event::Other })
                    }
                    _ => Ok(Event::Other),
                }
            }
        }
    }
}

fn parse_shortcuts(mut values: Dict) -> Result<Vec<(String, Option<String>)>> {
    let shortcuts: Shortcuts = values
        .remove("shortcuts")
        .ok_or(Error::Invalid)?
        .try_into()
        .map_err(|_| Error::Invalid)?;
    Ok(shortcuts
        .into_iter()
        .map(|(id, info)| {
            let trigger = info
                .get("trigger_description")
                .and_then(|value| value.downcast_ref::<&str>().ok())
                .map(str::to_owned);
            (id, trigger)
        })
        .collect())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
