use anyhow::{Context, Result};
use rand::prelude::*;
use ratatui::backend::CrosstermBackend;
use ratatui::crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::style::Color::{Black, Blue, Red};
use ratatui::symbols::Marker;
use ratatui::widgets::canvas::{Canvas, Circle, Rectangle};
use ratatui::widgets::Block;
use ratatui::{crossterm, Terminal};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};
use tokio::time;
mod fps_counter;
mod entity;

use crate::entity::{Balloon, Brick, Drawable, Entity};
use fps_counter::FpsCounter;


pub struct Model {
    pub hover_pos: (u16, u16),
    pub entities: Vec<Entity>,
    pub hover_entity: Entity,
    pub fps_counter: FpsCounter
}
#[derive(Clone, Debug)]
pub enum Message {
    Quit,
    Tick,
    Render,
    MouseLeftClick(u16, u16),
    MouseHoverPos(u16, u16),
}

pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    pub frame_rate: f64,
    pub tick_rate: f64,
    pub event_tx: UnboundedSender<Message>,
    pub event_rx: UnboundedReceiver<Message>,
    pub model: Model,
}

#[derive(Clone, Debug)]
pub enum UpdateCommand {
    None,
    Quit,
}

impl Tui {
    pub fn new(frame_rate: f64, tick_rate: f64) -> Result<Self> {
        let terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Ok(Self {
            terminal,
            frame_rate,
            tick_rate,
            event_tx,
            event_rx,
            model: Model {
                hover_pos: (0, 0),
                entities: Vec::new(),
                fps_counter: FpsCounter::new(),
                hover_entity: {
                    Entity::Balloon(
                        Balloon {
                            circle: Circle {
                                x: 0.0,
                                color: Blue,
                                radius: 1.0,
                                y: 0.0,
                            },
                            velocity_y: 0.0,
                        }
                    )
                },
            },
        })
    }
    fn enter(&mut self) -> Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(std::io::stdout(), EnterAlternateScreen, EnableMouseCapture)?;
        Ok(())
    }

    pub fn exit(&mut self) -> Result<()> {
        if crossterm::terminal::is_raw_mode_enabled()? {
            self.terminal.flush()?;
            crossterm::execute!(std::io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
            crossterm::terminal::disable_raw_mode()?;
            self.terminal.show_cursor()?;
            println!("Terminal exited.");
        }
        Ok(())
    }
    pub async fn run(&mut self) -> Result<()> {
        self.enter()?;
        let tick_rate = Duration::from_secs_f64(1.0 / self.tick_rate);
        let frame_rate = Duration::from_secs_f64(1.0 / self.frame_rate);
        let mut tick_interval = time::interval(tick_rate);
        let mut frame_interval = time::interval(frame_rate);
        loop {
            tokio::select! {
                _tick = tick_interval.tick() => {
                    if let Err(e) = self.event_tx.send(Message::Tick) {
                        return Err(anyhow::anyhow!("Failed to tick: {:?}", e));
                    }
                }
                _frame = frame_interval.tick() => {
                    if let Err(e) = self.event_tx.send(Message::Render) {
                        return Err(anyhow::anyhow!("Failed to render frame: {:?}", e));
                    }
                }
                Some(message) = self.event_rx.recv() => {
                    match self.update(message).await? {
                        UpdateCommand::Quit => return {
                            self.exit()?;
                            Ok(())
                        },
                        UpdateCommand::None => continue,
                    }
                }
                Ok(ready) = tokio::task::spawn_blocking(|| crossterm::event::poll(Duration::from_millis(100))) => {
                    match ready {
                        Ok(true) => {
                            let event = crossterm::event::read()?;
                            if let Err(e) = self.handle_event(event) {
                                return Err(anyhow::anyhow!("Failed to handle event: {:?}", e));
                            }
                        }
                        Ok(false) => continue,
                        Err(e) => {
                                return Err(anyhow::anyhow!("Failed to poll for events: {:?}", e));
                            }
                    }
                }
            }
        }
    }

    fn handle_event(&self, event: Event) -> Result<()> {
        match event {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press  && key.code == KeyCode::Esc{
                    self.event_tx.send(Message::Quit)?;
                }
            }
            Event::Mouse(mouse) => {
                match mouse.kind {
                    MouseEventKind::Down(mb) => {
                        if mb == MouseButton::Left {
                            self.event_tx.send(Message::MouseLeftClick(mouse.row, mouse.column))?;
                        }
                    }
                    MouseEventKind::Moved => {
                        self.event_tx.send(Message::MouseHoverPos(mouse.row, mouse.column))?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    async fn update(&mut self, message: Message) -> Result<UpdateCommand> {
        match message {
            Message::Quit => {
                Ok(UpdateCommand::Quit)
            }
            Message::Tick => {
                for obj in &mut self.model.entities {
                    obj.tick();
                }
                Ok(UpdateCommand::None)
            }
            Message::Render => {
                self.model.fps_counter.tick();
                self.view().context("Failed to render")?;
                Ok(UpdateCommand::None)
            }
            Message::MouseLeftClick(row, col) => {
                let x = col as f64;
                let y = self.terminal.size()?.height as f64 - row as f64;

                let clicked_entity = self.model.hover_entity.clone();
                self.model.entities.push(clicked_entity);

                let new_entity: Entity = if random::<bool>(){
                    Entity::Balloon(Balloon {
                        circle: Circle {
                            x,
                            y,
                            radius: 1.0,
                            color: Blue,
                        },
                        velocity_y: 0.0,
                    })
                } else {
                    Entity::Brick(Brick {
                        rectangle: Rectangle {
                            x,
                            y,
                            width: 1.0,
                            height: 1.0,
                            color: Red,
                        },
                        velocity_y: 0.0,
                    })
                };
                self.model.hover_entity = new_entity;

                Ok(UpdateCommand::None)
            }
            Message::MouseHoverPos(row, col) => {

                self.model.hover_pos = (row, col);
                match &mut self.model.hover_entity {
                    Entity::Balloon(balloon) => {
                        balloon.circle.x = col as f64;
                        balloon.circle.y = self.terminal.size()?.height as f64 - row as f64; //invert to match canvas coord system
                    }
                    Entity::Brick(brick) => {
                        brick.rectangle.x = col as f64;
                        brick.rectangle.y = self.terminal.size()?.height as f64 - row as f64;
                    }
                }
                Ok(UpdateCommand::None)
            }
        }
    }
    fn view(&mut self) -> Result<()> {
        let (term_width, term_height) = self.terminal.size().map(|s| (s.width, s.height))?;

        self.terminal.draw(|f| {
            let screen_area = f.area();

            let x_bounds = [0.0, term_width as f64];
            let y_bounds = [0.0, term_height as f64];


            let content = Canvas::default()
                .block(Block::bordered().title(format!("Esc to Quit. FPS: {}", self.model.fps_counter.fps)))
                .x_bounds(x_bounds)
                .y_bounds(y_bounds)
                .paint(|ctx| {
                    match &self.model.hover_entity {
                        Entity::Balloon(balloon) => {
                            balloon.draw(ctx);
                        }
                        Entity::Brick(brick) => {
                            brick.draw(ctx);
                        }
                    }

                    for entity in &self.model.entities {
                        entity.draw(ctx);
                    }
                    ctx.layer();
                })
                .background_color(Black)
                .marker(Marker::Braille);

            f.render_widget(content, screen_area);
        })?;

        Ok(())
    }
}


impl Drop for Tui {
    fn drop(&mut self) {
        self.exit().expect("Failed to end terminal mode");
    }
}