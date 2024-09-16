# Async [Ratatui](https://github.com/ratatui/ratatui) Event Loop with  [Immediate Mode Rendering](https://en.wikipedia.org/wiki/Immediate_mode_(computer_graphics)) in Rust
This is a personal learning project demonstrating how to set up an async render loop with immediate-mode rendering in the terminal with [Ratatui](https://github.com/ratatui/ratatui) and Rust.

This uses [event handling](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/) similar to [**The Elm Architecture (TEA)**](https://guide.elm-lang.org/architecture/) to asynchronously handle various events, such as mouse clicks, keyboard input, and rendering frames in an immediate-mode GUI.

![example](example.gif)

## The Terminal User Interface(TUI)
The Tui struct is the core structure that manages the terminal interface and controls the flow of the application. This implementation uses [Tokio](https://github.com/tokio-rs/tokio) for the runtime. Using [`tokio::sync::mpsc`](https://docs.rs/tokio/latest/tokio/sync/mpsc/) to send `Messages` to update the `Model`
```rust
pub struct Tui {
    pub terminal: Terminal<CrosstermBackend<std::io::Stdout>>,
    pub frame_rate: f64,
    pub tick_rate: f64,
    pub event_tx: UnboundedSender<Message>,
    pub event_rx: UnboundedReceiver<Message>,
    pub model: Model,
}
```

## State Management
The Model struct holds the application's current state. In this implementation, state management is central to how the UI is rendered and updated, as it tracks the position of entities, mouse hover locations, and performance data like FPS. The state is continually updated based on user interactions or timed events, and those changes are reflected in the terminal UI during rendering.

```rust
pub struct Model {
    pub hover_pos: (u16, u16),
    pub entities: Vec<Entity>,
    pub hover_entity: Entity,
    pub fps_counter: FpsCounter
}
```
The `Message` enum encapsulates different types of events that can affect the state of the application. Each Message represents an action, such as ticking (for timed updates), rendering, or processing user input like mouse clicks or hover positions.
```rust
pub enum Message {
    Quit,
    Tick,
    Render,
    MouseLeftClick(u16, u16),
    MouseHoverPos(u16, u16),
}
```
## Updating State
The update function is responsible for processing Message events and updating the state (Model) accordingly. For example `Message:Render` ticks the FPS counter and redraws the UI based on the current state.
```rust
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
            let y = self.terminal.size()?.height as f64 - row as f64; // invert to align coordinate system from terminal to canvas widget api

            let clicked_entity = self.model.hover_entity.clone();
            self.model.entities.push(clicked_entity); //push the current shape on cursor hover to be drawn by the UI

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
                    balloon.circle.y = self.terminal.size()?.height as f64 - row as f64; 
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
```
## View
The `view` function is responsible for rendering the current state of the application, as defined in the `Model`. It uses[`ratatui::widgets::canvas`](https://docs.rs/ratatui/latest/ratatui/widgets/canvas/index.html) to draw all entities, such as balloons and bricks, onto the terminal screen. This includes both the dynamic entities (e.g., objects affected by gravity) and any static UI elements.

```rust
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
```

## Asynchronous Event Loop and Rendering
The heart of this terminal-based UI application is the asynchronous event loop managed by the run function in the Tui struct. This loop concurrently handles three core tasks:
- Processing timed ticks that update the state of the entities (such as animations or physics). 
- Handling user input events like mouse clicks, keyboard press, hover interactions. 
- Rendering the terminal UI based on the current application state.

Key Components of the run Function
- **Tick Interval:** The tick interval controls how frequently the state of the entities (e.g., the position of balloons or bricks) is updated. This ensures periodic updates even if no user input occurs, which is useful for animations or timed updates. The rate is determined by tick_rate.

- **Frame Interval:** The frame interval controls how often the UI is redrawn. By separating the update and render cycles, you can have a consistent frame rate for rendering even if the state isn't changing rapidly. This helps ensure smooth visuals.

- **Event Handling:** Input events (keyboard and mouse) are processed asynchronously. This includes handling clicks, movement, and quitting the application by pressing the Esc key. Input events are processed via `crossterm::event::poll` to avoid blocking the main loop. This uses [`tokio::task::spawn_blocking`](https://docs.rs/tokio/latest/tokio/task/fn.spawn_blocking.html) that returns a future we can wait on.

- Tokio's [`select!`](https://docs.rs/tokio/latest/tokio/macro.select.html) Macro: The [`tokio::select!`](https://tokio.rs/tokio/tutorial/select) macro is used to listen for ticks, frames, and input events simultaneously. This ensures that all events are processed as they occur, without blocking the application. The allows the application to handle multiple asynchronous events in parallel.

```rust
    pub async fn run(&mut self) -> Result<()> {
        self.enter()?;
        let tick_rate = Duration::from_secs_f64(1.0 / self.tick_rate);
        let frame_rate = Duration::from_secs_f64(1.0 / self.frame_rate);
        let mut tick_interval = time::interval(tick_rate);
        let mut frame_interval = time::interval(frame_rate);
        loop {
            tokio::select! {
                // Handle ticking for state updates (e.g., entity movement or animation)
                _tick = tick_interval.tick() => {
                    if let Err(e) = self.event_tx.send(Message::Tick) {
                        return Err(anyhow::anyhow!("Failed to tick: {:?}", e));
                    }
                }
                // Handle frame rendering
                _frame = frame_interval.tick() => {
                    if let Err(e) = self.event_tx.send(Message::Render) {
                        return Err(anyhow::anyhow!("Failed to render frame: {:?}", e));
                    }
                }
                 // Handle incoming events. continue the loop the Quit message is received 
                Some(message) = self.event_rx.recv() => {
                    match self.update(message).await? {
                        UpdateCommand::Quit => return {
                            self.exit()?;
                            Ok(())
                        },
                        UpdateCommand::None => continue,
                    }
                }
                // Polling for user input events asynchronously
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
```

## That's it
This approach, running at 30 frames per second and 10 ticks per second, provides a responsive and efficient terminal UI suitable for most applications.
```rust
#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = run_app().await {
        println!("application exited with error: {}", e);
        std::process::exit(1);
    }
    Ok(())
}

pub async fn run_app() -> Result<()> {
    let mut app = Tui::new(30.0, 10.0).context("Failed to initialize the terminal user interface (TUI)")?;
    app.run().await?;
    Ok(())
}
```

## Animating Balloons and Bricks

In the example, two types of shapes are drawn randomly using the rand crate:
- Balloon: Floats upward and away, simulating a gentle upward motion due to gravity. 
- Brick: Falls and crashes to the ground, simulating a heavier object pulled down by gravity.


#### Using the trait `Drawable:`
```rust
pub trait Drawable {
    fn tick(&mut self);
    fn draw(&self, ctx: &mut canvas::Context);
}
```
Implemented shapes like `Balloon` can have hold the state of their `velocity_y` so for each `Tick` event the state of a Balloon shape can be affect by this "physics".
```rust
pub struct Balloon {
    pub circle: Circle,
    pub velocity_y: f64,
}
```
Each `Tick` event updates the vertical position of the Balloon and Brick based on their velocity. Gravity affects the velocity differently for each entity: a Balloon floats slowly upward, while a Brick falls more quickly downward.
```rust
impl Drawable for Balloon {
    fn tick(&mut self) {
        let gravity = 0.10; 
        self.velocity_y += gravity; 
        self.circle.y += self.velocity_y; // Move the balloon upward

        let bottom_y = self.circle.radius; 
        if self.circle.y < bottom_y {
            self.circle.y = bottom_y;  // Stop at the boundary
            self.velocity_y = 0.0; // Halt movement
        }
    }

    fn draw(&self, ctx: &mut canvas::Context) {
        ctx.draw(&self.circle); // Draw the balloon as a circle. This could be a more complicated shape, for simplicity it's just a circle. 
    }
}
```