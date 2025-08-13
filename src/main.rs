#![allow(non_snake_case)]
use std::time::Duration;

use dioxus::{prelude::*};
use dioxus_sdk::utils::timing::use_debounce;
use crate::sudoku::{generate_trivial, Sudoku};
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


// ASSETS

static CSS: Asset = asset!("assets/main.css");
const CAT_OPTIONS: ImageAssetOptions = ImageAssetOptions::new()
    .with_size(ImageSize::Manual { width: CAT_ASSET_PX, height: CAT_ASSET_PX })
    .with_format(ImageFormat::Avif);
const CAT_NORMAL: Asset = asset!( "assets/images/cat/mascot.png", CAT_OPTIONS.with_preload(true) );
const CAT_HEARTS: Asset = asset!( "assets/images/cat/hearts.png", CAT_OPTIONS );
const CAT_HAPPY:  Asset = asset!( "assets/images/cat/happy.png", CAT_OPTIONS );
const CAT_VERY_HAPPY:  Asset = asset!( "assets/images/cat/sparkle.png", CAT_OPTIONS );

// FUNCITONALITY

fn main() {
    dioxus::launch(app);
}

#[derive(Default, Clone, Copy, PartialEq)]
enum State{
    #[default]
    Normal,
    Happy,
    VeryHappy,
}
#[derive(Clone, Copy, Default)]
struct CatState{
    state: State
}

fn app() -> Element {
    // generate sudoku
    let (sudoku, solution) = generate_trivial(25);
    let mut focused = use_signal(move || false);
    use_context_provider(|| Signal::new(CatState::default()));
    rsx! (
        document::Link { rel: "icon", href: asset!("assets/favicon.ico") }
        document::Stylesheet { href: CSS },
        FontFace { family: "Mooli", style: "normal", weight: 400, 
            asset: asset!("/assets/fonts/Mooli.ttf") 
        },
        div { class: "container",
            div{
                class: "cntr",
                h1 { "Cadoku!" }
            },
            div { 
                onfocusin: move |_| {*focused.write() = true},
                onfocusout: move |_| {*focused.write() = false},
                Sudoku {
                    sudoku, 
                    solution, 
                    focused,
                },
            },
            div{
                Cat { }
            },
        }
    )
}

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
        }
    };

    rsx!(
        div {  
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

#[derive(PartialEq, Props, Clone)]
struct SudokuProps {
    sudoku: Sudoku,
    solution: [u8; 81],
    focused: Signal<bool>,
}

fn Sudoku(props:SudokuProps) -> Element {
    let mut cat_state = use_context::<Signal<CatState>>();
    let mut cat_reset = use_debounce(
        Duration::from_millis(CAT_EXPRESSION_DURATION), 
        move |_| cat_state.write().state = State::default()
    );
    let mut board  = use_signal(move || props.sudoku.clone());
    let mut focus = use_signal(move || None);

    // handle the entry of value `val` at coordinates `x`,`y`, including 
    // - checking against the solution
    // - updating the board state
    // - triggering an animation update of the cat
    let mut check_entry = move |x,y,val|{
        let i = x + 9*y;
        // if the input is accordance with the solution, set the square
        if board.read().is_zero(x,y) && val == props.solution[i]{
            let units_correct = board.read().count_completed_units();
            board.write().set(i, val);
            // on successful entry, trigger a sprite change of the cat
            let one_more_unit_done = board.read().count_completed_units() > units_correct;
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
            use_effect(move ||{*focus.write() = None;});
        };
    };

    rsx! (
        div { class: "btm",
        // onkeypress: move |e| {
        //     // move focus using arrow keys
        //     if *props.focused.read(){
        //         let mut updated = None;
        //         debug!("hi");
        //         if let Some((x,y)) = *focus.read() {
        //             match e.code() {
        //                 // add 8 to subtract 1 in mod 9
        //                 Code::ArrowDown  => {updated = Some((x, (y+1)%9))},
        //                 Code::ArrowLeft  => {updated = Some(((x+8)%9, y))},
        //                 Code::ArrowRight => {updated = Some(((x+1)%9, y))},
        //                 Code::ArrowUp    => {updated = Some((x, (y+8)%9))},
        //                 _ => {},
        //             };
        //         }
        //         if let Some(_) = updated{
        //             *focus.write() = updated;
        //         }
        //     } 
        // },
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
                                    class: if let Some((x_f, y_f)) = *focus.read() {
                                        if *props.focused.read() && ((3*gx+x) == x_f && (3*gy+y) == y_f) {
                                            "emptysquare strongly-highlighted"
                                        } else if *props.focused.read() && ((3*gx+x) == x_f || (3*gy+y) == y_f) {
                                            "emptysquare highlighted"
                                        } else {
                                            "emptysquare"
                                        } 
                                    } else {
                                        "emptysquare"
                                    },
                                    onkeydown: move |e| {
                                        // input by keystroke
                                        let val = match e.code() {
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
                                        check_entry(3*gx+x, 3*gy+y, val);
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
                                        "square highlighted"
                                        } else {"square"} 
                                    } else {"square"},
                                    "{props.solution[3*gx+x + 9*(3*gy+y)]}" },
                            },
                            span { 
                                class: "secret-hacker-hint", 
                                "{props.solution[3*gx+x + 9*(3*gy+y)]}",
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