use std::{sync::Arc, time::Duration};

use anyhow::Result;
use core_logic::Game;
use futures::{SinkExt, StreamExt};
use log::debug;
use std::fmt::Write;
use tokio::{
    io::AsyncReadExt,
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::Mutex,
};
use tokio_util::codec::{BytesCodec, Decoder, LinesCodec};

use crate::{delay_for_message, message_to_string};

pub async fn start_server() -> Result<()> {
    let mut game = Game::new();

    let addr = "0.0.0.0:8080".to_string();

    // Next up we create a TCP listener which will listen for incoming
    // connections. This TCP listener is bound to the address we determined
    // above and must be associated with an event loop, so we pass in a handle
    // to our event loop. After the socket's created we inform that we're ready
    // to go and start accepting connections.
    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    let mut next_player_id = 0;
    loop {
        // Asynchronously wait for an inbound socket.
        let (socket, socket_addr) = listener.accept().await?;

        let (command_sender, message_receiver) =
            game.add_player(format!("Player {}", next_player_id));
        next_player_id += 1;

        // We're parsing each socket with the `BytesCodec` included in `tokio::codec`.
        //TODO let framed = Arc::new(Mutex::new(LinesCodec::new().framed(socket)));
        //TODO let other_framed = Arc::clone(&framed);

        let (mut sink, mut stream) = LinesCodec::new().framed(socket).split::<String>();

        //TODO let (read_socket, write_socket) = socket.into_split();

        // spawn worker for sending messages to player
        tokio::spawn(async move {
            loop {
                let (message, game_time) = match message_receiver.recv() {
                    Ok(x) => x,
                    Err(_) => {
                        debug!("Message sender has been dropped");
                        panic!("Disconnected from game")
                    }
                };
                debug!("Got message: {message:?}");
                let delay = delay_for_message(&message);
                let rendered_message = message_to_string(message, Some(game_time));
                debug!("Rendered message:\n{rendered_message}");
                /* TODO
                sink.send(rendered_message)
                    .await
                    .expect("should be able to send rendered message");
                    */
                tokio::time::sleep(delay).await;
            }
        });

        // spawn worker for receiving commands from player
        tokio::spawn(async move {
            // The stream will return None once the client disconnects.
            while let Some(message) = stream.next().await {
                match message {
                    Ok(input) => {
                        debug!("Raw input: {input:?}");
                        command_sender
                            .send(input)
                            .expect("Command receiver should exist")
                    }
                    Err(err) => println!("Socket closed with error: {:?}", err),
                }
            }
            println!("Socket received FIN packet and closed connection");
        });
    }
}
