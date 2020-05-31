//! A die widget and associated data

use anyhow::Error;
use crossbeam_channel as channel;
use druid::widget::{Align, Flex, Label, TextBox};
use druid::{
    AppDelegate, AppLauncher, BoxConstraints, Color, Command, Data, DelegateCtx, Env, Event,
    EventCtx, LayoutCtx, Lens, LifeCycle, LifeCycleCtx, LocalizedString, PaintCtx, Rect,
    RenderContext, Selector, Size, Target, TimerToken, UpdateCtx, Widget, WidgetExt, WindowDesc,
    WindowId,
};
use rand::prelude::*;
use std::{convert::TryFrom, thread, time::Duration};

const ROLL_RATE: Duration = Duration::from_millis(100);

/// A value representing a value, for example a die that has stopped moving.
#[derive(Debug, Copy, Clone, PartialEq, Data)]
pub struct Score(pub u8);

impl Score {
    /// Create a new score with the given value.
    pub fn new(score: u8) -> Self {
        Self(score)
    }

    /// Locations of the dice points, used in painting. Only supports 0-6.
    fn points(self) -> &'static [(f64, f64)] {
        match self.0 {
            0 => &[],
            1 => &[(4.0, 4.0)],
            2 => &[(4.0, 3.0), (4.0, 5.0)],
            3 => &[(4.0, 2.0), (4.0, 4.0), (4.0, 6.0)],
            4 => &[(2.0, 2.0), (2.0, 6.0), (6.0, 2.0), (6.0, 6.0)],
            5 => &[(2.0, 2.0), (2.0, 6.0), (6.0, 2.0), (6.0, 6.0), (4.0, 4.0)],
            6 => &[
                (2.0, 2.0),
                (2.0, 4.0),
                (2.0, 6.0),
                (6.0, 2.0),
                (6.0, 4.0),
                (6.0, 6.0),
            ],
            _ => panic!("die score of {} not supported when drawing points", self.0),
        }
    }

    /// Create a Score with a random value between 1 and 6, for a six-sided die.
    pub fn random_die() -> Self {
        Self::random(1, 7)
    }

    /// Create a Score with a random value in the given range.
    pub fn random(low: u8, hi: u8) -> Self {
        // maybe todo: SmallRng
        let mut rng = thread_rng();
        let n: u8 = rng.gen_range(low, hi);
        Self(n)
    }

    /// Create a Score with a random value between 1 and 6, that isn't the current value.
    pub fn different_random_die(self) -> Self {
        self.different_random(1, 7)
    }

    /// Create a Score with a random value in a range, that isn't the current value.
    pub fn different_random(self, low: u8, hi: u8) -> Self {
        let old = self.0;
        assert!(
            low <= old && old <= hi,
            "the previous value {} must be in the range [{}, {}]"
        );
        let mut rng = thread_rng();
        // Smaller range because we are going to shift numbers >= the previous value.
        let n: u8 = rng.gen_range(low, hi - 1);
        Self(if n >= old { n + 1 } else { n })
    }
}

impl From<u8> for Score {
    fn from(val: u8) -> Self {
        Self(val)
    }
}

impl From<Score> for u8 {
    fn from(val: Score) -> Self {
        val.0
    }
}

/// The state of a die - either being rolled or having landed on a value.
#[derive(Debug, Copy, Clone, PartialEq, Data)]
enum DieState {
    Value(Score),
    Rolling,
}

impl DieState {
    fn new(value: u8) -> Self {
        Self::Value(Score(value))
    }

    fn is_rolling(&self) -> bool {
        match self {
            DieState::Rolling => true,
            _ => false,
        }
    }
}

/// The data required to render the Die widget.
#[derive(Debug, Copy, Clone, PartialEq, Data)]
pub struct DieData {
    /// Whether the die is being rolled or has stopped on a value.
    state: DieState,
    /// Whether the die should be displayed bright or not.
    ///
    /// Not bright can be used to indicate that the die is not selected, for example for re-rolls.
    pub bright: bool,
}

impl DieData {
    pub fn new(value: u8) -> Self {
        Self {
            state: DieState::new(value),
            bright: true,
        }
    }

    pub fn is_rolling(&self) -> bool {
        self.state.is_rolling()
    }

    pub fn value(&self) -> Option<Score> {
        match self.state {
            DieState::Value(v) => Some(v),
            _ => None,
        }
    }

    pub fn set_rolling(&mut self) -> &mut Self {
        self.state = DieState::Rolling;
        self
    }

    pub fn set_value(&mut self, value: impl Into<Score>) -> &mut Self {
        self.state = DieState::Value(value.into());
        self
    }

    pub fn bright(&self) -> bool {
        self.bright
    }

    pub fn set_bright(&mut self, bright: bool) -> &mut Self {
        self.bright = bright;
        self
    }
}

pub struct Die {
    rolling_timer: Option<TimerToken>,
    rolling_score: Score,
}

impl Die {
    pub fn new() -> Self {
        Self {
            rolling_timer: None,
            rolling_score: Score::random_die(),
        }
    }

    pub fn score(&self, data: &DieData) -> Score {
        match data.state {
            DieState::Value(score) => score,
            DieState::Rolling => self.rolling_score,
        }
    }
}

impl Widget<DieData> for Die {
    fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut DieData, _env: &Env) {
        match event {
            Event::Timer(tok) if self.rolling_timer.map(|t| t == *tok).unwrap_or(false) => {
                if data.is_rolling() {
                    self.rolling_score = self.rolling_score.different_random_die();
                    self.rolling_timer = Some(ctx.request_timer(ROLL_RATE));
                }
                ctx.request_paint();
            }
            _ => (),
        }
    }

    fn lifecycle(
        &mut self,
        _ctx: &mut LifeCycleCtx,
        event: &LifeCycle,
        _data: &DieData,
        _env: &Env,
    ) {
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &DieData, data: &DieData, _env: &Env) {
        match (data.is_rolling(), old_data.is_rolling()) {
            (true, false) => {
                // Setup the rolling effect.
                self.rolling_timer = Some(ctx.request_timer(ROLL_RATE));
            }
            (false, true) => {
                // Stop rolling effect on next tick (don't redraw yet).
            }
            (false, false) => {
                // Draw the new number
                ctx.request_paint();
            }
            (true, true) => {
                // Do nothing (continue to roll)
            }
        }
    }

    fn layout(
        &mut self,
        _ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &DieData,
        _env: &Env,
    ) -> Size {
        const SIZE: f64 = 9.0 * 4.0;
        bc.constrain((SIZE, SIZE))
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &DieData, env: &Env) {
        let score = self.score(data);
        let size = ctx.size();
        let bg = Rect::ZERO.with_size(size);
        let x_unit = size.width / 9.0;
        let y_unit = size.height / 9.0;

        let square = |xy: (f64, f64)| {
            let (x, y) = xy;
            Rect::new(
                x * x_unit,
                y * y_unit,
                (x + 1.0) * x_unit,
                (y + 1.0) * y_unit,
            )
        };

        let white_b = ctx.solid_brush(Color::WHITE);
        let black_b = ctx.solid_brush(Color::BLACK);

        // border & background
        ctx.fill(bg, &white_b);
        ctx.fill(bg.inset((-x_unit, -y_unit)), &black_b);
        for pt in score.points() {
            ctx.fill(square(*pt), &white_b);
        }
    }
}
