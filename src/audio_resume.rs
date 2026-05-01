use bevy::prelude::*;

pub struct AudioResumePlugin;

impl Plugin for AudioResumePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            app.insert_resource(UserInteractionState { interacted: false });
            app.add_systems(Update, check_user_interaction);
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            app.insert_resource(UserInteractionState { interacted: true });
        }
    }
}

#[derive(Resource)]
pub struct UserInteractionState {
    pub interacted: bool,
}

#[cfg(target_arch = "wasm32")]
fn check_user_interaction(mut state: ResMut<UserInteractionState>) {
    if state.interacted {
        return;
    }
    let window = web_sys::window().expect("no global window");
    let val = js_sys::Reflect::get(
        &window,
        &wasm_bindgen::JsValue::from_str("__user_has_interacted"),
    )
    .unwrap_or(wasm_bindgen::JsValue::FALSE);
    if val.as_bool().unwrap_or(false) {
        state.interacted = true;
    }
}
