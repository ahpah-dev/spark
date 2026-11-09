//! The "what is a game?" part of Spark.
//!
//! A [`Game`] is made of:
//! * **object types** ([`ObjectType`]): the kinds of things in your game (a hero, a wall, an alien...).
//!   Each one has a look, a behavior (how it moves) and a size.
//! * **levels** ([`Level`]): grids with [`Thing`]s placed in them.
//! * **rules** ([`Rule`]): "WHEN something happens, (IF something is true), DO these things".
//! * **sounds** and a tune, and a few **counters** (like "Keys").
//!
//! Everything can be saved to a `.spark` file and loaded again.

use crate::sound::{self, Music, Sound, SoundId};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A color as red, green, blue (each 0..=255).
pub type Rgb = [u8; 3];
pub type TypeId = u32;
pub type VarId = u32;

/// Ids of the object types every new game starts with.
pub mod ids {
    use super::TypeId;
    pub const PLAYER: TypeId = 1;
    pub const WALL: TypeId = 2;
    pub const COIN: TypeId = 3;
    pub const ENEMY: TypeId = 4;
    pub const GOAL: TypeId = 5;
    pub const SPIKES: TypeId = 6;
    pub const SPRING: TypeId = 7;
    pub const SIGN: TypeId = 8;
}

// ---------------------------------------------------------------------------
// Looks
// ---------------------------------------------------------------------------

/// The pictures that come with Spark. (You can also draw your own!)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub enum Pic {
    Player,
    Wall,
    Coin,
    Enemy,
    Goal,
    Spikes,
    Spring,
    Sign,
    Ball,
    Bullet,
    Brick,
    Paddle,
    Crate,
    Target,
    Key,
    Door,
    Star,
    Ship,
    Alien,
    Bomb,
    Rock,
    Heart,
    Plain,
}

impl Pic {
    pub const ALL: [Pic; 23] = [
        Pic::Player,
        Pic::Wall,
        Pic::Coin,
        Pic::Enemy,
        Pic::Goal,
        Pic::Spikes,
        Pic::Spring,
        Pic::Sign,
        Pic::Ball,
        Pic::Bullet,
        Pic::Brick,
        Pic::Paddle,
        Pic::Crate,
        Pic::Target,
        Pic::Key,
        Pic::Door,
        Pic::Star,
        Pic::Ship,
        Pic::Alien,
        Pic::Bomb,
        Pic::Rock,
        Pic::Heart,
        Pic::Plain,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Pic::Player => "Buddy",
            Pic::Wall => "Bricks",
            Pic::Coin => "Coin",
            Pic::Enemy => "Monster",
            Pic::Goal => "Flag",
            Pic::Spikes => "Spikes",
            Pic::Spring => "Spring",
            Pic::Sign => "Sign",
            Pic::Ball => "Ball",
            Pic::Bullet => "Bullet",
            Pic::Brick => "Block",
            Pic::Paddle => "Paddle",
            Pic::Crate => "Crate",
            Pic::Target => "Target",
            Pic::Key => "Key",
            Pic::Door => "Door",
            Pic::Star => "Star",
            Pic::Ship => "Spaceship",
            Pic::Alien => "Alien",
            Pic::Bomb => "Bomb",
            Pic::Rock => "Rock",
            Pic::Heart => "Heart",
            Pic::Plain => "Plain square",
        }
    }

    pub fn default_color(self) -> Rgb {
        match self {
            Pic::Player => [80, 140, 255],
            Pic::Wall => [126, 94, 72],
            Pic::Coin => [255, 200, 40],
            Pic::Enemy => [232, 72, 84],
            Pic::Goal => [60, 190, 96],
            Pic::Spikes => [150, 152, 170],
            Pic::Spring => [242, 142, 40],
            Pic::Sign => [176, 126, 76],
            Pic::Ball => [255, 255, 255],
            Pic::Bullet => [255, 236, 120],
            Pic::Brick => [236, 110, 70],
            Pic::Paddle => [90, 200, 255],
            Pic::Crate => [190, 140, 80],
            Pic::Target => [255, 90, 120],
            Pic::Key => [255, 210, 60],
            Pic::Door => [150, 100, 200],
            Pic::Star => [255, 220, 70],
            Pic::Ship => [120, 200, 255],
            Pic::Alien => [120, 230, 120],
            Pic::Bomb => [70, 70, 90],
            Pic::Rock => [140, 140, 150],
            Pic::Heart => [255, 80, 110],
            Pic::Plain => [150, 150, 220],
        }
    }
}

/// A picture you draw yourself, pixel by pixel. It can have a few frames to animate.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Sprite {
    pub w: usize,
    pub h: usize,
    /// Each frame is `w * h` pixels, row by row, as [red, green, blue, alpha]. Alpha 0 means see-through.
    pub frames: Vec<Vec<[u8; 4]>>,
    /// Frames per second when there is more than one frame.
    #[serde(default = "default_fps")]
    pub fps: f32,
    /// Changes whenever the pixels change (used to remember the picture on the graphics card).
    #[serde(skip)]
    pub rev: u64,
}

fn default_fps() -> f32 {
    6.0
}

impl Sprite {
    pub fn new(w: usize, h: usize) -> Sprite {
        let mut s = Sprite { w, h, frames: vec![vec![[0, 0, 0, 0]; w * h]], fps: 6.0, rev: 0 };
        s.refresh();
        s
    }

    pub fn get(&self, frame: usize, x: usize, y: usize) -> [u8; 4] {
        self.frames[frame.min(self.frames.len() - 1)][y * self.w + x]
    }

    pub fn set(&mut self, frame: usize, x: usize, y: usize, c: [u8; 4]) {
        if x < self.w && y < self.h && frame < self.frames.len() {
            self.frames[frame][y * self.w + x] = c;
        }
    }

    /// Recalculate `rev`. Call this after changing pixels.
    pub fn refresh(&mut self) {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |v: u64| {
            h ^= v;
            h = h.wrapping_mul(0x1000_0000_01b3);
        };
        mix(self.w as u64);
        mix(self.h as u64);
        for f in &self.frames {
            for p in f {
                mix(u32::from_le_bytes(*p) as u64);
            }
            mix(0xffff_ffff);
        }
        self.rev = h;
    }

    /// Which frame to show at this time (in seconds).
    pub fn frame_at(&self, time: f32) -> usize {
        if self.frames.len() <= 1 || self.fps <= 0.0 {
            0
        } else {
            (time * self.fps) as usize % self.frames.len()
        }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Art {
    /// One of the pictures that come with Spark (tinted with the object's color).
    Built(Pic),
    /// Your own pixel art.
    Pixels(Sprite),
}

// ---------------------------------------------------------------------------
// Behaviors
// ---------------------------------------------------------------------------

/// A direction. Used for flying things and for things that shoot.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Dir {
    /// "Whatever this object normally does".
    #[default]
    Default,
    Up,
    Down,
    Left,
    Right,
    /// The way the shooter is facing.
    Facing,
    /// Towards the player.
    TowardPlayer,
    Random,
}

impl Dir {
    pub fn label(self) -> &'static str {
        match self {
            Dir::Default => "The normal way",
            Dir::Up => "Up",
            Dir::Down => "Down",
            Dir::Left => "Left",
            Dir::Right => "Right",
            Dir::Facing => "The way I'm facing",
            Dir::TowardPlayer => "Towards the player",
            Dir::Random => "Any direction",
        }
    }

    /// Directions that make sense for something that flies by itself.
    pub const FLIGHT: [Dir; 5] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right, Dir::Random];
    pub const ALL: [Dir; 8] = [Dir::Default, Dir::Up, Dir::Down, Dir::Left, Dir::Right, Dir::Facing, Dir::TowardPlayer, Dir::Random];
}

/// How an object moves and acts. This is what turns a picture into a hero, an enemy or a wall.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Behavior {
    /// Sits there. Great for coins, flags and decorations.
    #[default]
    Still,
    /// A solid block. Things that move can't pass through it.
    Solid,
    /// You control it: run and jump with gravity.
    Platformer,
    /// You control it: walk up, down, left and right.
    TopDown,
    /// You control it: slide left and right only (spaceships, paddles).
    Sideways,
    /// You control it: slide up and down only.
    Vertical,
    /// You control it: hop one square at a time and push crates.
    Grid,
    /// Walks left and right, turning around at walls.
    PatrolH,
    /// Moves up and down, turning around at walls.
    PatrolV,
    /// Wanders around randomly.
    Wander,
    /// A ghost that floats towards you through walls.
    Follow,
    /// Falls down with gravity and lands on solid things.
    Fall,
    /// Flies in a straight line (bullets, bombs, falling stars).
    Drift,
    /// Bounces off walls like a ball.
    Bounce,
    /// A crate that a grid-hopper can push.
    Pushable,
}

impl Behavior {
    pub const ALL: [Behavior; 15] = [
        Behavior::Still,
        Behavior::Solid,
        Behavior::Platformer,
        Behavior::TopDown,
        Behavior::Sideways,
        Behavior::Vertical,
        Behavior::Grid,
        Behavior::PatrolH,
        Behavior::PatrolV,
        Behavior::Wander,
        Behavior::Follow,
        Behavior::Fall,
        Behavior::Drift,
        Behavior::Bounce,
        Behavior::Pushable,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Behavior::Still => "Just sits there",
            Behavior::Solid => "Solid block",
            Behavior::Platformer => "You: run and jump",
            Behavior::TopDown => "You: walk in 4 directions",
            Behavior::Sideways => "You: slide left and right",
            Behavior::Vertical => "You: slide up and down",
            Behavior::Grid => "You: hop one square at a time",
            Behavior::PatrolH => "Walks left and right",
            Behavior::PatrolV => "Moves up and down",
            Behavior::Wander => "Wanders around",
            Behavior::Follow => "Follows you (ghost)",
            Behavior::Fall => "Falls with gravity",
            Behavior::Drift => "Flies in a straight line",
            Behavior::Bounce => "Bounces like a ball",
            Behavior::Pushable => "Crate (can be pushed)",
        }
    }

    /// Is this the thing the player steers with the keyboard?
    pub fn controlled(self) -> bool {
        matches!(self, Behavior::Platformer | Behavior::TopDown | Behavior::Sideways | Behavior::Vertical | Behavior::Grid)
    }

    /// Does the speed setting do anything for this behavior?
    pub fn uses_speed(self) -> bool {
        !matches!(self, Behavior::Still | Behavior::Solid | Behavior::Fall | Behavior::Pushable)
    }
}

// ---------------------------------------------------------------------------
// Object types
// ---------------------------------------------------------------------------

/// A kind of thing in your game. Place as many of each as you like.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ObjectType {
    pub id: TypeId,
    pub name: String,
    pub art: Art,
    /// Tints the built-in pictures.
    pub color: Rgb,
    pub behavior: Behavior,
    /// How fast it moves, in squares per second.
    pub speed: f32,
    /// How high it jumps, in squares (for "run and jump" objects).
    pub jump_height: f32,
    /// Which way flying things fly.
    pub drift: Dir,
    /// How big it is, in squares.
    pub size_w: f32,
    pub size_h: f32,
    /// The part that touches things, as a fraction of its size.
    pub hit_w: f32,
    pub hit_h: f32,
    /// Does the touching part sit at the bottom (true) or the middle (false)?
    pub hit_bottom: bool,
    /// Shows its message when the player is near (like a sign).
    pub talks: bool,
    /// 0 = behind everything, 1 = normal, 2 = in front.
    pub layer: u8,
    /// Only one of these can be placed in each level.
    pub unique: bool,
    /// Show "how many are left" in the score bar.
    pub show_count: bool,
}

impl Default for ObjectType {
    fn default() -> Self {
        ObjectType {
            id: 0,
            name: "Thing".into(),
            art: Art::Built(Pic::Plain),
            color: Pic::Plain.default_color(),
            behavior: Behavior::Still,
            speed: 3.0,
            jump_height: 3.2,
            drift: Dir::Down,
            size_w: 1.0,
            size_h: 1.0,
            hit_w: 1.0,
            hit_h: 1.0,
            hit_bottom: false,
            talks: false,
            layer: 1,
            unique: false,
            show_count: false,
        }
    }
}

impl ObjectType {
    pub fn is_controlled(&self) -> bool {
        self.behavior.controlled()
    }

    pub fn pic(&self) -> Option<Pic> {
        match &self.art {
            Art::Built(p) => Some(*p),
            Art::Pixels(_) => None,
        }
    }
}

/// A ready-made object you can add with one click.
pub struct Template {
    pub name: &'static str,
    pub blurb: &'static str,
    pub make: fn(TypeId) -> ObjectType,
}

fn base(id: TypeId, name: &str, pic: Pic, behavior: Behavior) -> ObjectType {
    ObjectType { id, name: name.into(), art: Art::Built(pic), color: pic.default_color(), behavior, ..Default::default() }
}

pub const TEMPLATES: &[Template] = &[
    Template {
        name: "Hero (runs and jumps)",
        blurb: "For jumping games. Arrow keys move, Space jumps.",
        make: |id| ObjectType { speed: 6.0, hit_w: 0.7, hit_h: 0.9, hit_bottom: true, layer: 2, unique: true, ..base(id, "Hero", Pic::Player, Behavior::Platformer) },
    },
    Template {
        name: "Explorer (walks everywhere)",
        blurb: "For maze and adventure games. Arrow keys move in all directions.",
        make: |id| ObjectType { speed: 5.5, hit_w: 0.7, hit_h: 0.8, layer: 2, unique: true, ..base(id, "Explorer", Pic::Player, Behavior::TopDown) },
    },
    Template {
        name: "Spaceship (slides sideways)",
        blurb: "For shooting games. Left and right move it.",
        make: |id| ObjectType { speed: 9.0, hit_w: 0.8, hit_h: 0.7, layer: 2, unique: true, ..base(id, "Spaceship", Pic::Ship, Behavior::Sideways) },
    },
    Template {
        name: "Paddle (slides sideways)",
        blurb: "A wide paddle for ball games like breakout.",
        make: |id| ObjectType { speed: 11.0, size_w: 3.0, hit_h: 0.5, hit_bottom: true, layer: 2, unique: true, ..base(id, "Paddle", Pic::Paddle, Behavior::Sideways) },
    },
    Template {
        name: "Hopper (one square at a time)",
        blurb: "For puzzle games. Hops one square and can push crates.",
        make: |id| ObjectType { speed: 9.0, hit_w: 0.8, hit_h: 0.8, layer: 2, unique: true, ..base(id, "Hopper", Pic::Player, Behavior::Grid) },
    },
    Template { name: "Wall", blurb: "A solid block that nothing can pass.", make: |id| ObjectType { layer: 0, ..base(id, "Wall", Pic::Wall, Behavior::Solid) } },
    Template { name: "Block", blurb: "A colorful block. Add a rule to make it break!", make: |id| base(id, "Block", Pic::Brick, Behavior::Still) },
    Template {
        name: "Coin",
        blurb: "Something to collect.",
        make: |id| ObjectType { hit_w: 0.6, hit_h: 0.6, show_count: true, ..base(id, "Coin", Pic::Coin, Behavior::Still) },
    },
    Template {
        name: "Star",
        blurb: "A shiny thing to collect.",
        make: |id| ObjectType { hit_w: 0.7, hit_h: 0.7, show_count: true, ..base(id, "Star", Pic::Star, Behavior::Still) },
    },
    Template {
        name: "Heart",
        blurb: "Add a rule that gives an extra life!",
        make: |id| ObjectType { hit_w: 0.7, hit_h: 0.7, ..base(id, "Heart", Pic::Heart, Behavior::Still) },
    },
    Template {
        name: "Key",
        blurb: "Collect it to open doors (use a counter!).",
        make: |id| ObjectType { hit_w: 0.7, hit_h: 0.7, ..base(id, "Key", Pic::Key, Behavior::Still) },
    },
    Template { name: "Door", blurb: "A solid door. A rule can make it open.", make: |id| base(id, "Door", Pic::Door, Behavior::Solid) },
    Template {
        name: "Goal flag",
        blurb: "Touch it to finish the level.",
        make: |id| ObjectType { hit_w: 0.7, ..base(id, "Goal", Pic::Goal, Behavior::Still) },
    },
    Template {
        name: "Target",
        blurb: "A spot to push a crate onto.",
        make: |id| ObjectType { layer: 0, hit_w: 0.6, hit_h: 0.6, ..base(id, "Target", Pic::Target, Behavior::Still) },
    },
    Template {
        name: "Spikes",
        blurb: "Ouch!",
        make: |id| ObjectType { hit_w: 0.7, hit_h: 0.5, hit_bottom: true, ..base(id, "Spikes", Pic::Spikes, Behavior::Still) },
    },
    Template {
        name: "Spring",
        blurb: "Bounces the player up.",
        make: |id| ObjectType { hit_w: 0.8, hit_h: 0.5, hit_bottom: true, ..base(id, "Spring", Pic::Spring, Behavior::Still) },
    },
    Template {
        name: "Sign",
        blurb: "Shows a message you write when the player is near.",
        make: |id| ObjectType { talks: true, hit_h: 1.0, hit_bottom: true, ..base(id, "Sign", Pic::Sign, Behavior::Still) },
    },
    Template {
        name: "Walking monster",
        blurb: "Walks left and right.",
        make: |id| ObjectType { speed: 2.0, hit_w: 0.76, hit_h: 0.76, ..base(id, "Monster", Pic::Enemy, Behavior::PatrolH) },
    },
    Template {
        name: "Ghost",
        blurb: "Floats towards you through walls.",
        make: |id| ObjectType { speed: 1.6, hit_w: 0.7, hit_h: 0.7, ..base(id, "Ghost", Pic::Enemy, Behavior::Follow) },
    },
    Template {
        name: "Wanderer",
        blurb: "Wanders around by itself.",
        make: |id| ObjectType { speed: 2.0, hit_w: 0.76, hit_h: 0.76, ..base(id, "Wanderer", Pic::Enemy, Behavior::Wander) },
    },
    Template {
        name: "Alien",
        blurb: "Moves side to side. Great for shooting games.",
        make: |id| ObjectType { speed: 2.5, hit_w: 0.8, hit_h: 0.7, ..base(id, "Alien", Pic::Alien, Behavior::PatrolH) },
    },
    Template {
        name: "Bullet (flies up)",
        blurb: "Add a rule to shoot it from your ship!",
        make: |id| ObjectType { speed: 14.0, drift: Dir::Up, size_w: 0.4, size_h: 0.6, ..base(id, "Bullet", Pic::Bullet, Behavior::Drift) },
    },
    Template {
        name: "Bomb (falls down)",
        blurb: "Flies down. Hurts the player.",
        make: |id| ObjectType { speed: 6.0, drift: Dir::Down, size_w: 0.5, size_h: 0.5, ..base(id, "Bomb", Pic::Bomb, Behavior::Drift) },
    },
    Template {
        name: "Falling star",
        blurb: "A star that drifts down from the sky.",
        make: |id| ObjectType { speed: 5.0, drift: Dir::Down, size_w: 0.8, size_h: 0.8, ..base(id, "Falling star", Pic::Star, Behavior::Drift) },
    },
    Template {
        name: "Ball",
        blurb: "Bounces off walls. Use a paddle to keep it up!",
        make: |id| ObjectType { speed: 9.0, size_w: 0.6, size_h: 0.6, ..base(id, "Ball", Pic::Ball, Behavior::Bounce) },
    },
    Template { name: "Crate", blurb: "A crate that the hopper can push.", make: |id| base(id, "Crate", Pic::Crate, Behavior::Pushable) },
    Template {
        name: "Falling rock",
        blurb: "Falls with gravity.",
        make: |id| ObjectType { hit_w: 0.8, hit_h: 0.8, ..base(id, "Rock", Pic::Rock, Behavior::Fall) },
    },
    Template { name: "Blank (draw it yourself!)", blurb: "An empty object. Give it a look and a behavior.", make: |id| base(id, "My object", Pic::Plain, Behavior::Still) },
];

/// The object types a new game starts with.
pub fn default_types() -> Vec<ObjectType> {
    use ids::*;
    vec![
        ObjectType { speed: 6.0, hit_w: 0.7, hit_h: 0.9, hit_bottom: true, layer: 2, unique: true, ..base(PLAYER, "Player", Pic::Player, Behavior::Platformer) },
        ObjectType { layer: 0, ..base(WALL, "Wall", Pic::Wall, Behavior::Solid) },
        ObjectType { hit_w: 0.6, hit_h: 0.6, show_count: true, ..base(COIN, "Coin", Pic::Coin, Behavior::Still) },
        ObjectType { speed: 2.0, hit_w: 0.76, hit_h: 0.76, ..base(ENEMY, "Enemy", Pic::Enemy, Behavior::PatrolH) },
        ObjectType { hit_w: 0.7, ..base(GOAL, "Goal", Pic::Goal, Behavior::Still) },
        ObjectType { hit_w: 0.7, hit_h: 0.5, hit_bottom: true, ..base(SPIKES, "Spikes", Pic::Spikes, Behavior::Still) },
        ObjectType { hit_w: 0.8, hit_h: 0.5, hit_bottom: true, ..base(SPRING, "Spring", Pic::Spring, Behavior::Still) },
        ObjectType { talks: true, ..base(SIGN, "Sign", Pic::Sign, Behavior::Still) },
    ]
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

/// Keys the game understands. (Arrow keys and W A S D both work for the directions.)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Key {
    Left,
    Right,
    Up,
    Down,
    Space,
    Z,
    X,
    Enter,
}

impl Key {
    pub const ALL: [Key; 8] = [Key::Space, Key::Left, Key::Right, Key::Up, Key::Down, Key::Z, Key::X, Key::Enter];

    pub fn label(self) -> &'static str {
        match self {
            Key::Left => "Left arrow",
            Key::Right => "Right arrow",
            Key::Up => "Up arrow",
            Key::Down => "Down arrow",
            Key::Space => "Space bar",
            Key::Z => "Z key",
            Key::X => "X key",
            Key::Enter => "Enter key",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Press {
    /// The moment the key goes down.
    Pressed,
    /// All the time the key is held down.
    Held,
}

/// WHEN: what makes a rule run.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Trigger {
    /// When a level begins.
    Start,
    /// Every few seconds. For each object of the type (or just once if `who` is empty).
    Every { secs: f32, who: Option<TypeId> },
    /// When a key is pressed. Runs for each object of the type (or once if `who` is empty).
    Key { key: Key, mode: Press, who: Option<TypeId> },
    /// When `a` starts touching `b`.
    Touch { a: TypeId, b: TypeId },
    /// When `a` stops touching `b`.
    StopTouch { a: TypeId, b: TypeId },
    /// When there are none of this type left.
    NoneLeft(TypeId),
    /// A number of seconds after the level starts.
    AfterSeconds(f32),
    /// When the score gets up to this number.
    Score(i32),
    /// When a counter gets up to this number.
    Counter { var: VarId, n: i32 },
    /// When something of this type jumps.
    Jumps(TypeId),
    /// When something of this type flies out of the level.
    LeavesLevel(TypeId),
}

/// IF: extra checks. All of them must be true.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Condition {
    /// The first object lands on the second from above (like squashing an enemy).
    FromAbove,
    /// The opposite of the above.
    NotFromAbove,
    NoneLeft(TypeId),
    AnyLeft(TypeId),
    ScoreAtLeast(i32),
    CounterAtLeast { var: VarId, n: i32 },
    /// Only sometimes (a percent chance).
    Chance(u32),
}

/// Who an action is about.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Target {
    /// The object the rule is about (the first one in "A touches B", or the one that jumped...).
    Me,
    /// The other object (B in "A touches B").
    Other,
    /// Every object of a type.
    All(TypeId),
}

/// Where a new object appears.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SpawnAt {
    Me,
    Other,
    /// Where "me" started the level.
    MyStart,
    /// Somewhere along the top of the level.
    RandomTop,
    /// Anywhere in the level.
    RandomAnywhere,
}

impl SpawnAt {
    pub const ALL: [SpawnAt; 5] = [SpawnAt::Me, SpawnAt::Other, SpawnAt::MyStart, SpawnAt::RandomTop, SpawnAt::RandomAnywhere];

    pub fn label(self) -> &'static str {
        match self {
            SpawnAt::Me => "on me",
            SpawnAt::Other => "on the other thing",
            SpawnAt::MyStart => "where I started",
            SpawnAt::RandomTop => "along the top",
            SpawnAt::RandomAnywhere => "anywhere",
        }
    }
}

/// DO: what happens.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum Action {
    AddScore(i32),
    AddLives(i32),
    AddCounter { var: VarId, n: i32 },
    SetCounter { var: VarId, n: i32 },
    LoseLife,
    NextLevel,
    RestartLevel,
    GameOver,
    WinGame,
    Destroy(Target),
    Spawn { ty: TypeId, at: SpawnAt, dir: Dir },
    /// Send all the players back to where they started.
    GoToStart,
    /// Jump up this many squares.
    Launch(f32),
    /// Bounce off the other thing, like a ball off a paddle.
    Bounce,
    PlaySound(SoundId),
    /// Show a message on screen for a few seconds.
    Say(String),
}

impl Action {
    pub fn label(&self) -> &'static str {
        match self {
            Action::AddScore(_) => "Change the score",
            Action::AddLives(_) => "Change lives",
            Action::AddCounter { .. } => "Add to a counter",
            Action::SetCounter { .. } => "Set a counter",
            Action::LoseLife => "Lose a life",
            Action::NextLevel => "Finish the level",
            Action::RestartLevel => "Restart the level",
            Action::GameOver => "Game over",
            Action::WinGame => "Win the game",
            Action::Destroy(_) => "Make disappear",
            Action::Spawn { .. } => "Make a new object",
            Action::GoToStart => "Players go back to start",
            Action::Launch(_) => "Jump up high",
            Action::Bounce => "Bounce off it",
            Action::PlaySound(_) => "Play a sound",
            Action::Say(_) => "Say something",
        }
    }

    pub fn same_type(&self, other: &Action) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

/// WHEN `trigger` happens, IF all `conditions` are true, DO all the `actions`.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub trigger: Trigger,
    #[serde(default)]
    pub conditions: Vec<Condition>,
    pub actions: Vec<Action>,
}

impl Rule {
    pub fn new(trigger: Trigger, actions: &[Action]) -> Self {
        Rule { trigger, conditions: Vec::new(), actions: actions.to_vec() }
    }

    pub fn only_if(mut self, c: Condition) -> Self {
        self.conditions.push(c);
        self
    }
}

pub fn default_rules() -> Vec<Rule> {
    use ids::*;
    use sound::ids as snd;
    let touch = |b| Trigger::Touch { a: PLAYER, b };
    vec![
        Rule::new(touch(COIN), &[Action::AddScore(10), Action::Destroy(Target::Other), Action::PlaySound(snd::COIN)]),
        Rule::new(
            touch(ENEMY),
            &[Action::Destroy(Target::Other), Action::AddScore(20), Action::Launch(1.6), Action::PlaySound(snd::BLIP)],
        )
        .only_if(Condition::FromAbove),
        Rule::new(touch(ENEMY), &[Action::LoseLife, Action::PlaySound(snd::HURT)]).only_if(Condition::NotFromAbove),
        Rule::new(touch(SPIKES), &[Action::LoseLife, Action::PlaySound(snd::HURT)]),
        Rule::new(touch(GOAL), &[Action::NextLevel, Action::PlaySound(snd::WIN)]),
        Rule::new(touch(SPRING), &[Action::Launch(7.7), Action::PlaySound(snd::BOING)]),
        Rule::new(Trigger::Jumps(PLAYER), &[Action::PlaySound(snd::JUMP)]),
        Rule::new(Trigger::LeavesLevel(PLAYER), &[Action::LoseLife, Action::PlaySound(snd::HURT)]),
    ]
}

/// A counter, like "Keys" or "Ammo". It shows in the score bar if `show` is set.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Var {
    pub id: VarId,
    pub name: String,
    pub start: i32,
    pub show: bool,
}

// ---------------------------------------------------------------------------
// Levels
// ---------------------------------------------------------------------------

pub const MIN_LEVEL_W: i32 = 8;
pub const MAX_LEVEL_W: i32 = 80;
pub const MIN_LEVEL_H: i32 = 6;
pub const MAX_LEVEL_H: i32 = 40;

/// One object placed in a level, on a grid square.
#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Thing {
    pub ty: TypeId,
    pub x: i32,
    pub y: i32,
    /// `None` means "use the normal color for this type".
    #[serde(default)]
    pub color: Option<Rgb>,
    /// `None` means "behave like the type does".
    #[serde(default)]
    pub behavior: Option<Behavior>,
    #[serde(default)]
    pub speed: Option<f32>,
    /// A message (for things that talk).
    #[serde(default)]
    pub text: String,
}

impl Thing {
    pub fn new(ty: TypeId, x: i32, y: i32) -> Self {
        Thing { ty, x, y, color: None, behavior: None, speed: None, text: String::new() }
    }
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Level {
    pub name: String,
    pub width: i32,
    pub height: i32,
    /// Background (sky) color.
    pub sky: Rgb,
    pub things: Vec<Thing>,
}

impl Level {
    pub fn new(name: &str) -> Self {
        Level { name: name.to_string(), width: 26, height: 15, sky: [150, 210, 255], things: Vec::new() }
    }

    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.width && y < self.height
    }

    /// Change the size of the level. Things that fall outside are removed.
    pub fn resize(&mut self, w: i32, h: i32) {
        self.width = w.clamp(MIN_LEVEL_W, MAX_LEVEL_W);
        self.height = h.clamp(MIN_LEVEL_H, MAX_LEVEL_H);
        let (w, h) = (self.width, self.height);
        self.things.retain(|t| t.x < w && t.y < h);
    }
}

// ---------------------------------------------------------------------------
// The whole game
// ---------------------------------------------------------------------------

fn default_lives() -> i32 {
    3
}
fn default_win_message() -> String {
    "You won! Great job!".to_string()
}
fn default_volume() -> f32 {
    0.8
}
fn default_music_volume() -> f32 {
    0.5
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Game {
    pub title: String,
    #[serde(default = "default_lives")]
    pub lives: i32,
    #[serde(default = "default_win_message")]
    pub win_message: String,
    pub types: Vec<ObjectType>,
    #[serde(default)]
    pub vars: Vec<Var>,
    #[serde(default)]
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub sounds: Vec<Sound>,
    #[serde(default)]
    pub music: Music,
    #[serde(default = "default_music_volume")]
    pub music_volume: f32,
    #[serde(default = "default_volume")]
    pub sfx_volume: f32,
    pub levels: Vec<Level>,
}

/// Letters used when drawing levels with text. See [`Game::level`].
pub const LEGEND: &[(char, TypeId)] = &[
    ('#', ids::WALL),
    ('P', ids::PLAYER),
    ('C', ids::COIN),
    ('E', ids::ENEMY),
    ('G', ids::GOAL),
    ('^', ids::SPIKES),
    ('S', ids::SPRING),
    ('?', ids::SIGN),
];

impl Game {
    /// A new game with the usual objects, rules and sounds, and no levels yet. Add some with [`Game::level`].
    pub fn new(title: &str) -> Self {
        Game {
            title: title.to_string(),
            lives: 3,
            win_message: default_win_message(),
            types: default_types(),
            vars: Vec::new(),
            rules: default_rules(),
            sounds: sound::default_sounds(),
            music: Music::None,
            music_volume: 0.5,
            sfx_volume: 0.8,
            levels: Vec::new(),
        }
    }

    /// A game with one ready-to-play level.
    pub fn blank() -> Self {
        let mut g = Game::new("My Game");
        let l = g.starter_level("Level 1");
        g.levels.push(l);
        g
    }

    // ---- looking things up ----

    pub fn ty(&self, id: TypeId) -> Option<&ObjectType> {
        self.types.iter().find(|t| t.id == id)
    }

    pub fn ty_mut(&mut self, id: TypeId) -> Option<&mut ObjectType> {
        self.types.iter_mut().find(|t| t.id == id)
    }

    pub fn ty_index(&self, id: TypeId) -> Option<usize> {
        self.types.iter().position(|t| t.id == id)
    }

    pub fn type_name(&self, id: TypeId) -> String {
        self.ty(id).map(|t| t.name.clone()).unwrap_or_else(|| "(deleted)".into())
    }

    pub fn type_named(&self, name: &str) -> Option<TypeId> {
        self.types.iter().find(|t| t.name.eq_ignore_ascii_case(name)).map(|t| t.id)
    }

    pub fn sound(&self, id: SoundId) -> Option<&Sound> {
        self.sounds.iter().find(|s| s.id == id)
    }

    pub fn sound_name(&self, id: SoundId) -> String {
        self.sound(id).map(|s| s.name.clone()).unwrap_or_else(|| "(none)".into())
    }

    pub fn var_name(&self, id: VarId) -> String {
        self.vars.iter().find(|v| v.id == id).map(|v| v.name.clone()).unwrap_or_else(|| "(deleted)".into())
    }

    /// The first type that the player steers.
    pub fn player_type(&self) -> Option<TypeId> {
        self.types.iter().find(|t| t.is_controlled()).map(|t| t.id)
    }

    fn next_id(&self) -> TypeId {
        self.types.iter().map(|t| t.id).max().unwrap_or(0) + 1
    }

    // ---- adding and removing ----

    /// Add a new object type. Its id is chosen for you and returned.
    pub fn add_type(&mut self, mut t: ObjectType) -> TypeId {
        t.id = self.next_id();
        let id = t.id;
        self.types.push(t);
        id
    }

    /// Add one of the ready-made objects by (part of) its name, like `"Alien"` or `"Ball"`.
    pub fn add_template(&mut self, name: &str) -> Option<TypeId> {
        let lower = name.to_lowercase();
        let i = TEMPLATES
            .iter()
            .position(|t| t.name.to_lowercase().starts_with(&lower))
            .or_else(|| TEMPLATES.iter().position(|t| t.name.to_lowercase().contains(&lower)))?;
        Some(self.add_template_index(i))
    }

    /// Add the ready-made object number `index` of [`TEMPLATES`].
    pub fn add_template_index(&mut self, index: usize) -> TypeId {
        let id = self.next_id();
        let mut t = (TEMPLATES[index].make)(id);
        // Give a unique name if we already have one with this name.
        let base_name = t.name.clone();
        let mut n = 2;
        while self.types.iter().any(|o| o.name == t.name) {
            t.name = format!("{base_name} {n}");
            n += 1;
        }
        self.types.push(t);
        id
    }

    pub fn add_sound(&mut self, mut s: Sound) -> SoundId {
        s.id = self.sounds.iter().map(|s| s.id).max().unwrap_or(0) + 1;
        let id = s.id;
        self.sounds.push(s);
        id
    }

    pub fn add_var(&mut self, name: &str, start: i32, show: bool) -> VarId {
        let id = self.vars.iter().map(|v| v.id).max().unwrap_or(0) + 1;
        self.vars.push(Var { id, name: name.to_string(), start, show });
        id
    }

    /// Delete an object type, everything of that type in the levels, and rules that depend on it.
    pub fn remove_type(&mut self, id: TypeId) {
        self.types.retain(|t| t.id != id);
        for l in &mut self.levels {
            l.things.retain(|t| t.ty != id);
        }
        self.rules.retain(|r| !r.trigger.mentions(id));
        for r in &mut self.rules {
            r.conditions.retain(|c| !c.mentions(id));
            r.actions.retain(|a| !a.mentions(id));
        }
    }

    pub fn remove_sound(&mut self, id: SoundId) {
        self.sounds.retain(|s| s.id != id);
        for r in &mut self.rules {
            r.actions.retain(|a| *a != Action::PlaySound(id));
        }
    }

    pub fn remove_var(&mut self, id: VarId) {
        self.vars.retain(|v| v.id != id);
        self.rules.retain(|r| !matches!(r.trigger, Trigger::Counter { var, .. } if var == id));
        for r in &mut self.rules {
            r.conditions.retain(|c| !matches!(c, Condition::CounterAtLeast { var, .. } if *var == id));
            r.actions.retain(|a| !matches!(a, Action::AddCounter { var, .. } | Action::SetCounter { var, .. } if *var == id));
        }
    }

    // ---- levels and things ----

    /// A level with a floor, a player and a goal (if the game has those types).
    pub fn starter_level(&self, name: &str) -> Level {
        let mut l = Level::new(name);
        let wall = self.types.iter().find(|t| t.behavior == Behavior::Solid).map(|t| t.id);
        let player = self.player_type();
        let goal = self.type_named("Goal");
        if let Some(w) = wall {
            for x in 0..l.width {
                l.things.push(Thing::new(w, x, l.height - 1));
            }
        }
        if let Some(p) = player {
            l.things.push(Thing::new(p, 2, l.height - 2));
        }
        if let Some(g) = goal {
            l.things.push(Thing::new(g, l.width - 3, l.height - 2));
        }
        l
    }

    /// A new thing of this type, with default text for things that talk.
    pub fn new_thing(&self, ty: TypeId, x: i32, y: i32) -> Thing {
        let mut t = Thing::new(ty, x, y);
        if self.ty(ty).is_some_and(|o| o.talks) {
            t.text = "Hello! Press the arrow keys to move.".to_string();
        }
        t
    }

    /// How big a thing is, in squares.
    pub fn footprint(&self, thing: &Thing) -> (f32, f32) {
        self.ty(thing.ty).map(|t| (t.size_w, t.size_h)).unwrap_or((1.0, 1.0))
    }

    /// The thing in front at this square (big things cover several squares).
    pub fn thing_covering(&self, level: usize, x: i32, y: i32) -> Option<usize> {
        let l = &self.levels[level];
        let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
        let mut best: Option<(usize, u8)> = None;
        for (i, t) in l.things.iter().enumerate() {
            let (w, h) = self.footprint(t);
            if fx >= t.x as f32 && fx < t.x as f32 + w.max(1.0).ceil() && fy >= t.y as f32 && fy < t.y as f32 + h.max(1.0).ceil() {
                let layer = self.ty(t.ty).map(|o| o.layer).unwrap_or(1);
                if best.is_none_or(|(_, bl)| layer >= bl) {
                    best = Some((i, layer));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// Put a thing in a level. It replaces whatever started on that same square.
    /// Types marked "only one" move the old one. Returns false if outside the level.
    pub fn place(&mut self, level: usize, thing: Thing) -> bool {
        if !self.levels[level].in_bounds(thing.x, thing.y) {
            return false;
        }
        let unique = self.ty(thing.ty).is_some_and(|t| t.unique);
        let l = &mut self.levels[level];
        l.things.retain(|t| !(t.x == thing.x && t.y == thing.y));
        if unique {
            l.things.retain(|t| t.ty != thing.ty);
        }
        l.things.push(thing);
        true
    }

    /// Remove the front thing at a square. Returns true if something was removed.
    pub fn erase(&mut self, level: usize, x: i32, y: i32) -> bool {
        match self.thing_covering(level, x, y) {
            Some(i) => {
                self.levels[level].things.remove(i);
                true
            }
            None => false,
        }
    }

    pub fn player_start(&self, level: usize) -> Option<(i32, i32)> {
        self.levels[level].things.iter().find(|t| self.ty(t.ty).is_some_and(|o| o.is_controlled())).map(|t| (t.x, t.y))
    }

    // ---- friendly builder methods (so a game can be written in a few lines) ----

    /// Add a level drawn with letters:
    ///
    /// | letter | meaning                         |
    /// |--------|---------------------------------|
    /// | `#`    | wall                            |
    /// | `P`    | player                          |
    /// | `C`    | coin                            |
    /// | `E`    | enemy that walks left and right |
    /// | `U`    | enemy that moves up and down    |
    /// | `H`    | ghost that follows you          |
    /// | `G`    | goal flag                       |
    /// | `^`    | spikes                          |
    /// | `S`    | spring                          |
    /// | `?`    | sign                            |
    /// | `.`    | empty                           |
    pub fn level(self, name: &str, rows: &[&str]) -> Self {
        self.level_with(name, rows, &[])
    }

    /// Like [`Game::level`], but you can add your own letters: `&[('A', alien_id)]`.
    pub fn level_with(mut self, name: &str, rows: &[&str], legend: &[(char, TypeId)]) -> Self {
        let lvl = self.draw_level(name, rows, legend);
        self.levels.push(lvl);
        self
    }

    /// Build a level from text (see [`Game::level`]).
    pub fn draw_level(&self, name: &str, rows: &[&str], legend: &[(char, TypeId)]) -> Level {
        let mut level = Level::new(name);
        let width = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0) as i32;
        level.width = width.clamp(MIN_LEVEL_W, MAX_LEVEL_W);
        level.height = (rows.len() as i32).clamp(MIN_LEVEL_H, MAX_LEVEL_H);
        // If the picture is smaller than the minimum size, stick it to the bottom.
        let y_offset = (level.height - rows.len() as i32).max(0);
        // We use `place` on a temporary single-level view of the game so that "only one" works.
        let mut scratch = Game { levels: vec![level], ..self.clone() };
        for (ry, row) in rows.iter().enumerate() {
            for (cx, ch) in row.chars().enumerate() {
                let (x, y) = (cx as i32, ry as i32 + y_offset);
                let mut found = legend.iter().find(|(c, _)| *c == ch).map(|(_, id)| (*id, None, None));
                if found.is_none() {
                    found = match ch {
                        'U' | 'u' => Some((ids::ENEMY, Some(Behavior::PatrolV), None)),
                        'H' | 'h' => Some((ids::ENEMY, Some(Behavior::Follow), Some(1.6))),
                        other => {
                            let key = if other.is_alphabetic() { other.to_ascii_uppercase() } else { other };
                            LEGEND.iter().find(|(c, _)| *c == key).map(|(_, id)| (*id, None, None))
                        }
                    };
                }
                if let Some((ty, behavior, speed)) = found {
                    if scratch.ty(ty).is_none() {
                        continue;
                    }
                    let mut t = scratch.new_thing(ty, x, y);
                    t.behavior = behavior;
                    t.speed = speed;
                    scratch.place(0, t);
                }
            }
        }
        scratch.levels.remove(0)
    }

    pub fn lives(mut self, n: i32) -> Self {
        self.lives = n.max(1);
        self
    }

    pub fn win_message(mut self, text: &str) -> Self {
        self.win_message = text.to_string();
        self
    }

    pub fn music(mut self, m: Music) -> Self {
        self.music = m;
        self
    }

    /// Add your own rule. See [`Rule`].
    pub fn rule(mut self, rule: Rule) -> Self {
        self.rules.push(rule);
        self
    }

    /// Replace the rules for "`a` touches `b`" with these actions.
    pub fn when_touching(mut self, a: TypeId, b: TypeId, actions: &[Action]) -> Self {
        self.rules.retain(|r| r.trigger != Trigger::Touch { a, b });
        self.rules.push(Rule::new(Trigger::Touch { a, b }, actions));
        self
    }

    // ---- saving and loading ----

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Load a game from text. Games saved by older versions of Spark are upgraded automatically.
    pub fn from_json(text: &str) -> Result<Game, String> {
        let value: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("That file is not a Spark game ({e})"))?;
        let mut g = if value.get("types").is_some() {
            serde_json::from_value::<Game>(value).map_err(|e| format!("That file is not a Spark game ({e})"))?
        } else {
            crate::legacy::convert(value)?
        };
        g.finish_loading();
        Ok(g)
    }

    fn finish_loading(&mut self) {
        if self.levels.is_empty() {
            let l = self.starter_level("Level 1");
            self.levels.push(l);
        }
        for t in &mut self.types {
            if let Art::Pixels(s) = &mut t.art {
                s.refresh();
            }
        }
        if self.sounds.is_empty() {
            self.sounds = sound::default_sounds();
        }
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            if !dir.as_os_str().is_empty() {
                fs::create_dir_all(dir).map_err(|e| format!("Could not make folder: {e}"))?;
            }
        }
        fs::write(path, self.to_json()).map_err(|e| format!("Could not save: {e}"))
    }

    pub fn load(path: &Path) -> Result<Game, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("Could not open the file: {e}"))?;
        Game::from_json(&text)
    }
}

// ---------------------------------------------------------------------------
// Describing rules in plain words
// ---------------------------------------------------------------------------

impl Trigger {
    /// Does this trigger depend on the given object type?
    pub fn mentions(&self, id: TypeId) -> bool {
        match self {
            Trigger::Every { who, .. } | Trigger::Key { who, .. } => *who == Some(id),
            Trigger::Touch { a, b } | Trigger::StopTouch { a, b } => *a == id || *b == id,
            Trigger::NoneLeft(t) | Trigger::Jumps(t) | Trigger::LeavesLevel(t) => *t == id,
            _ => false,
        }
    }

    pub fn describe(&self, g: &Game) -> String {
        let who = |w: &Option<TypeId>| w.map(|t| format!("each {}", g.type_name(t))).unwrap_or_else(|| "the game".into());
        match self {
            Trigger::Start => "the level starts".into(),
            Trigger::Every { secs, who: w } => format!("every {secs:.1} seconds, for {}", who(w)),
            Trigger::Key { key, mode, who: w } => {
                format!("{} {} {}", if *mode == Press::Pressed { "you press" } else { "you hold" }, key.label(), if w.is_some() { format!("({})", who(w)) } else { String::new() })
            }
            Trigger::Touch { a, b } => format!("{} touches {}", g.type_name(*a), g.type_name(*b)),
            Trigger::StopTouch { a, b } => format!("{} stops touching {}", g.type_name(*a), g.type_name(*b)),
            Trigger::NoneLeft(t) => format!("no {} are left", g.type_name(*t)),
            Trigger::AfterSeconds(s) => format!("{s:.0} seconds have passed"),
            Trigger::Score(n) => format!("the score reaches {n}"),
            Trigger::Counter { var, n } => format!("{} reaches {n}", g.var_name(*var)),
            Trigger::Jumps(t) => format!("{} jumps", g.type_name(*t)),
            Trigger::LeavesLevel(t) => format!("{} leaves the level", g.type_name(*t)),
        }
    }
}

impl Condition {
    pub fn mentions(&self, id: TypeId) -> bool {
        matches!(self, Condition::NoneLeft(t) | Condition::AnyLeft(t) if *t == id)
    }

    pub fn describe(&self, g: &Game) -> String {
        match self {
            Condition::FromAbove => "it lands on top".into(),
            Condition::NotFromAbove => "it does not land on top".into(),
            Condition::NoneLeft(t) => format!("no {} are left", g.type_name(*t)),
            Condition::AnyLeft(t) => format!("there are still {}", g.type_name(*t)),
            Condition::ScoreAtLeast(n) => format!("score is at least {n}"),
            Condition::CounterAtLeast { var, n } => format!("{} is at least {n}", g.var_name(*var)),
            Condition::Chance(p) => format!("{p}% of the time"),
        }
    }
}

impl Action {
    pub fn mentions(&self, id: TypeId) -> bool {
        match self {
            Action::Destroy(Target::All(t)) => *t == id,
            Action::Spawn { ty, .. } => *ty == id,
            _ => false,
        }
    }

    pub fn describe(&self, g: &Game) -> String {
        let target = |t: &Target| match t {
            Target::Me => "me".to_string(),
            Target::Other => "the other thing".to_string(),
            Target::All(id) => format!("every {}", g.type_name(*id)),
        };
        match self {
            Action::AddScore(n) => format!("score {n:+}"),
            Action::AddLives(n) => format!("lives {n:+}"),
            Action::AddCounter { var, n } => format!("{} {n:+}", g.var_name(*var)),
            Action::SetCounter { var, n } => format!("set {} to {n}", g.var_name(*var)),
            Action::LoseLife => "lose a life".into(),
            Action::NextLevel => "finish the level".into(),
            Action::RestartLevel => "restart the level".into(),
            Action::GameOver => "game over".into(),
            Action::WinGame => "win the game".into(),
            Action::Destroy(t) => format!("{} disappears", target(t)),
            Action::Spawn { ty, at, .. } => format!("a {} appears {}", g.type_name(*ty), at.label()),
            Action::GoToStart => "players go back to the start".into(),
            Action::Launch(h) => format!("jump {h:.1} squares up"),
            Action::Bounce => "bounce".into(),
            Action::PlaySound(s) => format!("play '{}'", g.sound_name(*s)),
            Action::Say(t) => format!("say \"{t}\""),
        }
    }
}

/// Turn a game title into something safe to use as a file name.
pub fn safe_file_name(title: &str) -> String {
    let cleaned: String = title.chars().map(|c| if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' { c } else { '_' }).collect();
    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() { "My Game".to_string() } else { cleaned }
}
