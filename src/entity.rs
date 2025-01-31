use ratatui::widgets::canvas;
use ratatui::widgets::canvas::{Circle, Rectangle};
#[derive(Clone)]
pub struct Balloon {
    pub circle: Circle,
    pub velocity_y: f64,
}
#[derive(Clone)]
pub struct Brick {
    pub rectangle: Rectangle,
    pub velocity_y: f64,
}

#[derive(Clone)]
pub enum Entity {
    Balloon(Balloon),
    Brick(Brick),
}
// draw


pub trait Drawable {
    fn tick(&mut self);
    fn draw(&self, ctx: &mut canvas::Context);
}


impl Drawable for Balloon {
    fn tick(&mut self) {
        let gravity = 0.10;
        self.velocity_y += gravity;
        self.circle.y += self.velocity_y;

        let bottom_y = self.circle.radius;

        if self.circle.y < bottom_y {
            self.circle.y = bottom_y;
            self.velocity_y = 0.0;
        }
    }

    fn draw(&self, ctx: &mut canvas::Context) {
        ctx.draw(&self.circle);
    }
}

impl Drawable for Entity {
    fn tick(&mut self) {
        match self {
            Entity::Balloon(balloon) => balloon.tick(),
            Entity::Brick(brick) => brick.tick(),
        }
    }

    fn draw(&self, ctx: &mut canvas::Context) {
        match self {
            Entity::Balloon(balloon) => balloon.draw(ctx),
            Entity::Brick(brick) => brick.draw(ctx),
        }
    }
}


impl Drawable for Brick {
    fn tick(&mut self) {
        let gravity = 0.75;
        self.velocity_y += gravity;
        self.rectangle.y -= self.velocity_y;

        let bottom_y = self.rectangle.height;

        if self.rectangle.y <= bottom_y {
            self.rectangle.y = bottom_y;
            self.velocity_y = 0.0;
        }
    }

    fn draw(&self, ctx: &mut canvas::Context) {
        ctx.draw(&self.rectangle);
    }
}
// write some test
