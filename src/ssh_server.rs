use flume::{Receiver, Sender};
use futures::Future;
use log::debug;
use russh::server::{Auth, Session};
use russh::*;
use russh_keys::*;
use std::collections::HashMap;
use std::str::{from_utf8, FromStr};
use std::sync::{Arc, Mutex};
use std::thread;

use core_logic::*;

use crate::delay_for_message;
use crate::message_to_string::message_to_string;

pub struct Server {
    pub game: Game,
    pub client_pubkey: Arc<russh_keys::key::PublicKey>,
    pub next_id: usize,
}

pub struct Client {
    id: usize,
    command_sender: Sender<String>,
    message_receiver: Receiver<(GameMessage, Time)>,
}

impl server::Server for Server {
    type Handler = Client;
    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Client {
        debug!("Got new client with ID {}", self.next_id);
        let (command_sender, message_receiver) =
            self.game.add_player(format!("Player {}", self.next_id));
        let client = Client {
            id: self.next_id,
            command_sender,
            message_receiver,
        };

        self.next_id += 1;

        client
    }
}

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
