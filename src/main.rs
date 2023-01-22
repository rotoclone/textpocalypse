use anyhow::Result;
use crossterm::{
    cursor,
    style::Print,
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use log::debug;
use std::{
    io::{stdin, stdout, Write},
    sync::{
        atomic::{self, AtomicBool},
        Arc,
    },
    thread,
    time::Duration,
};

use core_logic::*;

mod message_to_string;
use message_to_string::*;

mod tcp_server;
use tcp_server::*;

const PROMPT: &str = "\n> ";

const SHORT_MESSAGE_DELAY: Duration = Duration::from_millis(333);
const LONG_MESSAGE_DELAY: Duration = Duration::from_millis(666);

/// Whether the game should be run as a server or just locally.
const SERVER_MODE: bool = true;

/// The amount of time after which a player is considered to be AFK.
const AFK_TIMEOUT: Duration = Duration::from_secs(90);

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "info"),
    );

    let game = Game::new(GameOptions {
        afk_timeout: Some(AFK_TIMEOUT),
    });

    if SERVER_MODE {
        start_server(game).await
    } else {
        setup_local(game)
    }
}

fn setup_local(mut game: Game) -> Result<()> {
    let (commands_sender, messages_receiver) = game.add_player("Player".to_string());

    let quitting = Arc::new(AtomicBool::new(false));
    let quitting_for_thread = Arc::clone(&quitting);

    thread::Builder::new()
        .name("message receiver".to_string())
        .spawn(move || loop {
            let (message, game_time) = match messages_receiver.recv() {
                Ok(x) => x,
                Err(_) => {
                    debug!("Message sender has been dropped");
                    if quitting_for_thread.load(atomic::Ordering::Relaxed) {
                        break;
                    }
                    panic!("Disconnected from game")
                }
            };
            debug!("Got message: {message:?}");
            let delay = delay_for_message(&message);
            render_message(message, game_time).unwrap();
            thread::sleep(delay);
        })?;

    let mut input_buf = String::new();
    loop {
        print!("{PROMPT}");
        stdin().read_line(&mut input_buf)?;
        debug!("Raw input: {input_buf:?}");
        let input = input_buf.trim();
        debug!("Trimmed input: {input:?}");

        if input == "quit" {
            quitting.store(true, atomic::Ordering::Relaxed);
            println!("ok bye");
            return Ok(());
        }

        commands_sender
            .send(input.to_string())
            .expect("Command receiver should exist");

        input_buf.clear();
    }
}

/// Determines the amount of time to wait after displaying the provided message.
fn delay_for_message(message: &GameMessage) -> Duration {
    let delay = match message {
        GameMessage::Message { delay, .. } => Some(delay),
        GameMessage::ValueChange(_, delay) => Some(delay),
        _ => None,
    };

    if let Some(delay) = delay {
        match delay {
            MessageDelay::None => Duration::ZERO,
            MessageDelay::Short => SHORT_MESSAGE_DELAY,
            MessageDelay::Long => LONG_MESSAGE_DELAY,
        }
    } else {
        Duration::ZERO
    }
}

/// Renders the provided `GameMessage` to the screen.
fn render_message(message: GameMessage, time: Time) -> Result<()> {
    let output = message_to_string(message, Some(time));

    stdout()
        .queue(Clear(ClearType::CurrentLine))?
        .queue(cursor::MoveToColumn(0))?
        .queue(Print(output))?
        .queue(Print("\n"))?
        .queue(Print(PROMPT))?
        .flush()?;

    Ok(())
}
