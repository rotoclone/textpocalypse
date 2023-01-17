use async_trait::async_trait;
use flume::{Receiver, Sender};
use log::debug;
use russh::server::{Msg, Session};
use russh::*;
use std::collections::HashMap;
use std::str::from_utf8;
use std::sync::{Arc, Mutex};
use std::thread;

use core_logic::*;

use crate::delay_for_message;
use crate::message_to_string::message_to_string;

#[derive(Clone)]
pub struct Server {
    pub game: Arc<Mutex<Game>>,
    pub id: usize,
    pub clients: Arc<Mutex<HashMap<(usize, ChannelId), Client>>>,
}

#[derive(Clone)]
pub struct Client {
    command_sender: Sender<String>,
    message_receiver: Receiver<(GameMessage, Time)>,
}

impl server::Server for Server {
    type Handler = Server;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Server {
        debug!("Got new client with ID {}", self.id);
        let server = self.clone();
        self.id += 1;

        server
    }
}

/* TODO
impl server::Handler for Client {
    type Error = anyhow::Error;
    type FutureAuth = futures::future::Ready<Result<(Self, server::Auth), anyhow::Error>>;
    type FutureUnit = futures::future::Ready<Result<(Self, Session), anyhow::Error>>;
    type FutureBool = futures::future::Ready<Result<(Self, Session, bool), anyhow::Error>>;

    fn finished_auth(self, auth: Auth) -> Self::FutureAuth {
        futures::future::ready(Ok((self, auth)))
    }

    fn finished_bool(self, b: bool, s: Session) -> Self::FutureBool {
        futures::future::ready(Ok((self, s, b)))
    }

    fn finished(self, s: Session) -> Self::FutureUnit {
        futures::future::ready(Ok((self, s)))
    }

    fn auth_none(self, _: &str) -> Self::FutureAuth {
        self.finished_auth(server::Auth::Accept)
    }

    fn auth_publickey(self, _: &str, _: &key::PublicKey) -> Self::FutureAuth {
        self.finished_auth(server::Auth::Accept)
    }

    fn channel_open_session(self, channel: ChannelId, session: Session) -> Self::FutureBool {
        debug!("Opening session for channel {channel:?}");

        let session_handle = session.handle();
        let thread_message_receiver = self.message_receiver.clone();

        thread::Builder::new()
            .name("message receiver".to_string())
            .spawn(move || loop {
                let (message, game_time) = match thread_message_receiver.recv() {
                    Ok(x) => x,
                    Err(_) => {
                        debug!("Message sender has been dropped");
                        panic!("Disconnected from game")
                    }
                };
                debug!("Got message: {message:?}");
                let delay = delay_for_message(&message);
                let rendered_message = message_to_string(message, Some(game_time));
                session_handle.data(channel, rendered_message.into());
                thread::sleep(delay);
            })
            .expect("should be able to spawn thread");

        self.finished_bool(true, session)
    }

    fn data(self, channel: ChannelId, data: &[u8], mut session: Session) -> Self::FutureUnit {
        let command_string = match from_utf8(data) {
            Ok(s) => s,
            Err(e) => {
                let response = format!("Error parsing input: {e}");
                session.data(channel, response.into());
                return self.finished(session);
            }
        };

        match self.command_sender.send(command_string.to_string()) {
            Ok(()) => self.finished(session),
            Err(e) => {
                let response = format!("Error sending command: {e}");
                session.data(channel, response.into());
                self.disconnected(session)
            }
        }
    }
}
*/

#[async_trait]
impl server::Handler for Server {
    type Error = anyhow::Error;

    async fn channel_open_session(
        self,
        channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        debug!("Opening session for channel {channel:?}");

        {
            let mut game = self.game.lock().unwrap();
            let (command_sender, message_receiver) = game.add_player(format!("Player {}", self.id));
            let client = Client {
                command_sender,
                message_receiver,
            };

            let mut clients = self.clients.lock().unwrap();
            clients.insert((self.id, channel.id()), client);
        }

        {
            let clients = self.clients.lock().unwrap();

            let session_handle = session.handle();
            let client = clients.get(&(self.id, channel.id())).unwrap();
            let thread_message_receiver = client.message_receiver.clone();

            thread::Builder::new()
                .name("message receiver".to_string())
                .spawn(move || loop {
                    let (message, game_time) = match thread_message_receiver.recv() {
                        Ok(x) => x,
                        Err(_) => {
                            debug!("Message sender has been dropped");
                            panic!("Disconnected from game")
                        }
                    };
                    debug!("Got message: {message:?}");
                    let delay = delay_for_message(&message);
                    let rendered_message = message_to_string(message, Some(game_time));
                    session_handle.data(channel.id(), rendered_message.into());
                    thread::sleep(delay);
                })
                .expect("should be able to spawn thread");

            debug!("Spawned thread");
        }

        Ok((self, true, session))
    }

    async fn data(
        mut self,
        channel: ChannelId,
        data: &[u8],
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let command_string = from_utf8(data)?;
        {
            let clients = self.clients.lock().unwrap();
            let client = clients.get(&(self.id, channel)).unwrap();
            client.command_sender.send(command_string.to_string())?;
        }

        Ok((self, session))
    }

    async fn tcpip_forward(
        self,
        address: &str,
        port: &mut u32,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        let handle = session.handle();
        let address = address.to_string();
        let port = *port;
        tokio::spawn(async move {
            let mut channel = handle
                .channel_open_forwarded_tcpip(address, port, "1.2.3.4", 1234)
                .await
                .unwrap();
            let _ = channel.data(&b"Hello from a forwarded port"[..]).await;
            let _ = channel.eof().await;
        });
        Ok((self, true, session))
    }
}
