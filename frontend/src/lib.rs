use leptos::*;
use leptos_router::*;

#[wasm_bindgen(start)]
pub fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(move |cx| {
        provide_meta_context(cx);
        view! { cx,
          <Router>
            <Routes>
              <Route path="" view=|| view! { cx, <h1>"Semantic Search"</h1> }/>
              <Route path="*"  view=|| view! { cx, <p>"Not found"</p> }/>
            </Routes>
          </Router>
        }
    });
}