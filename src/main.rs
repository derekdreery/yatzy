use anyhow::Error;
use crossbeam_channel as channel;
use druid::widget::{Align, Button, Flex, Label, TextBox};
use druid::{
    lens::Field, AppDelegate, AppLauncher, BoxConstraints, Color, Command, Data, DelegateCtx, Env,
    Event, EventCtx, LayoutCtx, Lens, LifeCycle, LifeCycleCtx, LocalizedString, PaintCtx, Rect,
    RenderContext, Selector, Size, Target, TimerToken, UpdateCtx, Widget, WidgetExt, WindowDesc,
    WindowId,
};
use match_derive::Matcher;
use rand::prelude::*;
use std::{convert::TryFrom, thread, time::Duration};

mod die;

use die::{Die, DieData, Score};

type Result<T = (), E = Error> = std::result::Result<T, E>;

const VERTICAL_WIDGET_SPACING: f64 = 20.0;
const LABEL_SPACING: f64 = 4.0;
const TEXT_BOX_WIDTH: f64 = 200.0;
const WINDOW_TITLE: LocalizedString<YatzyState> = LocalizedString::new("Yatzy!");
const ROLL: Selector<()> = Selector::new("die.roll");
const STOP_ROLL: Selector<Score> = Selector::new("die.stop-roll");
const START_GAME: Selector<()> = Selector::new("start-game");

#[derive(Debug, Clone, Data, Matcher)]
#[matcher(matcher_name = Yatzy)]
enum YatzyState {
    Starting(StartingState),
    InGame(InGameState),
}

impl YatzyState {
    fn start_game(&mut self) {
        let d = DieData::new(6);
        match self {
            YatzyState::Starting(state) => {
                *self = YatzyState::InGame(InGameState {
                    player_name: state.player_name.clone(),
                    dice: [d, d, d, d, d],
                })
            }
            YatzyState::InGame(state) => panic!("starting a new game when already in game"),
        }
    }
}

#[derive(Debug, Clone, Data, Lens)]
struct StartingState {
    player_name: String,
}

#[derive(Debug, Clone, Data, Lens)]
struct InGameState {
    player_name: String,
    dice: [DieData; 5],
}

pub fn main() -> Result {
    // describe the main window
    let main_window = WindowDesc::new(|| {
        YatzyState::matcher()
            .starting(build_starting())
            .in_game(build_in_game())
    })
    .title(WINDOW_TITLE)
    .window_size((400.0, 400.0));

    // create the initial app state
    let initial_state = YatzyState::Starting(StartingState {
        player_name: "".into(),
    });

    // setup die rolling periodically
    let launcher = AppLauncher::with_window(main_window);
    let sink = launcher.get_external_handle();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(1_000));
        sink.submit_command(ROLL, (), None).unwrap();
        thread::sleep(Duration::from_millis(1_000));
        sink.submit_command(STOP_ROLL, Score::random_die(), None)
            .unwrap();
    });

    // start the application
    launcher.delegate(Delegate).launch(initial_state)?;
    Ok(())
}

struct Delegate;

impl AppDelegate<YatzyState> for Delegate {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx,
        target: Target,
        cmd: &Command,
        data: &mut YatzyState,
        env: &Env,
    ) -> bool {
        if cmd.is(ROLL) {
            if let YatzyState::InGame(data) = data {
                data.dice[0].set_rolling();
            }
            false
        } else if cmd.is(START_GAME) {
            data.start_game();
            false
        } else if let Some(score) = cmd.get(STOP_ROLL) {
            if let YatzyState::InGame(data) = data {
                data.dice[0].set_value(*score);
            }
            false
        } else {
            true
        }
    }
}

fn build_starting() -> impl Widget<StartingState> {
    // a label that will determine its text based on the current app data.
    let label = Label::new("Player name:");

    // a textbox that modifies `name`.
    let textbox = TextBox::new()
        .with_placeholder("e.g. Joe Bloggs")
        .fix_width(TEXT_BOX_WIDTH)
        .lens(StartingState::player_name);

    let start_game_btn =
        Button::new("Start game!").on_click(|ctx, _data: &mut StartingState, _env| {
            ctx.submit_command(START_GAME, None);
        });

    // arrange the two widgets vertically, with some padding
    let layout = Flex::column()
        .with_child(
            Flex::row()
                .with_child(label)
                .with_spacer(LABEL_SPACING)
                .with_child(textbox),
        )
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(start_game_btn);

    // center the two widgets in the available space
    Align::centered(layout)
}

fn build_in_game() -> impl Widget<InGameState> {
    // a label that will determine its text based on the current app data.
    let player_name =
        Label::new(|data: &InGameState, _env: &Env| format!("Player: {}", data.player_name));

    macro_rules! die_lens {
        ($idx:expr) => {
            Field::new::<InGameState, _>(|s| &s.dice[$idx], |s| &mut s.dice[$idx])
        };
    }
    let dice = Flex::row()
        .with_child(Die::new().lens(die_lens!(0)))
        .with_spacer(LABEL_SPACING)
        .with_child(Die::new().lens(die_lens!(1)))
        .with_spacer(LABEL_SPACING)
        .with_child(Die::new().lens(die_lens!(2)))
        .with_spacer(LABEL_SPACING)
        .with_child(Die::new().lens(die_lens!(3)))
        .with_spacer(LABEL_SPACING)
        .with_child(Die::new().lens(die_lens!(4)));

    // arrange the two widgets vertically, with some padding
    let layout = Flex::column()
        .with_child(player_name)
        .with_spacer(VERTICAL_WIDGET_SPACING)
        .with_child(dice);

    // center the two widgets in the available space
    Align::centered(layout)
}
