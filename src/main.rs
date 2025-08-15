#![allow(non_snake_case)]
use std::time::Duration;

use dioxus::{logger::tracing::debug, prelude::*};
use dioxus_sdk::{storage::use_persistent, utils::timing::{use_debounce, use_interval}};
use crate::sudoku::{generate_subtractive, Sudoku};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumIter};
mod constants;
mod sudoku;


// SETTINGS

/// Size of the cat in pixels
const CAT_ASSET_PX: u32 = 300;
/// Distance in pixels required for sufficiently petting the cat
/// to get a reaction.
const CAT_PET_DIST:f64 = 500.;
/// Duration of the change in expression of the cat in milliseconds
const CAT_EXPRESSION_DURATION:u64 = 1500; 
/// Duration of each frame of the firework animation when winning in milliseconds
const CAT_FIREWORK_DURATION:u64 = 100; 

/// Number of hints for each difficulty
impl Difficulty{
    fn hints(&self)->usize{
        match self{
            Difficulty::Easy => 60,
            Difficulty::Medium => 45,
            Difficulty::Hard => 30,
            Difficulty::Challenge => 22,
        }
    }
}


// ASSETS

static CSS: Asset = asset!("assets/main.css");
const CAT_OPTIONS: ImageAssetOptions = ImageAssetOptions::new()
    .with_size(ImageSize::Manual { width: CAT_ASSET_PX, height: CAT_ASSET_PX })
    .with_format(ImageFormat::Avif);
const FIREWORK_OPTIONS: ImageAssetOptions = ImageAssetOptions::new()
    .with_size(ImageSize::Manual { width: CAT_ASSET_PX, height: 2*CAT_ASSET_PX })
    .with_format(ImageFormat::Avif);
const CAT_NORMAL: Asset = asset!( "assets/images/cat/mascot.png", CAT_OPTIONS.with_preload(true) );
const CAT_HEARTS:Asset = asset!( "assets/images/cat/hearts.png", CAT_OPTIONS );
const CAT_HAPPY: Asset = asset!( "assets/images/cat/happy.png", CAT_OPTIONS );
const CAT_VERY_HAPPY: Asset = asset!( "assets/images/cat/sparkle.png", CAT_OPTIONS );
const CAT_EASY: Asset = asset!( "assets/images/cat/easy.png", CAT_OPTIONS );
const CAT_MEDIUM: Asset = asset!( "assets/images/cat/medium.png", CAT_OPTIONS );
const CAT_HARD: Asset = asset!( "assets/images/cat/hard.png", CAT_OPTIONS );
const CAT_CHALLENGE: Asset = asset!( "assets/images/cat/challenge.png", CAT_OPTIONS );
const CAT_FIREWORK: Asset = asset!( "assets/images/cat/firework.png", CAT_OPTIONS );
const FIREWORK: [Asset; 10] = [
    asset!("assets/images/fireworks/0.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/1.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/2.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/3.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/4.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/5.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/6.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/7.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/8.png", FIREWORK_OPTIONS),
    asset!("assets/images/fireworks/9.png", FIREWORK_OPTIONS),
];

// FUNCITONALITY


fn main() {
    dioxus::launch(app);
}


fn app() -> Element {
    // generate sudoku
    let (sudoku, solution) = (Sudoku::empty(), [0u8;81]);
    let mut sudoku  = use_persistent("sudoku",move || sudoku);
    let mut solution  = use_persistent("solution",move || solution.to_vec());
    let mut focused = use_signal(move || false);
    let mut playing = use_persistent("playing",move || false);
    use_context_provider(|| Signal::new(CatState::default()));
    let mut cat_state = use_context::<Signal<CatState>>();
    let mut difficulty = use_signal(move || None);

    // define behaviour when quit button is pressed
    let on_quit = Callback::new(move |_| {
        *playing.write() = false;
        *difficulty.write() = None;
        *cat_state.write() = CatState::default();
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
            // header: title
            div{
                class: "cntr",
                h1 { "Cadoku!" }
            },
            if *playing.read(){
                // main game
                div { 
                    onfocusin: move |_| {*focused.write() = true},
                    onfocusout: move |_| {*focused.write() = false},
                    Sudoku { sudoku, solution, focused, on_quit, },
                },
            } else{
                // menu
                div { 
                    class: "cntr",
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
            div{
                Cat { }
            },
        }
    )
}


#[derive(PartialEq, Props, Clone)]
struct SudokuProps {
    sudoku: Signal<Sudoku>,
    solution: Signal<Vec<u8>>,
    focused: Signal<bool>,
    on_quit: EventHandler<()>,
}

fn Sudoku(props:SudokuProps) -> Element {
    let mut board = props.sudoku;
    let mut cat_state = use_context::<Signal<CatState>>();
    let mut cat_reset = use_debounce(
        Duration::from_millis(CAT_EXPRESSION_DURATION), 
        move |_| cat_state.write().state = State::default()
    );
    let mut focus = use_signal(move || None);

    // handle the entry of value `val` at coordinates `x`,`y`, including 
    // - checking against the solution
    // - updating the board state
    // - triggering an animation update of the cat
    let mut check_entry = move |x,y,val|{
        let i = x + 9*y;
        // if the input is accordance with the solution, set the square
        if board.read().is_zero(x,y) && val == props.solution.read()[i]{
            let units_correct = board.read().count_filled_units();
            board.write().set(i, val);
            // reset focus
            use_effect(move ||{*focus.write() = None;});
            // check win condition
            if board.read().filled(){
                // game has been won!
                cat_state.write().state = State::Fireworks(0);
                let _cat_firework_animation = use_interval(Duration::from_millis(CAT_FIREWORK_DURATION), move || {
                    let state = cat_state.read().state;
                    if let State::Fireworks(i) = state {
                        cat_state.write().state = State::Fireworks((i+1)%FIREWORK.len());
                    }
                });
                // if the animation should be interrupted at any point, insert a .cancel()
                // here
                return;
            }
            // on successful entry, trigger a sprite change of the cat
            let one_more_unit_done = board.read().count_filled_units() > units_correct;
            cat_state.write().state = if one_more_unit_done {
                // a new unit was completed
                // => cat is extra happy
                State::VeryHappy
            } else {
                // some square was completed
                // => cat is moderately happy
                State::Happy
            };
            // return the cat to its normal state after a set duration
            cat_reset.action(());
        };
    };
    let mut keyboard_check = move|e_code: Code, x:usize, y:usize|{
        // input by keystroke
        let val = match e_code {
            Code::Digit1 | Code::Numpad1 => 1u8,
            Code::Digit2 | Code::Numpad2 => 2u8,
            Code::Digit3 | Code::Numpad3 => 3u8,
            Code::Digit4 | Code::Numpad4 => 4u8,
            Code::Digit5 | Code::Numpad5 => 5u8,
            Code::Digit6 | Code::Numpad6 => 6u8,
            Code::Digit7 | Code::Numpad7 => 7u8,
            Code::Digit8 | Code::Numpad8 => 8u8,
            Code::Digit9 | Code::Numpad9 => 9u8,
            _ => {0u8}
        };
        check_entry(x, y, val);
    };
    let mut elements : Signal<[Option<std::rc::Rc<MountedData>>; 81]> = use_signal(|| [const { None }; 81]);

    rsx! (
        div { class: "btm",
        onkeydown: move |e| async move {
            // move focus using arrow keys
            if *props.focused.read(){
                let mut updated = None;
                if let Some((x,y)) = *focus.read() {
                    debug!("{:?}", e.code());
                    match e.code() {
                        // add 8 to subtract 1 in mod 9
                        Code::ArrowDown  => {updated = Some((x, (y+1)%9))},
                        Code::ArrowLeft  => {updated = Some(((x+8)%9, y))},
                        Code::ArrowRight => {updated = Some(((x+1)%9, y))},
                        Code::ArrowUp    => {updated = Some((x, (y+8)%9))},
                        _ => {},
                    };
                }
                if let Some(_) = updated{
                    // loose focus
                    if let Some((x,y)) = *focus.read(){
                        for elem in elements.read().iter(){
                            if let Some(elem) = elem.as_ref() {
                                elem.set_focus(false).await.unwrap();
                            }
                        }
                        debug!("{:?} {} {} ",elements, x,y);
                    }
                   
                    *focus.write() = updated;
                }
                if let Some((x,y)) = *focus.read() {
                    keyboard_check(e.code(),x,y);
                }
            } 
        },
        button { 
            class: "exit-btn",
            onclick: move |_| {
                props.on_quit.call(());
            },
            "Quit" 
        },
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
                                input { 
                                    onmounted: move |el| (*elements.write())[3*gx+x+9*(3*gy+y)] = Some(el.data()),
                                    class: if let Some((x_f, y_f)) = *focus.read() {
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
                                    onkeydown: move |e| {
                                        keyboard_check(e.code(), 3*gx+x, 3*gy+y);
                                        e.prevent_default();
                                    },
                                    // note which square is currently focused
                                    onfocusin: move |_|{ *focus.write() = Some((3*gx+x,3*gy+y));},
                                }
                            } else {
                                // if the square is not empty, show the number in it
                                span { 
                                    class: if let Some((x_f, y_f)) = *focus.read(){
                                        if *props.focused.read() && ((3*gx+x) == x_f || (3*gy+y) == y_f) {
                                        "square focused"
                                        } else {"square"} 
                                    } else {"square"},
                                    "{props.solution.read()[3*gx+x + 9*(3*gy+y)]}" },
                            },
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
        // alternative input: buttons that enter at the currently focused cell
        div { 
            class: "button-container",
            for val in 1..=9{
                button { 
                    class: "num-button", 
                    onclick: move |_| {
                        if *props.focused.read(){
                            if let Some((x, y)) = *focus.read(){
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

/// The most crucial component of this application: 
/// a cute cat that can be pet by clicking and dragging
/// and which is displays a happy reaction when the sudoku is filled out.
fn Cat() -> Element {
    let cat_state = use_context::<Signal<CatState>>();
    let mut coords: Signal<Option<(f64, f64)>> = use_signal(move || None);
    let mut dist: Signal<f64> = use_signal(move || 0.);
    // choose an asset for the cat depending on the state in the context
    let cat_asset = move ||{
        match cat_state.read().state{
            State::Normal => CAT_NORMAL,
            State::Happy => CAT_HAPPY,
            State::VeryHappy => CAT_VERY_HAPPY,
            State::EasyReaction => CAT_EASY,
            State::MediumReaction => CAT_MEDIUM,
            State::HardReaction => CAT_HARD,
            State::ChallengeReaction => CAT_CHALLENGE,
            State::Fireworks(_) => CAT_FIREWORK,
        }
    };
    
    rsx!(
        div {  
            if let State::Fireworks(i) = cat_state.read().state{
                img { 
                    class: "cat", 
                    style: "z-index:2;", 
                    src:  FIREWORK[i] 
                }
            }
            img { 
                class:"cat", 
                // if the cat was sufficiently pet, display love for given duration
                src: if *dist.read() > CAT_PET_DIST {CAT_HEARTS} else {cat_asset()}, 
                draggable: false,
                onpointerleave: move |_|{ *coords.write() = None; *dist.write() = 0.; },
                onpointerup: move |_|{ *coords.write() = None },
                onpointerdown: move |e|{ 
                    *coords.write() = Some((e.element_coordinates().x, e.element_coordinates().y)) 
                },
                onpointermove: move|e|{
                    // track the movement of the pointer while clicked to accumulate distance
                    let mut update = false;
                    if let Some((px, py)) = *coords.read() {
                        update = true;
                        let dx = e.element_coordinates().x - px;
                        let dy = e.element_coordinates().y - py;
                        dist += (dx*dx+dy*dy).sqrt();
                    }
                    if update{
                        *coords.write() = Some((e.element_coordinates().x, e.element_coordinates().y));
                    }
                }
            }
        }
    )
}


// State Definitions

#[derive(Default, Clone, Copy, PartialEq)]
enum State{
    #[default]
    Normal,
    Happy,
    VeryHappy,
    EasyReaction,
    MediumReaction,
    HardReaction,
    ChallengeReaction,
    Fireworks(usize),
}
#[derive(Clone, Copy, Default)]
struct CatState{
    state: State
}
#[derive(Default, EnumIter, Display, Copy, Clone, PartialEq)]
enum Difficulty{
    #[default]
    Easy,
    Medium,
    Hard,
    Challenge,
}
impl Difficulty{
    /// Get the `CatState` that illustrates the reaction to the given difficulty level.
    fn cat_state(&self)->CatState{
        match self{
            Difficulty::Easy => CatState { state: State::EasyReaction },
            Difficulty::Medium => CatState { state: State::MediumReaction },
            Difficulty::Hard => CatState { state: State::HardReaction },
            Difficulty::Challenge => CatState { state: State::ChallengeReaction },
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