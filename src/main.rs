#![allow(non_snake_case)]
use std::time::Duration;

use crate::{
    cat::{
        Cat, CatSprite, CatState, CAT_EXPRESSION_DURATION, CAT_FIREWORK_DURATION,
        CAT_FIREWORK_FRAMECOUNT,
    },
    sudoku::{generate_subtractive, Sudoku},
};
use dioxus::prelude::*;
use dioxus_sdk::{
    storage::use_persistent,
    utils::timing::{use_debounce, use_interval},
};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
mod cat;
mod constants;
mod sudoku;

// SETTINGS

/// Number of hints for each difficulty
impl Difficulty {
    fn hints(&self) -> usize {
        match self {
            Difficulty::Easy => 60,
            Difficulty::Medium => 45,
            Difficulty::Hard => 30,
            Difficulty::Challenge => 22,
        }
    }
}

// ASSETS

static CSS: Asset = asset!("assets/main.css", CssAssetOptions::new().with_preload(true));

// FUNCITONALITY

/// Main entry point of the application, containing only:
///```
/// dioxus::launch(app);
/// ```
fn main() {
    dioxus::launch(app);
}

/// Outermost component in the tree that manages game state (with persistance) as well as menu logic, either showing a menu for difficulty selection or the [`fn::Sudoku`] component, with the [`Cat`] component below it.
fn app() -> Element {
    // containers and signal definitions
    let (sudoku, solution) = (Sudoku::empty(), [0u8; 81]);
    // use persistent storage for sudoku and solution, such that reloads don't revert progress
    let mut sudoku = use_persistent("sudoku", move || sudoku);
    let mut solution = use_persistent("solution", move || solution.to_vec());
    // whether the sudoku grid is currently focused, which is unset if any other area is clicked
    let mut focused = use_signal(move || true);
    // whether a game is currently played or nor. Toggles the menu and game screens respectively
    let mut playing = use_persistent("playing", move || false);
    use_context_provider(|| Signal::new(CatState::default()));
    // current state (i.e. sprite) of the cat
    let mut cat_state = use_context::<Signal<CatState>>();
    // currently selected game difficulty in the menu
    let mut difficulty = use_signal(move || None);
    // singal saving the key code of the last pressed key and triggering input handlers
    // via `use_effect` hooks
    let mut key_pressed = use_signal(move || None);

    // define behaviour when quit button is pressed
    let on_quit = Callback::new(move |_| {
        *playing.write() = false;
        *difficulty.write() = None;
        *cat_state.write() = CatState::default();
    });
    // reset if already won on load (if persistent data is solution)
    use_effect(move || {
        if sudoku.peek().filled() {
            on_quit(());
        }
    });

    rsx! (
        // imports, stylesheets and font declarations
        document::Link { rel: "icon", href: asset!("assets/favicon.ico") }
        document::Stylesheet { href: CSS },
        FontFace { family: "Mooli", style: "normal", weight: 400,
            asset: asset!("/assets/fonts/Mooli.ttf")
        },
        // start of RSX content
        div { class: "container",
            onclick: move |_| {
                focused.set(false) ; },
            onkeydown: move |e| {*key_pressed.write() = Some(e.code());
            },
            // header: title and quit
            div{
                class: "cntr",
                h1 { "Cadoku!" },
                button {
                    class: "exit-btn",
                    style:  if !*playing.read() {"opacity: 0; cursor: auto;"} else {""},
                    onclick: move |_| { if *playing.read() { on_quit.call(()); }},
                    "Quit"
                },
            },
            if *playing.read(){
                // main game
                div { class: "btm",
                    onclick: move |e| {if !*focused.peek(){
                        focused.set(true);
                    }; e.stop_propagation();
                },
                    Sudoku { sudoku, solution, focused, key_pressed },
                },
            } else{
                // menu
                div {
                    class: "btm",
                    for diff in Difficulty::iter(){
                        // each of the buttons for difficulty levels
                        button {
                            class: if *difficulty.read() == Some(diff) {"menu-button menu-btn-focused"} else {"menu-button"},
                            onclick: move |_|  {
                                *difficulty.write() = Some(diff);
                                *cat_state.write() = diff.cat_state();
                            },
                            "{diff}"
                        }
                    }
                    // play button
                    button {
                        class: if difficulty.read().is_some() {"menu-button"} else {"menu-button play-unfocused"},
                        onclick: move |_| async move {
                            if let Some(diff) = *difficulty.read(){
                                let (new_sudoku, new_solution) = generate_subtractive(diff.hints());
                                *solution.write() = new_solution.to_vec();
                                *sudoku.write() = new_sudoku;
                                *playing.write() = true;
                                *cat_state.write() = CatState::default();
                            }
                        },
                        "Play!"
                    }
                }
            }
            // footer: cat
            Cat { }
        }
    )
}

#[derive(PartialEq, Props, Clone)]
struct SudokuProps {
    sudoku: Signal<Sudoku>,
    solution: Signal<Vec<u8>>,
    focused: Signal<bool>,
    key_pressed: Signal<Option<Code>>,
}

/// Main component of the game: a grid displaying the sudoku cues and providing input functionality.
/// Squares can be selected by clicking or moving the cursors with arrows keys, numbers can be input at the
/// cursor location via keyboard (includig the numpad) or buttons to click below the grid.
fn Sudoku(props: SudokuProps) -> Element {
    let mut board = props.sudoku;
    let mut cat_state = use_context::<Signal<CatState>>();
    let mut cat_reset = use_debounce(Duration::from_millis(CAT_EXPRESSION_DURATION), move |_| {
        cat_state.write().state = CatSprite::default()
    });
    let mut cursor = use_signal(move || None);

    // handle focus
    use_effect(move || {
        if !*props.focused.read() {
            *cursor.write() = None;
        }
    });

    // handle the entry of value `val` at coordinates `x`,`y`, including
    // - checking against the solution
    // - updating the board state
    // - triggering an animation update of the cat
    let mut check_entry = move |x, y, val| {
        let i = x + 9 * y;
        // if the input is accordance with the solution, set the square
        if board.peek().is_zero(x, y) && val == props.solution.read()[i] {
            let units_correct = board.peek().count_filled_units();
            let mut updated = board.peek().clone();
            updated.set(i, val);
            board.set(updated);
            // // reset focus
            // use_effect(move ||{*cursor.write() = None;});
            // check win condition
            if board.peek().filled() {
                // game has been won!
                cat_state.write().state = CatSprite::Fireworks(0);
                let _cat_firework_animation =
                    use_interval(Duration::from_millis(CAT_FIREWORK_DURATION), move || {
                        let state = cat_state.read().state;
                        if let CatSprite::Fireworks(i) = state {
                            cat_state.write().state =
                                CatSprite::Fireworks((i + 1) % CAT_FIREWORK_FRAMECOUNT);
                        }
                    });
                // if the animation should be interrupted at any point, insert a .cancel()
                // here
                return;
            }
            // on successful entry, trigger a sprite change of the cat
            let one_more_unit_done = board.peek().count_filled_units() > units_correct;
            cat_state.write().state = if one_more_unit_done {
                // a new unit was completed
                // => cat is extra happy
                CatSprite::VeryHappy
            } else {
                // some square was completed
                // => cat is moderately happy
                CatSprite::Happy
            };
            // return the cat to its normal state after a set duration
            cat_reset.action(());
        };
    };

    // handle keyboard inputs
    use_effect(move || {
        // keypress should be the ONLY dependency here, use `peek` to prevent
        // subscriptions to anything but the `key_pressed` prop, which should
        // trigger re-runs of this closure
        let keypress = *props.key_pressed.read();
        if *props.focused.peek() {
            let cursor_cur = *cursor.peek();
            if let Some((x, y)) = cursor_cur {
                if let Some(code) = keypress {
                    match code {
                        // check for numbers entered
                        Code::Digit1 | Code::Numpad1 => check_entry(x, y, 1u8),
                        Code::Digit2 | Code::Numpad2 => check_entry(x, y, 2u8),
                        Code::Digit3 | Code::Numpad3 => check_entry(x, y, 3u8),
                        Code::Digit4 | Code::Numpad4 => check_entry(x, y, 4u8),
                        Code::Digit5 | Code::Numpad5 => check_entry(x, y, 5u8),
                        Code::Digit6 | Code::Numpad6 => check_entry(x, y, 6u8),
                        Code::Digit7 | Code::Numpad7 => check_entry(x, y, 7u8),
                        Code::Digit8 | Code::Numpad8 => check_entry(x, y, 8u8),
                        Code::Digit9 | Code::Numpad9 => check_entry(x, y, 9u8),
                        // check for cursor movement
                        Code::ArrowDown => cursor.set(Some((x, (y + 1) % 9))),
                        Code::ArrowLeft => cursor.set(Some(((x + 8) % 9, y))),
                        Code::ArrowRight => cursor.set(Some(((x + 1) % 9, y))),
                        Code::ArrowUp => cursor.set(Some((x, (y + 8) % 9))),
                        _ => {}
                    };
                }
            }
        }
    });

    rsx! (
        div { class: "btm",
        div { class: "grid",
            for gy in 0..3 {
            for gx in 0..3 {
                div {
                    class: "subgrid" ,
                    for y in 0..3 {
                    for x in 0..3 {
                        // extra div to hold debug hints
                        div {  style: "position: relative;",
                            if board.read().is_zero(3*gx+x,3*gy+y){
                                // if the square is empty, show an input field
                                button {
                                    // whether the square is unfocused
                                    // lightly highlighted (in same row or column as cursor)
                                    // or strongly highlighted (at the cursor)
                                    // is managed via CSS classes
                                    class: if let Some((x_f, y_f)) = *cursor.read() {
                                        if *props.focused.read() && ((3*gx+x) == x_f && (3*gy+y) == y_f) {
                                            "emptysquare strongly-focused"
                                        } else if *props.focused.read() && ((3*gx+x) == x_f || (3*gy+y) == y_f) {
                                            "emptysquare focused"
                                        } else {
                                            "emptysquare"
                                        }
                                    } else {
                                        "emptysquare"
                                    },
                                    // prevent default HTML input event, since keystrokes
                                    // are already captured in a parent div and handled by a
                                    // use_effect hook on the `key_pressed` prop
                                    onkeydown: move |e| {e.prevent_default();},
                                    // focus the targeted square on click
                                    onfocusin: move |_|{ cursor.set(Some((3*gx+x,3*gy+y)));},
                                }
                            } else {
                                // if the square is not empty, show the number in it
                                span {
                                    class: if let Some((x_f, y_f)) = *cursor.read(){
                                        if *props.focused.read() && ((3*gx+x) == x_f || (3*gy+y) == y_f) {
                                        "square focused"
                                        } else {"square"}
                                    } else {"square"},
                                    "{props.solution.read()[3*gx+x + 9*(3*gy+y)]}" },
                            },
                            // for debugging  show the solution in the dom,
                            // but don't render it visibly
                            span {
                                class: "secret-hacker-hint",
                                "{props.solution.read()[3*gx+x + 9*(3*gy+y)]}",
                            },
                        }
                    }
                    }
                }
            }
            }
        },
        // alternative input: buttons that enter at the currently focused cell, if applicable
        // this enables playing with mouse or on a touch device
        div {
            class: "button-container",
            for val in 1..=9{
                button {
                    class: "num-button",
                    onclick: move |_| {
                        if *props.focused.peek(){
                            if let Some((x, y)) = *cursor.peek(){
                                check_entry(x, y, val);
                            }
                        }
                    },
                    "{val}",
                }
            },
        }
        }
    )
}

// State Definitions

#[derive(Default, EnumIter, Display, Copy, Clone, PartialEq)]
/// Game difficulty, which translates to the number of cues given initially
enum Difficulty {
    #[default]
    Easy,
    Medium,
    Hard,
    Challenge,
}
impl Difficulty {
    /// Get the [`CatState`] that illustrates the reaction to
    /// the given difficulty level in the menu screen
    fn cat_state(&self) -> CatState {
        match self {
            Difficulty::Easy => CatState {
                state: CatSprite::EasyReaction,
            },
            Difficulty::Medium => CatState {
                state: CatSprite::MediumReaction,
            },
            Difficulty::Hard => CatState {
                state: CatSprite::HardReaction,
            },
            Difficulty::Challenge => CatState {
                state: CatSprite::ChallengeReaction,
            },
        }
    }
}

// Components

#[component]
/// `@font-face` declaration https://github.com/DioxusLabs/dioxus/discussions/3777
fn FontFace(family: &'static str, style: &'static str, weight: usize, asset: Asset) -> Element {
    rsx! { document::Style {{
            format!("
                @font-face {{
                    font-display: swap;
                    font-family: '{}';
                    font-style: {};
                    font-weight: {};
                    src: url('{}') format('truetype');
                }}
                ", family, style, weight, asset
            )
        }}
    }
}
