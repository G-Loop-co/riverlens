use anyhow::{ensure, Context, Result};
use interprocess::local_socket::{
    tokio::{prelude::*, Stream},
    GenericNamespaced, ListenerOptions,
};
use poker_core::{
    agent,
    service::{Request, Service},
};
use rand::{distributions::Alphanumeric, Rng};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{timeout, Duration};

pub struct Bridge {
    pub socket: String,
    state: Mutex<Option<(String, bool)>>,
    pub service: Arc<Service>,
}
fn secret() -> String {
    rand::thread_rng()
        .sample_iter(Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}
pub fn credential(account: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new("app.riverlens.ai", account)?)
}
impl Bridge {
    pub async fn start(service: Arc<Service>) -> Result<Arc<Self>> {
        let socket = format!("riverlens-{}", secret());
        let listener = ListenerOptions::new()
            .name(socket.as_str().to_ns_name::<GenericNamespaced>()?)
            .create_tokio()?;
        let bridge = Arc::new(Self {
            socket,
            state: Mutex::new(None),
            service,
        });
        let b = bridge.clone();
        let permits = Arc::new(tokio::sync::Semaphore::new(8));
        tokio::spawn(async move {
            while let Ok(stream) = listener.accept().await {
                let Ok(permit) = permits.clone().try_acquire_owned() else {
                    continue;
                };
                let b = b.clone();
                tokio::spawn(async move {
                    let _permit = permit;
                    let _ = timeout(Duration::from_secs(30), b.serve(stream)).await;
                });
            }
        });
        Ok(bridge)
    }
    pub fn configure(&self, enabled: bool, notes: bool) -> Result<Value> {
        let account = format!("bridge:{}", self.socket);
        if enabled {
            let token = secret();
            credential(&account)?.set_password(&token)?;
            *self.state.lock().unwrap() = Some((token, notes));
        } else {
            *self.state.lock().unwrap() = None;
            let _ = credential(&account)?.delete_credential();
        }
        self.status()
    }
    pub fn status(&self) -> Result<Value> {
        let s = self.state.lock().unwrap();
        let command = std::env::current_exe()?;
        Ok(
            json!({"enabled":s.is_some(),"share_notes":s.as_ref().is_some_and(|s|s.1),"socket":self.socket,
            "config":{"mcpServers":{"riverlens":{"command":command,"args":["--mcp","--socket",self.socket]}}},
            "notice":"Local connector. External models may receive returned data. Connection authorization lasts until disabled or app exits."}),
        )
    }
    async fn serve(self: Arc<Self>, mut stream: Stream) -> Result<()> {
        #[cfg(unix)]
        {
            use interprocess::local_socket::traits::StreamCommon;
            ensure!(
                stream.peer_creds()?.euid() == Some(unsafe { libc::geteuid() }),
                "different OS user"
            );
        }
        let request = read_frame(&mut stream).await?;
        let (token, notes) = self
            .state
            .lock()
            .unwrap()
            .clone()
            .context("agent connection disabled")?;
        let supplied = request["token"].as_str().unwrap_or("");
        ensure!(
            supplied.len() == token.len()
                && supplied
                    .bytes()
                    .zip(token.bytes())
                    .fold(0u8, |x, (a, b)| x | (a ^ b))
                    == 0,
            "unauthorized"
        );
        let name = request["name"]
            .as_str()
            .context("tool missing")?
            .to_string();
        let arguments = request["arguments"].clone();
        agent::validate(&name, &arguments)?;
        // Recheck authorization immediately before executing the synchronous core operation.
        let b = self.clone();
        let result = tokio::task::spawn_blocking(move || {
            let state = b.state.lock().unwrap();
            ensure!(
                state.as_ref().is_some_and(|s| s.0 == token),
                "connection revoked"
            );
            b.service.handle(Request::AgentTool {
                name,
                arguments,
                share_notes: notes,
            })
        })
        .await?;
        let response = match result {
            Ok(v) => json!({"result":v}),
            Err(e) => json!({"error":e.to_string()}),
        };
        write_frame(&mut stream, &response).await
    }
}
impl Drop for Bridge {
    fn drop(&mut self) {
        let _ =
            credential(&format!("bridge:{}", self.socket)).and_then(|e| Ok(e.delete_credential()?));
    }
}
pub async fn read_frame(stream: &mut Stream) -> Result<Value> {
    let n = stream.read_u32().await?;
    ensure!(n <= 4_000_000, "message too large");
    let mut bytes = vec![0; n as usize];
    stream.read_exact(&mut bytes).await?;
    Ok(serde_json::from_slice(&bytes)?)
}
pub async fn write_frame(stream: &mut Stream, v: &Value) -> Result<()> {
    let bytes = serde_json::to_vec(v)?;
    ensure!(bytes.len() <= 4_000_000, "message too large");
    stream.write_u32(bytes.len() as u32).await?;
    stream.write_all(&bytes).await?;
    stream.flush().await?;
    Ok(())
}
pub async fn call(socket: &str, name: &str, arguments: Value) -> Result<Value> {
    agent::validate(name, &arguments)?;
    let token = credential(&format!("bridge:{socket}"))?
        .get_password()
        .context("Enable AI connection in RiverLens first")?;
    timeout(Duration::from_secs(30), async {
        let mut stream = Stream::connect(socket.to_ns_name::<GenericNamespaced>()?)
            .await
            .context("Open RiverLens and refresh connector configuration")?;
        write_frame(
            &mut stream,
            &json!({"token":token,"name":name,"arguments":arguments}),
        )
        .await?;
        let response = read_frame(&mut stream).await?;
        ensure!(response.get("error").is_none(), "{}", response["error"]);
        Ok(response["result"].clone())
    })
    .await?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ipc_auth_tool_scope_and_revocation() {
        let dir = tempfile::tempdir().unwrap();
        let service = Service::new(dir.path().join("db")).unwrap();
        let b = Bridge::start(service).await.unwrap();
        // Synthetic token avoids reading or writing any user's keychain.
        *b.state.lock().unwrap() = Some(("test-token".into(), false));
        async fn query(b: &Bridge, token: &str, name: &str) -> Result<Value> {
            let mut stream =
                Stream::connect(b.socket.as_str().to_ns_name::<GenericNamespaced>()?).await?;
            write_frame(
                &mut stream,
                &json!({"token":token,"name":name,"arguments":{}}),
            )
            .await?;
            read_frame(&mut stream).await
        }
        assert!(query(&b, "wrong", "get_data_catalog").await.is_err());
        let good = query(&b, "test-token", "get_data_catalog").await.unwrap();
        assert_eq!(good["result"]["data"]["coverage"]["hands"], 0);
        assert!(query(&b, "test-token", "restore").await.is_err());
        *b.state.lock().unwrap() = None;
        assert!(query(&b, "test-token", "get_data_catalog").await.is_err());
    }
}
