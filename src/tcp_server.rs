use anyhow::Result;
use core_logic::Game;
use futures::{SinkExt, StreamExt};
use log::{debug, info, trace, warn};
use tokio::net::TcpListener;
use tokio_util::codec::{Decoder, LinesCodec};

use crate::{delay_for_message, message_to_string};

pub async fn start_server(mut game: Game) -> Result<()> {
    let addr = "0.0.0.0:8080".to_string();

    // Next up we create a TCP listener which will listen for incoming
    // connections. This TCP listener is bound to the address we determined
    // above and must be associated with an event loop, so we pass in a handle
    // to our event loop. After the socket's created we inform that we're ready
    // to go and start accepting connections.
    let listener = TcpListener::bind(&addr).await?;
    info!("Listening on: {}", addr);

    let mut next_player_id = 0;
    loop {
        // Asynchronously wait for an inbound socket.
        let (socket, _) = listener.accept().await?;

        let player_id = next_player_id;
        next_player_id += 1;

        info!("Player {player_id} has connected");

        let (command_sender, message_receiver) = game.add_player(format!("Player {player_id}"));

        let (mut sink, mut stream) = LinesCodec::new().framed(socket).split::<String>();

        // spawn task for sending messages to player
        tokio::spawn(async move {
            loop {
                let (message, game_time) = match message_receiver.recv_async().await {
                    Ok(x) => x,
                    Err(_) => {
                        debug!("Message sender has been dropped");
                        break;
                    }
                };
                trace!("Got message: {message:?}");
                let delay = delay_for_message(&message);
                let rendered_message = message_to_string(message, Some(game_time));
                sink.send(format!("{rendered_message}\n"))
                    .await
                    .expect("should be able to send rendered message");
                tokio::time::sleep(delay).await;
            }
        });

        // spawn task for receiving commands from player
        tokio::spawn(async move {
            // The stream will return None once the client disconnects.
            while let Some(message) = stream.next().await {
                match message {
                    Ok(input) => {
                        debug!("Raw input: {input:?}");
                        if input == "quit" {
                            break;
                        }
                        command_sender
                            .send(input)
                            .expect("Command receiver should exist")
                    }
                    Err(err) => warn!("Socket closed with error: {err:?}"),
                }
            }

            info!("Player {player_id} has disconnected");
        });
    }
}
