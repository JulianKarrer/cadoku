use dioxus::prelude::*;

// SETTINGS

/// Size of the cat in pixels
pub const CAT_ASSET_PX: u32 = 300;
/// Distance in pixels required for sufficiently petting the cat
/// to get a reaction.
pub const CAT_PET_DIST: f64 = 500.;
/// Duration of the change in expression of the cat in milliseconds
pub const CAT_EXPRESSION_DURATION: u64 = 1500;
/// Duration of each frame of the firework animation when winning in milliseconds
pub const CAT_FIREWORK_DURATION: u64 = 100;
/// Number of frames of the firework animation
pub const CAT_FIREWORK_FRAMECOUNT: usize = FIREWORK.len();

/// The most crucial component of this application:
/// a cute cat that can be pet by clicking and dragging
/// and which is displays a happy reaction when the sudoku is filled out or won.
pub fn Cat() -> Element {
    let cat_state = use_context::<Signal<CatState>>();
    let mut coords: Signal<Option<(f64, f64)>> = use_signal(move || None);
    let mut dist: Signal<f64> = use_signal(move || 0.);
    // choose an asset for the cat depending on the state in the context
    let cat_asset = move || match cat_state.read().state {
        CatSprite::Normal => CAT_NORMAL,
        CatSprite::Happy => CAT_HAPPY,
        CatSprite::VeryHappy => CAT_VERY_HAPPY,
        CatSprite::EasyReaction => CAT_EASY,
        CatSprite::MediumReaction => CAT_MEDIUM,
        CatSprite::HardReaction => CAT_HARD,
        CatSprite::ChallengeReaction => CAT_CHALLENGE,
        CatSprite::Fireworks(_) => CAT_FIREWORK,
    };

    rsx!(
        div{
            // if displaying fireworks, display them over the grid
            style: if let CatSprite::Fireworks(_) = cat_state.read().state{
                "z-index: 1;"
            } else { "z-index: 0;" },
            div {
                // display fireworks if requested
                if let CatSprite::Fireworks(i) = cat_state.read().state{
                    img {
                        class: "cat",
                        style: "z-index:2;",
                        src:  FIREWORK[i]
                    }
                }
                img {
                    class:"cat",
                    style: "z-index:0;",
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
        }
    )
}

// ASSETS

const CAT_OPTIONS: ImageAssetOptions = ImageAssetOptions::new()
    .with_size(ImageSize::Manual {
        width: CAT_ASSET_PX,
        height: CAT_ASSET_PX,
    })
    .with_format(ImageFormat::Avif);
const FIREWORK_OPTIONS: ImageAssetOptions = ImageAssetOptions::new()
    .with_size(ImageSize::Manual {
        width: CAT_ASSET_PX,
        height: 2 * CAT_ASSET_PX,
    })
    .with_format(ImageFormat::Avif);
const CAT_NORMAL: Asset = asset!("assets/images/cat/mascot.png", CAT_OPTIONS);
const CAT_HEARTS: Asset = asset!("assets/images/cat/hearts.png", CAT_OPTIONS);
const CAT_HAPPY: Asset = asset!("assets/images/cat/happy.png", CAT_OPTIONS);
const CAT_VERY_HAPPY: Asset = asset!("assets/images/cat/sparkle.png", CAT_OPTIONS);
const CAT_EASY: Asset = asset!("assets/images/cat/easy.png", CAT_OPTIONS);
const CAT_MEDIUM: Asset = asset!("assets/images/cat/medium.png", CAT_OPTIONS);
const CAT_HARD: Asset = asset!("assets/images/cat/hard.png", CAT_OPTIONS);
const CAT_CHALLENGE: Asset = asset!("assets/images/cat/challenge.png", CAT_OPTIONS);
const CAT_FIREWORK: Asset = asset!("assets/images/cat/firework.png", CAT_OPTIONS);
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

// STATE DEFINITIONS

#[derive(Default, Clone, Copy, PartialEq)]
/// Current Reaction of the cat
pub enum CatSprite {
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
/// Holds the current reaction state of the cat
pub struct CatState {
    pub state: CatSprite,
}
