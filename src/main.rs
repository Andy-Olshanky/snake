use ggez::{
    event, graphics,
    input::keyboard::{KeyCode, KeyInput},
    Context, GameResult,
};
use oorandom::Rand32;
use std::collections::VecDeque;

const GRID_SIZE: (i16, i16) = (30, 20);
const GRID_CELL_SIZE: (i16, i16) = (32, 32);
const SCREEN_SIZE: (f32, f32) = (
    GRID_SIZE.0 as f32 * GRID_CELL_SIZE.0 as f32,
    GRID_SIZE.1 as f32 * GRID_CELL_SIZE.1 as f32,
);
const DESIRED_FPS: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GridPosition {
    x: i16,
    y: i16,
}

impl GridPosition {
    pub fn new(x: i16, y: i16) -> Self {
        GridPosition { x, y }
    }

    pub fn random(rng: &mut Rand32, max_x: i16, max_y: i16) -> Self {
        // You can use .into() to directly convert the i16 tuple to GridPosition
        // since From<i16, i16> is implemented into GridPosition below
        (
            rng.rand_range(0..(max_x as u32)) as i16,
            rng.rand_range(0..(max_y as u32)) as i16,
        )
            .into()
    }

    // using rem_euclid here since % can give a negative remainder which we don't want
    // rem_euclid only gives positive values (aka what it actually should be...)
    pub fn new_from_move(pos: GridPosition, dir: Direction) -> Self {
        match dir {
            Direction::Up => GridPosition::new(pos.x, (pos.y - 1).rem_euclid(GRID_SIZE.1)),
            Direction::Down => GridPosition::new(pos.x, (pos.y + 1).rem_euclid(GRID_SIZE.1)),
            Direction::Left => GridPosition::new((pos.x - 1).rem_euclid(GRID_SIZE.0), pos.y),
            Direction::Right => GridPosition::new((pos.x + 1).rem_euclid(GRID_SIZE.0), pos.y),
        }
    }
}

// Allows us to easily go from GridPosition to the graphics display
impl From<GridPosition> for graphics::Rect {
    fn from(pos: GridPosition) -> Self {
        graphics::Rect::new_i32(
            pos.x as i32 * GRID_CELL_SIZE.0 as i32,
            pos.y as i32 * GRID_CELL_SIZE.1 as i32,
            GRID_CELL_SIZE.0 as i32,
            GRID_CELL_SIZE.1 as i32,
        )
    }
}

// This allows us to go from (i16, i16) to GridPosition easily
impl From<(i16, i16)> for GridPosition {
    fn from(pos: (i16, i16)) -> Self {
        GridPosition { x: pos.0, y: pos.1 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub fn inverse(self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }

    pub fn from_keycode(key: KeyCode) -> Option<Direction> {
        match key {
            KeyCode::Up => Some(Direction::Up),
            KeyCode::Down => Some(Direction::Down),
            KeyCode::Left => Some(Direction::Left),
            KeyCode::Right => Some(Direction::Right),
            _ => None,
        }
    }
}

// Basically an abstraction for the snake segments
#[derive(Debug, Clone, Copy)]
struct Segment {
    pos: GridPosition,
}

impl Segment {
    pub fn new(pos: GridPosition) -> Self {
        Segment { pos }
    }
}

// Another abstraction but for food
struct Food {
    pos: GridPosition,
}

impl Food {
    pub fn new(pos: GridPosition) -> Self {
        Food { pos }
    }

    // not great for scaling, look up InstanceArray or SpriteBatch for future projects
    fn draw(&self, canvas: &mut graphics::Canvas) {
        // r g b opacity
        let color = [0.0, 0.0, 1.0, 1.0];

        canvas.draw(
            &graphics::Quad,
            graphics::DrawParam::new()
                .dest_rect(self.pos.into())
                .color(color),
        );
    }
}

#[derive(Debug, Clone, Copy)]
enum Ate {
    Itself,
    Food,
}

struct Snake {
    head: Segment,
    dir: Direction,
    body: VecDeque<Segment>,
    ate: Option<Ate>,
    last_update_dir: Direction,
    next_dir: Option<Direction>,
}

impl Snake {
    pub fn new(pos: GridPosition) -> Self {
        let mut body = VecDeque::new();
        body.push_back(Segment::new((pos.x - 1, pos.y).into()));
        Snake {
            head: Segment::new(pos),
            dir: Direction::Right,
            body,
            ate: None,
            last_update_dir: Direction::Right,
            next_dir: None,
        }
    }

    fn eats(&self, food: &Food) -> bool {
        self.head.pos == food.pos
    }

    fn eats_self(&self) -> bool {
        for seg in &self.body {
            if self.head.pos == seg.pos {
                return true;
            }
        }
        false
    }

    fn update(&mut self, food: &Food) {
        // if last_update_dir is the same as dir, and next_dir is a thing, set dir to next_dir
        if self.last_update_dir == self.dir && self.next_dir.is_some() {
            self.dir = self.next_dir.unwrap();
            self.next_dir = None;
        }

        let new_head_pos = GridPosition::new_from_move(self.head.pos, self.dir);
        let new_head = Segment::new(new_head_pos);

        // Add head to the front of the body, then set it to the new head
        self.body.push_front(self.head);
        self.head = new_head;

        // Now check if it ate something
        if self.eats_self() {
            self.ate = Some(Ate::Itself);
        } else if self.eats(food) {
            self.ate = Some(Ate::Food);
        } else {
            self.ate = None;
        }

        // If nothing was eaten, pop the end of the body to make it look like the body moved
        if self.ate.is_none() {
            self.body.pop_back();
        }

        // Finally updated last_update_dir to show where we moved
        self.last_update_dir = self.dir;
    }

    fn draw(&self, canvas: &mut graphics::Canvas) {
        for seg in &self.body {
            canvas.draw(
                &graphics::Quad,
                graphics::DrawParam::new()
                    .dest_rect(seg.pos.into())
                    .color([0.3, 0.3, 0.0, 1.0]),
            );
        }

        canvas.draw(
            &graphics::Quad,
            graphics::DrawParam::new()
                .dest_rect(self.head.pos.into())
                .color([1.0, 0.5, 0.0, 1.0]),
        );
    }
}

struct GameState {
    snake: Snake,
    food: Food,
    gameover: bool,
    rng: Rand32,
}

impl GameState {
    pub fn new() -> Self {
        let snake_pos = (GRID_SIZE.0 / 4, GRID_SIZE.1 / 2).into();

        let mut seed: [u8; 8] = [0; 8];
        getrandom::getrandom(&mut seed[..]).expect("Could not create RNG seed");
        let mut rng = Rand32::new(u64::from_ne_bytes(seed));

        let food_pos = GridPosition::random(&mut rng, GRID_SIZE.0, GRID_SIZE.1);

        GameState {
            snake: Snake::new(snake_pos),
            food: Food::new(food_pos),
            gameover: false,
            rng,
        }
    }
}

impl event::EventHandler<ggez::GameError> for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // built in timer that will cycle only when it is time

        while ctx.time.check_update_time(DESIRED_FPS) {
            if !self.gameover {
                // First update the snake
                self.snake.update(&self.food);
                // Check if the snake ate something
                if let Some(ate) = self.snake.ate {
                    match ate {
                        Ate::Food => {
                            let new_food_pos =
                                GridPosition::random(&mut self.rng, GRID_SIZE.0, GRID_SIZE.1);
                            self.food.pos = new_food_pos;
                        }
                        Ate::Itself => {
                            self.gameover = true;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        // First make a clear canvas
        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.0, 1.0, 0.0, 1.0]));

        // Then have the snake and food draw themselves
        self.snake.draw(&mut canvas);
        self.food.draw(&mut canvas);

        // "Flush" the draw commands
        canvas.finish(ctx)?;

        // Yield the thread until the next update and return success
        ggez::timer::yield_now();
        Ok(())
    }

    // This fires when a key gets pressed
    fn key_down_event(
        &mut self,
        _ctx: &mut Context,
        input: KeyInput,
        _repeated: bool,
    ) -> Result<(), ggez::GameError> {
        // Try to turn the keycode into a direction
        if let Some(dir) = input.keycode.and_then(Direction::from_keycode) {
            // If success, check if a new direction has been set
            // and make sure it's different from snake.dir
            // This is like buffering a new direction before the next one has been made
            if self.snake.dir != self.snake.last_update_dir && dir.inverse() != self.snake.dir {
                self.snake.next_dir = Some(dir);
            } else if dir.inverse() != self.snake.last_update_dir {
                // If no new direction has been set and it's not the inverse direction
                // of the previous move, set the snake dir to the new one pressed
                self.snake.dir = dir;
            }
        }
        Ok(())
    }
}

// TODO: Make sure the food does not collide with the snake when the food is made
// TODO: Add title screen w/ start and quit options
// TODO: Add audio (title, background, ate a thing, and failure. And Success I guess but im def not getting that lol)
// TODO: Add an end screen w/ start over, quit, and game over or you win message depending on end state
fn main() -> GameResult {
    // setup metadata about the game. Here title and author
    let (ctx, event_loop) = ggez::ContextBuilder::new("snake", "Me :)")
    // Here is the title in the bar of the window
    .window_setup(ggez::conf::WindowSetup::default().title("Snake!"))
    // Here is the size of the window
    .window_mode(ggez::conf::WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
    // Now we build. If it fails it'll panic with the message "Failed to build ggez context"
    .build()?;

    // Make a gamestate
    let state = GameState::new();
    // Run the jawn
    event::run(ctx, event_loop, state);
}
