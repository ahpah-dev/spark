//! The game engine: it takes a [`Game`] and runs it.
//!
//! Nothing in this file draws anything or opens a window, which makes it easy to test.
//! Call [`Runtime::update`] every frame with the current [`Input`]. Positions are measured in
//! grid squares. After each update, call [`Runtime::take_events`] to find out which sounds to play.
//!
//! How a frame works:
//! 1. every object moves, following its [`Behavior`] (in small steps so nothing slips through walls)
//! 2. the rules look at what happened (touching, keys, timers...) and run their actions

use crate::data::*;
use crate::sound::SoundId;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// How strongly gravity pulls, in squares per second squared.
pub const GRAVITY: f32 = 36.0;
const MAX_FALL_SPEED: f32 = 22.0;
const EPS: f32 = 1e-4;
const COYOTE_TIME: f32 = 0.10;
const JUMP_BUFFER: f32 = 0.12;
const GLIDE_SPEED: f32 = 11.0;

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }
}

/// A rectangle used for collisions.
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Aabb {
    pub fn overlaps(&self, o: &Aabb) -> bool {
        self.x < o.x + o.w && self.x + self.w > o.x && self.y < o.y + o.h && self.y + self.h > o.y
    }
    pub fn grow(&self, m: f32) -> Aabb {
        Aabb { x: self.x - m, y: self.y - m, w: self.w + 2.0 * m, h: self.h + 2.0 * m }
    }
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

/// What buttons the player is pressing right now.
#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    held: u16,
    pressed: u16,
    /// Restart button went down this frame (used on the win / game over screens).
    pub restart_pressed: bool,
}

fn bit(k: Key) -> u16 {
    1 << (k as u16)
}

impl Input {
    /// These keys are being held down (but did not just go down).
    pub fn hold(keys: &[Key]) -> Input {
        let mut i = Input::default();
        for k in keys {
            i.held |= bit(*k);
        }
        i
    }

    /// These keys just went down (and are held).
    pub fn tap(keys: &[Key]) -> Input {
        let mut i = Input::hold(keys);
        for k in keys {
            i.pressed |= bit(*k);
        }
        i
    }

    pub fn set_held(&mut self, k: Key, down: bool) {
        if down { self.held |= bit(k) } else { self.held &= !bit(k) }
    }

    pub fn set_pressed(&mut self, k: Key, down: bool) {
        if down { self.pressed |= bit(k) } else { self.pressed &= !bit(k) }
    }

    pub fn is_held(&self, k: Key) -> bool {
        self.held & bit(k) != 0
    }

    pub fn is_pressed(&self, k: Key) -> bool {
        self.pressed & bit(k) != 0
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    Playing,
    /// Level finished. Counts down, then the next level starts.
    LevelDone(f32),
    Won,
    GameOver,
}

/// Something the front-end should react to (play a sound...).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Sound(SoundId),
}

/// One thing in the running world.
#[derive(Clone, Debug)]
pub struct Obj {
    pub uid: u32,
    /// Index into `game.types`.
    pub ty: usize,
    pub tid: TypeId,
    /// Top-left corner of the object's box, in squares.
    pub pos: Vec2,
    pub vel: Vec2,
    /// Which way it faces (a unit-ish vector).
    pub face: Vec2,
    pub alive: bool,
    pub color: Rgb,
    pub behavior: Behavior,
    pub speed: f32,
    pub text: String,
    /// Where it started the level.
    pub home: Vec2,
    /// For the little squash animation of springs.
    pub squash: f32,
    pub size: Vec2,
    pub on_ground: bool,
    hit_off: Vec2,
    hit: Vec2,
    dir: f32,
    coyote: f32,
    jump_buf: f32,
    bounced: bool,
    glide: Option<Vec2>,
    wander_t: f32,
    step_cd: f32,
}

impl Obj {
    /// Which way a patrolling object is walking (+1 or -1).
    pub fn dir(&self) -> f32 {
        self.dir
    }
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub color: Rgb,
    pub size: f32,
}

#[derive(Clone, Copy, Default)]
struct Ctx {
    me: Option<usize>,
    other: Option<usize>,
}

/// A running game.
pub struct Runtime {
    pub game: Game,
    rules: Rc<Vec<Rule>>,
    pub level: usize,
    pub width: i32,
    pub height: i32,
    pub sky: Rgb,
    pub objs: Vec<Obj>,
    pub score: i32,
    pub lives: i32,
    /// Counter values, in the same order as `game.vars`.
    pub vars: Vec<i32>,
    pub state: State,
    pub particles: Vec<Particle>,
    /// Seconds since the game began.
    pub time: f32,
    /// Seconds since this level began.
    pub level_time: f32,
    /// Message from a sign the player is standing next to.
    pub sign_text: Option<String>,
    /// A message from a "say something" action, and how long it stays.
    pub banner: Option<(String, f32)>,
    /// Seconds of "can't be hurt" left after losing a life.
    pub invulnerable: f32,
    events: Vec<Event>,
    solid: Vec<bool>,
    solids_dirty: bool,
    next_uid: u32,
    touching: HashSet<(u32, u32)>,
    timers: HashMap<(usize, u32), f32>,
    started: bool,
    once: Vec<bool>,
    initial_counts: HashMap<TypeId, usize>,
    last_score: i32,
    last_vars: Vec<i32>,
    jumped: Vec<usize>,
    pending_level: Option<usize>,
    win_after: bool,
    rng: u32,
}

fn approach(v: f32, target: f32, step: f32) -> f32 {
    if v < target { (v + step).min(target) } else { (v - step).max(target) }
}

/// (top open, bottom open): can this kind of object leave through the top / bottom of the level?
fn edges(b: Behavior) -> (bool, bool) {
    match b {
        Behavior::Platformer | Behavior::Bounce => (false, true),
        Behavior::Fall => (true, true),
        _ => (false, false),
    }
}

impl Runtime {
    /// Start a game at the given level (0 is the first level).
    pub fn new(game: &Game, start_level: usize) -> Runtime {
        let mut game = game.clone();
        if game.levels.is_empty() {
            let l = game.starter_level("Level 1");
            game.levels.push(l);
        }
        let level = start_level.min(game.levels.len() - 1);
        let lives = game.lives.max(1);
        let vars: Vec<i32> = game.vars.iter().map(|v| v.start).collect();
        let mut rt = Runtime {
            rules: Rc::new(game.rules.clone()),
            level,
            width: 0,
            height: 0,
            sky: [150, 210, 255],
            objs: Vec::new(),
            score: 0,
            lives,
            last_vars: vars.clone(),
            vars,
            state: State::Playing,
            particles: Vec::new(),
            time: 0.0,
            level_time: 0.0,
            sign_text: None,
            banner: None,
            invulnerable: 0.0,
            events: Vec::new(),
            solid: Vec::new(),
            solids_dirty: false,
            next_uid: 1,
            touching: HashSet::new(),
            timers: HashMap::new(),
            started: false,
            once: Vec::new(),
            initial_counts: HashMap::new(),
            last_score: 0,
            jumped: Vec::new(),
            pending_level: None,
            win_after: false,
            rng: 0x2545_F491,
            game,
        };
        rt.load_level(level);
        rt
    }

    fn load_level(&mut self, index: usize) {
        self.level = index;
        let lvl = self.game.levels[index].clone();
        self.width = lvl.width;
        self.height = lvl.height;
        self.sky = lvl.sky;
        self.objs.clear();
        self.particles.clear();
        self.sign_text = None;
        self.touching.clear();
        self.timers.clear();
        self.started = false;
        self.level_time = 0.0;
        self.once = vec![false; self.rules.len()];
        self.jumped.clear();
        for t in &lvl.things {
            let Some(ti) = self.game.ty_index(t.ty) else { continue };
            let mut o = self.make_obj(ti, Vec2::new(t.x as f32, t.y as f32));
            if let Some(c) = t.color {
                o.color = c;
            }
            if let Some(b) = t.behavior {
                o.behavior = b;
                self.set_initial_velocity(&mut o);
            }
            if let Some(s) = t.speed {
                o.speed = s.max(0.1);
                self.set_initial_velocity(&mut o);
            }
            o.text = t.text.clone();
            self.objs.push(o);
        }
        self.initial_counts.clear();
        for o in &self.objs {
            *self.initial_counts.entry(o.tid).or_insert(0) += 1;
        }
        self.rebuild_solids();
        self.state = State::Playing;
    }

    fn make_obj(&mut self, ti: usize, pos: Vec2) -> Obj {
        let t = &self.game.types[ti];
        let size = Vec2::new(t.size_w.max(0.1), t.size_h.max(0.1));
        let hit = Vec2::new((t.hit_w.clamp(0.1, 1.0)) * size.x, (t.hit_h.clamp(0.1, 1.0)) * size.y);
        let hit_off = Vec2::new((size.x - hit.x) / 2.0, if t.hit_bottom { size.y - hit.y } else { (size.y - hit.y) / 2.0 });
        let mut o = Obj {
            uid: self.next_uid,
            ty: ti,
            tid: t.id,
            pos,
            vel: Vec2::default(),
            face: Vec2::new(1.0, 0.0),
            alive: true,
            color: t.color,
            behavior: t.behavior,
            speed: t.speed.max(0.1),
            text: String::new(),
            home: pos,
            squash: 0.0,
            size,
            on_ground: false,
            hit_off,
            hit,
            dir: 1.0,
            coyote: 0.0,
            jump_buf: 0.0,
            bounced: false,
            glide: None,
            wander_t: 0.0,
            step_cd: 0.0,
        };
        self.next_uid += 1;
        self.set_initial_velocity(&mut o);
        o
    }

    fn set_initial_velocity(&mut self, o: &mut Obj) {
        let drift = self.game.types[o.ty].drift;
        match o.behavior {
            Behavior::Drift => {
                let v = match drift {
                    Dir::Up => Vec2::new(0.0, -1.0),
                    Dir::Left => Vec2::new(-1.0, 0.0),
                    Dir::Right => Vec2::new(1.0, 0.0),
                    Dir::Random => self.random_axis(),
                    _ => Vec2::new(0.0, 1.0),
                };
                o.vel = Vec2::new(v.x * o.speed, v.y * o.speed);
            }
            Behavior::Bounce => {
                let s = o.speed * std::f32::consts::FRAC_1_SQRT_2;
                o.vel = Vec2::new(s, -s);
            }
            _ => {}
        }
    }

    fn rebuild_solids(&mut self) {
        self.solid = vec![false; (self.width * self.height).max(0) as usize];
        for o in &self.objs {
            if !o.alive || o.behavior != Behavior::Solid {
                continue;
            }
            let x0 = o.pos.x.floor() as i32;
            let y0 = o.pos.y.floor() as i32;
            let x1 = (o.pos.x + o.size.x - EPS).floor() as i32;
            let y1 = (o.pos.y + o.size.y - EPS).floor() as i32;
            for y in y0..=y1 {
                for x in x0..=x1 {
                    if x >= 0 && y >= 0 && x < self.width && y < self.height {
                        self.solid[(y * self.width + x) as usize] = true;
                    }
                }
            }
        }
        self.solids_dirty = false;
    }

    // -----------------------------------------------------------------------
    // Little helpers
    // -----------------------------------------------------------------------

    /// Take the list of things that happened (like sounds to play) since the last call.
    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    /// How many objects of this type are still around.
    pub fn count_left(&self, tid: TypeId) -> usize {
        self.objs.iter().filter(|o| o.alive && o.tid == tid).count()
    }

    pub fn hitbox(&self, i: usize) -> Aabb {
        let o = &self.objs[i];
        Aabb { x: o.pos.x + o.hit_off.x, y: o.pos.y + o.hit_off.y, w: o.hit.x, h: o.hit.y }
    }

    fn first_player(&self) -> Option<usize> {
        self.objs.iter().position(|o| o.alive && o.behavior.controlled())
    }

    fn rand(&mut self) -> f32 {
        // xorshift: a tiny random number maker. Gives a number from 0 to 1.
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }

    fn random_axis(&mut self) -> Vec2 {
        match (self.rand() * 4.0) as u32 {
            0 => Vec2::new(0.0, -1.0),
            1 => Vec2::new(0.0, 1.0),
            2 => Vec2::new(-1.0, 0.0),
            _ => Vec2::new(1.0, 0.0),
        }
    }

    fn var_pos(&self, id: VarId) -> Option<usize> {
        self.game.vars.iter().position(|v| v.id == id)
    }

    // -----------------------------------------------------------------------
    // Updating
    // -----------------------------------------------------------------------

    /// Move the game forward by `dt` seconds.
    pub fn update(&mut self, dt: f32, input: &Input) {
        let dt = dt.clamp(0.0, 0.1);
        self.update_particles(dt);
        if let Some((_, t)) = &mut self.banner {
            *t -= dt;
            if *t <= 0.0 {
                self.banner = None;
            }
        }

        match self.state {
            State::Playing => {
                self.time += dt;
                self.level_time += dt;
                if input.is_pressed(Key::Space) || input.is_pressed(Key::Up) {
                    for o in &mut self.objs {
                        if o.behavior == Behavior::Platformer {
                            o.jump_buf = JUMP_BUFFER;
                        }
                    }
                }
                // Small steps keep collisions accurate even when the frame is slow.
                let steps = ((dt / (1.0 / 120.0)).ceil() as usize).max(1);
                let h = dt / steps as f32;
                for _ in 0..steps {
                    self.move_all(h, input);
                }
                self.update_signs();
                self.process_rules(dt, input);
                self.cleanup();
                self.invulnerable = (self.invulnerable - dt).max(0.0);
                if let Some(l) = self.pending_level.take() {
                    if self.state == State::Playing {
                        self.load_level(l);
                    }
                }
            }
            State::LevelDone(t) => {
                let t = t - dt;
                if t > 0.0 {
                    self.state = State::LevelDone(t);
                } else if !self.win_after && self.level + 1 < self.game.levels.len() {
                    self.load_level(self.level + 1);
                } else {
                    self.state = State::Won;
                }
            }
            State::Won | State::GameOver => {
                if input.restart_pressed {
                    let game = self.game.clone();
                    *self = Runtime::new(&game, 0);
                }
            }
        }
    }

    fn cleanup(&mut self) {
        if self.objs.iter().any(|o| !o.alive) {
            self.objs.retain(|o| o.alive);
        }
        if self.solids_dirty {
            self.rebuild_solids();
        }
    }

    // -----------------------------------------------------------------------
    // Movement
    // -----------------------------------------------------------------------

    fn move_all(&mut self, h: f32, input: &Input) {
        for i in 0..self.objs.len() {
            if !self.objs[i].alive {
                continue;
            }
            self.objs[i].squash = (self.objs[i].squash - h * 2.5).max(0.0);
            if self.glide_step(i, h) {
                continue;
            }
            match self.objs[i].behavior {
                Behavior::Platformer => self.move_platformer(i, h, input),
                Behavior::TopDown => self.move_top_down(i, h, input),
                Behavior::Sideways => self.move_slider(i, h, input, true),
                Behavior::Vertical => self.move_slider(i, h, input, false),
                Behavior::Grid => self.move_grid(i, h, input),
                Behavior::PatrolH => self.move_patrol(i, h, true),
                Behavior::PatrolV => self.move_patrol(i, h, false),
                Behavior::Wander => self.move_wander(i, h),
                Behavior::Follow => self.move_follow(i, h),
                Behavior::Fall => self.move_fall(i, h),
                Behavior::Drift => {
                    let o = &mut self.objs[i];
                    o.pos.x += o.vel.x * h;
                    o.pos.y += o.vel.y * h;
                    if o.vel.x != 0.0 || o.vel.y != 0.0 {
                        let l = (o.vel.x * o.vel.x + o.vel.y * o.vel.y).sqrt();
                        o.face = Vec2::new(o.vel.x / l, o.vel.y / l);
                    }
                }
                Behavior::Bounce => self.move_bounce(i, h),
                Behavior::Still | Behavior::Solid | Behavior::Pushable => {}
            }
        }
    }

    /// Is this square blocked for object `i`? Level edges block, except where the object may leave.
    fn blocked_cell(&self, i: usize, tx: i32, ty: i32) -> bool {
        let (top_open, bottom_open) = edges(self.objs[i].behavior);
        if tx < 0 || tx >= self.width {
            return true;
        }
        if ty < 0 {
            return !top_open;
        }
        if ty >= self.height {
            return !bottom_open;
        }
        self.solid[(ty * self.width + tx) as usize]
    }

    fn box_cells(x: f32, y: f32, w: f32, h: f32) -> (i32, i32, i32, i32) {
        ((x + EPS).floor() as i32, (x + w - EPS).floor() as i32, (y + EPS).floor() as i32, (y + h - EPS).floor() as i32)
    }

    /// Would object `i` be stuck in something if its top-left corner were at (px, py)?
    fn blocked_at(&self, i: usize, px: f32, py: f32) -> bool {
        let o = &self.objs[i];
        let (x0, x1, y0, y1) = Self::box_cells(px + o.hit_off.x, py + o.hit_off.y, o.hit.x, o.hit.y);
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.blocked_cell(i, tx, ty) {
                    return true;
                }
            }
        }
        false
    }

    /// Move horizontally. Returns true if it bumped into something.
    fn move_x(&mut self, i: usize, dx: f32) -> bool {
        self.objs[i].pos.x += dx;
        let o = &self.objs[i];
        let (x0, x1, y0, y1) = Self::box_cells(o.pos.x + o.hit_off.x, o.pos.y + o.hit_off.y, o.hit.x, o.hit.y);
        let (hw, ox) = (o.hit.x, o.hit_off.x);
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.blocked_cell(i, tx, ty) {
                    if dx > 0.0 {
                        self.objs[i].pos.x = tx as f32 - hw - ox;
                    } else if dx < 0.0 {
                        self.objs[i].pos.x = tx as f32 + 1.0 - ox;
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Move vertically. Returns true if it bumped into something.
    fn move_y(&mut self, i: usize, dy: f32) -> bool {
        self.objs[i].pos.y += dy;
        let o = &self.objs[i];
        let (x0, x1, y0, y1) = Self::box_cells(o.pos.x + o.hit_off.x, o.pos.y + o.hit_off.y, o.hit.x, o.hit.y);
        let (hh, oy) = (o.hit.y, o.hit_off.y);
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.blocked_cell(i, tx, ty) {
                    if dy > 0.0 {
                        self.objs[i].pos.y = ty as f32 - hh - oy;
                    } else if dy < 0.0 {
                        self.objs[i].pos.y = ty as f32 + 1.0 - oy;
                    }
                    return true;
                }
            }
        }
        false
    }

    fn move_platformer(&mut self, i: usize, h: f32, input: &Input) {
        let jump_height = self.game.types[self.objs[i].ty].jump_height.max(0.5);
        let speed = self.objs[i].speed.max(0.5);
        let dir = (input.is_held(Key::Right) as i32 - input.is_held(Key::Left) as i32) as f32;
        let jump_held = input.is_held(Key::Space) || input.is_held(Key::Up);
        let o = &mut self.objs[i];
        if dir != 0.0 {
            o.face = Vec2::new(dir, 0.0);
        }
        let accel = if o.on_ground { 90.0 } else { 55.0 };
        o.vel.x = approach(o.vel.x, dir * speed, accel * h);

        // Gravity (a bit stronger when you let go of jump, so short taps make short hops).
        let mut g = GRAVITY;
        if o.vel.y < 0.0 && !jump_held && !o.bounced {
            g *= 2.2;
        }
        o.vel.y = (o.vel.y + g * h).min(MAX_FALL_SPEED);

        // Jumping, with a little forgiveness ("coyote time" and "jump buffering").
        o.coyote = if o.on_ground { COYOTE_TIME } else { o.coyote - h };
        o.jump_buf -= h;
        let mut jumped = false;
        if o.jump_buf > 0.0 && o.coyote > 0.0 {
            o.vel.y = -(2.0 * GRAVITY * jump_height).sqrt();
            o.jump_buf = 0.0;
            o.coyote = 0.0;
            o.on_ground = false;
            o.bounced = false;
            jumped = true;
        }
        let (dx, dy) = (o.vel.x * h, o.vel.y * h);
        if jumped {
            self.jumped.push(i);
            let c = Vec2::new(self.objs[i].pos.x + self.objs[i].size.x / 2.0, self.objs[i].pos.y + self.objs[i].size.y);
            self.burst(c, [230, 230, 240], 5, 2.0, 0.25);
        }
        if self.move_x(i, dx) {
            self.objs[i].vel.x = 0.0;
        }
        if dy > 0.0 {
            self.objs[i].on_ground = false;
        }
        if self.move_y(i, dy) {
            if dy > 0.0 {
                self.objs[i].on_ground = true;
                self.objs[i].bounced = false;
            }
            self.objs[i].vel.y = 0.0;
        }
    }

    fn move_top_down(&mut self, i: usize, h: f32, input: &Input) {
        let speed = self.objs[i].speed.max(0.5);
        let mut dx = (input.is_held(Key::Right) as i32 - input.is_held(Key::Left) as i32) as f32;
        let mut dy = (input.is_held(Key::Down) as i32 - input.is_held(Key::Up) as i32) as f32;
        let len = (dx * dx + dy * dy).sqrt();
        if len > 0.0 {
            dx /= len;
            dy /= len;
            self.objs[i].face = Vec2::new(dx, dy);
        }
        let o = &mut self.objs[i];
        o.vel.x = approach(o.vel.x, dx * speed, 100.0 * h);
        o.vel.y = approach(o.vel.y, dy * speed, 100.0 * h);
        let (mx, my) = (o.vel.x * h, o.vel.y * h);
        if self.move_x(i, mx) {
            self.objs[i].vel.x = 0.0;
        }
        if self.move_y(i, my) {
            self.objs[i].vel.y = 0.0;
        }
    }

    /// Slide along one line only (spaceships, paddles).
    fn move_slider(&mut self, i: usize, h: f32, input: &Input, horizontal: bool) {
        let speed = self.objs[i].speed.max(0.5);
        let (neg, pos) = if horizontal { (Key::Left, Key::Right) } else { (Key::Up, Key::Down) };
        let d = (input.is_held(pos) as i32 - input.is_held(neg) as i32) as f32;
        let o = &mut self.objs[i];
        if d != 0.0 {
            o.face = if horizontal { Vec2::new(d, 0.0) } else { Vec2::new(0.0, d) };
        }
        if horizontal {
            o.vel.x = approach(o.vel.x, d * speed, 120.0 * h);
            let dx = o.vel.x * h;
            if self.move_x(i, dx) {
                self.objs[i].vel.x = 0.0;
            }
        } else {
            o.vel.y = approach(o.vel.y, d * speed, 120.0 * h);
            let dy = o.vel.y * h;
            if self.move_y(i, dy) {
                self.objs[i].vel.y = 0.0;
            }
        }
    }

    /// Objects that are sliding to a new square (grid hopping and pushed crates).
    fn glide_step(&mut self, i: usize, h: f32) -> bool {
        let Some(t) = self.objs[i].glide else { return false };
        let o = &mut self.objs[i];
        let speed = if o.behavior == Behavior::Grid { o.speed.max(GLIDE_SPEED * 0.6) } else { GLIDE_SPEED };
        let (dx, dy) = (t.x - o.pos.x, t.y - o.pos.y);
        let dist = (dx * dx + dy * dy).sqrt();
        let step = speed * h;
        if dist <= step {
            o.pos = t;
            o.glide = None;
        } else {
            o.pos.x += dx / dist * step;
            o.pos.y += dy / dist * step;
        }
        true
    }

    fn move_grid(&mut self, i: usize, h: f32, input: &Input) {
        self.objs[i].step_cd -= h;
        let keys = [(Key::Left, -1, 0), (Key::Right, 1, 0), (Key::Up, 0, -1), (Key::Down, 0, 1)];
        let mut step = keys.iter().find(|(k, _, _)| input.is_pressed(*k)).map(|(_, dx, dy)| (*dx, *dy));
        if step.is_none() && self.objs[i].step_cd <= 0.0 {
            step = keys.iter().find(|(k, _, _)| input.is_held(*k)).map(|(_, dx, dy)| (*dx, *dy));
        }
        let Some((dx, dy)) = step else { return };
        self.objs[i].step_cd = 0.14;
        self.objs[i].face = Vec2::new(dx as f32, dy as f32);
        let (cx, cy) = (self.objs[i].pos.x.round() as i32, self.objs[i].pos.y.round() as i32);
        let (tx, ty) = (cx + dx, cy + dy);
        if self.blocked_cell(i, tx, ty) {
            return;
        }
        // Is there a crate in the way?
        if let Some(j) = self.pushable_at(tx, ty) {
            let (bx, by) = (tx + dx, ty + dy);
            if self.blocked_cell(j, bx, by) || self.pushable_at(bx, by).is_some() {
                return;
            }
            self.objs[j].glide = Some(Vec2::new(bx as f32, by as f32));
        }
        self.objs[i].glide = Some(Vec2::new(tx as f32, ty as f32));
    }

    fn pushable_at(&self, x: i32, y: i32) -> Option<usize> {
        self.objs.iter().position(|o| {
            o.alive && o.behavior == Behavior::Pushable && {
                let (px, py) = match o.glide {
                    Some(g) => (g.x.round() as i32, g.y.round() as i32),
                    None => (o.pos.x.round() as i32, o.pos.y.round() as i32),
                };
                px == x && py == y
            }
        })
    }

    fn move_patrol(&mut self, i: usize, h: f32, horizontal: bool) {
        let (pos, dir, speed) = (self.objs[i].pos, self.objs[i].dir, self.objs[i].speed);
        if horizontal {
            let nx = pos.x + dir * speed * h;
            if self.blocked_at(i, nx, pos.y) {
                self.objs[i].dir = -dir;
            } else {
                self.objs[i].pos.x = nx;
            }
            self.objs[i].face = Vec2::new(self.objs[i].dir, 0.0);
        } else {
            let ny = pos.y + dir * speed * h;
            if self.blocked_at(i, pos.x, ny) {
                self.objs[i].dir = -dir;
            } else {
                self.objs[i].pos.y = ny;
            }
            self.objs[i].face = Vec2::new(1.0, 0.0);
        }
    }

    fn move_wander(&mut self, i: usize, h: f32) {
        self.objs[i].wander_t -= h;
        if self.objs[i].wander_t <= 0.0 {
            let stand = self.rand() < 0.2;
            let v = self.random_axis();
            let t = 0.5 + self.rand();
            let s = self.objs[i].speed;
            let o = &mut self.objs[i];
            o.wander_t = t;
            o.vel = if stand { Vec2::default() } else { Vec2::new(v.x * s, v.y * s) };
            if v.x != 0.0 && !stand {
                o.face = Vec2::new(v.x, 0.0);
            }
        }
        let (dx, dy) = (self.objs[i].vel.x * h, self.objs[i].vel.y * h);
        let hit_x = dx != 0.0 && self.move_x(i, dx);
        let hit_y = dy != 0.0 && self.move_y(i, dy);
        if hit_x || hit_y {
            self.objs[i].wander_t = 0.0;
        }
    }

    fn move_follow(&mut self, i: usize, h: f32) {
        let Some(p) = self.first_player() else { return };
        let pb = self.hitbox(p).center();
        let o = &mut self.objs[i];
        let c = Vec2::new(o.pos.x + o.size.x / 2.0, o.pos.y + o.size.y / 2.0);
        let (vx, vy) = (pb.x - c.x, pb.y - c.y);
        let len = (vx * vx + vy * vy).sqrt();
        if len > 0.05 {
            o.pos.x += vx / len * o.speed * h;
            o.pos.y += vy / len * o.speed * h;
            o.face = Vec2::new(if vx >= 0.0 { 1.0 } else { -1.0 }, 0.0);
        }
    }

    fn move_fall(&mut self, i: usize, h: f32) {
        self.objs[i].vel.y = (self.objs[i].vel.y + GRAVITY * h).min(MAX_FALL_SPEED);
        let dy = self.objs[i].vel.y * h;
        if self.move_y(i, dy) {
            self.objs[i].vel.y = 0.0;
        }
    }

    fn move_bounce(&mut self, i: usize, h: f32) {
        let speed = self.objs[i].speed;
        let (dx, dy) = (self.objs[i].vel.x * h, self.objs[i].vel.y * h);
        if self.move_x(i, dx) {
            self.objs[i].vel.x = -self.objs[i].vel.x;
        }
        if self.move_y(i, dy) {
            self.objs[i].vel.y = -self.objs[i].vel.y;
        }
        // Keep the speed steady.
        let o = &mut self.objs[i];
        let l = (o.vel.x * o.vel.x + o.vel.y * o.vel.y).sqrt();
        if l > 0.01 {
            o.vel.x *= speed / l;
            o.vel.y *= speed / l;
        }
    }

    fn update_signs(&mut self) {
        let mut text = None;
        'outer: for p in 0..self.objs.len() {
            if !self.objs[p].alive || !self.objs[p].behavior.controlled() {
                continue;
            }
            let pb = self.hitbox(p).grow(0.25);
            for j in 0..self.objs.len() {
                let o = &self.objs[j];
                if o.alive && self.game.types[o.ty].talks && !o.text.trim().is_empty() && pb.overlaps(&self.hitbox(j)) {
                    text = Some(o.text.clone());
                    break 'outer;
                }
            }
        }
        self.sign_text = text;
    }

    // -----------------------------------------------------------------------
    // Rules
    // -----------------------------------------------------------------------

    fn process_rules(&mut self, dt: f32, input: &Input) {
        let rules = self.rules.clone();

        // The level just started.
        if !self.started {
            self.started = true;
            for r in rules.iter() {
                if r.trigger == Trigger::Start {
                    self.fire(r, Ctx::default());
                }
            }
        }

        // Somebody jumped.
        for i in std::mem::take(&mut self.jumped) {
            for r in rules.iter() {
                if matches!(r.trigger, Trigger::Jumps(t) if self.objs[i].tid == t) {
                    self.fire(r, Ctx { me: Some(i), other: None });
                }
            }
        }

        // Timers and keys.
        for (ri, r) in rules.iter().enumerate() {
            match &r.trigger {
                Trigger::Every { secs, who } => {
                    let secs = secs.max(0.05);
                    match who {
                        None => {
                            if self.tick_timer((ri, 0), dt, secs) {
                                self.fire(r, Ctx::default());
                            }
                        }
                        Some(t) => {
                            let list: Vec<usize> = (0..self.objs.len()).filter(|&i| self.objs[i].alive && self.objs[i].tid == *t).collect();
                            for i in list {
                                if self.objs[i].alive && self.tick_timer((ri, self.objs[i].uid), dt, secs) {
                                    self.fire(r, Ctx { me: Some(i), other: None });
                                }
                            }
                        }
                    }
                }
                Trigger::Key { key, mode, who } => {
                    let active = match mode {
                        Press::Pressed => input.is_pressed(*key),
                        Press::Held => input.is_held(*key),
                    };
                    if !active {
                        continue;
                    }
                    match who {
                        None => self.fire(r, Ctx::default()),
                        Some(t) => {
                            let list: Vec<usize> = (0..self.objs.len()).filter(|&i| self.objs[i].alive && self.objs[i].tid == *t).collect();
                            for i in list {
                                if self.objs[i].alive {
                                    self.fire(r, Ctx { me: Some(i), other: None });
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        self.process_touches(&rules);

        // Things that fly (or fall) out of the level. Players are put back at the start instead of vanishing.
        let (w, h) = (self.width as f32, self.height as f32);
        for i in 0..self.objs.len() {
            let o = &self.objs[i];
            if !o.alive {
                continue;
            }
            let out = o.pos.x + o.size.x < -1.0 || o.pos.x > w + 1.0 || o.pos.y + o.size.y < -1.0 || o.pos.y > h + 1.0;
            if !out {
                continue;
            }
            for r in rules.iter() {
                if matches!(r.trigger, Trigger::LeavesLevel(t) if self.objs[i].tid == t) {
                    self.fire(r, Ctx { me: Some(i), other: None });
                }
            }
            if self.objs[i].behavior.controlled() {
                let o = &mut self.objs[i];
                o.pos = o.home;
                o.vel = Vec2::default();
                o.glide = None;
            } else {
                self.objs[i].alive = false;
                if self.objs[i].behavior == Behavior::Solid {
                    self.solids_dirty = true;
                }
            }
        }

        // "None left", "after N seconds", score and counters.
        for (ri, r) in rules.iter().enumerate() {
            match &r.trigger {
                Trigger::NoneLeft(t) => {
                    if !self.once[ri] && self.initial_counts.get(t).copied().unwrap_or(0) > 0 && self.count_left(*t) == 0 {
                        self.once[ri] = true;
                        self.fire(r, Ctx::default());
                    }
                }
                Trigger::AfterSeconds(s) => {
                    if !self.once[ri] && self.level_time >= *s {
                        self.once[ri] = true;
                        self.fire(r, Ctx::default());
                    }
                }
                Trigger::Score(n) => {
                    if self.last_score < *n && self.score >= *n {
                        self.fire(r, Ctx::default());
                    }
                }
                Trigger::Counter { var, n } => {
                    if let Some(p) = self.var_pos(*var) {
                        if self.last_vars.get(p).copied().unwrap_or(0) < *n && self.vars[p] >= *n {
                            self.fire(r, Ctx::default());
                        }
                    }
                }
                _ => {}
            }
        }
        // Remember the values for next time. (Rules above may have changed them again; that is fine.)
        self.last_score = self.score;
        self.last_vars = self.vars.clone();
    }

    fn tick_timer(&mut self, key: (usize, u32), dt: f32, secs: f32) -> bool {
        if !self.timers.contains_key(&key) {
            let start = self.rand() * secs;
            self.timers.insert(key, start);
        }
        let t = self.timers.get_mut(&key).unwrap();
        *t += dt;
        if *t >= secs {
            *t %= secs;
            true
        } else {
            false
        }
    }

    fn process_touches(&mut self, rules: &Rc<Vec<Rule>>) {
        let mut pairs: HashSet<(TypeId, TypeId)> = HashSet::new();
        for r in rules.iter() {
            if let Trigger::Touch { a, b } | Trigger::StopTouch { a, b } = r.trigger {
                pairs.insert((a, b));
            }
        }
        if pairs.is_empty() {
            self.touching.clear();
            return;
        }
        let mut by_type: HashMap<TypeId, Vec<usize>> = HashMap::new();
        for (i, o) in self.objs.iter().enumerate() {
            if o.alive {
                by_type.entry(o.tid).or_default().push(i);
            }
        }
        let mut now: HashSet<(u32, u32)> = HashSet::new();
        for (a, b) in &pairs {
            let (Some(la), Some(lb)) = (by_type.get(a), by_type.get(b)) else { continue };
            for &i in la {
                let ha = self.hitbox(i).grow(0.01);
                for &j in lb {
                    if i != j && ha.overlaps(&self.hitbox(j)) {
                        now.insert((self.objs[i].uid, self.objs[j].uid));
                    }
                }
            }
        }
        let index_of: HashMap<u32, usize> = self.objs.iter().enumerate().map(|(i, o)| (o.uid, i)).collect();
        let entered: Vec<(u32, u32)> = now.difference(&self.touching).copied().collect();
        let left: Vec<(u32, u32)> = self.touching.difference(&now).copied().collect();
        self.touching = now;

        let mut events: Vec<(bool, (u32, u32))> = entered.into_iter().map(|p| (true, p)).chain(left.into_iter().map(|p| (false, p))).collect();
        // Make the order stable (the sets are in random order).
        events.sort_by_key(|(enter, (a, b))| (!*enter, *a, *b));
        for (enter, (ua, ub)) in events {
            let (Some(&i), Some(&j)) = (index_of.get(&ua), index_of.get(&ub)) else { continue };
            if !self.objs[i].alive || !self.objs[j].alive {
                continue;
            }
            let (ta, tb) = (self.objs[i].tid, self.objs[j].tid);
            for r in rules.iter() {
                let hit = match r.trigger {
                    Trigger::Touch { a, b } => enter && a == ta && b == tb,
                    Trigger::StopTouch { a, b } => !enter && a == ta && b == tb,
                    _ => false,
                };
                if hit && self.objs[i].alive && self.objs[j].alive {
                    self.fire(r, Ctx { me: Some(i), other: Some(j) });
                }
            }
        }
    }

    fn fire(&mut self, rule: &Rule, ctx: Ctx) {
        if !self.conditions_ok(rule, ctx) {
            return;
        }
        for a in &rule.actions {
            self.exec(a, ctx);
        }
    }

    fn conditions_ok(&mut self, rule: &Rule, ctx: Ctx) -> bool {
        for c in &rule.conditions {
            let ok = match c {
                Condition::FromAbove => self.from_above(ctx),
                Condition::NotFromAbove => !self.from_above(ctx),
                Condition::NoneLeft(t) => self.count_left(*t) == 0,
                Condition::AnyLeft(t) => self.count_left(*t) > 0,
                Condition::ScoreAtLeast(n) => self.score >= *n,
                Condition::CounterAtLeast { var, n } => self.var_pos(*var).is_some_and(|p| self.vars[p] >= *n),
                Condition::Chance(p) => self.rand() * 100.0 < *p as f32,
            };
            if !ok {
                return false;
            }
        }
        true
    }

    /// Did "me" land on "other" from above? (Only makes sense for run-and-jump objects.)
    fn from_above(&self, ctx: Ctx) -> bool {
        let (Some(i), Some(j)) = (ctx.me, ctx.other) else { return false };
        if self.objs[i].behavior != Behavior::Platformer {
            return false;
        }
        let (a, b) = (self.hitbox(i), self.hitbox(j));
        self.objs[i].vel.y > 0.5 && a.y + a.h <= b.y + b.h * 0.65
    }

    fn exec(&mut self, action: &Action, ctx: Ctx) {
        match action {
            Action::AddScore(n) => self.score += n,
            Action::AddLives(n) => {
                self.lives = (self.lives + n).max(0);
                if self.lives == 0 && self.state == State::Playing {
                    self.state = State::GameOver;
                }
            }
            Action::AddCounter { var, n } => {
                if let Some(p) = self.var_pos(*var) {
                    self.vars[p] += n;
                }
            }
            Action::SetCounter { var, n } => {
                if let Some(p) = self.var_pos(*var) {
                    self.vars[p] = *n;
                }
            }
            Action::LoseLife => self.lose_life(),
            Action::NextLevel => self.finish_level(false),
            Action::WinGame => self.finish_level(true),
            Action::RestartLevel => self.pending_level = Some(self.level),
            Action::GameOver => {
                if self.state == State::Playing {
                    self.state = State::GameOver;
                }
            }
            Action::Destroy(t) => {
                let targets: Vec<usize> = match t {
                    Target::Me => ctx.me.into_iter().collect(),
                    Target::Other => ctx.other.into_iter().collect(),
                    Target::All(id) => (0..self.objs.len()).filter(|&i| self.objs[i].tid == *id).collect(),
                };
                for i in targets {
                    self.destroy(i);
                }
            }
            Action::Spawn { ty, at, dir } => self.spawn_from(*ty, *at, *dir, ctx),
            Action::GoToStart => self.respawn_players(),
            Action::Launch(height) => {
                if let Some(i) = ctx.me {
                    if self.objs[i].alive {
                        self.objs[i].vel.y = -(2.0 * GRAVITY * height.max(0.1)).sqrt();
                        self.objs[i].on_ground = false;
                        self.objs[i].bounced = true;
                    }
                }
                if let Some(j) = ctx.other {
                    self.objs[j].squash = 1.0;
                }
            }
            Action::Bounce => {
                if let (Some(i), Some(j)) = (ctx.me, ctx.other) {
                    self.bounce_off(i, j);
                }
            }
            Action::PlaySound(id) => self.events.push(Event::Sound(*id)),
            Action::Say(text) => self.banner = Some((text.clone(), 3.0)),
        }
    }

    fn destroy(&mut self, i: usize) {
        if !self.objs[i].alive {
            return;
        }
        self.objs[i].alive = false;
        if self.objs[i].behavior == Behavior::Solid {
            self.solids_dirty = true;
        }
        let o = &self.objs[i];
        let c = Vec2::new(o.pos.x + o.size.x / 2.0, o.pos.y + o.size.y / 2.0);
        let (col, count) = (o.color, if o.size.x < 0.7 { 4 } else { 10 });
        self.burst(c, col, count, 4.0, 0.5);
    }

    fn lose_life(&mut self) {
        if self.state != State::Playing || self.invulnerable > 0.0 {
            return;
        }
        self.lives -= 1;
        let players: Vec<usize> = (0..self.objs.len()).filter(|&i| self.objs[i].alive && self.objs[i].behavior.controlled()).collect();
        for &i in &players {
            let o = &self.objs[i];
            let c = Vec2::new(o.pos.x + o.size.x / 2.0, o.pos.y + o.size.y / 2.0);
            let col = o.color;
            if c.y < self.height as f32 {
                self.burst(c, col, 18, 5.0, 0.7);
            }
        }
        if self.lives <= 0 {
            self.lives = 0;
            self.state = State::GameOver;
        } else {
            self.respawn_players();
            self.invulnerable = 1.5;
        }
    }

    fn respawn_players(&mut self) {
        for o in &mut self.objs {
            if o.alive && o.behavior.controlled() {
                o.pos = o.home;
                o.vel = Vec2::default();
                o.glide = None;
                o.on_ground = false;
                o.coyote = 0.0;
                o.jump_buf = 0.0;
                o.bounced = false;
            }
        }
    }

    fn finish_level(&mut self, win_game: bool) {
        if self.state != State::Playing {
            return;
        }
        self.state = State::LevelDone(1.4);
        self.win_after = win_game;
        if let Some(p) = self.first_player() {
            let c = self.hitbox(p).center();
            self.burst(c, [255, 230, 90], 30, 7.0, 1.0);
            self.burst(c, [120, 230, 140], 20, 5.0, 1.0);
        }
    }

    fn resolve_dir(&mut self, dir: Dir, me: Option<usize>) -> Option<Vec2> {
        match dir {
            Dir::Default => None,
            Dir::Up => Some(Vec2::new(0.0, -1.0)),
            Dir::Down => Some(Vec2::new(0.0, 1.0)),
            Dir::Left => Some(Vec2::new(-1.0, 0.0)),
            Dir::Right => Some(Vec2::new(1.0, 0.0)),
            Dir::Random => Some(self.random_axis()),
            Dir::Facing => {
                let f = me.map(|i| self.objs[i].face).unwrap_or(Vec2::new(1.0, 0.0));
                // Shooters aim along one of the four main directions.
                Some(if f.x.abs() >= f.y.abs() { Vec2::new(f.x.signum(), 0.0) } else { Vec2::new(0.0, f.y.signum()) })
            }
            Dir::TowardPlayer => {
                let p = self.first_player()?;
                let t = self.hitbox(p).center();
                let from = me.map(|i| Vec2::new(self.objs[i].pos.x + self.objs[i].size.x / 2.0, self.objs[i].pos.y + self.objs[i].size.y / 2.0))?;
                let (vx, vy) = (t.x - from.x, t.y - from.y);
                let l = (vx * vx + vy * vy).sqrt().max(0.001);
                Some(Vec2::new(vx / l, vy / l))
            }
        }
    }

    fn spawn_from(&mut self, ty: TypeId, at: SpawnAt, dir: Dir, ctx: Ctx) {
        let Some(ti) = self.game.ty_index(ty) else { return };
        let (w, h) = (self.width as f32, self.height as f32);
        let center_of = |o: &Obj| Vec2::new(o.pos.x + o.size.x / 2.0, o.pos.y + o.size.y / 2.0);
        let mut o = self.make_obj(ti, Vec2::default());
        let v = self.resolve_dir(dir, ctx.me);
        let c = match at {
            SpawnAt::Me => ctx.me.map(|i| center_of(&self.objs[i])),
            SpawnAt::Other => ctx.other.map(|i| center_of(&self.objs[i])),
            SpawnAt::MyStart => ctx.me.map(|i| Vec2::new(self.objs[i].home.x + self.objs[i].size.x / 2.0, self.objs[i].home.y + self.objs[i].size.y / 2.0)),
            SpawnAt::RandomTop => {
                let x = self.rand() * (w - o.size.x).max(0.0) + o.size.x / 2.0;
                Some(Vec2::new(x, o.size.y / 2.0))
            }
            SpawnAt::RandomAnywhere => {
                let x = self.rand() * (w - o.size.x).max(0.0) + o.size.x / 2.0;
                let y = self.rand() * (h - o.size.y).max(0.0) + o.size.y / 2.0;
                Some(Vec2::new(x, y))
            }
        };
        let Some(mut c) = c else { return };
        // A shot starts at the edge of the shooter, not in the middle of it.
        if at == SpawnAt::Me {
            if let (Some(i), Some(d)) = (ctx.me, v) {
                let me = &self.objs[i];
                c.x += d.x * (me.size.x / 2.0 + o.size.x / 2.0) * 0.8;
                c.y += d.y * (me.size.y / 2.0 + o.size.y / 2.0) * 0.8;
            }
        }
        o.pos = Vec2::new(c.x - o.size.x / 2.0, c.y - o.size.y / 2.0);
        o.home = o.pos;
        if let Some(d) = v {
            if matches!(o.behavior, Behavior::Drift | Behavior::Bounce | Behavior::Fall) {
                o.vel = Vec2::new(d.x * o.speed, d.y * o.speed);
            }
            o.face = d;
        }
        if o.behavior == Behavior::Solid {
            self.solids_dirty = true;
        }
        self.objs.push(o);
    }

    /// Reflect "me" (usually a ball) off "other" (a paddle, a brick...).
    fn bounce_off(&mut self, i: usize, j: usize) {
        let (a, b) = (self.hitbox(i), self.hitbox(j));
        let ox = (a.x + a.w).min(b.x + b.w) - a.x.max(b.x);
        let oy = (a.y + a.h).min(b.y + b.h) - a.y.max(b.y);
        if ox <= 0.0 || oy <= 0.0 {
            return;
        }
        let (ca, cb) = (a.center(), b.center());
        let other_is_player = self.objs[j].behavior.controlled();
        let speed = self.objs[i].speed.max(0.1);
        if ox < oy {
            let away = if ca.x < cb.x { -1.0 } else { 1.0 };
            self.objs[i].vel.x = away * self.objs[i].vel.x.abs().max(0.1);
            self.objs[i].pos.x += away * ox;
        } else {
            let away = if ca.y < cb.y { -1.0 } else { 1.0 };
            self.objs[i].vel.y = away * self.objs[i].vel.y.abs().max(0.1);
            self.objs[i].pos.y += away * oy;
            if other_is_player {
                // Where the ball hits the paddle changes its angle, so you can aim.
                let offset = ((ca.x - cb.x) / (b.w / 2.0).max(0.1)).clamp(-1.0, 1.0);
                self.objs[i].vel.x = offset * speed * 0.8;
            }
        }
        let o = &mut self.objs[i];
        let l = (o.vel.x * o.vel.x + o.vel.y * o.vel.y).sqrt().max(0.01);
        o.vel.x *= speed / l;
        o.vel.y *= speed / l;
    }

    // -----------------------------------------------------------------------
    // Particles (the little sparkles)
    // -----------------------------------------------------------------------

    fn burst(&mut self, at: Vec2, color: Rgb, count: usize, speed: f32, life: f32) {
        for _ in 0..count {
            let angle = self.rand() * std::f32::consts::TAU;
            let s = speed * (0.3 + self.rand() * 0.7);
            let l = life * (0.5 + self.rand() * 0.5);
            let size = 0.06 + self.rand() * 0.1;
            self.particles.push(Particle {
                pos: at,
                vel: Vec2::new(angle.cos() * s, angle.sin() * s - speed * 0.3),
                life: l,
                max_life: l,
                color,
                size,
            });
        }
        if self.particles.len() > 400 {
            let extra = self.particles.len() - 400;
            self.particles.drain(0..extra);
        }
    }

    fn update_particles(&mut self, dt: f32) {
        for p in &mut self.particles {
            p.life -= dt;
            p.vel.y += 14.0 * dt;
            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;
        }
        self.particles.retain(|p| p.life > 0.0);
    }
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
