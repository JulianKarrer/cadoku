use dioxus::prelude::*;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    rsx! (
        div { style: "text-align: center;",
            h1 { "CADOKU" }
        }
    )
}
