use crossterm::cursor::{Hide, Show};
use crossterm::event::{Event, KeyCode};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{event, terminal, ExecutableCommand};
use rust_invaders::frame::Drawable;
use rust_invaders::frame::{new_frame, Frame};
use rust_invaders::invaders::Invaders;
use rust_invaders::player::Player;
use rust_invaders::{frame, render};
use rusty_audio::Audio;
use std::error::Error;
use std::io::Stdout;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use std::{io, thread};

fn main() -> Result<(), Box<dyn Error>> {
    let mut audio = initialize_audio();
    let mut stdout = initialize_terminal()?;

    let (render_tx, render_rx) = mpsc::channel();
    let render_handle = thread::spawn(move || render(render_rx));

    game_loop(&mut audio, &render_tx)?;
    cleanup(&mut audio, &mut stdout, render_tx, render_handle)?;
    Ok(())
}

fn initialize_audio() -> Audio {
    let sounds: Vec<&str> = vec!["explode", "lose", "move", "pew", "startup", "win"];
    let mut audio = Audio::new();

    for sound in sounds {
        audio.add(sound, &format!("{}.wav", sound.to_string()))
    }
    audio.play("startup");
    audio
}

fn initialize_terminal() -> Result<Stdout, Box<dyn Error>> {
    let mut stdout = io::stdout();
    terminal::enable_raw_mode()?;
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide);
    Ok(stdout)
}

fn render(render_rx: Receiver<Frame>) {
    let mut last_frame = frame::new_frame();
    let mut stdout = io::stdout();
    render::render(&mut stdout, &last_frame, &last_frame, true);
    loop {
        let curr_frame = match render_rx.recv() {
            Ok(x) => x,
            Err(_) => break,
        };
        render::render(&mut stdout, &last_frame, &curr_frame, false);
        last_frame = curr_frame;
    }
}

fn game_loop(audio: &mut Audio, render_tx: &Sender<Frame>) -> Result<(), Box<dyn Error>> {
    let mut player = Player::new();
    let mut instant = Instant::now();
    let mut invaders = Invaders::new();

    'gameloop: loop {
        //Per-frame init
        let delta = instant.elapsed();
        instant = Instant::now();
        let mut curr_frame = new_frame();

        // Input
        while event::poll(Duration::default())? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Left => player.move_left(),
                    KeyCode::Right => player.move_right(),
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        if player.shoot() {
                            audio.play("pew")
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        audio.play("lose");
                        break 'gameloop;
                    }
                    _ => {}
                }
            }
        }
        // Updates
        player.update(delta);
        if invaders.update(delta) {
            audio.play("move")
        }
        if player.detect_hits(&mut invaders) {
            audio.play("explode")
        }

        // Draw and render
        let drawables: Vec<&dyn Drawable> = vec![&player, &invaders];
        for drawable in drawables {
            drawable.draw(&mut curr_frame)
        }
        let _ = render_tx.send(curr_frame); // ignore errors
        thread::sleep(Duration::from_millis(1));

        // Win or lose?
        if invaders.all_killed() {
            audio.play("win");
            break 'gameloop;
        }
        if invaders.reached_bottom() {
            audio.play("lose");
            break 'gameloop;
        }
    }
    Ok(())
}

fn cleanup(
    audio: &mut Audio,
    stdout: &mut Stdout,
    render_tx: Sender<Frame>,
    render_handle: JoinHandle<()>,
) ->Result<(),Box<dyn Error>> {
    drop(render_tx);
    render_handle.join().unwrap();
    audio.wait();
    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}
