use wasm_bindgen::prelude::*;
use web_sys::{ErrorEvent, MessageEvent, WebSocket};
extern crate console_error_panic_hook;

macro_rules! console_log {
    ($($t:tt)*) => (log(&format_args!($($t)*).to_string()))
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
struct AlarmClient {
    ws: Option<WebSocket>,
    on_open: Option<Closure<dyn FnMut()>>,
    on_message: Option<Closure<dyn FnMut(MessageEvent)>>,
    on_error: Option<Closure<dyn FnMut(ErrorEvent)>>,
}

#[wasm_bindgen]
impl AlarmClient {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self {
            ws: None,
            on_open: Some(Self::on_open_default()),
            on_message: Some(Self::on_message_default()),
            on_error: Some(Self::on_error_default()),
        }
    }

    #[wasm_bindgen]
    pub fn connect(&mut self, addr: &str) -> Result<(), JsValue> {
        self.ws = Some(WebSocket::new(addr)?);
        let ws = self.ws.as_ref().unwrap();
        ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

        ws.set_onopen(Some(
            self.on_open.as_ref().unwrap().as_ref().unchecked_ref(),
        ));
        self.on_open.take().unwrap().forget();

        ws.set_onmessage(Some(
            self.on_message.as_ref().unwrap().as_ref().unchecked_ref(),
        ));
        self.on_message.take().unwrap().forget();

        Ok(())
    }

    #[wasm_bindgen]
    pub fn set_onopen(&mut self, cb: js_sys::Function) {
        let onopen_callback = Closure::<dyn FnMut()>::new(move || {
            console_log!("socket opened");

            let this = JsValue::null();
            let _ = cb.call0(&this);
        });

        if let Some(ws) = self.ws.as_ref() {
            ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
            onopen_callback.forget();
        } else {
            self.on_open = Some(onopen_callback);
        }
    }

    #[wasm_bindgen]
    pub fn set_onmessage(&mut self, cb: js_sys::Function) {
        let this = JsValue::null();

        let onmessage_callback = Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            // Handle difference Text/Binary,...
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!("message event, received Text: {:?}", txt);
                let _ = cb.call1(&this, &txt);
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
                let _ = cb.call1(&this, &e.data());
            }
        });

        if let Some(ws) = self.ws.as_ref() {
            ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
            onmessage_callback.forget();
        } else {
            self.on_message = Some(onmessage_callback);
        }
    }

    #[wasm_bindgen]
    pub fn set_onerror(&mut self, cb: js_sys::Function) {
        let onerror_callback = Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!("error event: {:?}", e);
            let this = JsValue::null();
            let _ = cb.call1(&this, &e);
        });

        if let Some(ws) = self.ws.as_ref() {
            ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
            onerror_callback.forget();
        } else {
            self.on_error = Some(onerror_callback);
        }
    }

    fn on_open_default() -> Closure<dyn FnMut()> {
        Closure::<dyn FnMut()>::new(move || {
            console_log!("socket opened");
        })
    }

    fn on_message_default() -> Closure<dyn FnMut(MessageEvent)> {
        Closure::<dyn FnMut(_)>::new(move |e: MessageEvent| {
            if let Ok(txt) = e.data().dyn_into::<js_sys::JsString>() {
                console_log!("message event, received Text: {:?}", txt);
            } else {
                console_log!("message event, received Unknown: {:?}", e.data());
            }
        })
    }

    fn on_error_default() -> Closure<dyn FnMut(ErrorEvent)> {
        Closure::<dyn FnMut(_)>::new(move |e: ErrorEvent| {
            console_log!("error event: {:?}", e);
        })
    }
}

#[wasm_bindgen]
pub fn hello(msg: &str) -> String {
    std::format!("Hello World: {msg}")
}
