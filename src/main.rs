use ggez::{
    audio::{SoundSource, Source},
    event::{self, EventHandler, MouseButton},
    graphics::{self, Color, Rect, Text},
    input::keyboard::{KeyCode, KeyInput},
    mint::Point2,
    Context, GameResult,
};
use oorandom::Rand32;
use std::collections::VecDeque;

const GRID_SIZE: (i16, i16) = (30, 20);
const TARGET_LENGTH: u32 = (GRID_SIZE.0 * GRID_SIZE.1) as u32;
const GRID_CELL_SIZE: (i16, i16) = (32, 32);
const SCREEN_SIZE: (f32, f32) = (
    GRID_SIZE.0 as f32 * GRID_CELL_SIZE.0 as f32,
    GRID_SIZE.1 as f32 * GRID_CELL_SIZE.1 as f32,
);
const DESIRED_FPS: u32 = 10;

const TITLE_SCREEN: u8 = 1;
const GAMEPLAY: u8 = 2;
const GAME_LOSS: u8 = 3;
const GAME_WIN: u8 = 4;

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

    pub fn random_direction(rng: &mut Rand32) -> Self {
        let rand_num = rng.rand_range(0..4);
        match rand_num {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            _ => Direction::Right,
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
    num_segments: u32,
}

impl Snake {
    pub fn new(pos: GridPosition, direction: Direction) -> Self {
        let mut body = VecDeque::new();
        let pos2 = GridPosition::new_from_move(pos, direction);
        body.push_back(Segment::new((pos2.x, pos2.y).into()));
        let num_segments: u32 = (body.len() + 1) as u32;
        Snake {
            head: Segment::new(pos),
            dir: Direction::inverse(direction),
            body,
            ate: None,
            last_update_dir: Direction::Right,
            next_dir: None,
            num_segments: num_segments,
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
            self.num_segments += 1;
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

    fn get_food_space(&self, rng: &mut Rand32) -> GridPosition {
        let mut possible_positions: VecDeque<GridPosition> = VecDeque::new();
        for x in 0..GRID_SIZE.0 {
            for y in 0..GRID_SIZE.1 {
                let position = GridPosition::new(x, y);
                if !self.body.iter().any(|segment| segment.pos == position)
                    && self.head.pos != position
                {
                    possible_positions.push_back(position);
                }
            }
        }

        let index = rng.rand_range(0..(possible_positions.len() as u32)) as usize;
        possible_positions.get(index).copied().unwrap()
    }
}

struct GameState {
    snake: Snake,
    food: Food,
    rng: Rand32,
    game_state: u8,
    title_screen: OptionScreen,
    loss_screen: OptionScreen,
    win_screen: OptionScreen,
    title_music: Source,
    game_music: Source,
    win_music: Source,
    death_sound: Source,
    loss_music: Source,
    played_death_sound: bool,
}

impl GameState {
    pub fn new(ctx: &mut Context) -> Self {
        let mut seed: [u8; 8] = [0; 8];
        getrandom::getrandom(&mut seed[..]).expect("Could not create RNG seed");
        let mut rng = Rand32::new(u64::from_ne_bytes(seed));

        let snake_pos = GridPosition::random(&mut rng, GRID_SIZE.0, GRID_SIZE.1);
        let random_direction = Direction::random_direction(&mut rng);
        let snake = Snake::new(snake_pos, random_direction);

        let food_pos = snake.get_food_space(&mut rng);

        let title_screen = OptionScreen::new("Snake!", "Start", "Quit");
        let loss_screen = OptionScreen::new("Game Over", "Try Again?", "Quit");
        let win_screen = OptionScreen::new("You Won!", "Restart", "Quit");

        let mut title_music =
            Source::new(ctx, "/snake_jazz.mp3").expect("Could not find snake jazz");
        title_music.set_repeat(true);
        let mut game_music =
            Source::new(ctx, "/megalovania.mp3").expect("Could not find megalovania");
        game_music.set_repeat(true);
        game_music.set_volume(0.3);
        let mut win_music =
            Source::new(ctx, "/congratulations.mp3").expect("Could not find congratulations");
        win_music.set_repeat(true);
        let mut loss_music =
            Source::new(ctx, "/sad_violin.mp3").expect("Could not find sad violin");
        loss_music.set_repeat(true);
        let mut death_sound =
            Source::new(ctx, "/snake.mp3").expect("Could not find snake snake snaaaaake");
        death_sound.set_repeat(false);

        GameState {
            snake,
            food: Food::new(food_pos),
            rng,
            game_state: TITLE_SCREEN,
            title_screen,
            loss_screen,
            win_screen,
            title_music,
            game_music,
            win_music,
            death_sound,
            loss_music,
            played_death_sound: false,
        }
    }

    fn draw_gameplay(&mut self, ctx: &mut Context) -> GameResult {
        if self.title_music.playing() {
            self.title_music.pause();
        }
        if self.death_sound.playing() {
            self.death_sound.pause();
        }
        if self.loss_music.playing() {
            self.loss_music.pause();
        }
        if self.win_music.playing() {
            self.win_music.pause();
        }
        self.played_death_sound = false;
        if !self.game_music.playing() {
            self.game_music.play(ctx)?;
        }

        // First make a clear canvas
        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.0, 1.0, 0.0, 1.0]));

        // Then have the snake and food draw themselves
        self.snake.draw(&mut canvas);
        self.food.draw(&mut canvas);

        // "Flush" the draw commands
        canvas.finish(ctx)?;

        Ok(())
    }

    fn draw_title(&mut self, ctx: &mut Context) -> GameResult {
        if !self.title_music.playing() {
            self.title_music.play(ctx)?;
        }

        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.0, 0.0, 0.0, 1.0]));

        self.title_screen.draw(&mut canvas);

        canvas.finish(ctx)?;

        Ok(())
    }

    fn draw_win(&mut self, ctx: &mut Context) -> GameResult {
        if self.game_music.playing() {
            self.game_music.pause();
        }
        if !self.win_music.playing() {
            self.win_music.play(ctx)?;
        }

        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([0.0, 0.0, 1.0, 1.0]));

        self.win_screen.draw(&mut canvas);

        canvas.finish(ctx)?;

        Ok(())
    }

    fn draw_loss(&mut self, ctx: &mut Context) -> GameResult {
        if self.game_music.playing() {
            self.game_music.pause();
        }
        if !self.death_sound.playing() && !self.played_death_sound {
            self.death_sound.play(ctx)?;
            self.played_death_sound = true;
        }
        if !self.death_sound.playing() && self.played_death_sound {
            if !self.loss_music.playing() {
                self.loss_music.play(ctx)?;
            }
        }

        let mut canvas =
            graphics::Canvas::from_frame(ctx, graphics::Color::from([1.0, 0.0, 0.0, 1.0]));

        self.loss_screen.draw(&mut canvas);

        canvas.finish(ctx)?;

        Ok(())
    }

    fn reset(&mut self) {
        let snake_pos = GridPosition::random(&mut self.rng, GRID_SIZE.0, GRID_SIZE.1);
        let random_direction = Direction::random_direction(&mut self.rng);
        self.snake = Snake::new(snake_pos, random_direction);
        self.food = Food::new(GridPosition::random(
            &mut self.rng,
            GRID_SIZE.0,
            GRID_SIZE.1,
        ));
        self.game_state = GAMEPLAY;
    }
}

impl event::EventHandler<ggez::GameError> for GameState {
    fn update(&mut self, ctx: &mut Context) -> GameResult {
        // built in timer that will cycle only when it is time

        while ctx.time.check_update_time(DESIRED_FPS) {
            match self.game_state {
                TITLE_SCREEN => {
                    if self.title_screen.button1_clicked {
                        self.game_state = GAMEPLAY;
                    } else if self.title_screen.button2_clicked {
                        std::process::exit(0);
                    }
                    self.title_screen.button1_clicked = false;
                    self.title_screen.button2_clicked = false;
                }
                GAME_LOSS => {
                    if self.loss_screen.button1_clicked {
                        self.reset();
                    } else if self.loss_screen.button2_clicked {
                        std::process::exit(0);
                    }
                    self.loss_screen.button1_clicked = false;
                    self.loss_screen.button2_clicked = false;
                }
                GAME_WIN => {
                    if self.win_screen.button1_clicked {
                        self.reset();
                    } else if self.win_screen.button2_clicked {
                        std::process::exit(0);
                    }
                    self.win_screen.button1_clicked = false;
                    self.win_screen.button2_clicked = false;
                }
                GAMEPLAY => {
                    // First update the snake
                    self.snake.update(&self.food);
                    // Check if the snake ate something
                    if let Some(ate) = self.snake.ate {
                        match ate {
                            Ate::Food => {
                                if self.snake.num_segments == TARGET_LENGTH {
                                    self.game_state = GAME_WIN;
                                } else {
                                    self.food.pos = self.snake.get_food_space(&mut self.rng);
                                }
                            }
                            Ate::Itself => {
                                self.game_state = GAME_LOSS;
                            }
                        }
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> GameResult {
        match self.game_state {
            GAMEPLAY => self.draw_gameplay(ctx)?,
            TITLE_SCREEN => self.draw_title(ctx)?,
            GAME_LOSS => self.draw_loss(ctx)?,
            GAME_WIN => self.draw_win(ctx)?,
            _ => (),
        }

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
        match self.game_state {
            GAMEPLAY => {
                // Try to turn the keycode into a direction
                if let Some(dir) = input.keycode.and_then(Direction::from_keycode) {
                    // If success, check if a new direction has been set
                    // and make sure it's different from snake.dir
                    // This is like buffering a new direction before the next one has been made
                    if self.snake.dir != self.snake.last_update_dir
                        && dir.inverse() != self.snake.dir
                    {
                        self.snake.next_dir = Some(dir);
                    } else if dir.inverse() != self.snake.last_update_dir {
                        // If no new direction has been set and it's not the inverse direction
                        // of the previous move, set the snake dir to the new one pressed
                        self.snake.dir = dir;
                    }
                }
            }
            TITLE_SCREEN => match input.keycode {
                Some(KeyCode::Return) => {
                    self.title_screen.button1_clicked = true;
                }
                Some(KeyCode::Escape) => {
                    self.title_screen.button2_clicked = true;
                }
                _ => (),
            },
            GAME_LOSS => match input.keycode {
                Some(KeyCode::Return) => {
                    self.loss_screen.button1_clicked = true;
                }
                Some(KeyCode::Escape) => {
                    self.loss_screen.button2_clicked = true;
                }
                _ => (),
            },
            GAME_WIN => match input.keycode {
                Some(KeyCode::Return) => {
                    self.win_screen.button1_clicked = true;
                }
                Some(KeyCode::Escape) => {
                    self.win_screen.button2_clicked = true;
                }
                _ => (),
            },
            _ => (),
        }

        Ok(())
    }

    fn mouse_button_down_event(
        &mut self,
        _ctx: &mut Context,
        button: MouseButton,
        x: f32,
        y: f32,
    ) -> Result<(), ggez::GameError> {
        if button == MouseButton::Left {
            match self.game_state {
                TITLE_SCREEN => {
                    if self.title_screen.button1.contains(Point2 { x, y }) {
                        self.title_screen.button1_clicked = true;
                    }

                    if self.title_screen.button2.contains(Point2 { x, y }) {
                        self.title_screen.button2_clicked = true;
                    }
                }
                GAME_LOSS => {
                    if self.loss_screen.button1.contains(Point2 { x, y }) {
                        self.loss_screen.button1_clicked = true;
                    }

                    if self.loss_screen.button2.contains(Point2 { x, y }) {
                        self.loss_screen.button2_clicked = true;
                    }
                }
                GAME_WIN => {
                    if self.win_screen.button1.contains(Point2 { x, y }) {
                        self.win_screen.button1_clicked = true;
                    }

                    if self.win_screen.button2.contains(Point2 { x, y }) {
                        self.win_screen.button2_clicked = true;
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }
}

struct OptionScreen {
    title: Text,
    button1: Rect,
    button2: Rect,
    button1_text: Text,
    button2_text: Text,
    button1_clicked: bool,
    button2_clicked: bool,
}

impl OptionScreen {
    fn new(title: &str, button1_text: &str, button2_text: &str) -> Self {
        let title = Text::new(title);
        let button1 = Rect::new(
            SCREEN_SIZE.0 / 2.0 - 100.0,
            SCREEN_SIZE.1 / 2.0 + 50.0,
            SCREEN_SIZE.0 / 8.0,
            SCREEN_SIZE.1 / 10.0,
        );
        let button2 = Rect::new(
            SCREEN_SIZE.0 / 2.0 + 100.0,
            SCREEN_SIZE.1 / 2.0 + 50.0,
            SCREEN_SIZE.0 / 8.0,
            SCREEN_SIZE.1 / 10.0,
        );
        let button1_text = Text::new(button1_text);
        let button2_text = Text::new(button2_text);

        OptionScreen {
            title,
            button1,
            button2,
            button1_text,
            button2_text,
            button1_clicked: false,
            button2_clicked: false,
        }
    }

    fn draw(&self, canvas: &mut graphics::Canvas) {
        canvas.draw(
            &self.title,
            Point2 {
                x: SCREEN_SIZE.0 / 2.0,
                y: SCREEN_SIZE.1 / 2.0 - 100.0,
            },
        );

        canvas.draw(
            &graphics::Quad,
            graphics::DrawParam::new()
                .dest_rect(self.button1)
                .color(Color::WHITE),
        );
        let button1_center = Point2 {
            x: self.button1.x + self.button1.w / 2.0,
            y: self.button1.y + self.button1.h / 2.0,
        };
        canvas.draw(
            &self.button1_text,
            graphics::DrawParam::new()
                .dest(button1_center)
                .color(Color::BLACK),
        );

        canvas.draw(
            &graphics::Quad,
            graphics::DrawParam::new()
                .dest_rect(self.button2)
                .color(Color::WHITE),
        );
        let button1_center = Point2 {
            x: self.button2.x + self.button2.w / 2.0,
            y: self.button2.y + self.button2.h / 2.0,
        };
        canvas.draw(
            &self.button2_text,
            graphics::DrawParam::new()
                .dest(button1_center)
                .color(Color::BLACK),
        );
    }
}

impl EventHandler for OptionScreen {
    fn update(&mut self, _ctx: &mut Context) -> Result<(), ggez::GameError> {
        self.button1_clicked = false;
        self.button2_clicked = false;

        Ok(())
    }

    fn draw(&mut self, _ctx: &mut Context) -> Result<(), ggez::GameError> {
        Ok(())
    }
}

// TODO: Clean up OptionScreens
// TODO: Add audio (title, background, ate a thing, and failure. And Success I guess but im def not getting that lol)
fn main() -> GameResult {
    // setup metadata about the game. Here title and author
    let (mut ctx, event_loop) = ggez::ContextBuilder::new("snake", "Me :)")
        // Here is the title in the bar of the window
        .window_setup(ggez::conf::WindowSetup::default().title("Snake!"))
        // Here is the size of the window
        .window_mode(ggez::conf::WindowMode::default().dimensions(SCREEN_SIZE.0, SCREEN_SIZE.1))
        // Now we build. If it fails it'll panic with the message "Failed to build ggez context"
        .build()?;

    // Make a gamestate
    let state = GameState::new(&mut ctx);
    // Run the jawn
    event::run(ctx, event_loop, state);
}
